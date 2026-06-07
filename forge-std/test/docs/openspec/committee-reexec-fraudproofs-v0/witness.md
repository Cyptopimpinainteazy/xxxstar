# Deterministic Witness Encoding Format v0 (SchedulerMismatchV1)

**Status:** ACCEPTED  
**Layer:** Consensus / Fraud Proof  
**Depends on:** `MAX_TXS_PER_BLOCK` runtime constant, block header format  
**Stability:** Stable for v0; changes require new `rules_version`

---

## Goals

| Goal | Notes |
|------|-------|
| **Canonical** | One valid encoding per logical witness |
| **Bounded** | Strict size caps / counts to prevent DoS |
| **Self-contained** | Verifier recomputes commitment from block header + included tx list, nothing else |
| **Stable across versions** | Versioned and hash-domain separated via `rules_version` |

## Non-goals (v0)

- Proving GPU kernel execution correctness
- Including full conflict graph bytes if cheaply reconstructable
- Supporting multiple scheduler algorithms in one witness

---

## 1. Constants

All constants are runtime/governance constants unless noted.

```text
MAX_WITNESS_BYTES    = 65_536   (64 KiB)
MAX_TXS_PER_BLOCK   = (existing runtime constant)
MAX_ACCESSES_PER_TX = 256
MAX_EDGES_OVERRIDE  = 0         (reserved, must be 0 in v0)
WITNESS_VERSION     = 1
```

### Hash primitive

```text
H256 = blake2_256(...)
tx_id = blake2_256(canonical_tx_bytes)
```

---

## 2. Witness Structure (SCALE-encoded)

### `SchedulerWitnessV1`

Encoded into `reexec_witness: BoundedVec<u8, MAX_WITNESS_BYTES>`.

| Field | Type | Value |
|-------|------|-------|
| `version` | `u8` | must be `1` |
| `rules_version` | `u32` | must match `block.rules_version` |
| `tx_count` | `Compact<u32>` | number of transactions |
| `tx_ids` | `Vec<H256>` | length = `tx_count` |
| `access_lists` | `Vec<AccessListV1>` | length = `tx_count` |
| `seed` | `Option<H256>` | omit unless scheduler uses external seed |
| `reserved` | `BoundedVec<u8, 32>` | **must be empty bytes in v0** |

### `AccessListV1`

| Field | Type | Value |
|-------|------|-------|
| `access_count` | `Compact<u32>` | must equal `accesses.len()` |
| `accesses` | `Vec<AccessKeyV1>` | length = `access_count` |

### `AccessKeyV1`

| Field | Type | Notes |
|-------|------|-------|
| `domain` | `u8` | see table below |
| `key` | `H256` | the 32-byte key within that domain |

#### Access Domains

| Value | Name | Notes |
|-------|------|-------|
| `0` | `StorageKey` | pallet storage map key |
| `1` | `Account` | account balance / nonce container |
| `2` | `Contract` | EVM/SVM contract storage |
| `3` | `Nonce` | account nonce slot |
| `4` | `Custom` | user-defined; reserved for v1+ |

---

## 3. Canonicalization Rules

These rules ensure one encoding per logical witness. Any violation → `InvalidWitnessEncoding`.

1. `tx_ids` **must** be strictly increasing lexicographically (byte-ascending, no ties allowed because tx_ids are 32-byte hashes).
2. Each `AccessListV1.accesses` **must** be:
   - deduplicated
   - strictly increasing lexicographically on the compound key `(domain_byte || key_bytes)` (33 bytes total, big-endian domain byte)
3. `access_count` must exactly equal the actual list length.

---

## 4. What the Witness Represents

The witness is **not** the graph bytes themselves. It is the minimum data to deterministically derive:

- **Conflict edges**: `edge(tx_i, tx_j)` iff `access_lists[i]` and `access_lists[j]` share any `(domain, key)` pair.
- **Deterministic order**: Kahn's topological sort on the conflict DAG, with tie-breaks by `tx_id` lexicographic order (ascending).

This means the verifier can reconstitute:
- `conflict_graph_bytes` in a canonical representation
- `order_bytes` in a canonical representation

---

## 5. Canonical Graph Encoding (for commitment recomputation)

```
1. Enumerate txs in ascending tx_id order (i = 0..tx_count-1, already guaranteed by canonicalization)
2. For each tx i:
   a. Compute outgoing edges to all j > i where access_lists[i] ∩ access_lists[j] ≠ ∅
   b. Sort j indices ascending
3. Emit:
   - tx_count as Compact<u32>
   - For each i: out_degree as Compact<u32>, then each j as Compact<u32> ascending
```

### Commitment derivations

```
graph_commitment     = blake2_256(graph_bytes)
order_commitment     = blake2_256(order_bytes)
tx_set_commitment    = blake2_256(SCALE(tx_ids))
scheduler_commitment = blake2_256(
    graph_commitment
    || order_commitment
    || tx_set_commitment
    || rules_version.to_le_bytes()
)
```

---

## 6. Canonical Ordering Rules — `rules_version = 1`

### SchedulerRulesV1 (pinned algorithm)

1. Build a DAG: for each conflicting pair `(i, j)` where `tx_id[i] < tx_id[j]`, add directed edge `i → j`.
2. Run Kahn's algorithm with a deterministic priority queue: the ready set is always processed in ascending `tx_id` order.
3. Emit `order_bytes` as a SCALE-encoded `Vec<Compact<u32>>` of tx indices in execution order.

If cycle detection finds a cycle (shouldn't happen in v0 DAG construction), return `InvalidWitnessEncoding::CycleDetected`.

New scheduling semantics (e.g. write-before-read priorities, fee classes) → `rules_version = 2`.

---

## 7. Verification Cost and DoS Controls

Fraud proof verification weight is bounded by:

```
tx_count       ≤ MAX_TXS_PER_BLOCK
Σ access_count ≤ tx_count × MAX_ACCESSES_PER_TX
witness_bytes  ≤ MAX_WITNESS_BYTES
```

Implementation efficiency note: build a `BTreeMap<(domain, key), SmallVec<[u32; 4]>>` mapping each access key to the set of tx indices that access it, then derive conflict edges without O(n²) full pairwise comparison.

---

## 8. Witness Validation Checklist (MUST REJECT)

| # | Condition | Error |
|---|-----------|-------|
| 1 | `version != 1` | `InvalidWitnessEncoding::BadVersion` |
| 2 | `rules_version != block.rules_version` | `InvalidWitnessEncoding::RulesVersionMismatch` |
| 3 | `tx_count` mismatch or exceeds `MAX_TXS_PER_BLOCK` | `InvalidWitnessEncoding::TxCountMismatch` |
| 4 | `tx_ids` not strictly increasing | `InvalidWitnessEncoding::TxIdsNotSorted` |
| 5 | Any `accesses` list not strictly increasing or contains duplicate | `InvalidWitnessEncoding::AccessListNotSorted` |
| 6 | Any `access_count > MAX_ACCESSES_PER_TX` | `InvalidWitnessEncoding::AccessCountExceeded` |
| 7 | `reserved` bytes non-empty | `InvalidWitnessEncoding::ReservedNonEmpty` |
| 8 | `witness_bytes > MAX_WITNESS_BYTES` | `InvalidWitnessEncoding::WitnessTooLarge` |
| 9 | Cycle detected in conflict DAG | `InvalidWitnessEncoding::CycleDetected` |

---

## 9. How the Proposer Produces the Witness

The GPU proposer already computes access sets and the conflict graph as part of scheduling. For v0:

1. Collect `tx_ids` from the block in ascending order.
2. For each tx, record the set of `(domain, key)` accesses touched during scheduling (or from static analysis / tx metadata).
3. Sort each access list and deduplicate.
4. SCALE-encode to `SchedulerWitnessV1`, prepend size, submit in `propose_block` extrinsic.

**v0 trust model**: The witness describes the scheduler's *claimed* access basis. A fraud proof shows the proposed `scheduler_commitment` does not match the commitment derived from the included witness. Proving *true* accesses requires execution traces — that is v1+.

---

## 10. Required Invariants

See `tests/invariants/registry.toml`:

- `WITNESS-CANON-001` — same logical witness, different byte orderings → exactly one must pass validation
- `WITNESS-CANON-002` — same witness on different nodes produces identical `scheduler_commitment`
- `WITNESS-BOUNDS-001` — `MAX_WITNESS_BYTES`, `MAX_ACCESSES_PER_TX` caps enforced
- `WITNESS-GRAPH-001` — adjacency encoding deterministic for identical inputs
- `WITNESS-ORDER-001` — Kahn sort with tx_id tie-break is deterministic

---

## 11. Implementation Artifacts

| File | Purpose |
|------|---------|
| `runtime/src/fraud_proofs/witness_v1.rs` | SCALE decode, canonicality validation, commitment derivation |
| `runtime/src/fraud_proofs/mod.rs` | Module re-exports |
| `tests/fraud_proofs_witness_v1.rs` | Unit + property tests |
| `tests/invariants/registry.toml` | 5 new invariant entries |
