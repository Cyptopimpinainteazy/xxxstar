//! Conflict detector: given a set of pending transactions with declared
//! access lists, produce conflict-free parallel waves.
//!
//! A *wave* is a set of transactions with pairwise-disjoint access lists
//! that can be executed concurrently with no data hazards.  Transactions
//! within a wave are then committed in their original serial order to
//! preserve deterministic state-root equivalence.
//!
//! Algorithm: greedy wave assignment.  For each transaction (in serial
//! order) assign it to the first wave where it does not conflict with any
//! already-assigned member.  If it conflicts with all existing waves, open
//! a new wave.  This is O(n * w * n) where n = tx count, w = wave count.
//! For typical block sizes (≤ 512 tx) it is fast enough; a more
//! sophisticated topological sort can replace it later.

use alloc::vec::Vec;

use crate::access_list::AccessList;

/// One wave of conflict-free transactions (indices into the original batch).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wave {
    /// Indices into the original transaction batch, in serial order.
    pub tx_indices: Vec<usize>,
}

/// Detector: groups transactions into conflict-free waves.
pub struct ConflictDetector;

impl ConflictDetector {
    /// Assign every transaction in `access_lists` to a wave.
    ///
    /// The returned `Vec<Wave>` is ordered: wave 0 runs first, then wave 1,
    /// etc.  Transactions in the same wave have pairwise-disjoint access and
    /// can execute in any order (or concurrently).
    pub fn assign_waves(access_lists: &[AccessList]) -> Vec<Wave> {
        let mut waves: Vec<Wave> = Vec::new();
        // Per-wave collected access lists (union of all tx in the wave).
        let mut wave_union: Vec<AccessList> = Vec::new();

        'tx: for (idx, al) in access_lists.iter().enumerate() {
            // Try to place this tx in an existing wave.
            for (w, union) in wave_union.iter_mut().enumerate() {
                if !union.conflicts_with(al) && !al.conflicts_with(union) {
                    waves[w].tx_indices.push(idx);
                    // Extend the wave's union access list.
                    union.reads.extend_from_slice(&al.reads);
                    union.writes.extend_from_slice(&al.writes);
                    continue 'tx;
                }
            }
            // No compatible wave found — open a new one.
            let mut new_union = AccessList::default();
            new_union.reads.extend_from_slice(&al.reads);
            new_union.writes.extend_from_slice(&al.writes);
            wave_union.push(new_union);
            waves.push(Wave {
                tx_indices: alloc::vec![idx],
            });
        }

        waves
    }

    /// Count how many waves would be required.  Useful for observability.
    pub fn wave_count(access_lists: &[AccessList]) -> usize {
        Self::assign_waves(access_lists).len()
    }
}
