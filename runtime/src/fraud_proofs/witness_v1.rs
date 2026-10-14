//! Deterministic Witness Encoding Format v0 — SchedulerMismatchV1
//!
//! Implements SCALE decode, canonicality validation, conflict-graph construction,
//! Kahn topological sort, and scheduler-commitment derivation as specified in:
//!   `openspec/committee-reexec-fraudproofs-v0/witness.md`
//!
//! # Invariants referenced
//! - WITNESS-CANON-001, WITNESS-CANON-002
//! - WITNESS-BOUNDS-001
//! - WITNESS-GRAPH-001, WITNESS-ORDER-001

#![allow(dead_code)]

use codec::{Compact, Decode, DecodeWithMemTracking, Encode};
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_std::{collections::btree_map::BTreeMap, prelude::*, vec::Vec};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Wire version tag — must be 1 for v0 witnesses.
pub const WITNESS_VERSION: u8 = 1;

/// Maximum total encoded witness size (64 KiB).
pub const MAX_WITNESS_BYTES: usize = 65_536;

/// Maximum number of access entries per transaction.
pub const MAX_ACCESSES_PER_TX: u32 = 256;

/// Reserved field byte capacity (must be 0 in v0).
pub const MAX_RESERVED_BYTES: u32 = 32;

// ── Error type ────────────────────────────────────────────────────────────────

/// All reasons a witness can be rejected, returned as `Err(WitnessError)`.
/// The verifier MUST emit `InvalidWitnessEncoding` with the specific variant;
/// never a generic mismatch error.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking)]
pub enum WitnessError {
    /// `version` field != 1.
    BadVersion,
    /// `rules_version` field does not match the block's pinned version.
    RulesVersionMismatch,
    /// `tx_count` mismatches the number of `tx_ids` / `access_lists`, or
    /// exceeds `MAX_TXS_PER_BLOCK`.
    TxCountMismatch,
    /// `tx_ids` are not strictly increasing (lexicographically, ascending).
    TxIdsNotSorted,
    /// An `access_list` is not strictly increasing or contains a duplicate.
    AccessListNotSorted,
    /// An `access_count` field does not equal the actual list length, or
    /// exceeds `MAX_ACCESSES_PER_TX`.
    AccessCountExceeded,
    /// `reserved` field is non-empty in v0.
    ReservedNonEmpty,
    /// Encoded witness byte length exceeds `MAX_WITNESS_BYTES`.
    WitnessTooLarge,
    /// Conflict DAG contains a cycle (should be impossible under v0 rules).
    CycleDetected,
    /// SCALE decode failure.
    DecodeError,
}

// ── Access key ────────────────────────────────────────────────────────────────

/// Domain tag for an access key.
/// Values 0-3 are stable; 4 (`Custom`) is reserved for v1+.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, DecodeWithMemTracking,
)]
#[repr(u8)]
pub enum AccessDomain {
    StorageKey = 0,
    Account = 1,
    Contract = 2,
    Nonce = 3,
    Custom = 4,
}

/// A single resource access: a (domain, key) pair.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking)]
pub struct AccessKeyV1 {
    pub domain: u8, // raw byte; validated against known domain values implicitly
    pub key: H256,
}

impl AccessKeyV1 {
    /// 33-byte compound sort key: `domain_byte || key_bytes`.
    #[inline]
    fn sort_key(&self) -> [u8; 33] {
        let mut buf = [0u8; 33];
        buf[0] = self.domain;
        buf[1..].copy_from_slice(self.key.as_bytes());
        buf
    }
}

/// The access set for a single transaction.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking)]
pub struct AccessListV1 {
    /// Redundant length field (canonical: must equal `accesses.len()`).
    pub access_count: Compact<u32>,
    /// Sorted, deduplicated accesses.
    pub accesses: Vec<AccessKeyV1>,
}

// ── Witness ───────────────────────────────────────────────────────────────────

/// Top-level v0 witness structure.  SCALE-encoded into `reexec_witness` bytes.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking)]
pub struct SchedulerWitnessV1 {
    pub version: u8,
    pub rules_version: u32,
    pub tx_count: Compact<u32>,
    pub tx_ids: Vec<H256>,
    pub access_lists: Vec<AccessListV1>,
    pub seed: Option<H256>,
    /// Must be empty (`BoundedVec<u8, 32>` encoded as zero-length in v0).
    pub reserved: Vec<u8>,
}

// ── Commitments ───────────────────────────────────────────────────────────────

/// The four commitments derived from a validated witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulerCommitments {
    /// `blake2_256(SCALE(tx_ids))`
    pub tx_set_commitment: H256,
    /// `blake2_256(canonical_graph_bytes)`
    pub graph_commitment: H256,
    /// `blake2_256(canonical_order_bytes)`
    pub order_commitment: H256,
    /// `blake2_256(graph || order || tx_set || rules_version_le)`
    pub scheduler_commitment: H256,
}

// ── Core API ──────────────────────────────────────────────────────────────────

impl SchedulerWitnessV1 {
    /// Decode raw bytes and validate ALL canonicality rules.
    ///
    /// `expected_rules_version` — the `rules_version` pinned in the block header.
    /// `max_tx_count` — `MAX_TXS_PER_BLOCK` runtime constant.
    ///
    /// Returns a fully-validated witness on `Ok`, or a specific `WitnessError` on `Err`.
    pub fn decode_and_validate(
        bytes: &[u8],
        expected_rules_version: u32,
        max_tx_count: u32,
    ) -> Result<Self, WitnessError> {
        // ── Rule 8: byte cap ──────────────────────────────────────────────────
        if bytes.len() > MAX_WITNESS_BYTES {
            return Err(WitnessError::WitnessTooLarge);
        }

        // ── SCALE decode ─────────────────────────────────────────────────────
        let witness = Self::decode(&mut &bytes[..]).map_err(|_| WitnessError::DecodeError)?;

        // ── Rule 1: version ───────────────────────────────────────────────────
        if witness.version != WITNESS_VERSION {
            return Err(WitnessError::BadVersion);
        }

        // ── Rule 2: rules_version ─────────────────────────────────────────────
        if witness.rules_version != expected_rules_version {
            return Err(WitnessError::RulesVersionMismatch);
        }

        // ── Rule 3: tx_count coherence ────────────────────────────────────────
        let tc: u32 = witness.tx_count.into();
        if tc > max_tx_count
            || tc as usize != witness.tx_ids.len()
            || tc as usize != witness.access_lists.len()
        {
            return Err(WitnessError::TxCountMismatch);
        }

        // ── Rule 4: tx_ids strictly increasing ────────────────────────────────
        for i in 1..witness.tx_ids.len() {
            if witness.tx_ids[i - 1].as_bytes() >= witness.tx_ids[i].as_bytes() {
                return Err(WitnessError::TxIdsNotSorted);
            }
        }

        // ── Rules 5 & 6: access list canonicality ─────────────────────────────
        for al in &witness.access_lists {
            let claimed: u32 = al.access_count.into();
            if claimed != al.accesses.len() as u32 || claimed > MAX_ACCESSES_PER_TX {
                return Err(WitnessError::AccessCountExceeded);
            }
            for j in 1..al.accesses.len() {
                if al.accesses[j - 1].sort_key() >= al.accesses[j].sort_key() {
                    return Err(WitnessError::AccessListNotSorted);
                }
            }
        }

        // ── Rule 7: reserved empty ────────────────────────────────────────────
        if !witness.reserved.is_empty() {
            return Err(WitnessError::ReservedNonEmpty);
        }

        Ok(witness)
    }

    /// Compute all four commitments from a validated witness.
    ///
    /// Deterministic: identical witnesses on any node produce identical commitments.
    pub fn compute_commitments(&self) -> Result<SchedulerCommitments, WitnessError> {
        let tx_count = self.tx_ids.len();

        // ── tx_set_commitment ─────────────────────────────────────────────────
        let tx_set_enc = self.tx_ids.encode();
        let tx_set_commitment = H256(blake2_256(&tx_set_enc));

        // ── Build conflict graph ──────────────────────────────────────────────
        // index: (domain, key) -> list of tx indices
        let mut key_to_txs: BTreeMap<[u8; 33], Vec<u32>> = BTreeMap::new();
        for (i, al) in self.access_lists.iter().enumerate() {
            for ak in &al.accesses {
                key_to_txs.entry(ak.sort_key()).or_default().push(i as u32);
            }
        }

        // adjacency list: edges[i] = sorted set of j > i that conflict with i
        let mut edges: Vec<Vec<u32>> = vec![Vec::new(); tx_count];
        for txs in key_to_txs.values() {
            for a in 0..txs.len() {
                for b in (a + 1)..txs.len() {
                    let i = txs[a] as usize;
                    let j = txs[b] as usize;
                    // `txs` is appended in ascending index order, so `a < b`
                    // implies `i < j`; min/max states the canonical ordering
                    // explicitly without an unobservable branch.
                    let lo = i.min(j);
                    let hi = i.max(j);
                    // insert hi into edges[lo] (dedup)
                    if !edges[lo].contains(&(hi as u32)) {
                        edges[lo].push(hi as u32);
                    }
                }
            }
        }
        // sort each edge list ascending
        for e in &mut edges {
            e.sort_unstable();
        }

        // ── graph_commitment ──────────────────────────────────────────────────
        let graph_bytes = encode_canonical_graph(tx_count, &edges);
        let graph_commitment = H256(blake2_256(&graph_bytes));

        // ── order_commitment ──────────────────────────────────────────────────
        let order_bytes = kahn_topological_sort(tx_count, &edges, &self.tx_ids)?;
        let order_commitment = H256(blake2_256(&order_bytes));

        // ── scheduler_commitment ──────────────────────────────────────────────
        let mut hashin = Vec::new();
        hashin.extend_from_slice(graph_commitment.as_bytes());
        hashin.extend_from_slice(order_commitment.as_bytes());
        hashin.extend_from_slice(tx_set_commitment.as_bytes());
        hashin.extend_from_slice(&self.rules_version.to_le_bytes());
        let scheduler_commitment = H256(blake2_256(&hashin));

        Ok(SchedulerCommitments {
            tx_set_commitment,
            graph_commitment,
            order_commitment,
            scheduler_commitment,
        })
    }
}

// ── Internal: canonical graph encoding ───────────────────────────────────────

/// Encode the adjacency list into canonical graph bytes.
///
/// Format:
/// ```text
/// tx_count: Compact<u32>
/// for each i in 0..tx_count:
///   out_degree: Compact<u32>
///   for each j in edges[i] (ascending):
///     j: Compact<u32>
/// ```
fn encode_canonical_graph(tx_count: usize, edges: &[Vec<u32>]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&Compact::<u32>(tx_count as u32).encode());
    for e in edges.iter().take(tx_count) {
        buf.extend_from_slice(&Compact::<u32>(e.len() as u32).encode());
        for &j in e {
            buf.extend_from_slice(&Compact::<u32>(j).encode());
        }
    }
    buf
}

// ── Internal: Kahn topological sort ──────────────────────────────────────────

/// Kahn's algorithm with deterministic tie-breaking by `tx_id` (ascending).
///
/// Returns SCALE-encoded `Vec<Compact<u32>>` of tx indices in execution order.
/// Returns `Err(WitnessError::CycleDetected)` if the graph contains a cycle.
fn kahn_topological_sort(
    tx_count: usize,
    edges: &[Vec<u32>],
    tx_ids: &[H256],
) -> Result<Vec<u8>, WitnessError> {
    // Build full adjacency (edges contains only lo→hi; we need DAG i→j where tx_id[i] < tx_id[j])
    // Under v0: tx_ids are pre-sorted ascending so i < j implies tx_id[i] < tx_id[j] lexicographically.
    // edges[i] already contains {j > i | conflict(i,j)}, so the DAG is edges itself.

    let mut in_degree: Vec<u32> = vec![0u32; tx_count];
    for edge in edges.iter().take(tx_count) {
        for &j in edge {
            in_degree[j as usize] += 1;
        }
    }

    // Ready set: all nodes with in_degree == 0, sorted by tx_id ascending (already index order since tx_ids sorted)
    // Use a sorted Vec as a min-heap by index (index order = tx_id order since tx_ids are sorted)
    let mut ready: Vec<u32> = (0..tx_count as u32)
        .filter(|&i| in_degree[i as usize] == 0)
        .collect();
    ready.sort_unstable_by(|&a, &b| tx_ids[a as usize].cmp(&tx_ids[b as usize]));

    let mut order: Vec<u32> = Vec::with_capacity(tx_count);

    while !ready.is_empty() {
        // Pop the smallest tx_id from ready
        let cur = ready.remove(0);
        order.push(cur);

        // For each successor, decrement in_degree
        let mut newly_ready: Vec<u32> = Vec::new();
        for &succ in &edges[cur as usize] {
            in_degree[succ as usize] -= 1;
            if in_degree[succ as usize] == 0 {
                newly_ready.push(succ);
            }
        }
        // Insert newly ready into ready, maintaining sort by tx_id
        if !newly_ready.is_empty() {
            newly_ready.sort_unstable_by(|&a, &b| tx_ids[a as usize].cmp(&tx_ids[b as usize]));
            for nr in newly_ready {
                let pos = ready
                    .binary_search_by(|&x| tx_ids[x as usize].cmp(&tx_ids[nr as usize]))
                    .unwrap_or_else(|p| p);
                ready.insert(pos, nr);
            }
        }
    }

    // If not all nodes emitted, there is a cycle
    if order.len() != tx_count {
        return Err(WitnessError::CycleDetected);
    }

    // Encode as Vec<Compact<u32>>
    let mut buf = Vec::new();
    for idx in order {
        buf.extend_from_slice(&Compact::<u32>(idx).encode());
    }
    Ok(buf)
}

// (codec helpers removed — use .encode() + extend_from_slice directly)

// ─────────────────────────────────────────────────────────────────────────────
// Tests: canonicality rules, graph encoding, and the Kahn order oracle.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn access(domain: u8, key_byte: u8) -> AccessKeyV1 {
        AccessKeyV1 {
            domain,
            key: H256([key_byte; 32]),
        }
    }

    /// Strictly increasing sort keys: index `i` is encoded big-endian across
    /// the first and last key byte so the list is sorted for any length.
    fn ascending_access(i: u32) -> AccessKeyV1 {
        let mut key = [0u8; 32];
        key[0] = (i / 256) as u8;
        key[31] = (i % 256) as u8;
        AccessKeyV1 {
            domain: 1,
            key: H256(key),
        }
    }

    fn list(keys: Vec<AccessKeyV1>) -> AccessListV1 {
        AccessListV1 {
            access_count: Compact(keys.len() as u32),
            accesses: keys,
        }
    }

    fn witness(tx_ids: Vec<H256>, access_lists: Vec<AccessListV1>) -> SchedulerWitnessV1 {
        SchedulerWitnessV1 {
            version: WITNESS_VERSION,
            rules_version: 1,
            tx_count: Compact(tx_ids.len() as u32),
            tx_ids,
            access_lists,
            seed: None,
            reserved: Vec::new(),
        }
    }

    fn ids(n: u8) -> Vec<H256> {
        (0..n).map(|i| H256([i; 32])).collect()
    }

    // ── Rule 8: byte cap ─────────────────────────────────────────────────────

    /// While the input stays at or under `MAX_WITNESS_BYTES`, trailing bytes
    /// after the SCALE body are ignored (only the cap rejects).  This pins the
    /// inclusive boundary exactly.
    #[test]
    fn byte_cap_is_inclusive() {
        let encoded = witness(ids(1), vec![list(vec![])]).encode();

        let mut at_cap = encoded.clone();
        at_cap.resize(MAX_WITNESS_BYTES, 0);
        assert_eq!(at_cap.len(), MAX_WITNESS_BYTES);
        assert!(
            SchedulerWitnessV1::decode_and_validate(&at_cap, 1, 100).is_ok(),
            "exactly MAX_WITNESS_BYTES must be accepted"
        );

        let mut over_cap = encoded;
        over_cap.resize(MAX_WITNESS_BYTES + 1, 0);
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&over_cap, 1, 100),
            Err(WitnessError::WitnessTooLarge)
        );
    }

    // ── Rule 3: tx_count coherence ───────────────────────────────────────────

    /// `tc == max_tx_count` is valid; one more is not.
    #[test]
    fn tx_count_cap_boundary() {
        let encoded = witness(ids(2), vec![list(vec![]), list(vec![])]).encode();
        assert!(SchedulerWitnessV1::decode_and_validate(&encoded, 1, 2).is_ok());
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 1),
            Err(WitnessError::TxCountMismatch)
        );
    }

    /// `tx_count` must match BOTH `tx_ids.len()` and `access_lists.len()`;
    /// matching one of the two is not enough.
    #[test]
    fn tx_count_must_match_both_lengths() {
        let encoded = witness(ids(2), vec![list(vec![])]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::TxCountMismatch)
        );

        let encoded = witness(ids(1), vec![list(vec![]), list(vec![])]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::TxCountMismatch)
        );
    }

    /// Non-coherent `tx_count` cannot be masked by a permissive byte/version.
    #[test]
    fn tx_count_mismatch_beats_no_other_rule() {
        let encoded = witness(ids(3), vec![list(vec![]), list(vec![])]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 3),
            Err(WitnessError::TxCountMismatch)
        );
    }

    // ── Rules 5 & 6: access list canonicality ────────────────────────────────

    /// `claimed == MAX_ACCESSES_PER_TX` is valid; one more is not.
    #[test]
    fn access_count_cap_boundary() {
        let at_cap: Vec<AccessKeyV1> = (0..MAX_ACCESSES_PER_TX).map(ascending_access).collect();
        let encoded = witness(ids(1), vec![list(at_cap)]).encode();
        assert!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100).is_ok(),
            "exactly MAX_ACCESSES_PER_TX accesses must be accepted"
        );

        let over_cap: Vec<AccessKeyV1> = (0..=MAX_ACCESSES_PER_TX).map(ascending_access).collect();
        let encoded = witness(ids(1), vec![list(over_cap)]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::AccessCountExceeded)
        );
    }

    /// `claimed` must equal `accesses.len()` even when both are under the cap.
    #[test]
    fn access_count_must_match_actual_length() {
        let mut w = witness(ids(1), vec![list(vec![access(1, 1)])]);
        w.access_lists[0].access_count = Compact(5u32);
        let encoded = w.encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::AccessCountExceeded)
        );
    }

    /// Duplicate/unsorted accesses are rejected (`<=` would be needed to pass).
    #[test]
    fn access_list_must_be_strictly_increasing() {
        let encoded = witness(ids(1), vec![list(vec![access(1, 2), access(1, 1)])]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::AccessListNotSorted)
        );

        let encoded = witness(ids(1), vec![list(vec![access(1, 1), access(1, 1)])]).encode();
        assert_eq!(
            SchedulerWitnessV1::decode_and_validate(&encoded, 1, 100),
            Err(WitnessError::AccessListNotSorted)
        );
    }

    // ── Canonical graph encoding ─────────────────────────────────────────────

    /// WITNESS-GRAPH-001: the encoded adjacency list is exactly
    /// `Compact(n) || for i: Compact(out_deg(i)) || Compact(j)*`.
    #[test]
    fn canonical_graph_bytes_shape() {
        let edges = vec![vec![1u32], vec![2u32], vec![]];
        let bytes = encode_canonical_graph(3, &edges);
        let mut expected = Vec::new();
        expected.extend_from_slice(&Compact::<u32>(3).encode());
        expected.extend_from_slice(&Compact::<u32>(1).encode());
        expected.extend_from_slice(&Compact::<u32>(1).encode());
        expected.extend_from_slice(&Compact::<u32>(1).encode());
        expected.extend_from_slice(&Compact::<u32>(2).encode());
        expected.extend_from_slice(&Compact::<u32>(0).encode());
        assert_eq!(bytes, expected);
    }

    // ── Kahn topological order ───────────────────────────────────────────────

    /// WITNESS-ORDER-001: the emitted order is index (= tx_id) ascending when
    /// all blocking predecessors are processed first.
    #[test]
    fn kahn_emits_exact_order_bytes() {
        let tx_ids = ids(3);
        let edges = vec![vec![2u32], vec![2u32], vec![]];
        let bytes = kahn_topological_sort(3, &edges, &tx_ids).expect("acyclic");
        let mut expected = Vec::new();
        expected.extend_from_slice(&Compact::<u32>(0).encode());
        expected.extend_from_slice(&Compact::<u32>(1).encode());
        expected.extend_from_slice(&Compact::<u32>(2).encode());
        assert_eq!(bytes, expected);
    }

    /// A graph with only one node yields exactly one index byte.
    #[test]
    fn kahn_single_node() {
        let bytes = kahn_topological_sort(1, &[vec![]], &ids(1)).expect("acyclic");
        assert_eq!(bytes, Compact::<u32>(0).encode());
    }

    // ── End-to-end commitments ───────────────────────────────────────────────

    /// The four commitments are deterministic for identical witnesses, and
    /// every downstream commitment depends on its canonical input.
    #[test]
    fn commitments_are_deterministic_and_complete() {
        let w = witness(
            ids(3),
            vec![
                list(vec![access(1, 5)]),
                list(vec![access(2, 6)]),
                list(vec![access(1, 5), access(2, 6)]),
            ],
        );
        let a = w.compute_commitments().expect("acyclic");
        let b = w.compute_commitments().expect("acyclic");
        assert_eq!(a, b);

        let mut expected_scheduler = Vec::new();
        expected_scheduler.extend_from_slice(a.graph_commitment.as_bytes());
        expected_scheduler.extend_from_slice(a.order_commitment.as_bytes());
        expected_scheduler.extend_from_slice(a.tx_set_commitment.as_bytes());
        expected_scheduler.extend_from_slice(&w.rules_version.to_le_bytes());
        assert_eq!(
            a.scheduler_commitment,
            H256(blake2_256(&expected_scheduler))
        );

        let mut other_rules = w.clone();
        other_rules.rules_version = 2;
        assert_ne!(
            a.scheduler_commitment,
            other_rules
                .compute_commitments()
                .expect("acyclic")
                .scheduler_commitment
        );
    }
}
