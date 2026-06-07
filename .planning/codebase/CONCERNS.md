# Codebase Concerns

**Analysis Date:** 2026-05-19

> This document is the GPS map to mainnet. Every item below is a gap between the current codebase and a shippable testnet/mainnet.
> Items are ordered by priority. P0 = blocks testnet launch. P1 = blocks mainnet. P2 = hardening.

---

## P0 — Testnet Blockers (Cannot Ship Without These)

---

### P0-CRITICAL [Workspace-Gap]: 91 Crates on Disk Are Excluded from Workspace

- **Issue:** `crates/` directory contains **124 crate directories**. The root `Cargo.toml` workspace `[members]` lists only **33**. This means 91 crates (including critical ones like `x3-ast`, `x3-ir`, `x3-bridge`, `custody-service`, `apotheosis-tx`, etc.) are on disk but **never compiled by `cargo check --workspace`**. These crates are unknown to the compiler, cannot be depended on by workspace members, and are invisible to CI.
- **Files:** `Cargo.toml` `[workspace.members]` section; `crates/` directory
- **Evidence:** `ls crates/ | wc -l` → 124; `grep '"crates/' Cargo.toml | wc -l` → 33; diff shows 91 dirs not in workspace.
- **Impact:** If any pallet or runtime imports a crate from the 91 excluded crates, the build fails with "package not found". Features advertised in docs/specs that depend on excluded crates are non-functional.
- **Fix approach — Option A (add to workspace):** Add the critical excluded crates to `[workspace.members]` in `Cargo.toml`. Each must have a valid `Cargo.toml`. Verify each compiles after addition. Start with those imported by active pallets.
- **Fix approach — Option B (prune dead crates):** Audit which of the 91 excluded crates are actually imported by pallets/runtime. If a crate is not imported anywhere, archive or delete it. Only add genuinely needed crates to the workspace.
- **Recommended:** Option B first (audit imports), then Option A for needed crates. Do not add all 91 blindly — many may be stale or broken.

---

### P0-CRITICAL [Gap-0A]: x3-liquidity-core Missing

- **Issue:** `crates/x3-liquidity-core/` is listed as workspace member. Directory does not exist. Referenced by `pallets/x3-cross-vm-router/Cargo.toml` and `tests/e2e/Cargo.toml`. LiquidityCore traits are used by cross-vm-router for settlement bounds validation (line 1010 in router).
- **Files:** `Cargo.toml` line ~168; `pallets/x3-cross-vm-router/Cargo.toml`; `pallets/x3-cross-vm-router/src/lib.rs` line ~1010
- **Impact:** Router pallet cannot compile. Cross-VM routing (the core product feature) is not buildable.
- **Fix:** Copy from `.rc4-worktrees/old/crates/x3-liquidity-core/` to `crates/x3-liquidity-core/`; verify it compiles; add to workspace member list.
- **Timeline:** Must be fixed before any other pallet work.

---

### P0-CRITICAL [Gap-3]: Launchpad → TokenFactory Completely Disconnected

- **Issue:** `pallets/x3-launchpad/src/lib.rs` is described as "Phase 7 scaffold." It has zero calls to `pallet-x3-token-factory`. No `T::TokenFactory` trait bound in Config. `close_launch` only changes `LaunchState.status` to Successful or Failed — no token minting, no DEX pool creation, no distribution logic, no claim mechanism.
- **Files:** `pallets/x3-launchpad/src/lib.rs`; `pallets/x3-token-factory/src/lib.rs`
- **Evidence:** `grep -n "TokenFactory\|token_factory\|mint\|graduate\|dex\|DEX" pallets/x3-launchpad/src/lib.rs` → **zero matches**
- **Impact:** X3 Launchpad is the primary user-facing feature for token launches. As-is it cannot create tokens when a presale closes. Product gap — not a compile error, but a critical feature gap.
- **Fix:** 
  1. Add `type TokenFactory: x3_token_factory::MintTokens<Self>` to `Config` in launchpad
  2. In `open_launch`: call `T::TokenFactory::register_pending_token(...)` 
  3. In `close_launch` success path: call `T::TokenFactory::mint(...)` to create OmniToken
  4. Wire DEX pool creation: call `T::Dex::create_initial_pool(token_id, liquidity)` 
  5. Add claim extrinsic for contributors
  6. Wire into runtime Config with real implementations

---

### P0-CRITICAL [Gap-4]: LP Locker Pallet Does Not Exist

- **Issue:** `MAINNET_RC1_SCOPE.md` describes LiquidityCore LP lock as an RC-1 feature ("kernel invariants: LP locked after graduation"). No `pallets/x3-lp-locker/` directory exists anywhere. The anti_rug logic in `.rc4-worktrees/old/crates/x3-liquidity-core/src/anti_rug.rs` uses in-memory `BTreeMap`, not chain storage — cannot survive node restarts.
- **Files:** No file — the pallet does not exist
- **Impact:** LP token locks cannot be enforced on-chain. Anti-rug protection is non-functional for production.
- **Fix:** Create `pallets/x3-lp-locker/` with:
  - `LockedLp: StorageMap<PoolId, LockRecord>` where `LockRecord = { owner, amount, unlock_block }`
  - `lock_lp(origin, pool_id, amount, unlock_period)` extrinsic
  - `unlock_lp(origin, pool_id)` extrinsic (only after unlock_block)
  - `is_locked(pool_id) -> bool` read function
  - Wire into launchpad graduation: auto-lock LP on `close_launch` success

---

### P0-CRITICAL [Gap-5]: Graduation Auto-Finalize Not Wired

- **Issue:** Launchpad graduation (auto-close when target met, auto-create DEX pool) requires an `on_initialize` hook scanning for matured launches. This does not exist. There is also no manual `finalize_launch` path that handles DEX pool creation.
- **Files:** `pallets/x3-launchpad/src/lib.rs`
- **Impact:** Successful raises would sit in `Successful` state indefinitely with no token creation or DEX listing.
- **Fix:**
  1. In launchpad `Hooks::on_initialize`: scan `ActiveLaunches` for block_number >= end_block AND status == Open AND raised >= target
  2. Trigger graduation: mint tokens, create DEX pool, lock LP, distribute to contributors
  3. Set block weight budget for scan (O(n) scans are dangerous — limit per block or use a work queue)

---

### P0 [x3-lang Compiler 3 Crates Broken]: Lexer, VM, Compiler Cannot Build

- **Issue:** In `.kilo/worktrees/silky-petalite/x3-lang/crates/`:
  - `x3-lang-lexer`: missing `src/cursor.rs` and `src/lexer.rs`; crate has `mod cursor;` and `mod lexer;` but files don't exist
  - `x3vm`: 49× E0117 orphan trait violations (implementing external traits on external types)
  - `x3-lang-compiler`: depends on broken `x3vm`; will not compile
- **Files:** `.kilo/worktrees/silky-petalite/x3-lang/crates/x3-lang-lexer/src/`, `.kilo/worktrees/silky-petalite/x3-lang/crates/x3vm/`, `.kilo/worktrees/silky-petalite/x3-lang/crates/compiler/`
- **Impact:** x3-lang compiler stack cannot produce X3IR. No custom contract language support until fixed.
- **Fix:** 
  - For lexer: create `cursor.rs` (char-by-char cursor over source) and `lexer.rs` (token scanner)
  - For x3vm: use newtype wrapper pattern or define trait in own crate to avoid orphan rule violations
  - For compiler: unblocked after vm is fixed

---

## P1 — Mainnet Blockers (Must Fix Before Open Participation)

---

### P1 [Gap-1]: Route Limits Not Enforced (Daily / Per-Wallet)

- **Issue:** `RouteLimits` struct in `pallets/x3-cross-vm-router/src/lib.rs` has `daily_limit: Balance` and `per_wallet_daily_limit: Balance` fields, but `do_initiate_transfer` never checks them. There is no `DailyVolume: StorageMap<Route, Balance>` or `WalletDailyVolume: StorageDoubleMap<(Route, AccountId), Balance>` storage.
- **Files:** `pallets/x3-cross-vm-router/src/lib.rs`
- **Evidence:** `grep -n "DailyVolume\|per_wallet_daily\|daily_limit" pallets/x3-cross-vm-router/src/lib.rs` → no matches in enforcement code
- **Impact:** Router has no rate limiting. An attacker can route unlimited volume per day, exhausting liquidity pools.
- **Fix:**
  1. Add `DailyVolume: StorageMap<(RouteId, T::BlockNumber /* day */), T::Balance>` 
  2. Add `WalletDailyVolume: StorageDoubleMap<(RouteId, T::BlockNumber), T::AccountId, T::Balance>`
  3. In `do_initiate_transfer`: check `DailyVolume + amount <= route.limits.daily_limit`; check `WalletDailyVolume + amount <= route.limits.per_wallet_daily_limit`
  4. Add reset mechanism: subtract when day bucket rolls over (use `frame_system::Pallet::<T>::block_number() / T::BlocksPerDay::get()` as day key)
  5. Add tests for: daily limit hit, per-wallet limit hit, limit resets correctly

---

### P1 [Gap-2]: Nonce Model Duplicated (Router vs AccountRegistry)

- **Issue:** Two parallel nonce systems exist:
  - `pallet-x3-cross-vm-router`: `NextNonce: StorageMap<AccountId, u64>` + `NonceBatchAllocation: StorageMap<(AccountId, u64), bool>` + replay protection via `UsedMessages: StorageMap<MessageHash, ()>`
  - `pallet-x3-account-registry`: `CrossVmNonces: StorageMap<AccountId, u64>` + `anchor_nonce` extrinsic
  - Router does NOT consume `CrossVmNonces` from account-registry. Two separate counters per account.
- **Files:** `pallets/x3-cross-vm-router/src/lib.rs`, `pallets/x3-account-registry/src/lib.rs`
- **Impact:** External tools (explorers, wallets, SDKs) that query the account-registry nonce will submit transactions with wrong nonces. Nonce drift causes rejected transactions and confused state.
- **Fix — Option A (recommended):** Keep router's own nonce system as canonical. Remove `CrossVmNonces` and `anchor_nonce` from account-registry. Update docs to say router nonce is authoritative for cross-VM ops.
- **Fix — Option B:** Make router consume `T::CrossVmNonceProvider::get_and_increment(account)` via trait, implemented by account-registry. Requires Config trait change.
- **Coordination:** Whichever approach is chosen, wallets and SDKs must be updated to query the canonical source.

---

### P1 [Zero Tests — fraud-proofs, northern-swarm, pallet-x3-control]:

- **Issue:** Three pallets wired into `construct_runtime!` have zero tests:
  - `pallets/fraud-proofs/` — 0 tests
  - `pallets/northern-swarm/` — 0 tests
  - `pallets/pallet-x3-control/` — 0 tests
- **Files:** `pallets/fraud-proofs/src/`, `pallets/northern-swarm/src/`, `pallets/pallet-x3-control/src/`
- **Impact:** Any regression in these pallets goes undetected. Fraud proofs are a critical security mechanism. X3-control manages emergency powers.
- **Fix:** Add `mock.rs` + `tests.rs` to each pallet with at minimum:
  - Happy path for every public extrinsic
  - Failure path for invalid origin / unauthorized caller
  - Error path for each custom Error variant

---

### P1 [CI Coverage Gap]: Integration Tests Never Run in CI

- **Issue:** `.github/workflows/ci.yml` runs `cargo check` on the 33-member workspace. This compiles but does NOT run the integration tests under `integration-tests/` because that directory has no `Cargo.toml` and is not a workspace member. Four test files (`cross-vm-atomic-test.rs`, `cross-vm-pallet-test.rs`, `parallel-proposer-integration.rs`, `svm-counter-test/`) exist as loose files that `cargo test` never touches.
- **Files:** `integration-tests/`; `.github/workflows/ci.yml`
- **Impact:** Critical cross-VM atomicity regressions are invisible to CI. The most important correctness guarantee (atomic cross-VM settlement) has no CI coverage.
- **Fix:** Create `integration-tests/Cargo.toml` declaring it as a workspace member with `[[test]]` targets; add `cargo test -p integration-tests` to CI workflow.

---

### P1 [Runtime RC+1 Extensions Commented Out]:

- **Issue:** `runtime/src/lib.rs` lines 822–824 have three signed extension hook points commented out:
  ```rust
  // CapabilityEnvelopeCheck,   // RC+1
  // AtomicSettlementCheck,     // RC+1
  // FlashFinalityExtension,    // RC+1
  ```
  And line 1595: `// TODO: wire to SwarmEventBroadcaster once the security swarm subscriber is live.`
- **Files:** `runtime/src/lib.rs`
- **Impact:** These extensions handle capability envelope validation, atomic settlement verification, and flash finality proofs. Without them, the runtime's security guarantees are weaker than described in the whitepaper.
- **Fix:** Each must be implemented as a `SignedExtension` impl, tested, and uncommented before mainnet. They are intentionally deferred to RC+1 — track as P1 tasks for RC+1 milestone.

---

### P1 [main_stub.rs Present]:

- **Issue:** `node/src/main_stub.rs` exists alongside `node/src/main.rs`. Both define a `main` function or entry point. The stub must not be reachable in production builds — having two `main` modules can cause unexpected behavior if a future `mod main_stub;` is added by mistake.
- **Files:** `node/src/main_stub.rs`, `node/src/main.rs`
- **Fix:** Delete `node/src/main_stub.rs` or gate it entirely behind `#[cfg(test)]` if it serves a testing purpose.

---

### P1 [Gap-6]: DEX Runtime Wiring Unverified

- **Issue:** `X3Dex` is wired into all 4 `construct_runtime!` blocks in `runtime/src/lib.rs`. However, the end-to-end path of: provide liquidity → swap → receive output — via `LiquidityCore` spot swap — has not been verified to work with a running chain. `pallets/x3-dex/` has 27 tests but they are unit tests; no integration test exercises the full swap path.
- **Files:** `runtime/src/lib.rs`, `pallets/x3-dex/src/lib.rs`, `crates/x3-liquidity-core/` (currently missing)
- **Impact:** DEX is a core product feature. If the runtime Config binding for LiquidityCore is wrong, swaps fail silently or panic.
- **Fix:** 
  1. Restore `crates/x3-liquidity-core/` (unblocked by Gap-0A fix)
  2. Write an integration test that runs a full liquidity add → swap → verify output cycle
  3. Add that test to CI as a required gate

---

## P2 — Hardening (Required Before Public Testnet)

---

### P2 [apps/explorer Empty]:

- **Issue:** `apps/explorer/` contains only `package.json` and `package-lock.json`. No source code. Block explorer is essential for users to inspect transactions, balances, and cross-VM routing history.
- **Files:** `apps/explorer/` (all files in it)
- **Impact:** No user-facing chain exploration. Validators and users cannot inspect finalized state without external tooling.
- **Fix:** Implement basic block explorer using Next.js App Router:
  1. Connect to Substrate JSON-RPC (`@polkadot/api`)
  2. Show block list, block detail, extrinsic detail
  3. Show account balances, transfer history
  4. Show cross-VM routing events

---

### P2 [Sparse Test Coverage — Critical Pallets]:

- **Issue:** Pallets with very few tests despite complex logic:
  - `pallet-evolution-core`: 1 test (evolution logic is complex AI governance)
  - `pallet-meme-overlord`: 1 test
  - `pallet-x3-verifier`: 1 test (verifies proofs — security-critical)
  - `pallet-x3-slash`: 3 tests (slashing — directly affects validator stake)
- **Files:** `pallets/evolution-core/src/tests.rs`, `pallets/meme-overlord/src/tests.rs`, `pallets/x3-verifier/src/tests.rs`, `pallets/x3-slash/src/tests.rs`
- **Impact:** Regressions in verifier, slash, and evolution logic go undetected.
- **Fix:** Add failure-path tests, edge case tests, and invariant-preservation tests to each.

---

### P2 [Integration Tests Not Wired]:

- **Issue:** `integration-tests/cross-vm-atomic-test.rs`, `cross-vm-pallet-test.rs`, `parallel-proposer-integration.rs`, `svm-counter-test/` exist as loose `.rs` files with no `Cargo.toml`. `cargo test` does not run them.
- **Files:** `integration-tests/` (all files)
- **Impact:** Critical cross-VM paths are not tested in CI. Regressions in cross-VM atomicity are invisible.
- **Fix:** Create `integration-tests/Cargo.toml` to make it a workspace member; or move tests into `tests/` directory of relevant pallets.

---

### P2 [FEATURE_REGISTRY.toml Stale]:

- **Issue:** Every entry in `FEATURE_REGISTRY.toml` has `readiness_score = 0`. Many features are actually implemented (UAK, router, settlement engine, token factory). Registry is completely stale.
- **Files:** `FEATURE_REGISTRY.toml`
- **Impact:** Incorrect readiness signaling. Teams using this for release decisions get false information.
- **Fix:** Audit each registry entry against actual pallet code and update scores honestly using the [0-100] scale.

---

### P2 [Gap-7]: Stale Docs Reference Non-Existent Storage

- **Issue:** `CURRENT_MAINNET_STATUS.md` and cross-vm-router header comments reference `UsedNonces: StorageMap<>` which no longer exists in the router. The router uses `UsedMessages: StorageMap<MessageHash, ()>` and `NextNonce` instead.
- **Files:** `CURRENT_MAINNET_STATUS.md`, `pallets/x3-cross-vm-router/src/lib.rs` (header comment)
- **Impact:** Developers and auditors are misled about the nonce/replay protection mechanism.
- **Fix:** Update docs to reference correct storage names.

---

### P2 [Hardhat Config Placeholder Values]:

- **Issue:** `hardhat.config.ts` at root contains `<RPC_ENDPOINT_1>` through `<RPC_ENDPOINT_103>` and `<PRIVATE_KEY>` as literal placeholder strings. This is not a functional deploy config.
- **Files:** `hardhat.config.ts`
- **Impact:** EVM contract deployment via Hardhat is non-functional. No actual chain deployments can be scripted.
- **Fix:** Either replace with real testnet RPC endpoints and env var references (`process.env.PRIVATE_KEY`), or mark the file as example-only.

---

## P3 — Security Observations

---

### P3 [3 unwrap() in Critical Paths]:

- **Issue:** 3 bare `unwrap()` or `expect()` calls found in pallet critical paths. In Substrate, panicking inside a pallet extrinsic causes the block production to fail and can cause validator isolation.
- **Files:** `pallets/x3-cross-vm-router/src/lib.rs`, `pallets/x3-atomic-kernel/src/lib.rs`, `pallets/x3-supply-ledger/src/lib.rs` (exact lines not recorded — re-run `grep -n 'unwrap()\|expect(' pallets/*/src/lib.rs`)
- **Impact:** If the unwrap target is None/Err in production, the node panics mid-block. GRANDPA validators go out of consensus.
- **Fix:** Replace each `unwrap()` with `ok_or(Error::<T>::SomeError)?` or `unwrap_or(default)` as appropriate.

---

### P3 [SVM Disabled — Dependency Conflict]:

- **Issue:** SVM integration (`pallets/svm-runtime/`) is in `DISABLED_BLOCKED` state. Known cause: `solana-address` crate conflict with Substrate workspace (multiple incompatible versions of a transitive dependency).
- **Files:** `pallets/svm-runtime/`, `TESTNET_FEATURE_FLAGS.toml`
- **Impact:** X3 cannot execute SVM/Solana programs on-chain. SVM programs at `programs/` have no execution path.
- **Fix:** Audit `pallets/svm-runtime/Cargo.toml` for conflicting deps; try `cargo update --precise` or `[patch.crates-io]` overrides to force compatible versions.

---

### P3 [BTC Gateway Disabled]:

- **Issue:** `btc_mainnet_gateway = "DISABLED_BLOCKED"`. BTC bridge has no implementation visible at root (no `pallets/btc-*` directory). BTC fortress is SIM_TESTNET only.
- **Files:** `TESTNET_FEATURE_FLAGS.toml`
- **Impact:** No BTC bridge for RC-1 or near-term mainnet. This is by design per `MAINNET_RC1_SCOPE.md` but must be tracked.
- **Fix:** Defined as post-RC1 work. Track in backlog.

---

## Mainnet Readiness Assessment

Based on this analysis, the path to a stable testnet requires resolving all P0 items first:

1. **Fix root Cargo.toml** — restore 80–90 missing crates from `.rc4-worktrees/old/crates/` OR prune phantom entries
2. **Restore x3-liquidity-core** — critical dependency for router and DEX
3. **Connect launchpad → token factory** — core product feature
4. **Create LP locker pallet** — required for LP lock invariant
5. **Wire graduation auto-finalize** — required for launchpad to work end-to-end
6. **Fix x3-lang lexer** — required for custom language to compile

After P0 resolution, the chain can be started locally and P1 items pursued:
- Daily/per-wallet limits in router
- Nonce model reconciliation
- Tests for zero-test pallets
- RC+1 signed extensions tracked for mainnet milestone

---

*Concerns audit: 2026-05-19*
