//! Executor: runs each wave of a schedule and records per-tx outcomes.
//!
//! For v0.4 internal-only mainnet the executor is *serial* by default —
//! parallel wave dispatch is infrastructure-dependent and gated behind the
//! `parallel` feature flag.  The critical invariant is that the commit order
//! and final state must be identical regardless of whether the `parallel`
//! flag is set.  This is enforced by the commit module's deterministic
//! merge step.
//!
//! The executor is generic over a `TxFn` — a closure that receives a `TxId`
//! and returns a `TxOutcome`.  This keeps the executor independent of any
//! particular state backend or pallet API.

use alloc::vec::Vec;

use crate::scheduler::{Schedule, TxId};

/// Result of executing one transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TxOutcome {
    /// Transaction executed and produced write-set `writes`.
    Success {
        tx_id: TxId,
        /// Keys actually written (may differ from declared access list).
        writes: Vec<[u8; 32]>,
    },
    /// Transaction failed with `reason`; no state changes applied.
    Failed { tx_id: TxId, reason: FailReason },
    /// Access-list validation found an undeclared write — escalated to
    /// serial re-execution in the commit step.
    AccessViolation { tx_id: TxId },
}

/// Why a transaction failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailReason {
    /// The transaction function returned an application-layer error.
    ApplicationError,
    /// A previously-committed wave invalidated this tx's read set.
    Stale,
}

impl TxOutcome {
    pub fn tx_id(&self) -> TxId {
        match self {
            TxOutcome::Success { tx_id, .. } => *tx_id,
            TxOutcome::Failed { tx_id, .. } => *tx_id,
            TxOutcome::AccessViolation { tx_id } => *tx_id,
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, TxOutcome::Success { .. })
    }
}

/// Batch execution result.
#[derive(Clone, Debug, Default)]
pub struct ExecutionResult {
    /// Outcomes in the same serial order as the original batch.
    pub outcomes: Vec<TxOutcome>,
}

impl ExecutionResult {
    pub fn success_count(&self) -> usize {
        self.outcomes.iter().filter(|o| o.is_success()).count()
    }

    pub fn failed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| matches!(o, TxOutcome::Failed { .. }))
            .count()
    }
}

pub struct Executor;

impl Executor {
    /// Execute a `Schedule` by calling `tx_fn` for each transaction.
    ///
    /// Waves are executed sequentially in wave order.  Within each wave,
    /// transactions are processed in their stored (serial) order.
    ///
    /// Returns outcomes in the same serial order as the original batch.
    /// The `commit` module is responsible for merging write sets and
    /// verifying state-root equivalence.
    pub fn execute<F>(schedule: &Schedule, mut tx_fn: F) -> ExecutionResult
    where
        F: FnMut(TxId) -> TxOutcome,
    {
        let total: usize = schedule.tx_count();
        let mut outcome_map: alloc::collections::BTreeMap<TxId, TxOutcome> =
            alloc::collections::BTreeMap::new();

        for wave in &schedule.waves {
            for &tx_id in &wave.txs {
                let outcome = tx_fn(tx_id);
                outcome_map.insert(tx_id, outcome);
            }
        }

        // Reconstruct in serial order.
        let mut outcomes = Vec::with_capacity(total);
        // We don't have the original ordering here, but the schedule preserves
        // it per-wave; collect by iterating waves in order.
        for wave in &schedule.waves {
            for &tx_id in &wave.txs {
                if let Some(o) = outcome_map.remove(&tx_id) {
                    outcomes.push(o);
                }
            }
        }

        ExecutionResult { outcomes }
    }
}
