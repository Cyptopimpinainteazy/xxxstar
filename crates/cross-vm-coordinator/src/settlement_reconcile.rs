//! Deterministic chain reconciliation for settlement outbox records.
//!
//! The observer is deliberately abstract: RPC/subxt/node wiring can implement
//! it later. The decision table itself is pure and fail-closed.

use crate::{
    CoordinatorError, SettlementOutboxRecord, SettlementOutboxStatus,
};
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementTxObservation {
    /// Authoritative query did not find the tx. For a Signed record this means
    /// it is safe to rebroadcast the exact same persisted signed bytes.
    Unknown,
    Pending,
    Included { block_number: u64 },
    Rejected { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSettlementObservation {
    Unknown,
    NonTerminal,
    Finalized,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementReconcileDecision {
    SignPrepared,
    /// Never create a new transaction. Re-send only record.signed_extrinsic.
    RebroadcastExactSignedExtrinsic,
    WaitForInclusion,
    MarkIncluded { block_number: u64 },
    MarkFailed { reason: String },
    ObserveSettlementState,
    MarkTerminalObserved,
    Done,
    ManualHalt { reason: String },
}

#[async_trait]
pub trait SettlementChainObserver: Send + Sync {
    async fn transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<SettlementTxObservation, CoordinatorError>;

    async fn settlement_state(
        &self,
        runtime_intent_id: [u8; 32],
    ) -> Result<RuntimeSettlementObservation, CoordinatorError>;
}

/// Pure reconciliation decision. No network calls and no state writes.
///
/// Purpose is encoded in the outbox as:
/// - 0 = Claim
/// - 1 = Refund
pub fn decide_settlement_reconciliation(
    record: &SettlementOutboxRecord,
    tx: Option<&SettlementTxObservation>,
    runtime: Option<RuntimeSettlementObservation>,
) -> SettlementReconcileDecision {
    match record.status {
        SettlementOutboxStatus::Prepared => SettlementReconcileDecision::SignPrepared,

        SettlementOutboxStatus::Signed | SettlementOutboxStatus::Broadcast => {
            let Some(tx) = tx else {
                return SettlementReconcileDecision::WaitForInclusion;
            };
            match tx {
                SettlementTxObservation::Unknown => {
                    if record.status == SettlementOutboxStatus::Signed
                        && record
                            .signed_extrinsic
                            .as_ref()
                            .is_some_and(|bytes| !bytes.is_empty())
                        && record.tx_id.is_some()
                    {
                        SettlementReconcileDecision::RebroadcastExactSignedExtrinsic
                    } else {
                        // A Broadcast tx disappearing is ambiguous: do not
                        // create or sign a replacement transaction.
                        SettlementReconcileDecision::WaitForInclusion
                    }
                }
                SettlementTxObservation::Pending => SettlementReconcileDecision::WaitForInclusion,
                SettlementTxObservation::Included { block_number } => {
                    SettlementReconcileDecision::MarkIncluded {
                        block_number: *block_number,
                    }
                }
                SettlementTxObservation::Rejected { reason } => {
                    SettlementReconcileDecision::MarkFailed {
                        reason: reason.clone(),
                    }
                }
            }
        }

        SettlementOutboxStatus::Included => {
            let Some(runtime) = runtime else {
                return SettlementReconcileDecision::ObserveSettlementState;
            };
            match (record.purpose, runtime) {
                (_, RuntimeSettlementObservation::Unknown)
                | (_, RuntimeSettlementObservation::NonTerminal) => {
                    SettlementReconcileDecision::ObserveSettlementState
                }
                (0, RuntimeSettlementObservation::Finalized)
                | (1, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::MarkTerminalObserved
                }
                (0, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "claim submission observed Refunded terminal state".into(),
                    }
                }
                (1, RuntimeSettlementObservation::Finalized) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "refund submission observed Finalized terminal state".into(),
                    }
                }
                (_, RuntimeSettlementObservation::Finalized)
                | (_, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "unknown settlement submission purpose at terminal state".into(),
                    }
                }
            }
        }

        SettlementOutboxStatus::TerminalObserved => SettlementReconcileDecision::Done,

        SettlementOutboxStatus::Failed => SettlementReconcileDecision::ManualHalt {
            reason: record
                .error
                .clone()
                .unwrap_or_else(|| "settlement submission is in Failed state".into()),
        },
    }
}

/// Query only the evidence required by the current durable outbox state and
/// return a pure reconciliation decision.
pub async fn observe_and_decide<O: SettlementChainObserver>(
    observer: &O,
    record: &SettlementOutboxRecord,
) -> Result<SettlementReconcileDecision, CoordinatorError> {
    match record.status {
        SettlementOutboxStatus::Signed | SettlementOutboxStatus::Broadcast => {
            let tx_id = record.tx_id.as_deref().ok_or_else(|| {
                CoordinatorError::Internal(
                    "signed/broadcast settlement record has no tx id".into(),
                )
            })?;
            let tx = observer.transaction_status(tx_id).await?;
            Ok(decide_settlement_reconciliation(record, Some(&tx), None))
        }
        SettlementOutboxStatus::Included => {
            let runtime = observer
                .settlement_state(record.runtime_intent_id)
                .await?;
            Ok(decide_settlement_reconciliation(
                record,
                None,
                Some(runtime),
            ))
        }
        _ => Ok(decide_settlement_reconciliation(record, None, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(status: SettlementOutboxStatus, purpose: u8) -> SettlementOutboxRecord {
        SettlementOutboxRecord {
            submission_id: [1u8; 32],
            session_id: "swap-a".into(),
            runtime_intent_id: [2u8; 32],
            purpose,
            call_index: 33,
            call_args_hash: [3u8; 32],
            proof_hashes: vec![[4u8; 32]],
            status,
            tx_id: Some("0xabc".into()),
            signed_extrinsic: Some(vec![1, 2, 3]),
            block_number: None,
            owner_id: Some("worker-a".into()),
            fence: Some(7),
            prepared_at: 100,
            updated_at: 101,
            error: None,
        }
    }

    #[test]
    fn signed_unknown_rebroadcasts_only_exact_bytes() {
        let r = record(SettlementOutboxStatus::Signed, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Unknown),
                None,
            ),
            SettlementReconcileDecision::RebroadcastExactSignedExtrinsic
        );
    }

    #[test]
    fn broadcast_unknown_never_recommends_new_transaction() {
        let r = record(SettlementOutboxStatus::Broadcast, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Unknown),
                None,
            ),
            SettlementReconcileDecision::WaitForInclusion
        );
    }

    #[test]
    fn included_claim_accepts_only_finalized_terminal() {
        let r = record(SettlementOutboxStatus::Included, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Finalized),
            ),
            SettlementReconcileDecision::MarkTerminalObserved
        );
        assert!(matches!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Refunded),
            ),
            SettlementReconcileDecision::ManualHalt { .. }
        ));
    }

    #[test]
    fn included_refund_accepts_only_refunded_terminal() {
        let r = record(SettlementOutboxStatus::Included, 1);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Refunded),
            ),
            SettlementReconcileDecision::MarkTerminalObserved
        );
        assert!(matches!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Finalized),
            ),
            SettlementReconcileDecision::ManualHalt { .. }
        ));
    }

    #[test]
    fn included_tx_moves_to_included_block() {
        let r = record(SettlementOutboxStatus::Broadcast, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Included { block_number: 77 }),
                None,
            ),
            SettlementReconcileDecision::MarkIncluded { block_number: 77 }
        );
    }
}
