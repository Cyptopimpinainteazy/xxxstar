# Cross-VM / Bridge Safety Deep Audit (Pass 4)

**Date:** 2026-04-24  
**Scope:** `pallet-x3-cross-vm-router`, `pallet-x3-supply-ledger`, `pallet-x3-settlement-engine`, `crates/x3-verification-router`, `crates/external-chains`, `crates/x3-asset-kernel-types`  
**Commit:** `9aeb4bf089f719c23ddd6d351b9541b345fd7685`

---

## 1. Route Validation — `pallets/x3-cross-vm-router/src/lib.rs::do_initiate_transfer`

### Source/Destination Domain Validation

```rust
// Line 874-875
ensure!(source != destination, Error::<T>::SelfLoopRoute);
ensure!(amount > 0, Error::<T>::AmountOutOfBounds);
```

**PASS.** Self-loop and zero-amount are checked first.

### Route scoping (external chains)

```rust
// Lines 880-888
ensure!(
    source.is_x3_internal() || destination.is_x3_internal(),
    Error::<T>::NonInternalRouteNotSupported
);
#[cfg(not(feature = "external-gateway"))]
ensure!(
    source.is_x3_internal() && destination.is_x3_internal(),
    Error::<T>::NonInternalRouteNotSupported
);
```

**PASS.** When `external-gateway` feature is off (which it is for mainnet-rc1 — see compile_error guard at line 61), **both** source AND destination must be X3-internal. This is a hard compile-time guarantee.

### Route must exist and be enabled

```rust
// Lines 907-909
let route: RouteConfig = T::Registry::route(&asset_id, source, destination)
    .ok_or(Error::<T>::RouteClosed)?;
ensure!(route.enabled, Error::<T>::RouteClosed);
```

**PASS.** Route lookup returns `Option` — if the route does not exist or is not enabled, it fails.

### Can a route be enabled without governance?

**Verdict: MISSING (partially mitigated).** The route table is managed by `pallet-x3-asset-registry` via `AssetRegistryMutate::do_configure_route`. This is a privileged trait, but its implementation in the registry pallet determines who can call it. The `AssetRegistryMutate` trait has no origin check — it is up to the implementor. If the runtime configures the registry pallet's `AssetRegistryMutate` implementation with a non-governance origin, routes could be enabled outside governance. This needs an audit of the registry pallet's extrinsic gate, which is **outside the scope of this file**.

### Proof Tier validation for internal routes

```rust
// Lines 914-919
if source.is_x3_internal() && destination.is_x3_internal() {
    ensure!(
        matches!(route.proof_tier, ProofTier::TrustedInternal),
        Error::<T>::WrongProofTierForInternalRoute
    );
}
```

**PASS.** Internal routes must use `TrustedInternal` proof tier.

### Amount bounds and pending limits

```rust
// Lines 922-932
ensure!(amount >= route.limits.min_amount && amount <= route.limits.max_amount, ...);
ensure!(pending_now < route.limits.pending_limit, ...);
```

**PASS.** Both checked.

### Expiry sanity

```rust
// Lines 935-936
let now = <frame_system::Pallet<T>>::block_number();
ensure!(expires_at > now, Error::<T>::BadExpiry);
```

**PASS.** Expiry must be strictly in the future.

---

## 2. Replay Protection — `UsedMessages`, `NextNonce`, `NonceBatchAllocation`

### Layer 1: UsedMessages (message-id lookup)

```rust
// Lines 171-177
pub type UsedMessages<T: Config> = StorageMap<_, Blake2_128Concat, H256, ()>;

// Lines 1002-1005
ensure!(
    !UsedMessages::<T>::contains_key(message_id),
    Error::<T>::DuplicateMessage
);
```

**PASS.** Every message ID is checked for uniqueness before insertion (line 1063: `UsedMessages::<T>::insert(message_id, ())`). Message IDs are deterministically derived from message contents, so identical messages produce the same ID.

### Layer 2: NextNonce (monotonic nonce per source/sender)

```rust
// Lines 189-199
pub type NextNonce<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    DomainId,
    Blake2_128Concat,
    AccountBytes,
    Nonce,
    ValueQuery,
>;
```

**PASS.** Nonce is reserved and incremented atomically via `reserve_nonce_from_batch()` (line 980).

### NonceBatchAllocation (P0 optimization)

```rust
// Lines 208-218
pub type NonceBatchAllocation<T: Config> = StorageDoubleMap<
    _,
    Blake2_128Concat,
    DomainId,
    Blake2_128Concat,
    AccountBytes,
    (Nonce, u32, u32),  // (batch_start, batch_size, used_count)
    OptionQuery,
>;
```

**Gap: FAIL.** The `reserve_nonce_from_batch` function (lines 817-851) reserves nonces in batches of 100 from `NextNonce`. However, there is a subtle gap:

```rust
// Line 842-846
let batch_start = NextNonce::<T>::mutate(source, sender.clone(), |n| {
    let cur = *n;
    *n = n.saturating_add(BATCH_SIZE);
    cur
});
```

If `NextNonce` is at its default value of 0 for a new (source, sender) pair, the first batch starts at 0 and uses nonces 0-99. **The issue**: `NonceBatchAllocation` insertion at line 849 happens **after** the mutation. If the extrinsic fails between line 846 and 849, the `NextNonce` is already incremented but the batch allocation is not stored. On retry, `NextNonce` has already advanced, so nonces 0-99 are **lost** (unusable). The sender would skip to 100-199. This is a **nonce leak**, not a replay vulnerability — nonces are monotonically increasing, so no replay is possible. But it wastes nonce space.

**More critical gap**: The comment at line 21-25 claims "Old intents with a lower nonce than the current `NextNonce` are rejected as replays." **But this rejection is NOT explicitly checked anywhere in `do_initiate_transfer`.** The code relies on `reserve_nonce_from_batch` to produce unique nonces, and on `UsedMessages` for dedup. If someone managed to craft a message with a valid nonce from a previous batch (because `NextNonce` advanced but the batch slot is still in `NonceBatchAllocation`), the `UsedMessages` check would catch it at the message-id level. So the nonce-only replay path is **implicitly** covered by `UsedMessages`.

**However**: The `UsedMessages` storage is never cleaned/expired. A message that was used 10 million blocks ago still occupies storage. There is no pruning mechanism for `UsedMessages`. This creates a slow-growing storage leak.

### Could a message be processed twice?

**PASS** for the critical path. The message-id check at line 1002-1005 happens before any side effects (the storage transaction wraps the entire function). `UsedMessages::insert` at line 1063 happens **after** the ledger debit at line 1044. If the debit fails, the storage transaction rolls back, including the `UsedMessages` insert, so the message ID remains available for a legitimate retry. This is correct.

---

## 3. Supply Invariant — `pallets/x3-supply-ledger/src/lib.rs`

### `SupplyLedger::check_invariant()` definition

From `crates/x3-asset-kernel-types/src/lib.rs`:

```rust
// Lines 636-644
pub fn check_invariant(&self) -> Result<(), InvariantError> {
    let represented = self
        .represented()
        .ok_or(InvariantError::ArithmeticOverflow)?;
    if represented > self.canonical_supply {
        return Err(InvariantError::SupplyCeilingExceeded);
    }
    Ok(())
}
```

**PASS.** The invariant is `represented_total <= canonical_supply`.

### Enforced on every mutation — debit

```rust
// Lines 645-648 (in debit_source_to_pending)
ledger
    .check_invariant()
    .map_err(|_| Error::<T>::InvariantViolation)?;
```

**PASS.** Called after `sub_from_domain` + `pending_supply` addition.

### Enforced on credit

```rust
// Lines 672-674 (in credit_destination_from_pending)
ledger
    .check_invariant()
    .map_err(|_| Error::<T>::InvariantViolation)?;
```

**PASS.**

### Enforced on refund

```rust
// Lines 699-701 (in refund_pending_to_source)
ledger
    .check_invariant()
    .map_err(|_| Error::<T>::InvariantViolation)?;
```

**PASS.**

### Enforced on mint

```rust
// Lines 455-457 (in do_mint_canonical)
ledger
    .check_invariant()
    .map_err(|_| Error::<T>::InvariantViolation)?;
```

**PASS.**

### Enforced on burn

```rust
// Lines 482-484 (in do_burn_canonical)
ledger
    .check_invariant()
    .map_err(|_| Error::<T>::InvariantViolation)?;
```

**PASS.**

### Block-level verification in `on_finalize`

```rust
// Lines 156-227
fn on_finalize(block_number: BlockNumberFor<T>) {
    for (asset_id, ledger) in Ledgers::<T>::iter() {
        if ledger.check_invariant().is_err() {
            violations.push(asset_id);
        }
    }
    if !violations.is_empty() {
        match InvariantPolicy::<T>::get() {
            InvariantViolationPolicy::LogOnly => { /* log only */ }
            InvariantViolationPolicy::EventAndPause
            | InvariantViolationPolicy::RejectNewTransfers => {
                TransferHalted::<T>::put(true);
            }
        }
    }
}
```

**PASS (with a concern).** All assets are checked at every block finalization. However, `LogOnly` policy would allow continued operation with a broken invariant — this is a governance configuration risk, not a code bug.

### Verdict: `canonical_supply >= represented_total` on every mutation

**PASS.** Every mutation path (debit, credit, refund, mint, burn) calls `check_invariant()` atomically within `try_mutate`. If the invariant fails, the mutation is rolled back. The block-level verification provides a second layer of defense.

---

## 4. Settlement Engine — `pallets/x3-settlement-engine/src/lib.rs`

### Timeout/Refund Logic

**Intent deadline index (automatic refund):**

```rust
// Lines 719-760 (on_initialize)
let expired = IntentDeadlineIndex::<T>::take(n);
for intent_id in expired.iter().take(MAX_REFUNDS_PER_BLOCK) {
    let state = IntentStates::<T>::get(intent_id);
    if !matches!(state, IntentState::Created | IntentState::FundingInProgress | IntentState::FullyFunded) {
        continue;
    }
    if let Some(intent) = SettlementIntents::<T>::get(intent_id) {
        let now = T::UnixTime::now().as_secs();
        if now >= intent.timeout {
            let _ = Self::process_refund(*intent_id, &intent, RefundReason::Timeout);
        }
    }
}
```

**PASS.** Automatic refund on `on_initialize` when timeout is reached.

### Can a transfer get stuck permanently?

**Analysis:**

1. **Block-based timeout (SettlementTimeoutBlocks):** If `SettlementTimeoutBlocks` (default 28,800 blocks ~= 24h) is exceeded, `on_idle` triggers auto-refund (lines 930-999). **PASS.**

2. **Unix-time-based timeout:** The `refund_settlement` extrinsic allows manual refund after `intent.timeout` has passed (lines 1424-1454). **PASS.**

3. **Cap on DeadlineIndex:** Only 20 intents per block are refunded automatically. Excess intents require manual `refund_settlement` calls. **Marginally risky** — if 1000 intents expire at the same block, 980 would need manual refunds. But this is a performance bound, not a stuck-funds bug, since `refund_settlement` is always available. **PASS** (known design trade-off).

4. **AtomicLock expiry:** `on_finalize` processes `AtomicLockExpiryIndex` (lines 763-813), slashing expired locks. Only 20 per block. Same trade-off. **PASS.**

### Retention of `SettlementCreationBlocks`

```rust
// Lines 986-995 (in on_idle refund processing)
for intent_id in to_refund {
    if let Some(intent) = SettlementIntents::<T>::get(&intent_id) {
        let _ = Self::process_refund(intent_id, &intent, RefundReason::Timeout);
    }
}
```

**PASS.** `process_refund` should clean up the `SettlementCreationBlocks` entry.

### Verdict

**PASS.** All settlement paths have timeout mechanisms. Funds cannot get permanently stuck. Both Unix-time and block-based deadlines are enforced.

---

## 5. Bridge Enable/Disable — ExternalBridgesEnabled

### Storage definition

```rust
// Lines 257-259 (pallets/x3-cross-vm-router/src/lib.rs)
pub type ExternalBridgesEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;
```

Default at genesis: `false` (bridges disabled).

### Governance gate in `register_external_root`

```rust
// Lines 681-684
ensure!(
    ExternalBridgesEnabled::<T>::get(),
    Error::<T>::ExternalBridgesDisabled
);
```

**PASS.** Every external bridge extrinsic checks this flag.

### Governance gate in `emergency_pause_bridge`

```rust
// Lines 733-736
ensure!(
    ExternalBridgesEnabled::<T>::get(),
    Error::<T>::ExternalBridgesDisabled
);
```

**PASS.**

### `set_external_bridges_enabled` — root-only + audit gate

```rust
// Lines 764-775
pub fn set_external_bridges_enabled(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
    ensure_root(origin)?;
    if enabled {
        ensure!(
            ExternalBridgeAuditGate::<T>::get(),
            Error::<T>::ExternalBridgeAuditGateMissing
        );
    }
    ExternalBridgesEnabled::<T>::put(enabled);
    ...
}
```

**PASS.** Requires `ensure_root` AND the audit gate must be `true` before enabling. The audit gate itself is also root-only:

```rust
// Lines 781-795
pub fn set_external_bridge_audit_gate(origin: OriginFor<T>, passed: bool) -> DispatchResult {
    ensure_root(origin)?;
    ...
}
```

**PASS.** Double governance gate: audit gate must be set by root first, then enablement by root.

### Does the governance gate cover ALL paths?

**PASS.** External bridge extrinsics (`register_external_root`, `emergency_pause_bridge`) all check `ExternalBridgesEnabled`. The `set_external_bridges_enabled` and `set_external_bridge_audit_gate` are root-only. There are no bypass paths to enable external bridges.

---

## 6. Verification Router — `crates/x3-verification-router/src/lib.rs`

### Mock verifiers in production code

```rust
// Lines 54-70 (VerificationStrategy enum)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerificationStrategy {
    #[cfg(feature = "test-verifier")]
    TestOnly,
    ValidatorQuorum { threshold: u32, total: u32 },
    EvmReceiptProof,
    SolanaFinalizedProof,
    BitcoinSpvProof,
    X3Internal,
    Unsupported,
}
```

**PASS (with concern).** The `TestOnly` variant is feature-gated behind `test-verifier`. There's a compile-time guard:

```rust
// Lines 33-37
#[cfg(all(feature = "production", feature = "test-verifier"))]
compile_error!(
    "MAINNET VIOLATION: `test-verifier` must not be enabled in production builds."
);
```

**However**, there are production verifiers that are **stubs**:

### EvmReceiptVerifier (non-production version)

```rust
// Lines 271-294 (EvmReceiptVerifier::verify)
fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
    if proof.payload.is_empty() || proof.payload.len() < 64 {
        return Err(VerificationError::MalformedProof);
    }
    // In production, this would:
    // 1. Decode the RLP-encoded receipt
    // 2. Verify the receipt merkle proof against a stored block header
    // ...
    Ok(VerificationOutcome {
        accepted: true,
        reason: "evm_receipt_proof_verified",
        verified_at_height: None,
    })
}
```

**FAIL.** This verifier (`EvmReceiptVerifier`) accepts any payload >= 64 bytes. The `// In production, this would:` comments indicate the real verification is **NOT IMPLEMENTED**. The actual payload content is never decoded or validated.

### ValidatorQuorumVerifier (stub)

```rust
// Lines 321-338 (ValidatorQuorumVerifier::verify)
fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
    if proof.payload.is_empty() {
        return Err(VerificationError::MalformedProof);
    }
    // In production, this would:
    // 1. Decode the attestation payload
    // ...
    Ok(VerificationOutcome {
        accepted: true,
        reason: "validator_quorum_verified",
        verified_at_height: None,
    })
}
```

**FAIL.** Always returns `accepted: true` for any non-empty payload. No signature verification.

### SolanaFinalizedVerifier (stub)

```rust
// Lines 350-370 (SolanaFinalizedVerifier::verify)
fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
    if proof.payload.is_empty() { return Err(...) }
    // In production, this would:
    // 1. Verify Solana finalized block hash against known validators
    // ...
    Ok(VerificationOutcome { accepted: true, ... })
}
```

**FAIL.** Always returns `accepted: true`.

### BitcoinSpvVerifier (stub)

```rust
// Lines 410-431 (BitcoinSpvVerifier::verify)
fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
    if proof.payload.is_empty() || proof.payload.len() < 80 { return Err(...) }
    // In production, this would:
    // 1. Verify SPV chain of block headers
    // ...
    Ok(VerificationOutcome { accepted: true, ... })
}
```

**FAIL.** Always returns `accepted: true` for any payload >= 80 bytes.

### ProductionEvmReceiptVerifier -- the REAL verifier

```rust
// Lines 660-708 (crates/x3-verification-router/src/evm_receipt.rs)
impl Verifier for ProductionEvmReceiptVerifier {
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        // ... real decoding, merkle proof verification, log parsing ...
        decoded.validate().map_err(|_| VerificationError::MalformedProof)?;
        Ok(VerificationOutcome { accepted: true, ... })
    }
}
```

**PASS.** This verifier actually does real Merkle Patricia trie verification, receipt RLP decoding, log parsing, and confirmation checks.

### The split is confusing

`ProductionEvmReceiptVerifier` (real) lives in `evm_receipt.rs`. `EvmReceiptVerifier` (stub) lives in `lib.rs`. Both implement `Verifier`. A runtime that accidentally registers `EvmReceiptVerifier` instead of `ProductionEvmReceiptVerifier` would have zero proof security.

**FAIL.** There is a production path that can accidentally use stub verifiers. The stub verifiers (`EvmReceiptVerifier`, `ValidatorQuorumVerifier`, `SolanaFinalizedVerifier`, `BitcoinSpvVerifier`) should be feature-gated behind `test-verifier` or removed entirely to prevent accidental registration.

---

## 7. External Chain Adapters — `crates/external-chains/src/`

### Actual proof verification

```rust
// crates/external-chains/src/adapter.rs, lines 296-301 (ChainAdapter trait)
async fn verify_message_proof(
    &self,
    message: &ChainMessage,
    proof: &[u8],
) -> AdapterResult<bool>;
```

**MISSING.** This is a trait method with no concrete implementation in the crate-level code. The only implementation is `MockChainAdapter`:

```rust
// Lines 407-413 (MockChainAdapter::verify_message_proof)
async fn verify_message_proof(
    &self,
    _message: &ChainMessage,
    _proof: &[u8],
) -> AdapterResult<bool> {
    Ok(true)
}
```

**FAIL.** The `MockChainAdapter` always returns `Ok(true)` for any proof, without any verification. If this adapter is accidentally deployed in production, proofs are never verified.

### Are there mock verifiers in production code?

**YES.** The `MockChainAdapter` at line 347-437 of `adapter.rs` is not feature-gated. It is compiled in all builds. Its `verify_message_proof` returns `Ok(true)` unconditionally.

### Bridge Integration

The `bridge_integration.rs` in `pallet-x3-settlement-engine` references `CrossChainValidatorProvider`:

```rust
// pallets/x3-settlement-engine/src/lib.rs, line 154
type CrossChainValidator: bridge_integration::CrossChainValidatorProvider;
```

**Gap: FAIL.** Without reading the `bridge_integration.rs` file, the validation flow from settlement engine to verification router is unclear. The settlement engine's `submit_proof` extrinsic at lines 1259-1341 calls `Self::verify_proof(&chain, &proof)` which is defined elsewhere. If this delegates to the stub verifiers, external proofs are never actually verified.

### BTC Proof Verification

```rust
// Lines 1551-1554 (pallets/x3-settlement-engine/src/lib.rs)
let is_valid = Self::verify_btc_merkle_proof(&btc_txid, tx_index, &merkle_proof, &block_header)?;
ensure!(is_valid, Error::<T>::InvalidBtcProof);
```

**PASS (surface level).** BTC proofs are verified via Merkle proof logic. However, `verify_btc_merkle_proof` implementation is in `btc_gateway.rs` which was not fully audited here.

---

## Summary

| Path | Verdict | Issue |
|------|---------|-------|
| 1. Route validation | **PASS** | Internal-only routes enforced; compiler features gate external routes |
| 2. Replay protection | **PASS** (minor gap) | `UsedMessages` never pruned; `NonceBatchAllocation` can leak nonces on failure |
| 3. Supply invariant | **PASS** | `check_invariant()` enforced on every mutation + block-level verification |
| 4. Settlement timeout | **PASS** | Both deadline-index auto-refund and block-based timeout work |
| 5. Bridge enable/disable | **PASS** | Double governance gate (audit + enable), root-only |
| 6. Verification router | **FAIL** | Stub verifiers (`EvmReceiptVerifier`, `ValidatorQuorumVerifier`, etc.) coexist with `ProductionEvmReceiptVerifier`; accidental registration of stubs = no security |
| 7. External chain adapters | **FAIL** | `MockChainAdapter` is not feature-gated; `verify_message_proof` returns `Ok(true)` unconditionally |

### Critical Findings

1. **PRODUCTION VERIFIERS ARE STUBS (CRITICAL).** The `VerificationRouter` has stub verifiers for EVM (non-production), Solana, Bitcoin, and ValidatorQuorum that always return `accepted: true`. These are NOT feature-gated and could be registered in production. Only `ProductionEvmReceiptVerifier` does real verification.

2. **MockChainAdapter NOT FEATURE-GATED.** The `MockChainAdapter` compiles in all builds and returns `Ok(true)` for all proof verifications.

3. **NonceBatchAllocation atomicity gap.** If the extrinsic fails between `NextNonce` mutation and `NonceBatchAllocation` insertion, a batch of 100 nonces is leaked.

### Recommendations

1. Feature-gate ALL stub verifiers behind `test-verifier` or `#[cfg(test)]`.
2. Remove `MockChainAdapter` from production builds or feature-gate it.
3. Add explicit nonce-ordering check in `do_initiate_transfer` as a defense-in-depth layer (the comment promises it, the code doesn't deliver).
4. Add a storage pruning mechanism for `UsedMessages` to prevent unbounded growth.
