//! Scheduler: produces an ordered execution plan from a raw transaction batch.
//!
//! Responsibilities:
//! 1. Accept a batch of `(AccessList, TxId)` pairs.
//! 2. Run the conflict detector to assign waves.
//! 3. Return a `Schedule` that the executor will process wave-by-wave.
//!
//! The scheduler does **not** execute transactions; it only determines order.
//! This separation makes the scheduling logic independently testable and
//! auditable.

use alloc::vec::Vec;

use crate::access_list::AccessList;
use crate::conflict::{ConflictDetector, Wave};

/// Opaque transaction identifier.  For Substrate blocks this would be the
/// extrinsic index; for unit tests any u64 works.
pub type TxId = u64;

/// A fully planned schedule.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Schedule {
    /// Ordered waves; each wave is conflict-free internally.
    pub waves: Vec<WaveEntry>,
}

/// One wave in the schedule, holding resolved TxIds rather than raw indices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaveEntry {
    /// Transaction ids in this wave, in original serial order.
    pub txs: Vec<TxId>,
}

impl Schedule {
    pub fn wave_count(&self) -> usize {
        self.waves.len()
    }

    pub fn tx_count(&self) -> usize {
        self.waves.iter().map(|w| w.txs.len()).sum()
    }
}

pub struct Scheduler;

impl Scheduler {
    /// Build a `Schedule` from a slice of `(AccessList, TxId)` pairs.
    ///
    /// The input slice is treated as the *serial* order of the block — the
    /// commit step will preserve this ordering regardless of which wave a tx
    /// belongs to.
    pub fn schedule(batch: &[(AccessList, TxId)]) -> Schedule {
        if batch.is_empty() {
            return Schedule::default();
        }

        let access_lists: Vec<&AccessList> = batch.iter().map(|(al, _)| al).collect();
        let ids: Vec<TxId> = batch.iter().map(|(_, id)| *id).collect();

        let raw_waves: Vec<Wave> = ConflictDetector::assign_waves(
            &access_lists
                .iter()
                .map(|al| (*al).clone())
                .collect::<Vec<_>>(),
        );

        let waves = raw_waves
            .into_iter()
            .map(|w| WaveEntry {
                txs: w.tx_indices.iter().map(|&i| ids[i]).collect(),
            })
            .collect();

        Schedule { waves }
    }
}
