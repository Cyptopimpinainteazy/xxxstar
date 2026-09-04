# X3 Canonical Readiness — Single Source of Truth

**Generated: 2026-06-17**
**Canonical source: `FEATURE_REGISTRY.toml` (root level)**

All status documents must derive from this file. Run `scripts/check-readiness-consistency.sh` in CI.

## Feature Readiness Scores

| Feature | Mode | Score | Key Blockers |
|---|---|---|---|
| `atomic_kernel` | LIVE_TESTNET | 75% | Economic halt not multi-validator tested |
| `atomic_router` | LIVE_TESTNET | 85% | External bridge disabled at genesis |
| `atomic_lock` | LIVE_TESTNET | 60% | LP lock E2E not in CI |
| `axe` | GUARDED_TESTNET | 55% | Advanced DEX features compile-time guarded |
| `triforge_runtime` | GUARDED_TESTNET | 50% | Upgrade rehearsal not hard CI gate |
| `x3_forge` | GUARDED_TESTNET | 50% | Requires sentinel score, no CI gate |
| `atomic_gateway` | GUARDED_TESTNET | 45% | ExternalBridgesEnabled=false, no CI gate for audit/revoke |
| `x3_reactor` | LIVE_TESTNET | 40% | Benchmark not in CI critical path |
| `launch_gate` | LIVE_TESTNET | 35% | Not in CI, manual approval |
| `x3_sentinel` | GUARDED_TESTNET | 30% | Not in CI critical path |
| `x3_wallet_pallet` | LIVE_TESTNET | 30% | No CI hard gate, biometrics unreviewed |
| `x3_broadcast` | LIVE_TESTNET | 30% | Non-consensus tooling |
| `btc_fortress_gateway` | SIM_TESTNET | 25% | Testnet only, no real BTC signer quorum |
| `x3_swarm_core` | GUARDED_TESTNET | 25% | Experimental, no partition-tolerance testing |
| `repo_scanner_agent` | LIVE_TESTNET | 25% | Dev-ops tooling, single test |
| `auditor_agent` | LIVE_TESTNET | 25% | Experimental, findings not auto-triaged |
| `x3_grantsmith` | LIVE_TESTNET | 20% | Non-consensus tooling |
| `testbuilder_agent` | GUARDED_TESTNET | 20% | Experimental, tests not validated |
| `breaker_agent` | LIVE_TESTNET | 20% | Experimental |
| `tauri_os` | GUARDED_TESTNET | 15% | Desktop UI, dead buttons, no CI |
| `fixer_agent` | GUARDED_TESTNET | 15% | Experimental, human approval required |
| `marketing_agent` | LIVE_TESTNET | 15% | Non-consensus tooling, single test |
| `grant_agent` | LIVE_TESTNET | 15% | Non-consensus tooling, single test |

**Average readiness: ~36%** (up from ~32% before 2026-06-17 review)

## What this review (2026-06-17) improved

| Change | Detail |
|---|---|
| `atomic_router` 80→85 | x3-lang VM IF/LOOP/REQUIRE/ON_FAIL/ON_TIMEOUT/ATOMIC_BEGIN/END/ROLLBACK now execute real semantics with 10+ E2E tests — routing guards enforceable in bytecode |
| `atomic_gateway` 35→45 | Bridge `init_production_backend()` with 4 verifier families (evm-light-client, svm-light-client, evm-rpc, svm-rpc) — no longer dry-run-only |
| `launch_gate` 30→35 | CI consistency check (`scripts/check-readiness-consistency.sh`) prevents contradictory status claims |

## What was NOT changed

The VM executor, bridge backend, GPU validator, and readiness engine fixes address specific gaps. Features untouched by this review (`atomic_kernel`, `triforge_runtime`, `btc_fortress_gateway`, axe, x3_forge, atomic_lock, sentinel, reactor, broadcast, grantsmith, tauri_os, all swarm agents, wallet_pallet) retain their original scores and blockers. Those blockers require operational work (multi-validator testing, CI gate wiring, external audit, mainnet deployment) that no code change in this review can satisfy.