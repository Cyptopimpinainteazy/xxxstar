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
                let (lo, hi) = (txs[a] as usize, txs[b] as usize);
                let (lo, hi) = if lo < hi { (lo, hi) } else { (hi, lo) };
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
    for i in 0..n {
        for &j in &edges[i] {
            in_degree[j as usize] += 1;
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
            in_degree[succ as usize] -= 1;
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
