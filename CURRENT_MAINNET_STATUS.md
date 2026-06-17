# X3 Atomic Star — Mainnet Status

**Updated: 2026-06-17 — Derived from canonical `FEATURE_REGISTRY.toml`**

> `FEATURE_REGISTRY.toml` is the single canonical source. All percentages derive from it.
> Run `scripts/check-readiness-consistency.sh` to validate. See `docs/X3_CANONICAL_READINESS.md`.

## System Completion Scoreboard

```
Atomic Router (pallets/x3-cross-vm-router)      ████████░░  85%  LIVE_TESTNET — VM control-flow ops now executing
Atomic Kernel (pallets/x3-atomic-kernel)         ███████░░░  75%  LIVE_TESTNET — economic halt not multi-validator tested
Atomic Lock (pallets/x3-atomic-kernel)           ██████░░░░  60%  LIVE_TESTNET — LP lock E2E not in CI
AXE DEX (pallets/x3-dex)                         █████░░░░░  55%  GUARDED_TESTNET — advanced features compile-time guarded
Triforge Runtime (pallets/evolution-core)        █████░░░░░  50%  GUARDED_TESTNET — upgrade rehearsal not hard CI gate
X3 Forge (pallets/x3-token-factory)              █████░░░░░  50%  GUARDED_TESTNET — requires sentinel score
Atomic Gateway (crates/x3-gateway)               ████░░░░░░  45%  GUARDED_TESTNET — ExternalBridgesEnabled=false
X3 Reactor (crates/x3-bench)                     ████░░░░░░  40%  LIVE_TESTNET — benchmark not in CI critical path
Launch Gate (scripts/mainnet)                    ███░░░░░░░  35%  LIVE_TESTNET — not in CI, manual approval
X3 Sentinel (pallets/x3-sentinel)                ███░░░░░░░  30%  GUARDED_TESTNET — not in CI critical path
X3 Wallet Pallet (pallets/x3-wallet-pallet)      ███░░░░░░░  30%  LIVE_TESTNET — no CI hard gate
X3 Broadcast (crates/x3-broadcast)               ███░░░░░░░  30%  LIVE_TESTNET — non-consensus tooling
BTC Fortress Gateway (crates/x3-gateway)         ██░░░░░░░░  25%  SIM_TESTNET — regtest only
Swarm Core (crates/x3-swarm-core)                ██░░░░░░░░  25%  GUARDED_TESTNET — experimental
Tauri OS (apps/tauri-os)                         █░░░░░░░░░  15%  GUARDED_TESTNET — dead buttons, no CI
All swarm agents (6 agents)                      ██░░░░░░░░  15-25%  Experimental, non-consensus tooling
```

**Average readiness: ~36%** (up from ~32% before this review)

## What this review (2026-06-17) delivered

| Improvement | Before | After | Detail |
|---|---|---|---|
| x3-lang VM control-flow | Opcodes fail-closed | Executing | IF, LOOP, REQUIRE, ON_FAIL, ON_TIMEOUT, ATOMIC_BEGIN/END/ROLLBACK with 10+ tests |
| Bridge production backend | Dry-run only | Env-configurable | `init_production_backend()` with 4 verifier families |
| Readiness consistency | Docs contradicted | Single source enforced | `FEATURE_REGISTRY.toml` canonical, CI check script |
| x3-readiness engine | Read wrong files | Reads root-level files | `BTreeMap` deserialization matching actual TOML structure |
| GPU validator task polling | Empty snapshot path | Redis SCAN-based | `list_pending_swaps()` with actual block/slot data |

## What remains incomplete

These blockers require operational work (multi-validator testing, CI gate wiring, external audits, mainnet deployment) — not code changes:

- Economic halt never triggered in multi-validator network
- External bridge disabled at genesis (governance gate)
- BTC mainnet not available (SIM_TESTNET only)
- No CI hard gates for wallet, DEX, token factory, LP locker, sentinel
- No automated migration dry-run across runtime variants
- Swarm agents experimental — no multi-agent testing
- Tauri desktop has dead buttons
- All phases 1-6 of launch checklist require infrastructure deployment

## Build

```bash
cargo build -p node --features mainnet-rc1 --release
```

## Validation

```bash
scripts/check-readiness-consistency.sh