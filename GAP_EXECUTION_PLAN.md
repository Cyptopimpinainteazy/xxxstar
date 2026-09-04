# X3 Gap Execution Plan
**Generated:** 2026-05-19  
**Based on:** Cross-Chain Token System Completion Assessment + live repo inspection  
**Scope:** Internal product integration gaps (UAK ✓ core, Launchpad, DEX graduation, LP lock, anti-rug, route limits, nonce model) and build-breaking workspace issues

---

## 0. Critical Build Fixes (do first, blocks everything else)

### 0-A. `crates/x3-liquidity-core` workspace member is missing

**Status:** **BUILD BREAKING**  
**Evidence:** `Cargo.toml` line 168 lists `"crates/x3-liquidity-core"` as a workspace member. The directory `crates/x3-liquidity-core/` does **not exist** in the main workspace — it only exists in `.rc4-worktrees/old/crates/x3-liquidity-core/`. `tests/e2e/Cargo.toml` and `pallets/x3-cross-vm-router/Cargo.toml` also reference `path = "../../crates/x3-liquidity-core"`.  
**Fix:** Restore the crate from `.rc4-worktrees/old/crates/x3-liquidity-core/` into `crates/x3-liquidity-core/`, or remove the workspace member entry and all `path`-referencing dependents if it is truly deferred.

```bash
# Option A: restore
cp -r .rc4-worktrees/old/crates/x3-liquidity-core crates/x3-liquidity-core

# Option B: remove (if deferring)
# 1. Remove line 168 from Cargo.toml
# 2. Comment out x3-liquidity-core dep in pallets/x3-cross-vm-router/Cargo.toml
# 3. Comment out x3-liquidity-core dep in tests/e2e/Cargo.toml
```

**Owner:** unassigned  
**Effort:** S (1–2 h)  
**Priority:** P0 — workspace won't build cleanly without this

---

## 1. Route Limits: Daily + Per-Wallet Enforcement

**Status:** **Schema declared, enforcement missing**  
**Evidence:**  
- `RouteLimits` in `crates/x3-asset-kernel-types` has `daily_limit` and `per_wallet_daily_limit` fields.  
- `pallets/x3-asset-registry` validates `daily_limit >= max_amount` at route configuration time (line 393).  
- `pallets/x3-cross-vm-router/src/lib.rs` `do_initiate_transfer` (line 835) checks `min_amount`/`max_amount` and `pending_limit` only. **No daily_limit or per_wallet_daily_limit check anywhere in router execution path.**

**Required work:**

1. Add `DailyVolume` storage map to `pallet-x3-cross-vm-router`:
   ```
   DailyVolume: StorageDoubleMap<_, Blake2_128Concat, (AssetId, DomainId, DomainId), Blake2_128Concat, u32 /* day_index */, u128>
   WalletDailyVolume: StorageNMap<_, (AssetId, DomainId, DomainId, AccountBytes, u32), u128>
   ```
2. In `do_initiate_transfer`, after amount/pending checks, compute `day_index = frame_system::Pallet::<T>::block_number() / T::BlocksPerDay::get()` and:
   - Read `DailyVolume[(asset, src, dst)][day_index]`, ensure `vol + amount <= route.limits.daily_limit`.
   - Read `WalletDailyVolume[(asset, src, dst, sender, day_index)]`, ensure `wallet_vol + amount <= route.limits.per_wallet_daily_limit`.
3. In `do_initiate_transfer` success path, increment both counters.
4. Add `T::BlocksPerDay` to pallet `Config`.
5. Add tests:
   - `daily_limit_enforced_across_multiple_transfers`
   - `per_wallet_daily_limit_enforced_per_sender`
   - `daily_counters_reset_after_day_boundary`

**Owner:** unassigned  
**Effort:** M (1–2 days)  
**Priority:** P1 — route-control story is incomplete without this; routes can be unlimited if operator sets `daily_limit = u128::MAX` today

---

## 2. Nonce Model: Authoritative Decision

**Status:** **Duplicated, stale docs**  
**Evidence:**  
- Router `lib.rs` header and `CURRENT_MAINNET_STATUS.md` both mention `UsedMessages + UsedNonces`.  
- Actual implementation uses `UsedMessages + NextNonce / NonceBatchAllocation` (monotonic). The comment in router explicitly says `UsedNonces` map is intentionally absent.  
- `pallets/x3-account-registry` has a separate `CrossVmNonces` storage and `anchor_nonce` extrinsic.  
- Router does **not** consume `CrossVmNonces`.

**Decision required (pick one):**

| Option | Description |
|--------|-------------|
| **A (keep router nonces)** | Router `NextNonce` is authoritative. Remove `CrossVmNonces` from account registry, or document it as application-layer only. Update status docs and router header comment. |
| **B (unify via account registry)** | Router consumes `AccountRegistry::anchor_nonce` for all sender dedup. Remove router's own `NextNonce`/`NonceBatchAllocation` storage. |

**Recommended:** Option A. The router monotonic nonce scheme is already implemented and tested; merging it with account registry adds coupling with no new safety. Delete the duplicated concept.

**Required work (Option A):**
1. Remove or rename `CrossVmNonces` + `anchor_nonce` in `pallets/x3-account-registry` (or clearly comment it as non-consensus application layer).
2. Update router `lib.rs` header to remove stale `UsedNonces` references.
3. Update `CURRENT_MAINNET_STATUS.md` replay protection section.
4. Add doc comment to `NextNonce` storage clarifying it replaces per-sender nonce dedup.

**Owner:** unassigned  
**Effort:** S (2–4 h)  
**Priority:** P1 — no safety gap today, but stale docs will mislead auditors

---

## 3. Launchpad → Token Factory Integration

**Status:** **Completely disconnected scaffold**  
**Evidence:**  
- `pallets/x3-launchpad/src/lib.rs` Cargo description: "Phase 7 scaffold".  
- `LaunchState` has `token_asset_id: u32` but stores it as a plain field; no `T::TokenFactory` trait bound in `Config`. No call to token factory in `open_launch`, `close_launch`, or `claim`.  
- `pallets/x3-launchpad/Cargo.toml` has no dependency on `pallet-x3-token-factory` or `x3-asset-kernel-types`.  
- Current `close_launch` just changes status to `Successful`/`Failed`. It does not mint tokens, create escrow, or call DEX.

**Required work:**

### 3-A: Token factory trait coupling
1. Add `type TokenFactory: x3_token_factory_traits::CreateToken + MintTo` to `pallet-x3-launchpad` `Config`.
2. Add `x3-asset-kernel-types` and the token factory traits crate to `pallets/x3-launchpad/Cargo.toml`.
3. In `open_launch` (or a new `create_launch` that wraps it), call `T::TokenFactory::create_token(LaunchTokenConfig { ... })` to register the token before accepting contributions. Store the returned `AssetId` in `LaunchState`.
4. Guard `contribute` to only accept funds after token is registered.

### 3-B: Graduation trigger on close
In `close_launch` (or the on-initialize auto-finalize path), on `Successful`:
1. Call `T::TokenFactory::mint_to(asset_id, launch_escrow_account, total_tokens_sold)`.
2. Call `T::Dex::create_pool(token_asset, base_asset, initial_token_liquidity, initial_base_liquidity, fee_bps)`.
3. Emit `LaunchGraduated { launch_id, pool_id, asset_id }`.
4. Transition status to `Completed`.

### 3-C: Claim logic
Update `claim` to transfer tokens from `launch_escrow_account` to contributor proportionally using `T::Assets::transfer`, rather than a numeric-only allocation.

**Owner:** unassigned  
**Effort:** L (3–5 days including tests)  
**Priority:** P0-Product — without this, the launchpad is not a launch product

---

## 4. Pallet-Backed LP Lock Storage

**Status:** **In-memory only, no pallet exists**  
**Evidence:**  
- `crates/x3-liquidity-core/src/anti_rug.rs` (in `.rc4-worktrees/old`) explicitly states: *"This is an in-memory registry used by the CLI and devnet harness. The production on-chain variant lives in a pallet StorageMap."*  
- No `pallets/x3-lp-locker` or equivalent exists anywhere in the workspace.  
- Anti-rug `LpLockRegistry::lock/get/withdraw` operate on `BTreeMap`, not chain storage.

**Required work:**

### 4-A: Create `pallets/x3-lp-locker`
New pallet with:
```
LpLocks: StorageDoubleMap<_, Blake2_128Concat, AccountId, Blake2_128Concat, PoolId, LpLockRecord>
LocksByPool: StorageMap<_, Blake2_128Concat, PoolId, BoundedVec<(AccountId, u128), T::MaxLocksPerPool>>
```
```rust
pub struct LpLockRecord {
    pub lp_amount: u128,
    pub unlock_at_block: BlockNumberFor<T>,
    pub locked_at_block: BlockNumberFor<T>,
}
```
Extrinsics:
- `lock_lp(pool_id, lp_amount, unlock_at_block)` — transfers LP tokens to pallet escrow account, writes storage.
- `unlock_lp(pool_id)` — checks `current_block >= unlock_at_block`, returns LP tokens.
- `extend_lock(pool_id, new_unlock_at_block)` — only allows extending, never shortening.

Events: `LpLocked { who, pool_id, amount, unlock_at_block }`, `LpUnlocked { who, pool_id, amount }`, `LockExtended { who, pool_id, old_unlock, new_unlock }`

### 4-B: Wire locker into graduation
In the graduation path (§3-B above), after `create_pool`, call `T::LpLocker::lock_lp(pool_id, initial_lp_amount, current_block + T::DefaultLockDuration::get())`.

### 4-C: Anti-rug score as runtime query
Add a `#[pallet::call]` or a `fn compute_rug_score(pool_id)` runtime API that reads `LpLocks` + DEX pool data and returns a `RugScoreResult` on-chain (or at least as an RPC endpoint).

**Owner:** unassigned  
**Effort:** M (2–3 days)  
**Priority:** P0-Product — anti-rug is a named product feature; without on-chain locks it is not enforceable

---

## 5. Graduation as Atomic On-Chain State Transition

**Status:** **Library logic only, no pallet execution path**  
**Evidence:**  
- `crates/x3-liquidity-core/src/launchpad.rs` (`.rc4-worktrees/old`) has `check_graduation(config, state)` and `execute_graduation(config, state, dex)` as pure functions.  
- `execute_graduation` returns an `AMMPoolRequest` but does not call any pallet; it is a helper for CLI/devnet.  
- No runtime hook drives graduation check at end-of-block or on a threshold event.

**Required work:**

### 5-A: Add `GraduationConfig` storage to `pallet-x3-launchpad`
```
LaunchGraduationConfig: StorageMap<_, Blake2_128Concat, LaunchId, GraduationConfig>
```
Set during `open_launch` from config or governance.

### 5-B: `on_initialize` graduation check
In `pallet-x3-launchpad::on_initialize`, for every `Active` launch:
1. Read `GraduationConfig` for the launch.
2. Read `LaunchState` (tokens_sold, total_raised, contributor_count, current_block).
3. If `check_graduation(config, state)` passes, call `do_graduate(launch_id)`.

### 5-C: `do_graduate` (atomic)
```
fn do_graduate(launch_id: LaunchId) -> DispatchResult {
    // 1. Call token factory to mint final token supply
    // 2. Call DEX to create pool with raised funds + token allocation
    // 3. Call LP locker to lock initial LP (§4-B)
    // 4. Transition LaunchState.status = Completed
    // 5. Emit LaunchGraduated
}
```
Everything in a `with_transaction` block to ensure atomicity.

**Owner:** unassigned  
**Effort:** L (2–4 days — depends on §3 and §4 being done first)  
**Priority:** P0-Product (sequentially after §3 and §4)

---

## 6. DEX Runtime Wiring Verification

**Status:** **Ambiguous — pallet exists, critical-path inclusion not verified**  
**Evidence:**  
- `pallets/x3-dex` exists and is in `Cargo.toml` workspace.  
- Corrected wiring audit (`launch-gates/reports/audit-01-wiring-CORRECTED.json`) does NOT list `X3Dex` in the verified critical-path runtime set (alongside `X3AssetRegistry`, `X3SupplyLedger`, `X3CrossVmRouter`, `X3TokenFactory`).  
- `crates/x3-dex` has advanced modules but runtime wiring for even basic spot AMM is unverified.

**Required work:**
1. Run `cargo test -p pallet-x3-dex -- --nocapture` and confirm tests pass.
2. Check `runtime/src/lib.rs` for `impl pallet_x3_dex::Config for Runtime` and that it appears in `construct_runtime!`.
3. Run a node-level smoke test: start dev node, submit `x3Dex.createPool` extrinsic via Polkadot.js, verify pool created.
4. If missing from runtime, add to `construct_runtime!` and implement `Config`.
5. Regenerate wiring audit after fix.

**Owner:** unassigned  
**Effort:** S–M (4 h to 1 day depending on wiring state)  
**Priority:** P1 — graduation pipeline (§5) depends on DEX being runtime-callable

---

## 7. Stale Documentation Sync

**Status:** **Known stale, misleads auditors**  
**Evidence:**  
- Router `lib.rs` header mentions `UsedNonces` (line ~20) but implementation uses `NextNonce` + `UsedMessages`.  
- `CURRENT_MAINNET_STATUS.md` and other status docs reference `UsedMessages + UsedNonces`.  
- `README.md` and status docs reference `.github/workflows/ci.yml` but visible workflow is `.github/workflows/build.yml`.

**Required work (low effort, high audit value):**
1. Update router `lib.rs` header comment: replace "UsedNonces" with "NextNonce/NonceBatchAllocation (monotonic)" and explain the deliberate design.
2. Update `CURRENT_MAINNET_STATUS.md` replay protection section to match.
3. Update `README.md` CI badge and workflow references from `ci.yml` to `build.yml`.
4. Confirm whether `ci.yml` exists at all; if not, create a minimal redirect or rename note.

**Owner:** unassigned  
**Effort:** XS (2–4 h)  
**Priority:** P2 — no functional impact, but stale docs cause audit findings

---

## Execution Order

```
Week 1
 [0-A] Restore x3-liquidity-core crate (unblocks build)        P0 / S
 [7]   Fix stale doc references                                 P2 / XS
 [2]   Decide nonce model, clean up CrossVmNonces               P1 / S

Week 2
 [6]   Verify + runtime-wire pallet-x3-dex                     P1 / S-M
 [1]   Implement daily/per-wallet route limit enforcement        P1 / M

Week 3–4
 [3-A+B+C]  Launchpad → token factory integration               P0 / L
 [4-A+B+C]  pallet-x3-lp-locker creation + graduation wiring    P0 / M

Week 5–6
 [5-A+B+C]  Atomic graduation on-chain state transition         P0 / L
```

---

## Success Criteria (per gap)

| Gap | Done when |
|-----|-----------|
| 0-A | `cargo check` clean on root workspace; no missing-path errors |
| 1   | `cargo test -p pallet-x3-cross-vm-router daily_limit` green; daily cap rejects transfers over threshold |
| 2   | Single authoritative nonce concept in router; status docs match code |
| 3   | `cargo test -p pallet-x3-launchpad close_launch_mints_tokens` green; node-level smoke: open→contribute→close produces on-chain token balance |
| 4   | `cargo test -p pallet-x3-lp-locker` green; LP tokens locked at graduation, unlock blocked before `unlock_at_block` |
| 5   | `cargo test -p pallet-x3-launchpad graduation_triggers_atomically` green; graduation fails atomically if DEX rejects pool |
| 6   | `X3Dex` present in `construct_runtime!`; node-level createPool extrinsic succeeds |
| 7   | `grep UsedNonces pallets/x3-cross-vm-router/src/lib.rs` returns 0 stale references; `ci.yml` reference removed from README |

---

## Out of Scope (explicitly deferred per RC-1 freeze)

- External gateway activation (Base/Ethereum/Solana): compile-gated behind `external-gateway` feature; do not lift until post-audit.
- `crates/x3-crosschain-gateway` runtime wiring.
- `crates/x3-finality-oracle` production chain-specific proof ingestion.
- `crates/x3-relayer` public testnet deployment.
- Advanced DEX (concentrated liquidity, perps, options): deferred to DEX v0.2+.
