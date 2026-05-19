"""
P4/P5 Deep Stress Validation Suite
==================================

Longer-running deterministic stress tests for the cross-chain GPU validator
and production-readiness logic, intended to catch flakiness and state drift.
"""

import random

from tests.p4_p5_crosschain_gpu_validator import (
    AtomicSwapIntent,
    AtomicSwapOrchestrator,
    CrossChainMonitor,
    EvmGpuKernel,
)
from tests.p4_p5_production_release import (
    REQUIRED_BUNDLE_FILES,
    CrossChainTestnet,
    MainnetReadinessReport,
    ReleaseBundleManifest,
)


class TestDeepAtomicSoak:
    def test_100k_swaps_with_2pct_adversarial_inputs(self):
        random.seed(1337)
        kernel = EvmGpuKernel()
        orchestrator = AtomicSwapOrchestrator(kernel)

        count = 100_000
        violating_inputs = 0
        committed = 0
        rolled_back = 0

        orchestrator.full_3pac(AtomicSwapIntent("base", 100.0, 100.0))

        for i in range(count):
            if random.random() < 0.02:
                left, right = 100.0, 99.0
                violating_inputs += 1
            else:
                left, right = 100.0, 100.0

            _ok, result = orchestrator.full_3pac(
                AtomicSwapIntent(f"id_{i}", left, right)
            )
            if result == "committed":
                committed += 1
            else:
                rolled_back += 1

        assert orchestrator.violations == violating_inputs
        assert committed + rolled_back == count
        assert committed == count - violating_inputs
        assert rolled_back == violating_inputs
        assert len(orchestrator.active_swaps) == 0

    def test_gpu_atomic_verify_opcode_frequency(self):
        kernel = EvmGpuKernel()
        orchestrator = AtomicSwapOrchestrator(kernel)

        orchestrator.full_3pac(AtomicSwapIntent("base", 100.0, 100.0))
        for i in range(10_000):
            orchestrator.full_3pac(AtomicSwapIntent(f"s{i}", 100.0, 100.0))

        gpu_atomic_calls = [op for op in kernel.call_log if op == 0xD8]
        assert len(gpu_atomic_calls) == 10_001

    def test_deterministic_outcome_same_seed(self):
        def run_once(seed: int):
            random.seed(seed)
            kernel = EvmGpuKernel()
            orchestrator = AtomicSwapOrchestrator(kernel)
            orchestrator.full_3pac(AtomicSwapIntent("base", 100.0, 100.0))

            violations = 0
            for i in range(20_000):
                if random.random() < 0.03:
                    left, right = 100.0, 99.0
                    violations += 1
                else:
                    left, right = 100.0, 100.0
                orchestrator.full_3pac(AtomicSwapIntent(f"r{i}", left, right))

            return orchestrator.violations, len(orchestrator.committed), len(orchestrator.rolled_back)

        a = run_once(20260325)
        b = run_once(20260325)
        assert a == b

    def test_commit_ratio_above_95pct_under_2pct_fault_injection(self):
        random.seed(8080)
        kernel = EvmGpuKernel()
        orchestrator = AtomicSwapOrchestrator(kernel)
        orchestrator.full_3pac(AtomicSwapIntent("base", 100.0, 100.0))

        total = 50_000
        for i in range(total):
            if random.random() < 0.02:
                orchestrator.full_3pac(AtomicSwapIntent(f"c{i}", 100.0, 99.0))
            else:
                orchestrator.full_3pac(AtomicSwapIntent(f"c{i}", 100.0, 100.0))

        commit_ratio = len(orchestrator.committed) / (len(orchestrator.committed) + len(orchestrator.rolled_back))
        assert commit_ratio > 0.95


class TestDeepMonitoringAndTestnet:
    def test_24h_stability_100k_swaps_no_violations(self):
        net = CrossChainTestnet()
        net.boot_both()
        net.simulate_24h(100_000)

        assert net._atomic_violations == 0
        assert net.timeout_rate_pct == 0.0
        assert net.success_rate_pct == 100.0
        assert net.both_live
        assert net.in_sync

    def test_invariant_monitor_alerts_match_injected_failures(self):
        monitor = CrossChainMonitor()
        failures = 0

        for i in range(10_000):
            before = 1000.0
            if i % 137 == 0:
                after = 999.0
                failures += 1
            else:
                after = 1000.0
            monitor.check_invariant(before, after)

        assert monitor.invariant_violations == failures
        assert len(monitor.alerts) == failures

    def test_sync_stability_over_long_head_progression(self):
        net = CrossChainTestnet()
        net.boot_both()

        for _ in range(5_000):
            net.advance_both(1)

        assert net.in_sync
        assert net.svm.head == 5_000
        assert net.evm.head == 5_000

    def test_testnet_recovery_after_evm_crash(self):
        net = CrossChainTestnet()
        net.boot_both()
        net.advance_both(100)

        net.evm.crash()
        assert not net.evm.is_live
        net.evm.recover()
        assert net.evm.is_live
        assert net.svm.is_live


class TestDeepReleaseBundleIntegrity:
    def _bundle(self):
        bundle = ReleaseBundleManifest(version="v1.1.0")
        for fname in REQUIRED_BUNDLE_FILES:
            bundle.add_file(fname, content=f"content_{fname}".encode())
        bundle.sign()
        return bundle

    def test_signature_survives_reverification_loop(self):
        bundle = self._bundle()
        for _ in range(1_000):
            assert bundle.verify_signature()

    def test_tamper_matrix_detects_all_file_mutations(self):
        base = self._bundle()
        detected = 0

        for idx, filename in enumerate(REQUIRED_BUNDLE_FILES):
            candidate = ReleaseBundleManifest(version=base.version)
            for f in REQUIRED_BUNDLE_FILES:
                candidate.add_file(f, content=f"content_{f}".encode())
            candidate.sign()
            candidate.add_file(filename, content=f"MUTATED_{idx}".encode())
            if not candidate.verify_signature():
                detected += 1

        assert detected == len(REQUIRED_BUNDLE_FILES)

    def test_missing_file_matrix(self):
        missing_detected = 0
        for filename in REQUIRED_BUNDLE_FILES:
            bundle = ReleaseBundleManifest(version="v1.1.0")
            for f in REQUIRED_BUNDLE_FILES:
                if f != filename:
                    bundle.add_file(f)
            if filename in bundle.missing_files():
                missing_detected += 1
        assert missing_detected == len(REQUIRED_BUNDLE_FILES)

    def test_checksum_uniqueness_for_distinct_payloads(self):
        bundle = ReleaseBundleManifest(version="v1.1.0")
        for i in range(1000):
            bundle.add_file(f"f_{i}", content=f"payload_{i}".encode())
        checksum_set = set(bundle.checksums.values())
        assert len(checksum_set) == 1000


class TestDeepMainnetGoNoGoResilience:
    def test_go_decision_blocks_each_missing_approval(self):
        base = MainnetReadinessReport(
            soak_days_completed=10.0,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=4,
            security_findings_critical=0,
        )

        scenarios = [
            ("external_audit_passed", False),
            ("rollback_runbook_approved", False),
            ("governance_approved", False),
            ("key_management_complete", False),
        ]

        for field, value in scenarios:
            report = MainnetReadinessReport(**base.__dict__)
            setattr(report, field, value)
            go, blockers = report.go_decision()
            assert go is False
            assert "approvals_incomplete" in blockers

    def test_go_decision_requires_seven_day_soak(self):
        report = MainnetReadinessReport(
            soak_days_completed=6.99,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=4,
            security_findings_critical=0,
        )
        go, blockers = report.go_decision()
        assert go is False
        assert any("soak_days" in b for b in blockers)

    def test_go_decision_requires_min_validators(self):
        report = MainnetReadinessReport(
            soak_days_completed=7.0,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=2,
            security_findings_critical=0,
        )
        go, blockers = report.go_decision()
        assert go is False
        assert any("validators=" in b for b in blockers)

    def test_go_decision_requires_zero_critical_findings(self):
        report = MainnetReadinessReport(
            soak_days_completed=7.0,
            external_audit_passed=True,
            rollback_runbook_approved=True,
            governance_approved=True,
            key_management_complete=True,
            validator_count=4,
            security_findings_critical=2,
        )
        go, blockers = report.go_decision()
        assert go is False
        assert any("critical_findings=" in b for b in blockers)
