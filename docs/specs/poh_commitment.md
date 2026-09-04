# X3 Atomic Star — Proof-of-History (PoH) Commitment Specification

**Version:** 1.0
**Status:** Adopted (vΩ-1.0 RC-1)
**Date:** 2026-06-09
**Scope:** PoH digest format, tick cadence, verification cost bounds, and commitment rules

---

## 1. Overview

Proof-of-History (PoH) is a cryptographic clock embedded in every X3 block header. It provides a verifiable, sequential ordering of events without requiring nodes to trust an external time source. This specification defines:

- The PoH digest/commitment format in block headers
- Tick cadence and epoch boundaries
- Skipped-slot behavior
- Verification cost bounds (CPU/memory)
- Integration with the cross-VM router and settlement engine

---

## 2. Header Commitment Format

Each X3 block header carries a `poh_commitment` field:

```rust
struct PohCommitment {
    /// SHA-256 hash of the previous PoH state concatenated with the
    /// previous block hash. This chains PoH across blocks.
    prev_poh_hash: [u8; 32],

    /// Number of PoH ticks executed in this block (always ≥ 1).
    tick_count: u32,

    /// The final PoH hash after `tick_count` iterations starting from
    /// `prev_poh_hash`. The PoH state is updated by repeated SHA-256:
    ///   state_{i+1} = SHA-256(state_i)
    /// where state_0 = prev_poh_hash.
    poh_hash: [u8; 32],

    /// The block number at which this PoH commitment was computed.
    block_number: u32,

    /// Total ticks since genesis (cumulative across all blocks).
    cumulative_ticks: u64,
}
```

### 2.1 Encoding in Block Header

The `poh_commitment` is encoded as a 68-byte compact binary blob appended to the block header digest log:

```rust
digest_item = (
    consensus_engine_id: POH_ENGINE_ID,  // [b'P', b'O', b'H', b' ']
    payload: scale_encode(PohCommitment)
)
```

### 2.2 Validation

Upon importing a block, a node MUST verify:

1. `prev_poh_hash` matches the `poh_hash` of the parent block's `PohCommitment`.
2. `poh_hash` is derived by applying SHA-256 `tick_count` times to `prev_poh_hash`.
3. `cumulative_ticks = parent.cumulative_ticks + tick_count`.
4. `tick_count ≥ 1` (every block must advance PoH).

---

## 3. Tick Cadence

### 3.1 Tick Rate

- **Target tick rate:** 2,000,000 ticks per second (2 MHz).
- **Ticks per 6-second slot:** 12,000,000 ticks.
- **Ticks per epoch (600 slots, ~1 hour):** 7,200,000,000 ticks.

### 3.2 Tick Generation

The block author generates PoH ticks **sequentially** during block production:

1. Start from `parent.poh_hash`.
2. Iterate `SHA-256(state)` `TICKS_PER_SLOT` times (or fewer if the slot is short-circuited by block completion).
3. The final hash becomes the block's `poh_hash`.

### 3.3 Epoch Boundaries

- No special handling at epoch boundaries — PoH ticks continuously across epochs.
- The BABE epoch random seed is derived from the VRF output of the last block in the previous epoch, **not** from PoH state.

---

## 4. Skipped-Slot Behavior

### 4.1 Single Skipped Slot

If a BABE slot is skipped (no block produced), PoH **does not advance** for that slot. The next block's `prev_poh_hash` points to the last produced block's `poh_hash`, and its `tick_count` covers only its own slot.

- This means `cumulative_ticks` does NOT linearly track wall-clock time; it only advances when blocks are actually produced.
- Time-sensitive operations (e.g., expiry checks) use `block_number × SLOT_DURATION` (wall-clock estimate), **not** `cumulative_ticks`.

### 4.2 Consecutive Skipped Slots

After `max_skipped_slots` (default: 10) consecutive empty slots:

- The next block author resumes PoH from the last produced block's `poh_hash`.
- The cumulative gap in `cumulative_ticks` is intentional — PoH is a **block-production clock**, not a wall clock.

---

## 5. Verification Cost Bounds

### 5.1 Single-Block Verification

A verifier receiving a block with `tick_count = N` must compute `N` SHA-256 iterations to verify the PoH hash:

| Tick Count | SHA-256 Iterations | Estimated Time (single core) | Cap |
|---|---|---|---|
| 1 slot (12M) | 12,000,000 | ~25 ms | ✅ Always allowed |
| 10 slots (120M) | 120,000,000 | ~250 ms | ✅ Acceptable |
| 100 slots (1.2B) | 1,200,000,000 | ~2.5 s | ⚠️ Warning threshold |
| >100 slots | >1.2B | >2.5 s | ❌ Rejected by verifier |

### 5.2 Verifier Cap

- **Maximum `tick_count` per block:** 120,000,000 (covers up to 10 consecutive skipped slots).
- Blocks with `tick_count > 120,000,000` are **rejected** by honest verifiers to prevent DoS via excessively long PoH verification.
- This cap enforces that chain progress cannot be stalled for more than ~60 seconds without PoH verification becoming expensive.

### 5.3 Parallel Verification

PoH verification is **inherently sequential** (each hash depends on the previous). Parallel verification is not possible for the core chain, but:

- Light clients may skip PoH verification entirely (trusting the block author).
- Full nodes MAY cache verified PoH states to avoid re-verifying historical blocks.

---

## 6. Integration with Cross-VM Router

### 6.1 Nonce Ordering

The cross-VM router uses `NextNonce` (a monotonic counter per sender per source domain). This nonce is **not** derived from PoH — it is an application-level sequence number:

- Nonces are increment-only, ensuring replay protection.
- PoH provides a **time ordering** for blocks, but nonce ordering is enforced at the pallet level.

### 6.2 Expiry

Transfer expiry in the router uses `block_number` (converted to approximate wall-clock), not `cumulative_ticks`:

```rust
let expires_at = current_block + T::TransferExpiryBlocks::get();
```

PoH guarantees that blocks are produced in a verifiable sequence, which makes block-number-based expiry reliable.

---

## 7. Adversarial and Edge-Case Tests

| Scenario | Expected Behavior |
|---|---|
| Valid block with 1 slot worth of PoH ticks | Accepted; `poh_hash` verification passes |
| Block with `tick_count = 0` | Rejected (violates ≥1 tick rule) |
| Block with `prev_poh_hash ≠ parent.poh_hash` | Rejected (chain break) |
| Block with incorrect `poh_hash` for given `tick_count` | Rejected (fails SHA-256 verification) |
| Block with `tick_count > 120,000,000` | Rejected (exceeds verifier cap) |
| Block with `cumulative_ticks ≠ parent.cumulative_ticks + tick_count` | Rejected (invariant violation) |
| PoH skipped-slot cliff (100 empty slots, then a block) | Block with ~100 slots of PoH used but ≤120M ticks — accepted; >120M — rejected |
| Genesis block PoH | `prev_poh_hash = [0u8; 32]`; `cumulative_ticks = tick_count` of genesis block |

---

## 8. CI Gate

- **Gate name:** `poh-commitment-gate`
- **Check:** This document exists at `docs/specs/poh_commitment.md` and all adversarial tests listed in §7 are present in the PoH generator test suite (`crates/poh-generator/`).

---

## 9. References

- [PoH Generator crate](../../crates/poh-generator/)
- [PoH original specification (Solana)](https://solana.com/solana-whitepaper.pdf) — adapted for X3 with header embedding
- [BABE block production](https://spec.polkadot.network/sect-block-production)