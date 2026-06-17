# X3 Atomic Star — Mainnet Status

**Updated: 2026-06-17 — Derived from canonical `FEATURE_REGISTRY.toml`**

> `FEATURE_REGISTRY.toml` is the single canonical source. All percentages derive from it.
> Run `scripts/check-readiness-consistency.sh` to validate. See `docs/X3_CANONICAL_READINESS.md`.

## System Completion Scoreboard

```
Atomic Router (pallets/x3-cross-vm-router)      █████████░  88%  LIVE_TESTNET — 8 invariants passing, CI gate wired
Atomic Kernel (pallets/x3-atomic-kernel)         ████████░░  85%  LIVE_TESTNET — 6 tests passing, EconomicHalt wired
AXE DEX (pallets/x3-dex)                         ███████░░░  75%  GUARDED_TESTNET — EconomicHalt runtime gate, CI wired
X3 Forge (pallets/x3-token-factory)              ███████░░░  75%  GUARDED_TESTNET — EconomicHalt runtime gate, CI wired
Atomic Lock (pallets/x3-atomic-kernel)           ██████░░░░  68%  LIVE_TESTNET — CI gate wired via atomic_kernel
Atomic Gateway (crates/x3-gateway)               ██████░░░░  65%  GUARDED_TESTNET — 4 verifier families, code review done
Triforge Runtime (pallets/evolution-core)        █████░░░░░  55%  GUARDED_TESTNET — build step wired, migration not automated
X3 Wallet Pallet (pallets/x3-wallet-pallet)      █████░░░░░  55%  LIVE_TESTNET — CI gate verified
X3 Sentinel (pallets/x3-sentinel)                █████░░░░░  50%  GUARDED_TESTNET — CI gate wired
X3 Reactor (crates/x3-bench)                     ████░░░░░░  40%  LIVE_TESTNET — benchmark not in CI critical path
Launch Gate (scripts/mainnet)                    ████░░░░░░  40%  LIVE_TESTNET — consistency check wired
X3 Broadcast (crates/x3-broadcast)               ███░░░░░░░  30%  LIVE_TESTNET — non-consensus tooling
BTC Fortress Gateway (crates/x3-gateway)         ██░░░░░░░░  25%  SIM_TESTNET — regtest only
Swarm Core (crates/x3-swarm-core)                ██░░░░░░░░  25%  GUARDED_TESTNET — experimental
Tauri OS (apps/tauri-os)                         █░░░░░░░░░  15%  GUARDED_TESTNET — dead buttons, no CI
All swarm agents (6 agents)                      ██░░░░░░░░  15-25%  Experimental, non-consensus tooling
```

**Average readiness: ~52%** (up from ~36% → ~47% → ~52%)

## What this review (2026-06-17) delivered

| Improvement | Before | After | Detail |
|---|---|---|---|
| x3-lang VM control-flow | Opcodes fail-closed | Executing | IF, LOOP, REQUIRE, ON_FAIL, ON_TIMEOUT, ATOMIC_BEGIN/END/ROLLBACK with 10+ tests |
| Bridge production backend | Dry-run only | Env-configurable | `init_production_backend()` with 4 verifier families |
| Readiness consistency | Docs contradicted | Single source enforced | `FEATURE_REGISTRY.toml` canonical, CI check script passes |
| x3-readiness engine | Read wrong files | Reads root-level files | `BTreeMap` deserialization matching actual TOML structure |
| GPU validator task polling | Empty snapshot path | Redis SCAN-based | `list_pending_swaps()` with actual block/slot data |
| CI gate wiring | No pallet CI gates | 9 targets in Makefile | `production-gate.yml` enforces `make test-all-pallets` |
| DEX/forge guards audit | Believed compile-time | Confirmed runtime | EconomicHalt::is_halted() already used, no code change needed |

## What remains incomplete

- Economic halt never triggered in multi-validator network (operational)
- External bridge disabled at genesis (governance gate)
- BTC mainnet not available (SIM_TESTNET only)
- Cross-pallet integration (launchpad → LP locker) not in CI
- No automated migration dry-run across runtime variants
- Biometric + recovery logic lacks security review
- Swarm agents experimental — no multi-agent testing
- Tauri desktop has dead buttons

## Build

```bash
cargo build -p node --features mainnet-rc1 --release
```

## Validation

```bash
scripts/check-readiness-consistency.sh   # PASS
make guard                               # stub + test-cheat detection
make test-all-pallets                    # 9 pallet + crate test suites
make audit                               # invariants + release gate + consistency