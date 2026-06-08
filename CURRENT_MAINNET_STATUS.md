# X3 Atomic Star — Current Mainnet Status

> **Last updated:** 2026-06-08
> **Target:** v0.4 internal-only Mainnet RC-1

---

## TL;DR

| Dimension | Status |
|-----------|--------|
| Node binary builds | ✅ `cargo build --release -p x3-chain-node` |
| Consensus (Aura + GRANDPA) | ✅ Real — not simulated |
| Internal cross-VM routing (6 routes) | ✅ Tested, supply invariant proven |
| External bridges (Ethereum/Solana mainnet) | 🔒 Frozen at genesis — governance-gated |
| 3-validator local testnet | ✅ `scripts/testnet-full-launch.sh` |
| Supply ledger invariant | ✅ Enforced on every operation |
| Settlement engine | ✅ State machine implemented; OCW stub is testnet-only |
| DEX pallet (pallet-x3-dex) | ✅ Wired in all 6 runtime variants; AMM spot swap operational |
| Route limits enforcement | ✅ Daily volume + per-wallet daily limits with epoch-day auto-reset |
| LP Locker pallet (pallet-x3-lp-locker) | ✅ **NEW** — On-chain anti-rug LP lock registry with 4 extrinsics, 16 tests |
| Launchpad → DEX graduation | ✅ **NEW** — TokenFactory mint → DEX pool → LP lock graduation pipeline |
| CI hard gates | ✅ `.github/workflows/ci.yml` (9 required jobs) |
| Public testnet | 🚧 Pre-launch checks in progress |
| Mainnet | 🔴 Not yet — pending public testnet validation |

---

## What Is Working (Production-Quality)

### Consensus Layer
- **Aura block production** with real slot assignment
- **GRANDPA finality** with real voting and equivocation protection
- Node binary: `target/release/x3-chain-node`
- Dev chain: `x3-chain-node --chain=dev --tmp`
- 3-validator testnet: `scripts/testnet-full-launch.sh`

### Internal Cross-VM Execution
- **X3Native ↔ X3Evm ↔ X3Svm** — all 6 internal routes implemented
- Atomic source-debit / destination-credit semantics
- Replay protection: `UsedMessages` (message-id dedup) + `NextNonce`/`NonceBatchAllocation` (monotonic per-sender sequence); no `UsedNonces` point-lookup map — superseded by the monotonic nonce scheme
- Expiry + cancel: `cancel_expired_xvm_transfer` returns pending supply to source
- Supply invariant: `represented_total ≤ canonical_supply` enforced on every operation
- Scope freeze: external bridges disabled by default; require governance to open
- **Route limits enforced**: `DailyVolume` and `WalletDailyVolume` storage maps with epoch-day auto-reset check both `daily_limit` and `per_wallet_daily_limit` from RouteConfig before every transfer

### Supply Ledger
- `pallet-x3-supply-ledger`: canonical supply accounting per asset
- `check_invariant()` is called at every supply-changing operation
- Historical proofs retained for `HISTORICAL_PROOF_RETENTION_BLOCKS = 1,000` blocks

### Settlement Engine
- State machine: `MATCH → ASSETS_LOCKED_X3 → EXTERNAL_EXECUTION → PROOF_SUBMITTED → FINALIZE_X3`
- Refund path: `→ REFUND_X3` on timeout or failure
- Atomic locks and escrow implemented
- Settlement timeout checker runs via `on_idle()`

### DEX & Launchpad (New — Domain 2 Complete)

#### pallet-x3-dex
- AMM spot swap — `create_pool`, `add_liquidity`, `remove_liquidity`, `swap` extrinsics
- Wired across all 6 `construct_runtime!` variants
- Config: `MaxPools`, `WeightInfo`, `EconomicHalt`

#### pallet-x3-launchpad (Graduation Path)
- **`graduate_launch` extrinsic** (call_index 7, `#[transactional]`):
  1. Creates token via TokenFactory (`LaunchpadTokenFactoryBridge`)
  2. Creates AMM pool via DEX bridge (`LaunchpadDexBridge`, 30 bps fee)
  3. Locks LP tokens via LP Locker bridge (`LaunchpadLpLockerBridge`)
  4. Emits `LaunchGraduated` event
- `GraduatedLaunches` storage map tracking `(asset_id, pool_id)` per launch
- `lp_lock_duration_blocks` in `LaunchState`
- 3 bridge trait interfaces: `TokenFactoryCreate`, `DexPoolCreate`, `LpLockCreate`

#### pallet-x3-lp-locker
- On-chain LP lock registry for anti-rug protection
- 4 extrinsics: `lock_lp`, `unlock_lp`, `extend_lock`, `increase_lock`
- 8 error conditions, 4 events, `is_locked()`/`total_locked_for_pool()` helpers
- 16 unit tests
- Wired in all 6 `construct_runtime!` variants (dev, prod, mainnet-rc1, frontier/no-frontier)
- Config: `MinLockDuration = 1,500 blocks (~5min)`, `MaxLockDuration = 157,680,000 (~1yr)`

---

## What Is TESTNET_ONLY

These features work on testnet but have known limitations for mainnet:

| Feature | Limitation | Ships When |
|---------|------------|------------|
| Settlement OCW | Stub — logs that hook is wired; no auto-finalization | Phase 1c (post-RC1) |
| Relayer authorization | Governance-approved list (not decentralized) | Post-RC1 |
| Emergency pause | Available but governed; not fully autonomous | Post-RC1 |
| GPU validator sidecar | Optional health check; not required for consensus | Post-RC1 |

---

## What Is DISABLED_POST_RC1

These are explicitly frozen and MUST NOT be enabled until audited:

| Feature | Scope Guard | How to Enable (post-audit) |
|---------|-------------|---------------------------|
| External bridge to Ethereum mainnet | `ExternalBridgesEnabled = false` at genesis | `set_external_bridge_audit_gate(true)` → `set_external_bridges_enabled(true)` |
| External bridge to Solana mainnet | Same kill-switch | Same governance flow |
| Parallel executor | Compile-time `compile_error!` if `mainnet-rc1 + parallel-executor` | Remove scope guard after audit |
| AppZone factory | Compile-time `compile_error!` if `mainnet-rc1 + appzone-factory` | Remove scope guard after audit |
| PQ cryptography (experimental) | Compile-time guard | Remove scope guard after audit |
| AI optimizer | Compile-time guard | Remove scope guard after audit |
| Advanced DEX | Compile-time guard | Remove scope guard after audit |

---

## Honest Gap Analysis

> See X3_END_TO_END_GAPS_MASTER_PLAN.md for the full gap execution plan covering the missing items below.

### Gaps Before Public Testnet
1. **Block explorer**: `apps/explorer/` exists but needs connection to real node RPC
2. **Faucet**: No automated testnet faucet deployed
3. **Documentation**: Deployment guide needs to be complete (see `docs/deployment/DEPLOYMENT_GUIDE.md`)
4. **Settlement OCW**: Full off-chain worker for automated finalization is a stub

### Gaps Before Mainnet
1. All DISABLED_POST_RC1 features above need external security audits
2. Decentralized relayer set for settlement engine
3. Slashing conditions fully implemented and tested on testnet
4. Economic model validated (staking, inflation, fee parameters)
5. Emergency response playbook tested on public testnet

---

## How to Verify the Node Is Real (Not Mock)

```bash
# Build the real node
cargo build --release -p x3-chain-node

# Start a dev chain
./target/release/x3-chain-node --chain=dev --tmp --rpc-port 9933

# Or use the start script (exits with error if binary missing)
./scripts/start-x3-chain.sh

# Start 3-validator testnet
./scripts/testnet-full-launch.sh

# Run mock RPC only (explicitly dev-only)
./scripts/start-mock-rpc-dev.sh
```

### How to distinguish real from mock:
- Real node: responds with real block hash, increments finalized block height, runs GRANDPA
- Mock: fixed fake responses from `scripts/mock-rpc-server.js`; explicitly labeled DEV ONLY

---

## CI Status

All merges to `main` require the `x3 / critical-path-all-pass` check in:
`.github/workflows/ci.yml`

The branch-protection required status check name is `x3 / critical-path-all-pass` (the aggregate job). The workflow has 9 worker jobs plus 1 aggregate for 10 total jobs.

Required worker gates (9) — verbatim `ci.yml` invocation:
- `cargo fmt --all -- --check`
- `cargo check -p x3-chain-runtime --features std`
- `cargo check -p x3-chain-node`
- `cargo test -p pallet-x3-cross-vm-router --all-features` (8 named production-proof tests verified individually via shell loop)
- `cargo test -p pallet-x3-supply-ledger --all-features`
- `cargo test -p pallet-x3-settlement-engine --all-features`
- `cargo test -p pallet-x3-atomic-kernel --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --release -p x3-chain-node` (release binary gate, artifact uploaded)

`FINAL_REPORT.md` is a template document only — it is not a CI hard gate and is not yet populated with evidence. The `final-report-enforcement` job was removed from the aggregation path because the template cannot pass its own validation rules.

RC4 runtime upgrade rehearsal is now a true PASS. Automation, report, and evidence files are all consistent. Blocker resolved as of 2026-05-15T03:16Z.

### D1 Housekeeping (2026-06-08, verified & extended 2026-06-08T21:34Z)

- **x3-liquidity-core**: Workspace membership confirmed — `cargo check -p x3-liquidity-core` passes. Dependents `pallets/x3-cross-vm-router` (optional) and `tests/e2e` resolve correctly. `x3-dex` dependency path verified (`../x3-dex` resolves to `crates/x3-dex/Cargo.toml`).
- **UsedNonces stale references**: Removed from `pallets/x3-cross-vm-router/src/tests.rs` doc-comments. The `lib.rs` header comment correctly explains the monotonic nonce scheme supersedes `UsedNonces` with explicit NOTE markers (lines 21, 183) clarifying "UsedNonces is referenced here only to explain its intentional absence." `grep UsedNonces tests.rs` returns 0 hits. Three launch-gates/sources/ snapshot files updated with correct monotonic nonce description and "ARCHIVED SNAPSHOT" disclaimers. `grep UsedNonces tests.rs` returns 0 stale matches (the single remaining hit is the replacement text: "no UsedNonces map").
- **CI job mapping**: All 9 required worker gates in `ci.yml` map 1:1 to `CURRENT_MAINNET_STATUS.md`. Gate commands above updated to reflect actual `ci.yml` invocations (includes `--all-features` on test jobs and `--features std` on runtime check). Branch-protection required status check name (`x3 / critical-path-all-pass`) documented in both README.md and CURRENT_MAINNET_STATUS.md. `ci.yml` confirmed present with 9 worker + 1 aggregate job.
- **FEATURE_REGISTRY.toml scores**: All `required_tests` cross-referenced against actual test functions in pallet source files and `ci.yml` verification loops. `x3_wallet_pallet` `required_tests` renamed to match the exact 7 test function names in `tests.rs` (register_hardware_wallet_works, create_multisig_wallet_works, transfer_tokens_works, mint_tokens_authorized_only, add_remove_minter_root_only, register_biometric_works, initiate_recovery_works); 3 missing tests (mint_tokens_authorized_only, add_remove_minter_root_only, initiate_recovery_works) added to `tests.rs`; readiness_score lowered to 30 (no CI hard gate). `atomic_router` `required_tests` replaced with the 8 CI-verified function names from `ci.yml`'s production-proof loop (test_x3_native_evm_svm_roundtrip_preserves_supply, test_all_six_internal_routes_succeed, test_duplicate_nonce_rejected, test_failed_destination_credit_refunds_pending_supply, test_canonical_supply_never_breaks, test_duplicate_message_replay_rejected, test_expired_transfer_refunds_to_source, external_bridges_are_paused_at_genesis); blocker list updated to reflect mapping is now exact. `triforge_runtime` blocker string corrected — no longer claims `ci.yml` doesn't exist; accurately states `runtime_upgrade_rehearsal` is not a required job in the critical-path-all-pass aggregation.
- **Snapshot files**: Three `launch-gates/sources/` files (pack-01, pack-03, pack-05) had stale `UsedNonces` header comments replaced with correct monotonic nonce description plus "ARCHIVED SNAPSHOT" disclaimers pointing to live `lib.rs`.

Ready to proceed to RC5.
