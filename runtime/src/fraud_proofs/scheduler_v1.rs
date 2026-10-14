//! CPU reference scheduler v1 — deterministic, hardware-independent.
//!
//! This is the *constitutional truth layer*: the same algorithm the runtime uses
//! to verify fraud proofs. GPU proposers must produce commitments matching this
//! output; any divergence is slashable (see `verifier.rs`).
//!
//! Invariants:
//!   WITNESS-GRAPH-001 — canonical graph encoding is deterministic
//!   WITNESS-ORDER-001 — Kahn sort with tx_id tie-break is deterministic

#![allow(dead_code)]

use sp_core::H256;
use sp_std::prelude::*;

use super::witness_v1::{SchedulerCommitments, SchedulerWitnessV1, WitnessError};

/// Decode, validate, then compute all four commitments for a raw witness payload.
///
/// This is the single entry point for fraud-proof recomputation.
///
/// `expected_rules_version` — from the disputed block header.
/// `max_tx_count` — `MAX_TXS_PER_BLOCK` runtime constant.
pub fn recompute_from_bytes(
    witness_bytes: &[u8],
    expected_rules_version: u32,
    max_tx_count: u32,
) -> Result<SchedulerCommitments, WitnessError> {
    let witness = SchedulerWitnessV1::decode_and_validate(
        witness_bytes,
        expected_rules_version,
        max_tx_count,
    )?;
    witness.compute_commitments()
}

/// Return the canonical `scheduler_commitment` from raw bytes.
/// Convenience wrapper used by the verifier hot-path.
pub fn scheduler_commitment_from_bytes(
    witness_bytes: &[u8],
    expected_rules_version: u32,
    max_tx_count: u32,
) -> Result<H256, WitnessError> {
    recompute_from_bytes(witness_bytes, expected_rules_version, max_tx_count)
        .map(|c| c.scheduler_commitment)
}

// ── CPU-reference graph + order helpers exposed for testing ──────────────────

/// Build a sorted conflict edge list for a decoded witness.
/// Returns `edges[i]` = sorted vec of `j > i` that conflict with `i`.
/// Used internally and in tests to assert graph structure.
pub fn build_conflict_edges(witness: &SchedulerWitnessV1) -> Vec<Vec<u32>> {
    let n = witness.tx_ids.len();
    let mut edges: Vec<Vec<u32>> = vec![Vec::new(); n];

    // Build (domain,key) → [tx indices] index for O(n·k) instead of O(n²·k²)
    use sp_std::collections::btree_map::BTreeMap;
    let mut key_to_txs: BTreeMap<[u8; 33], Vec<u32>> = BTreeMap::new();
    for (i, al) in witness.access_lists.iter().enumerate() {
        for ak in &al.accesses {
            let mut k = [0u8; 33];
            k[0] = ak.domain;
            k[1..].copy_from_slice(ak.key.as_bytes());
            key_to_txs.entry(k).or_default().push(i as u32);
        }
    }

    for txs in key_to_txs.values() {
        for a in 0..txs.len() {
            for b in (a + 1)..txs.len() {
                // `txs` is appended in ascending index order, so `a < b` implies
                // `txs[a] < txs[b]`; min/max makes the canonical (lo, hi) ordering
                // explicit without a branch split that no witness can observe.
                let lo = txs[a].min(txs[b]) as usize;
                let hi = txs[a].max(txs[b]) as usize;
                if !edges[lo].contains(&(hi as u32)) {
                    edges[lo].push(hi as u32);
                }
            }
        }
    }
    for e in &mut edges {
        let e: &mut Vec<u32> = e;
        e.sort_unstable();
    }
    edges
}

/// Produce the canonical execution order for a decoded witness.
/// Returns tx indices in deterministic topological order.
pub fn canonical_order(witness: &SchedulerWitnessV1) -> Result<Vec<u32>, WitnessError> {
    let edges = build_conflict_edges(witness);
    let n = witness.tx_ids.len();

    let mut in_degree = vec![0u32; n];
    for edge in edges.iter().take(n) {
        for &j in edge {
            // saturating_add: a malformed duplicate edge must not panic the
            // verifier path; canonical witnesses never saturate.
            in_degree[j as usize] = in_degree[j as usize].saturating_add(1);
        }
    }

    // Ready set: sorted by tx_id (== sorted by index, since tx_ids are pre-sorted ascending)
    let mut ready: Vec<u32> = (0..n as u32)
        .filter(|&i| in_degree[i as usize] == 0)
        .collect();
    ready.sort_unstable_by(|&a, &b| witness.tx_ids[a as usize].cmp(&witness.tx_ids[b as usize]));

    let mut order = Vec::with_capacity(n);
    while !ready.is_empty() {
        let cur = ready.remove(0);
        order.push(cur);
        let mut newly: Vec<u32> = Vec::new();
        for &succ in &edges[cur as usize] {
            in_degree[succ as usize] = in_degree[succ as usize].saturating_sub(1);
            if in_degree[succ as usize] == 0 {
                newly.push(succ);
            }
        }
        if !newly.is_empty() {
            newly.sort_unstable_by(|&a, &b| {
                witness.tx_ids[a as usize].cmp(&witness.tx_ids[b as usize])
            });
            for nr in newly {
                let pos = ready
                    .binary_search_by(|&x| {
                        witness.tx_ids[x as usize].cmp(&witness.tx_ids[nr as usize])
                    })
                    .unwrap_or_else(|p| p);
                ready.insert(pos, nr);
            }
        }
    }

    if order.len() != n {
        return Err(WitnessError::CycleDetected);
    }
    Ok(order)
}

// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fraud_proofs::witness_v1::{
        AccessKeyV1, AccessListV1, SchedulerWitnessV1, WITNESS_VERSION,
    };
    use codec::Compact;

    fn access(domain: u8, key_byte: u8) -> AccessKeyV1 {
        AccessKeyV1 {
            domain,
            key: H256([key_byte; 32]),
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

    /// WITNESS-GRAPH-001: no txs, no edges.
    #[test]
    fn conflict_edges_empty_witness_is_empty() {
        let w = witness(Vec::new(), Vec::new());
        assert_eq!(build_conflict_edges(&w), Vec::<Vec<u32>>::new());
    }

    /// WITNESS-GRAPH-001: every conflicting pair is linked once, j > i, sorted.
    #[test]
    fn conflict_edges_links_each_key_once_and_sorts() {
        // tx0 touches keys 9 and 1; tx1 touches key 9; tx2 touches key 1.
        // The key-1 walk pushes edge (0, 2) before the key-9 walk pushes
        // (0, 1), so the per-node list must be re-sorted to be canonical.
        let w = witness(
            ids(3),
            vec![
                list(vec![access(1, 9), access(1, 1)]),
                list(vec![access(1, 9)]),
                list(vec![access(1, 1)]),
            ],
        );
        assert_eq!(build_conflict_edges(&w), vec![vec![1, 2], vec![], vec![]]);
    }

    /// Two shared keys between the same pair must still produce exactly one edge.
    #[test]
    fn conflict_edges_dedups_shared_pairs() {
        let w = witness(
            ids(2),
            vec![
                list(vec![access(1, 1), access(1, 2)]),
                list(vec![access(1, 1), access(1, 2)]),
            ],
        );
        assert_eq!(build_conflict_edges(&w), vec![vec![1], vec![]]);
    }

    /// Same key bytes under different domains are independent resources.
    #[test]
    fn conflict_edges_distinct_domains_do_not_conflict() {
        let w = witness(
            ids(2),
            vec![list(vec![access(1, 7)]), list(vec![access(2, 7)])],
        );
        assert_eq!(
            build_conflict_edges(&w),
            vec![Vec::<u32>::new(), Vec::new()]
        );
    }

    /// WITNESS-ORDER-001: empty witness has the empty canonical order.
    #[test]
    fn canonical_order_empty_is_ok_empty() {
        assert_eq!(
            canonical_order(&witness(Vec::new(), Vec::new())),
            Ok(Vec::new())
        );
    }

    /// WITNESS-ORDER-001: independent txs keep ascending index (= tx_id) order.
    #[test]
    fn canonical_order_independent_txs_keep_index_order() {
        let w = witness(ids(3), vec![list(vec![]), list(vec![]), list(vec![])]);
        assert_eq!(canonical_order(&w), Ok(vec![0, 1, 2]));
    }

    /// A conflict forces the successor after its predecessor; free txs still run.
    #[test]
    fn canonical_order_conflicting_tx_waits_for_predecessor() {
        let w = witness(
            ids(3),
            vec![
                list(vec![access(1, 5)]),
                list(vec![access(1, 5)]),
                list(vec![]),
            ],
        );
        assert_eq!(canonical_order(&w), Ok(vec![0, 1, 2]));
    }

    /// Two predecessors of the same successor: the successor is emitted only
    /// after the second one releases it, and never re-emitted.
    #[test]
    fn canonical_order_emits_shared_successor_once() {
        let w = witness(
            ids(3),
            vec![
                list(vec![access(1, 5)]),
                list(vec![access(2, 6)]),
                list(vec![access(1, 5), access(2, 6)]),
            ],
        );
        assert_eq!(canonical_order(&w), Ok(vec![0, 1, 2]));
    }

    /// build_conflict_edges output must never depend on map iteration order:
    /// the sorted edge list is the canonical graph.
    #[test]
    fn canonical_order_is_repeatable() {
        let w = witness(
            ids(4),
            vec![
                list(vec![access(1, 1), access(1, 2)]),
                list(vec![access(1, 1)]),
                list(vec![access(1, 2)]),
                list(vec![access(3, 1)]),
            ],
        );
        assert_eq!(canonical_order(&w), canonical_order(&w));
        assert_eq!(build_conflict_edges(&w), build_conflict_edges(&w));
    }
}
