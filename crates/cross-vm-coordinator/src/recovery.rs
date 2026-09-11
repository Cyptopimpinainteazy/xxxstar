//! Deterministic recovery reconciler for restarted cross-domain swaps.
//!
//! Recovery decisions are derived only from persisted session state plus
//! canonical operation/proof evidence. Ambiguous/conflicting terminal evidence
//! fails closed into ManualHalt.

use crate::{CanonicalOperationResult, CoordinatorOperation, OperationAttempt, OperationAttemptStatus, SwapPhase, SwapSession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Nothing left to do; terminal Complete is already authoritative.
    Complete,
    /// Nothing left to do; terminal Refunded is already authoritative.
    Refunded,
    /// An external tx/signature was broadcast but has no canonical proof yet.
    WaitFinality,
    /// Retry or continue the current non-terminal coordinator operation.
    Resume,
    /// Fast-side claim is the next safe action.
    ClaimFast,
    /// Slow-side claim is the next safe action.
    ClaimSlow,
    /// Refund path is the next safe action.
    Refund,
    /// Evidence conflicts or is insufficiently coherent for automation.
    ManualHalt,
}

#[derive(Debug, Clone)]
pub struct RecoveryEvidence<'a> {
    pub attempts: &'a [OperationAttempt],
    pub canonical_results: &'a [CanonicalOperationResult],
}

impl<'a> RecoveryEvidence<'a> {
    fn canonical(&self, operation: CoordinatorOperation) -> Option<&CanonicalOperationResult> {
        self.canonical_results
            .iter()
            .find(|result| result.operation == operation)
    }

    fn has_unfinalized_broadcast(&self) -> bool {
        self.attempts.iter().any(|attempt| {
            attempt.status == OperationAttemptStatus::Broadcast
                && !self.canonical_results.iter().any(|result| {
                    result.attempt_id == attempt.attempt_id
                        || result.attempt_id
                            == attempt
                                .attempt_id
                                .strip_suffix("#broadcast")
                                .unwrap_or(attempt.attempt_id.as_str())
                })
        })
    }
}

pub struct RecoveryReconciler;

impl RecoveryReconciler {
    pub fn decide(
        session: &SwapSession,
        evidence: &RecoveryEvidence<'_>,
    ) -> RecoveryAction {
        let fast_claim = evidence.canonical(CoordinatorOperation::FastClaim).is_some();
        let slow_claim = evidence.canonical(CoordinatorOperation::SlowClaim).is_some();
        let refund = evidence.canonical(CoordinatorOperation::RefundBoth).is_some();

        // Any canonical claim + canonical refund combination is a terminal
        // contradiction. Automated recovery must stop immediately.
        if refund && (fast_claim || slow_claim) {
            return RecoveryAction::ManualHalt;
        }

        // Canonical slow-claim proof implies the swap has reached the claim
        // terminal path. If persisted phase disagrees with a refund terminal,
        // fail closed rather than rewriting history.
        if slow_claim {
            return match session.phase {
                SwapPhase::Refunded => RecoveryAction::ManualHalt,
                _ => RecoveryAction::Complete,
            };
        }

        if refund {
            return match session.phase {
                SwapPhase::Complete => RecoveryAction::ManualHalt,
                _ => RecoveryAction::Refunded,
            };
        }

        // A broadcast without canonical finality proof must be observed rather
        // than blindly retried, preventing duplicate external execution.
        if evidence.has_unfinalized_broadcast() {
            return RecoveryAction::WaitFinality;
        }

        match session.phase {
            SwapPhase::Complete => RecoveryAction::Complete,
            SwapPhase::Refunded => RecoveryAction::Refunded,
            SwapPhase::Failed => RecoveryAction::ManualHalt,
            SwapPhase::Aborting => RecoveryAction::Refund,

            SwapPhase::ClaimingSlow => RecoveryAction::ClaimSlow,

            SwapPhase::ClaimingFast => {
                if fast_claim {
                    RecoveryAction::ClaimSlow
                } else {
                    RecoveryAction::ClaimFast
                }
            }

            // If a canonical fast claim exists but persistence lagged behind
            // the phase transition, continue to the slow claim instead of
            // revealing/claiming fast twice.
            SwapPhase::LegsComplete if fast_claim => RecoveryAction::ClaimSlow,

            SwapPhase::LegsComplete => RecoveryAction::ClaimFast,

            SwapPhase::Setup
            | SwapPhase::LockingHtlcs
            | SwapPhase::HtlcsLocked
            | SwapPhase::ExecutingFlashLegs => RecoveryAction::Resume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HtlcHash, SwapSession};

    fn session(phase: SwapPhase) -> SwapSession {
        SwapSession {
            session_id: "swap-r".into(),
            hash_lock: HtlcHash([7u8; 32]),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: vec![],
            leg_outcomes: vec![],
            phase,
            timelock_fast: 100,
            timelock_slow: 200,
            created_at: 1,
            updated_at: 1,
            operation_journal: vec![],
            requires_merkle_verification: false,
        }
    }

    fn canonical(
        operation: CoordinatorOperation,
        attempt_id: &str,
        byte: u8,
    ) -> CanonicalOperationResult {
        CanonicalOperationResult {
            session_id: "swap-r".into(),
            operation,
            attempt_id: attempt_id.into(),
            tx_id: format!("tx-{byte}"),
            proof_hash: [byte; 32],
            finalized_at: 10,
        }
    }

    fn broadcast(operation: CoordinatorOperation, attempt_id: &str) -> OperationAttempt {
        OperationAttempt {
            session_id: "swap-r".into(),
            operation,
            attempt_id: attempt_id.into(),
            owner_id: "worker-a".into(),
            fence: 3,
            domain: "ethereum".into(),
            status: OperationAttemptStatus::Broadcast,
            tx_id: Some("0xabc".into()),
            proof_hash: None,
            started_at: 1,
            updated_at: 2,
            error: None,
        }
    }

    #[test]
    fn broadcast_without_proof_waits_instead_of_retrying() {
        let attempts = vec![broadcast(CoordinatorOperation::FastClaim, "a1")];
        let evidence = RecoveryEvidence {
            attempts: &attempts,
            canonical_results: &[],
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::ClaimingFast), &evidence),
            RecoveryAction::WaitFinality
        );
    }

    #[test]
    fn persisted_phase_lag_after_fast_claim_moves_to_slow_claim() {
        let results = vec![canonical(CoordinatorOperation::FastClaim, "a1", 1)];
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &results,
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::LegsComplete), &evidence),
            RecoveryAction::ClaimSlow
        );
    }

    #[test]
    fn canonical_slow_claim_dominates_nonterminal_persisted_phase() {
        let results = vec![canonical(CoordinatorOperation::SlowClaim, "a2", 2)];
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &results,
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::ClaimingSlow), &evidence),
            RecoveryAction::Complete
        );
    }

    #[test]
    fn canonical_refund_dominates_aborting_phase() {
        let results = vec![canonical(CoordinatorOperation::RefundBoth, "r1", 3)];
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &results,
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::Aborting), &evidence),
            RecoveryAction::Refunded
        );
    }

    #[test]
    fn claim_and_refund_canonical_evidence_halts() {
        let results = vec![
            canonical(CoordinatorOperation::FastClaim, "c1", 4),
            canonical(CoordinatorOperation::RefundBoth, "r1", 5),
        ];
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &results,
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::ClaimingSlow), &evidence),
            RecoveryAction::ManualHalt
        );
    }

    #[test]
    fn terminal_phase_conflicting_with_canonical_evidence_halts() {
        let results = vec![canonical(CoordinatorOperation::RefundBoth, "r1", 6)];
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &results,
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::Complete), &evidence),
            RecoveryAction::ManualHalt
        );
    }

    #[test]
    fn failed_session_requires_manual_halt() {
        let evidence = RecoveryEvidence {
            attempts: &[],
            canonical_results: &[],
        };
        assert_eq!(
            RecoveryReconciler::decide(&session(SwapPhase::Failed), &evidence),
            RecoveryAction::ManualHalt
        );
    }
}
