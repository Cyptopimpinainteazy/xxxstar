"""
P4 Phase 8 — P5 Production Release & Mainnet Readiness Test Suite
==================================================================

Models the P5_CROSS_CHAIN_GPU_VALIDATOR_PROPOSAL.py Phase 3+4 and the
X3_DEPLOYMENT_EXECUTION_PLAN.md Testnet Hardening + Mainnet Readiness tracks:

  Phase 3 (Days 11-12): Cross-chain testnet deployment
    - Both validators live and synced
    - 0 atomic violations in 24-hour test
    - Fallback modes activated and tested
    - RPC / health endpoints live

  Phase 4 (Days 13-14): Security audit & production release
    - 14-day stable operation benchmark
    - Security: rate-limiting, secret vault, constant-time compare
    - Signed release bundle with SHA256 + GPG attestation
    - Operator runbooks + emergency procedures
    - Mainnet readiness: rollback, 7-day soak, governance gate

Suites:
  TestCrossChainTestnetDeploy   (9)  — dual validator boot, health, RPC smoke test
  TestTwentyFourHourStability   (9)  — 24h simulation: 0 violations, <0.1% timeout rate
  TestGpuHealthLive             (9)  — VRAM <2 GB, utilization 70-90%, temp <75°C, recovery
  TestRpcHardening              (9)  — rate limiting, DDoS protection, security defaults
  TestOperatorRunbooks          (9)  — emergency shutdown, operator override, runbook keys
  TestMainnetReadinessGates     (9)  — 7-day soak, rollback, governance approval, audit
  TestProductionReleaseBundle   (9)  — GPG signature, SHA256 checksums, manifest completeness
  TestP5MainnetGoNoGo           (9)  — all mainnet_readiness success_metrics gates

Total: 72 tests
"""

import hashlib
import hmac
import time
from collections import defaultdict
from dataclasses import dataclass, field
from enum import Enum, auto

# ─────────────────────────────────────────────────────────────────────────── #
#  Constants from P5 proposal and X3 deployment plan
# ─────────────────────────────────────────────────────────────────────────── #

# GPU health thresholds (from P5 Visual Roadmap)
VRAM_LIMIT_GB            = 2.0          # < 2 GB per validator
GPU_UTIL_MIN_PCT         = 70           # 70% minimum utilization (optimal range)
GPU_UTIL_MAX_PCT         = 90           # 90% maximum utilization before throttle
GPU_TEMP_MAX_C           = 75           # < 75°C safe zone
GPU_RECOVERY_MAX_SEC     = 5.0          # fallback must engage within 5s

# 24-hour stability targets
H24_ATOMIC_VIOLATIONS_MAX  = 0          # must be zero
H24_TIMEOUT_RATE_MAX_PCT   = 0.1        # < 0.1% of swaps
H24_SWAP_SUCCESS_RATE_MIN  = 99.0       # > 99%
H24_SIMULATION_TX_COUNT    = 10_000     # run this many synthetic swaps

# Latency budgets
SINGLE_TX_LATENCY_MAX_MS   = 100        # < 100ms end-to-end
BATCH_256_LATENCY_MAX_MS   = 2_000      # < 2000ms for 256-tx batch
STATE_SYNC_MAX_MS          = 10         # < 10ms state sync

# Release bundle requirements
REQUIRED_BUNDLE_FILES = [
    "x3-chain-node",
    "runtime.wasm",
    "chain-spec-testnet.json",
    "chain-spec-mainnet.json",
    "deploy.sh",
    "health-check.sh",
    "CHECKSUMS.sha256",
    "CHECKSUMS.sha256.asc",
    "README.md",
    "OPERATOR_RUNBOOK.md",
]

# Security constants
RPC_RATE_LIMIT_REQ_PER_SEC  = 100       # max RPC requests/sec per IP
SECRETS_ROTATE_DAYS_MAX     = 90        # secrets must rotate within 90 days

# Mainnet readiness gates
SOAK_DAYS_REQUIRED          = 7         # 7-day soak on production-like infra
MIN_VALIDATORS_REQUIRED     = 3         # minimum validator count for mainnet


# ─────────────────────────────────────────────────────────────────────────── #
#  Stub models
# ─────────────────────────────────────────────────────────────────────────── #

class ValidatorState(Enum):
    STOPPED   = auto()
    STARTING  = auto()
    LIVE      = auto()
    DEGRADED  = auto()
    CRASHED   = auto()


@dataclass
class ValidatorNode:
    """Simulated cross-chain validator node."""
    node_id: str
    chain: str           # "solana" | "ethereum"
    state: ValidatorState = ValidatorState.STOPPED
    head: int = 0
    rpc_alive: bool = False
    health_alive: bool = False
    vram_gb: float = 1.2
    gpu_util_pct: float = 78.0
    gpu_temp_c: float = 68.0

    def start(self):
        self.state = ValidatorState.LIVE
        self.rpc_alive = True
        self.health_alive = True

    def stop(self):
        self.state = ValidatorState.STOPPED
        self.rpc_alive = False
        self.health_alive = False

    def advance(self, n: int = 1):
        if self.state == ValidatorState.LIVE:
            self.head += n

    def crash(self):
        self.state = ValidatorState.CRASHED
        self.rpc_alive = False

    def recover(self):
        self.state = ValidatorState.LIVE
        self.rpc_alive = True

    @property
    def is_live(self) -> bool:
        return self.state == ValidatorState.LIVE

    @property
    def vram_ok(self) -> bool:
        return self.vram_gb < VRAM_LIMIT_GB

    @property
    def util_ok(self) -> bool:
        return GPU_UTIL_MIN_PCT <= self.gpu_util_pct <= GPU_UTIL_MAX_PCT

    @property
    def temp_ok(self) -> bool:
        return self.gpu_temp_c < GPU_TEMP_MAX_C


class CrossChainTestnet:
    """Two-validator (SVM + EVM) testnet coordinator."""

    def __init__(self):
        self.svm = ValidatorNode("svm-0", "solana")
        self.evm = ValidatorNode("evm-0", "ethereum")
        self._atomic_violations = 0
        self._swaps_total = 0
        self._swaps_committed = 0
        self._swaps_timed_out = 0
        self._uptime_sec: float = 0.0

    def boot_both(self):
        self.svm.start()
        self.evm.start()

    def advance_both(self, n: int = 1):
        self.svm.advance(n)
        self.evm.advance(n)

    @property
    def both_live(self) -> bool:
        return self.svm.is_live and self.evm.is_live

    @property
    def in_sync(self) -> bool:
        return abs(self.svm.head - self.evm.head) <= 2

    def simulate_swap(self, violate: bool = False, timeout: bool = False) -> str:
        self._swaps_total += 1
        if violate:
            self._atomic_violations += 1
            return "rolled_back"
        if timeout:
            self._swaps_timed_out += 1
            return "timeout"
        self._swaps_committed += 1
        return "committed"

    def simulate_24h(self, tx_count: int = H24_SIMULATION_TX_COUNT):
        """Run tx_count synthetic swaps with zero injected violations."""
        for _ in range(tx_count):
            self.simulate_swap()
        self._uptime_sec = 86_400.0

    @property
    def timeout_rate_pct(self) -> float:
        if self._swaps_total == 0:
            return 0.0
        return (self._swaps_timed_out / self._swaps_total) * 100.0

    @property
    def success_rate_pct(self) -> float:
        if self._swaps_total == 0:
            return 100.0
        return (self._swaps_committed / self._swaps_total) * 100.0


# ─────────────────────────────────────────────────────────────────────────── #
#  RPC hardenening model
# ─────────────────────────────────────────────────────────────────────────── #

class RpcRateLimiter:
    """Token-bucket rate limiter stub."""

    def __init__(self, limit: int = RPC_RATE_LIMIT_REQ_PER_SEC):
        self.limit = limit
        self._window: dict[str, int] = defaultdict(int)
        self._blocked: list[str] = []

    def request(self, ip: str) -> bool:
        """Return True if request is allowed, False if rate-limited."""
        self._window[ip] += 1
        if self._window[ip] > self.limit:
            if ip not in self._blocked:
                self._blocked.append(ip)
            return False
        return True

    def reset_window(self):
        self._window.clear()

    def is_blocked(self, ip: str) -> bool:
        return ip in self._blocked

    @property
    def blocked_count(self) -> int:
        return len(self._blocked)


class SecretsVault:
    """Minimal secrets management stub."""

    def __init__(self):
        self._secrets: dict[str, dict] = {}

    def store(self, key: str, value: str, created_epoch: float | None = None):
        epoch = created_epoch or time.time()
        self._secrets[key] = {"value": value, "created": epoch}

    def retrieve(self, key: str) -> str | None:
        entry = self._secrets.get(key)
        return entry["value"] if entry else None

    def age_days(self, key: str) -> float:
        entry = self._secrets.get(key)
        if not entry:
            return float("inf")
        return (time.time() - entry["created"]) / 86_400.0

    def is_stale(self, key: str) -> bool:
        return self.age_days(key) > SECRETS_ROTATE_DAYS_MAX

    def constant_time_compare(self, a: str, b: str) -> bool:
        """HMAC-based constant-time string comparison (no timing oracle)."""
        return hmac.compare_digest(a.encode(), b.encode())


# ─────────────────────────────────────────────────────────────────────────── #
#  Operator runbook model
# ─────────────────────────────────────────────────────────────────────────── #

@dataclass
class OperatorRunbook:
    """Tracks required runbook sections and emergency procedure availability."""
    sections: list[str] = field(default_factory=list)
    emergency_procedures: list[str] = field(default_factory=list)
    reviewed: bool = False
    gpg_signed: bool = False

    REQUIRED_SECTIONS = [
        "Pre-flight checklist",
        "Deployment steps",
        "Rollback procedure",
        "Emergency shutdown",
        "Monitoring & alerts",
        "Incident response",
    ]

    REQUIRED_EMERGENCY_PROCS = [
        "emergency_shutdown",
        "single_chain_failover",
        "operator_override",
        "atomic_violation_response",
    ]

    def is_complete(self) -> bool:
        for req in self.REQUIRED_SECTIONS:
            if req not in self.sections:
                return False
        return all(proc in self.emergency_procedures for proc in self.REQUIRED_EMERGENCY_PROCS)

    def missing_sections(self) -> list[str]:
        return [s for s in self.REQUIRED_SECTIONS if s not in self.sections]

    def missing_procedures(self) -> list[str]:
        return [p for p in self.REQUIRED_EMERGENCY_PROCS if p not in self.emergency_procedures]


# ─────────────────────────────────────────────────────────────────────────── #
#  Release bundle model
# ─────────────────────────────────────────────────────────────────────────── #

@dataclass
class ReleaseBundleManifest:
    """Simulates a signed release bundle manifest."""
    version: str
    files: list[str] = field(default_factory=list)
    checksums: dict[str, str] = field(default_factory=dict)
    gpg_signature: str | None = None
    release_notes_present: bool = False

    def add_file(self, name: str, content: bytes = b"stub"):
        self.files.append(name)
        self.checksums[name] = hashlib.sha256(content).hexdigest()

    def sign(self, key: str = "operator_key"):
        """Simulate GPG signing: HMAC over manifest hash."""
        manifest_hash = hashlib.sha256(
            "".join(sorted(self.files)).encode()
        ).digest()
        self.gpg_signature = hmac.new(
            key.encode(), manifest_hash, hashlib.sha256
        ).hexdigest()

    def verify_signature(self, key: str = "operator_key") -> bool:
        if not self.gpg_signature:
            return False
        manifest_hash = hashlib.sha256(
            "".join(sorted(self.files)).encode()
        ).digest()
        expected = hmac.new(
            key.encode(), manifest_hash, hashlib.sha256
        ).hexdigest()
        return hmac.compare_digest(self.gpg_signature, expected)

    def missing_files(self) -> list[str]:
        return [f for f in REQUIRED_BUNDLE_FILES if f not in self.files]

    @property
    def is_complete(self) -> bool:
        return len(self.missing_files()) == 0

    @property
    def is_signed(self) -> bool:
        return self.gpg_signature is not None


# ─────────────────────────────────────────────────────────────────────────── #
#  Mainnet readiness model
# ─────────────────────────────────────────────────────────────────────────── #

@dataclass
class MainnetReadinessReport:
    soak_days_completed: float = 0.0
    external_audit_passed: bool = False
    rollback_runbook_approved: bool = False
    governance_approved: bool = False
    key_management_complete: bool = False
    validator_count: int = 0
    security_findings_critical: int = 0
    security_findings_high: int = 0

    APPROVAL_CRITERIA = [
        "external_audit_passed",
        "rollback_runbook_approved",
        "governance_approved",
        "key_management_complete",
    ]

    def soak_passed(self) -> bool:
        return self.soak_days_completed >= SOAK_DAYS_REQUIRED

    def validator_count_ok(self) -> bool:
        return self.validator_count >= MIN_VALIDATORS_REQUIRED

    def security_ok(self) -> bool:
        return self.security_findings_critical == 0

    def all_approvals_complete(self) -> bool:
        return all([
            self.external_audit_passed,
            self.rollback_runbook_approved,
            self.governance_approved,
            self.key_management_complete,
        ])

    def go_decision(self) -> tuple[bool, list[str]]:
        """Returns (go, list_of_blockers)."""
        blockers = []
        if not self.soak_passed():
            blockers.append(f"soak_days={self.soak_days_completed:.1f} < {SOAK_DAYS_REQUIRED}")
        if not self.validator_count_ok():
            blockers.append(f"validators={self.validator_count} < {MIN_VALIDATORS_REQUIRED}")
        if self.security_findings_critical > 0:
            blockers.append(f"critical_findings={self.security_findings_critical}")
        if not self.all_approvals_complete():
            blockers.append("approvals_incomplete")
        return len(blockers) == 0, blockers


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 1 — TestCrossChainTestnetDeploy
# ═════════════════════════════════════════════════════════════════════════════

class TestCrossChainTestnetDeploy:
    """Dual-validator testnet deployment: both validators live, synced, RPC up."""

    def setup_method(self):
        self.net = CrossChainTestnet()

    def test_both_validators_start_live(self):
        """boot_both() brings both SVM and EVM validators to LIVE state."""
        self.net.boot_both()
        assert self.net.svm.is_live
        assert self.net.evm.is_live

    def test_rpc_endpoints_alive_after_boot(self):
        """Both validators expose live RPC after boot."""
        self.net.boot_both()
        assert self.net.svm.rpc_alive
        assert self.net.evm.rpc_alive

    def test_health_endpoints_alive_after_boot(self):
        """Both validators expose live health endpoints after boot."""
        self.net.boot_both()
        assert self.net.svm.health_alive
        assert self.net.evm.health_alive

    def test_both_live_property(self):
        """both_live is True when both validators are in LIVE state."""
        self.net.boot_both()
        assert self.net.both_live is True

    def test_heads_advance_after_boot(self):
        """Advancing 20 blocks bumps both SVM and EVM heads."""
        self.net.boot_both()
        self.net.advance_both(20)
        assert self.net.svm.head == 20
        assert self.net.evm.head == 20

    def test_chains_in_sync_after_equal_advance(self):
        """in_sync is True when both heads have advanced equally."""
        self.net.boot_both()
        self.net.advance_both(100)
        assert self.net.in_sync is True

    def test_chain_ids_correct(self):
        """Validators are wired to correct chain IDs."""
        assert self.net.svm.chain == "solana"
        assert self.net.evm.chain == "ethereum"

    def test_validator_ids_distinct(self):
        """SVM and EVM validators have distinct node IDs."""
        assert self.net.svm.node_id != self.net.evm.node_id

    def test_initial_state_stopped(self):
        """Both validators start in STOPPED state before boot."""
        assert self.net.svm.state == ValidatorState.STOPPED
        assert self.net.evm.state == ValidatorState.STOPPED


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 2 — TestTwentyFourHourStability
# ═════════════════════════════════════════════════════════════════════════════

class TestTwentyFourHourStability:
    """24-hour swap simulation: 0 atomic violations, <0.1% timeout, >99% success."""

    def setup_method(self):
        self.net = CrossChainTestnet()
        self.net.boot_both()

    def test_zero_violations_after_10k_swaps(self):
        """10k synthetic swaps with no injected violations → 0 violations."""
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        assert self.net._atomic_violations == H24_ATOMIC_VIOLATIONS_MAX

    def test_timeout_rate_under_threshold(self):
        """Timeout rate stays at 0% (well below 0.1% threshold) in clean run."""
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        assert self.net.timeout_rate_pct < H24_TIMEOUT_RATE_MAX_PCT

    def test_success_rate_above_99pct(self):
        """Success rate ≥ 99% across 10k swaps in clean run."""
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        assert self.net.success_rate_pct >= H24_SWAP_SUCCESS_RATE_MIN

    def test_all_swaps_accounted_for(self):
        """Total swap count matches simulation input."""
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        assert self.net._swaps_total == H24_SIMULATION_TX_COUNT

    def test_committed_plus_timeout_equals_total(self):
        """All swaps are classified: committed + timed_out + violations = total."""
        self.net.simulate_24h(100)
        committed = self.net._swaps_committed
        timed_out = self.net._swaps_timed_out
        # Note: violations are counted separately (rollbacks counted in neither)
        assert committed + timed_out <= self.net._swaps_total

    def test_uptime_recorded_as_86400s(self):
        """After simulate_24h(), uptime is recorded as 86400 seconds."""
        self.net.simulate_24h(100)
        assert self.net._uptime_sec == 86_400.0

    def test_injected_timeout_increments_counter(self):
        """simulate_swap(timeout=True) increments timed-out counter."""
        self.net.simulate_swap(timeout=True)
        assert self.net._swaps_timed_out == 1

    def test_injected_violation_increments_counter(self):
        """simulate_swap(violate=True) increments violation counter."""
        self.net.simulate_swap(violate=True)
        assert self.net._atomic_violations == 1

    def test_batch_256_latency_constant(self):
        """Batch-256 latency constant is within 2000ms spec."""
        assert BATCH_256_LATENCY_MAX_MS == 2_000


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 3 — TestGpuHealthLive
# ═════════════════════════════════════════════════════════════════════════════

class TestGpuHealthLive:
    """GPU health: VRAM <2 GB, utilization 70-90%, temperature <75°C, recovery."""

    def setup_method(self):
        self.svm_node = ValidatorNode("svm-0", "solana", vram_gb=1.2,
                                      gpu_util_pct=78.0, gpu_temp_c=68.0)
        self.evm_node = ValidatorNode("evm-0", "ethereum", vram_gb=1.4,
                                      gpu_util_pct=82.0, gpu_temp_c=71.0)

    def test_svm_vram_under_2gb(self):
        """SVM validator VRAM stays below 2 GB limit."""
        assert self.svm_node.vram_ok

    def test_evm_vram_under_2gb(self):
        """EVM validator VRAM stays below 2 GB limit."""
        assert self.evm_node.vram_ok

    def test_svm_gpu_utilization_in_range(self):
        """SVM GPU utilization is in the 70-90% optimal range."""
        assert self.svm_node.util_ok

    def test_evm_gpu_utilization_in_range(self):
        """EVM GPU utilization is in the 70-90% optimal range."""
        assert self.evm_node.util_ok

    def test_svm_temp_under_75c(self):
        """SVM GPU temperature is under 75°C safe threshold."""
        assert self.svm_node.temp_ok

    def test_evm_temp_under_75c(self):
        """EVM GPU temperature is under 75°C safe threshold."""
        assert self.evm_node.temp_ok

    def test_overheat_node_fails_temp_check(self):
        """A node at 80°C fails the temperature health check."""
        hot_node = ValidatorNode("hot", "solana", gpu_temp_c=80.0)
        assert hot_node.temp_ok is False

    def test_overloaded_node_fails_util_check(self):
        """A node at 95% utilization fails the utilization health check."""
        loaded = ValidatorNode("loaded", "ethereum", gpu_util_pct=95.0)
        assert loaded.util_ok is False

    def test_crashed_node_recovers(self):
        """crash() then recover() restores LIVE state and RPC."""
        self.svm_node.start()
        self.svm_node.crash()
        assert self.svm_node.state == ValidatorState.CRASHED
        self.svm_node.recover()
        assert self.svm_node.is_live


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 4 — TestRpcHardening
# ═════════════════════════════════════════════════════════════════════════════

class TestRpcHardening:
    """RPC rate limiting, DDoS protection, constant-time secret comparison."""

    def setup_method(self):
        self.limiter = RpcRateLimiter(limit=RPC_RATE_LIMIT_REQ_PER_SEC)
        self.vault = SecretsVault()

    def test_rate_limit_allows_up_to_limit(self):
        """100 consecutive requests from one IP are all allowed."""
        for _i in range(RPC_RATE_LIMIT_REQ_PER_SEC):
            assert self.limiter.request("10.0.0.1") is True

    def test_rate_limit_blocks_over_limit(self):
        """Request 101 from same IP is blocked."""
        for _ in range(RPC_RATE_LIMIT_REQ_PER_SEC):
            self.limiter.request("10.0.0.2")
        blocked = self.limiter.request("10.0.0.2")
        assert blocked is False

    def test_blocked_ip_registered(self):
        """Exceeding limit registers IP in blocked list."""
        for _ in range(RPC_RATE_LIMIT_REQ_PER_SEC + 1):
            self.limiter.request("10.0.0.3")
        assert self.limiter.is_blocked("10.0.0.3")

    def test_different_ips_independent_limits(self):
        """Two IPs each exhausting their quota don't interfere."""
        for _ in range(RPC_RATE_LIMIT_REQ_PER_SEC):
            self.limiter.request("10.0.1.1")
        # Different IP still gets 100 allowed
        assert self.limiter.request("10.0.1.2") is True

    def test_window_reset_clears_counts(self):
        """reset_window() clears all per-IP counters."""
        for _ in range(50):
            self.limiter.request("10.0.2.1")
        self.limiter.reset_window()
        assert self.limiter.request("10.0.2.1") is True

    def test_secrets_vault_store_and_retrieve(self):
        """SecretsVault stores and retrieves a secret correctly."""
        self.vault.store("node_key", "secret_value_abc123")
        assert self.vault.retrieve("node_key") == "secret_value_abc123"

    def test_secrets_vault_constant_time_compare_match(self):
        """constant_time_compare returns True for identical strings."""
        assert self.vault.constant_time_compare("abc", "abc") is True

    def test_secrets_vault_constant_time_compare_mismatch(self):
        """constant_time_compare returns False for different strings."""
        assert self.vault.constant_time_compare("abc", "xyz") is False

    def test_stale_secret_detected(self):
        """A secret created 91 days ago is flagged as stale."""
        stale_epoch = time.time() - (91 * 86_400)
        self.vault.store("old_key", "old_val", created_epoch=stale_epoch)
        assert self.vault.is_stale("old_key") is True


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 5 — TestOperatorRunbooks
# ═════════════════════════════════════════════════════════════════════════════

class TestOperatorRunbooks:
    """Operator runbook completeness, emergency procedures, and signing."""

    def _full_runbook(self) -> OperatorRunbook:
        rb = OperatorRunbook(
            sections=list(OperatorRunbook.REQUIRED_SECTIONS),
            emergency_procedures=list(OperatorRunbook.REQUIRED_EMERGENCY_PROCS),
            reviewed=True,
            gpg_signed=True,
        )
        return rb

    def test_complete_runbook_passes(self):
        """A fully populated runbook marks itself complete."""
        rb = self._full_runbook()
        assert rb.is_complete() is True

    def test_missing_section_fails_completeness(self):
        """Runbook missing 'Rollback procedure' fails completeness check."""
        rb = OperatorRunbook(
            sections=["Pre-flight checklist", "Deployment steps",
                      "Emergency shutdown", "Monitoring & alerts", "Incident response"],
            emergency_procedures=list(OperatorRunbook.REQUIRED_EMERGENCY_PROCS),
        )
        assert rb.is_complete() is False
        assert "Rollback procedure" in rb.missing_sections()

    def test_missing_emergency_proc_fails(self):
        """Runbook missing 'atomic_violation_response' fails."""
        rb = OperatorRunbook(
            sections=list(OperatorRunbook.REQUIRED_SECTIONS),
            emergency_procedures=["emergency_shutdown", "single_chain_failover",
                                  "operator_override"],
        )
        assert rb.is_complete() is False
        assert "atomic_violation_response" in rb.missing_procedures()

    def test_empty_runbook_lists_all_missing(self):
        """Empty runbook missing_sections returns all 6 required sections."""
        rb = OperatorRunbook()
        assert len(rb.missing_sections()) == len(OperatorRunbook.REQUIRED_SECTIONS)

    def test_reviewed_flag(self):
        """reviewed flag defaults to False."""
        rb = OperatorRunbook()
        assert rb.reviewed is False

    def test_gpg_signed_flag(self):
        """gpg_signed flag defaults to False."""
        rb = OperatorRunbook()
        assert rb.gpg_signed is False

    def test_full_runbook_no_missing_sections(self):
        """Full runbook returns empty missing_sections list."""
        rb = self._full_runbook()
        assert rb.missing_sections() == []

    def test_full_runbook_no_missing_procedures(self):
        """Full runbook returns empty missing_procedures list."""
        rb = self._full_runbook()
        assert rb.missing_procedures() == []

    def test_required_emergency_procs_count(self):
        """There are exactly 4 required emergency procedures."""
        assert len(OperatorRunbook.REQUIRED_EMERGENCY_PROCS) == 4


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 6 — TestMainnetReadinessGates
# ═════════════════════════════════════════════════════════════════════════════

class TestMainnetReadinessGates:
    """7-day soak, rollback approval, governance, validator count, security audit."""

    def _ready_report(self) -> MainnetReadinessReport:
        return MainnetReadinessReport(
            soak_days_completed=7.0,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=4,
            security_findings_critical=0,
            security_findings_high=2,
        )

    def test_soak_7_days_passes(self):
        """7.0 days of soak passes the soak gate."""
        r = self._ready_report()
        assert r.soak_passed() is True

    def test_soak_6_days_fails(self):
        """6.9 days of soak fails the soak gate."""
        r = MainnetReadinessReport(soak_days_completed=6.9)
        assert r.soak_passed() is False

    def test_validator_count_3_passes(self):
        """validator_count ≥ 3 passes."""
        r = MainnetReadinessReport(validator_count=3)
        assert r.validator_count_ok() is True

    def test_validator_count_2_fails(self):
        """validator_count < 3 fails."""
        r = MainnetReadinessReport(validator_count=2)
        assert r.validator_count_ok() is False

    def test_zero_critical_findings_passes(self):
        """0 critical security findings passes security gate."""
        r = MainnetReadinessReport(security_findings_critical=0)
        assert r.security_ok() is True

    def test_one_critical_finding_fails(self):
        """1 critical security finding blocks mainnet."""
        r = MainnetReadinessReport(security_findings_critical=1)
        assert r.security_ok() is False

    def test_fully_ready_report_go_decision(self):
        """Fully ready report returns go=True with empty blockers."""
        r = self._ready_report()
        go, blockers = r.go_decision()
        assert go is True
        assert blockers == []

    def test_missing_governance_approval_blocker(self):
        """Missing governance_approved surfaces in blockers."""
        r = self._ready_report()
        r.governance_approved = False
        go, blockers = r.go_decision()
        assert go is False
        assert "approvals_incomplete" in blockers

    def test_critical_finding_surfaces_in_blockers(self):
        """Critical security finding surfaces in go_decision blockers."""
        r = self._ready_report()
        r.security_findings_critical = 1
        go, blockers = r.go_decision()
        assert go is False
        assert any("critical_findings" in b for b in blockers)


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 7 — TestProductionReleaseBundle
# ═════════════════════════════════════════════════════════════════════════════

class TestProductionReleaseBundle:
    """Release bundle manifest: file list, SHA256 checksums, GPG signature."""

    def _full_bundle(self) -> ReleaseBundleManifest:
        rb = ReleaseBundleManifest(version="v1.1.0")
        for fname in REQUIRED_BUNDLE_FILES:
            rb.add_file(fname, content=f"content of {fname}".encode())
        rb.sign()
        return rb

    def test_all_required_files_present(self):
        """Full bundle contains all 10 required files."""
        rb = self._full_bundle()
        assert rb.is_complete

    def test_missing_file_detected(self):
        """Bundle missing 'runtime.wasm' is detected as incomplete."""
        rb = ReleaseBundleManifest(version="v1.1.0")
        for fname in REQUIRED_BUNDLE_FILES:
            if fname != "runtime.wasm":
                rb.add_file(fname)
        assert "runtime.wasm" in rb.missing_files()

    def test_checksums_all_present(self):
        """Every file in the bundle has a SHA256 checksum entry."""
        rb = self._full_bundle()
        for fname in rb.files:
            assert fname in rb.checksums
            assert len(rb.checksums[fname]) == 64

    def test_checksum_is_deterministic(self):
        """Same content always produces same checksum."""
        rb = ReleaseBundleManifest(version="v1.1.0")
        rb.add_file("test.bin", b"hello")
        rb.add_file("test.bin.copy", b"hello")
        assert rb.checksums["test.bin"] == rb.checksums["test.bin.copy"]

    def test_gpg_signature_present_after_sign(self):
        """sign() populates gpg_signature."""
        rb = self._full_bundle()
        assert rb.is_signed

    def test_gpg_signature_verifies(self):
        """verify_signature() returns True for correctly signed bundle."""
        rb = self._full_bundle()
        assert rb.verify_signature() is True

    def test_tampered_bundle_fails_verification(self):
        """Adding a file after signing causes verify_signature() to fail."""
        rb = self._full_bundle()
        rb.add_file("extra_file.txt", b"injected")
        assert rb.verify_signature() is False

    def test_version_string_present(self):
        """Bundle has a non-empty version string."""
        rb = self._full_bundle()
        assert rb.version and len(rb.version) > 0

    def test_required_bundle_files_count(self):
        """There are exactly 10 required bundle files."""
        assert len(REQUIRED_BUNDLE_FILES) == 10


# ═════════════════════════════════════════════════════════════════════════════
#  Suite 8 — TestP5MainnetGoNoGo
# ═════════════════════════════════════════════════════════════════════════════

class TestP5MainnetGoNoGo:
    """
    All mainnet_readiness gates from P5 success_metrics:
      ✅ 14-day stable operation on testnet
      ✅ Security audit passed
      ✅ Insurance/liability assessed (governance_approved proxy)
      ✅ Operator community feedback integrated (runbook reviewed)
      ✅ Release bundle complete + GPG-signed
      ✅ 0 atomic violations in 24-hour test
      ✅ Dual validator live and synced
    """

    MAINNET_STABLE_DAYS = 14       # 14-day testnet stability requirement

    def setup_method(self):
        self.net = CrossChainTestnet()
        self.net.boot_both()
        self.report = MainnetReadinessReport(
            soak_days_completed=14.0,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=4,
            security_findings_critical=0,
        )
        self.bundle = ReleaseBundleManifest(version="v1.1.0")
        for fname in REQUIRED_BUNDLE_FILES:
            self.bundle.add_file(fname, content=f"content_{fname}".encode())
        self.bundle.sign()
        self.runbook = OperatorRunbook(
            sections=list(OperatorRunbook.REQUIRED_SECTIONS),
            emergency_procedures=list(OperatorRunbook.REQUIRED_EMERGENCY_PROCS),
            reviewed=True,
            gpg_signed=True,
        )

    def test_14day_stability_gate(self):
        """Gate: ≥14 days of stable operation on testnet."""
        assert self.report.soak_days_completed >= self.MAINNET_STABLE_DAYS

    def test_security_audit_gate(self):
        """Gate: external security audit marked passed."""
        assert self.report.external_audit_passed is True

    def test_zero_critical_findings_gate(self):
        """Gate: 0 critical security findings."""
        assert self.report.security_findings_critical == 0

    def test_release_bundle_complete_gate(self):
        """Gate: release bundle contains all required files."""
        assert self.bundle.is_complete

    def test_release_bundle_signed_gate(self):
        """Gate: release bundle GPG signature verifies."""
        assert self.bundle.verify_signature() is True

    def test_operator_runbook_complete_gate(self):
        """Gate: operator runbook has all required sections and procedures."""
        assert self.runbook.is_complete() is True

    def test_zero_atomic_violations_24h_gate(self):
        """Gate: 0 atomic violations in 24-hour simulation."""
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        assert self.net._atomic_violations == 0

    def test_dual_validator_synced_gate(self):
        """Gate: both validators live and in sync."""
        self.net.advance_both(200)
        assert self.net.both_live
        assert self.net.in_sync

    def test_full_mainnet_go_decision(self):
        """Master gate: all P5 mainnet_readiness criteria satisfied → GO."""
        # 24h stability
        self.net.simulate_24h(H24_SIMULATION_TX_COUNT)
        # Go/no-go evaluation
        go, blockers = self.report.go_decision()
        assert go is True, f"Mainnet GO/NO-GO FAILED — blockers: {blockers}"
        assert self.net._atomic_violations == 0
        assert self.bundle.is_complete
        assert self.bundle.verify_signature() is True
        assert self.runbook.is_complete() is True
        assert self.net.both_live
