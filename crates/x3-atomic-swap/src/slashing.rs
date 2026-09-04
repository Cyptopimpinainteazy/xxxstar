//! # Slashing - Penalty engine for misbehaving actors.
//!
//! Tracks slashing cases against relayers, solvers, validator-provers, and
//! watchers.  Maintains per-actor reputation scores and stake balances, and
//! provides convenience methods for common slashable offences.
//!
//! ## Actors
//!
//! - [`SlashableActor::Relayer`] - Failed to relay or claim.
//! - [`SlashableActor::Solver`] - Submitted false proofs or stale fills.
//! - [`SlashableActor::ValidatorProver`] - Invalid proof submissions.
//! - [`SlashableActor::Watcher`] - Censorship or RPC manipulation.
//!
//! ## Offences
//!
//! See [`SlashReason`] for all supported reasons.

use crate::error::SwapError;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Actor type
// ---------------------------------------------------------------------------

/// Actor type that can be slashed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SlashableActor {
    /// Relayer responsible for watching and claiming.
    Relayer,
    /// Solver that matched intents.
    Solver,
    /// Validator or prover submitting proofs.
    ValidatorProver,
    /// Watcher monitoring chain activity.
    Watcher,
}

impl SlashableActor {
    /// Human-readable name for this actor type.
    pub fn name(&self) -> &'static str {
        match self {
            SlashableActor::Relayer => "Relayer",
            SlashableActor::Solver => "Solver",
            SlashableActor::ValidatorProver => "ValidatorProver",
            SlashableActor::Watcher => "Watcher",
        }
    }
}

// ---------------------------------------------------------------------------
// Slash reason
// ---------------------------------------------------------------------------

/// Reason for slashing an actor.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SlashReason {
    /// Submitted a false proof.
    FalseProof(String),
    /// Missed an assigned claim for an intent.
    MissedAssignedClaim(String),
    /// Missed an assigned refund for an intent.
    MissedAssignedRefund(String),
    /// Failed to fill a stale quote.
    StaleQuoteFillFailure(String),
    /// Made an invalid finality claim.
    InvalidFinalityClaim(String),
    /// Manipulated RPC responses.
    RpcManipulation(String),
    /// Engaged in censorship or griefing.
    CensorshipGriefing(String),
    /// Reported a fake success.
    FakeSuccessReport(String),
}

impl SlashReason {
    /// Short machine-readable code for this reason.
    pub fn code(&self) -> &'static str {
        match self {
            SlashReason::FalseProof(_) => "FALSE_PROOF",
            SlashReason::MissedAssignedClaim(_) => "MISSED_CLAIM",
            SlashReason::MissedAssignedRefund(_) => "MISSED_REFUND",
            SlashReason::StaleQuoteFillFailure(_) => "STALE_QUOTE",
            SlashReason::InvalidFinalityClaim(_) => "INVALID_FINALITY",
            SlashReason::RpcManipulation(_) => "RPC_MANIPULATION",
            SlashReason::CensorshipGriefing(_) => "CENSORSHIP_GRIEFING",
            SlashReason::FakeSuccessReport(_) => "FAKE_SUCCESS",
        }
    }

    /// Human-readable description of this reason (includes the embedded detail).
    pub fn description(&self) -> &str {
        match self {
            SlashReason::FalseProof(s) => s.as_str(),
            SlashReason::MissedAssignedClaim(s) => s.as_str(),
            SlashReason::MissedAssignedRefund(s) => s.as_str(),
            SlashReason::StaleQuoteFillFailure(s) => s.as_str(),
            SlashReason::InvalidFinalityClaim(s) => s.as_str(),
            SlashReason::RpcManipulation(s) => s.as_str(),
            SlashReason::CensorshipGriefing(s) => s.as_str(),
            SlashReason::FakeSuccessReport(s) => s.as_str(),
        }
    }
}

// ---------------------------------------------------------------------------
// Slash case status
// ---------------------------------------------------------------------------

/// Status of a slashing case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SlashCaseStatus {
    /// Case has been opened but not yet reviewed.
    Open,
    /// Case is under active review.
    UnderReview,
    /// Case has been resolved (slash applied).
    Resolved,
    /// Case has been rejected (no slash applied).
    Rejected,
}

// ---------------------------------------------------------------------------
// Slash record
// ---------------------------------------------------------------------------

/// A slashing record capturing a complete case.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlashRecord {
    /// Unique slash identifier.
    pub slash_id: u64,
    /// ID of the actor being slashed.
    pub actor_id: String,
    /// Type of the actor.
    pub actor_type: SlashableActor,
    /// Optional intent ID that triggered this case.
    pub intent_id: Option<u64>,
    /// Reason for slashing.
    pub reason: SlashReason,
    /// Binary evidence supporting the case.
    pub evidence: Vec<u8>,
    /// Amount being slashed.
    pub amount: u128,
    /// Current status of the case.
    pub status: SlashCaseStatus,
    /// Timestamp (seconds) when the case was created.
    pub created_at: u64,
    /// Timestamp (seconds) when the case was resolved or rejected.
    pub resolved_at: Option<u64>,
}

// ---------------------------------------------------------------------------
// Slash summary
// ---------------------------------------------------------------------------

/// Summary of slashing state across all actors.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SlashSummary {
    /// Total number of cases ever opened.
    pub total_cases: u64,
    /// Number of currently open cases.
    pub open_cases: u64,
    /// Number of resolved cases.
    pub resolved_cases: u64,
    /// Number of rejected cases.
    pub rejected_cases: u64,
    /// Total amount slashed across all resolved cases.
    pub total_slashed_amount: u128,
    /// Active slash amount (open + under review cases).
    pub active_slash_amount: u128,
    /// Worst actors sorted by reputation ascending (worst first).
    pub worst_actors: Vec<(String, i64)>,
}

// ---------------------------------------------------------------------------
// Slashing engine
// ---------------------------------------------------------------------------

/// Slashing engine managing all slashing cases, actor reputations, and stakes.
#[derive(Debug, Clone)]
pub struct SlashingEngine {
    /// Map of slash_id -> SlashRecord.
    pub slash_cases: BTreeMap<u64, SlashRecord>,
    /// actor_id -> reputation score.
    pub actor_reputation: BTreeMap<String, i64>,
    /// actor_id -> current stake.
    pub actor_stake: BTreeMap<String, u128>,
    /// Next slash ID to assign.
    pub next_slash_id: u64,
    /// Minimum evidence size in bytes required to open a case.
    pub min_evidence_size: usize,
}

impl SlashingEngine {
    /// Create a new empty slashing engine.
    ///
    /// Default `min_evidence_size` is 8 bytes.
    pub fn new() -> Self {
        Self {
            slash_cases: BTreeMap::new(),
            actor_reputation: BTreeMap::new(),
            actor_stake: BTreeMap::new(),
            next_slash_id: 1,
            min_evidence_size: 8,
        }
    }

    /// Open a new slashing case.
    ///
    /// Returns the assigned `slash_id` on success.
    ///
    /// # Errors
    ///
    /// - `InsufficientEvidence` if `evidence.len() < min_evidence_size`.
    pub fn open_case(
        &mut self,
        actor_id: String,
        actor_type: SlashableActor,
        intent_id: Option<u64>,
        reason: SlashReason,
        evidence: Vec<u8>,
        slash_amount: u128,
    ) -> Result<u64, SwapError> {
        if evidence.len() < self.min_evidence_size {
            return Err(SwapError::InsufficientEvidence {
                minimum: self.min_evidence_size,
                actual: evidence.len(),
            });
        }

        let slash_id = self.next_slash_id;
        self.next_slash_id += 1;

        // Ensure reputation entry exists.
        self.actor_reputation.entry(actor_id.clone()).or_insert(0);
        // Ensure stake entry exists.
        self.actor_stake.entry(actor_id.clone()).or_insert(0);

        let record = SlashRecord {
            slash_id,
            actor_id: actor_id.clone(),
            actor_type,
            intent_id,
            reason,
            evidence,
            amount: slash_amount,
            status: SlashCaseStatus::Open,
            created_at: 0,
            resolved_at: None,
        };

        self.slash_cases.insert(slash_id, record);
        Ok(slash_id)
    }

    /// Resolve an open or under-review case, applying the slash.
    ///
    /// # Errors
    ///
    /// - `SlashNotFound` if the case does not exist.
    /// - `InvalidSlashStatus` if the case is already resolved or rejected.
    pub fn resolve_case(&mut self, slash_id: u64) -> Result<(), SwapError> {
        let record = self
            .slash_cases
            .get_mut(&slash_id)
            .ok_or(SwapError::SlashNotFound { slash_id })?;

        match record.status {
            SlashCaseStatus::Open | SlashCaseStatus::UnderReview => {
                // Apply the slash: reduce the actor's stake.
                let actor_id = record.actor_id.clone();
                let amount = record.amount;
                let current_stake = self.actor_stake.get(&actor_id).copied().unwrap_or(0);
                let new_stake = current_stake.saturating_sub(amount);
                self.actor_stake.insert(actor_id, new_stake);

                record.status = SlashCaseStatus::Resolved;
                record.resolved_at = Some(1); // non-zero timestamp placeholder
                Ok(())
            }
            _ => Err(SwapError::InvalidSlashStatus {
                slash_id,
                reason: alloc::format!("cannot resolve case in status {:?}", record.status),
            }),
        }
    }

    /// Reject an open or under-review case (no slash applied).
    ///
    /// # Errors
    ///
    /// - `SlashNotFound` if the case does not exist.
    /// - `InvalidSlashStatus` if the case is already resolved or rejected.
    pub fn reject_case(&mut self, slash_id: u64) -> Result<(), SwapError> {
        let record = self
            .slash_cases
            .get_mut(&slash_id)
            .ok_or(SwapError::SlashNotFound { slash_id })?;

        match record.status {
            SlashCaseStatus::Open | SlashCaseStatus::UnderReview => {
                record.status = SlashCaseStatus::Rejected;
                record.resolved_at = Some(1);
                Ok(())
            }
            _ => Err(SwapError::InvalidSlashStatus {
                slash_id,
                reason: alloc::format!("cannot reject case in status {:?}", record.status),
            }),
        }
    }

    /// Get a slashing case by ID.
    pub fn get_case(&self, slash_id: u64) -> Option<&SlashRecord> {
        self.slash_cases.get(&slash_id)
    }

    /// Get all slashing cases for a given actor.
    pub fn get_actor_cases(&self, actor_id: &str) -> Vec<&SlashRecord> {
        self.slash_cases
            .values()
            .filter(|r| r.actor_id == actor_id)
            .collect()
    }

    /// Get the reputation score for an actor (default 0).
    pub fn get_actor_reputation(&self, actor_id: &str) -> i64 {
        self.actor_reputation.get(actor_id).copied().unwrap_or(0)
    }

    /// Get the stake for an actor (default 0).
    pub fn get_actor_stake(&self, actor_id: &str) -> u128 {
        self.actor_stake.get(actor_id).copied().unwrap_or(0)
    }

    /// Directly slash an actor's stake without opening a case.
    ///
    /// # Errors
    ///
    /// - `InsufficientStake` but still reduces to 0.
    pub fn slash_actor(&mut self, actor_id: &str, amount: u128) -> Result<(), SwapError> {
        let current = self.actor_stake.get(actor_id).copied().unwrap_or(0);
        let new_stake = current.saturating_sub(amount);
        self.actor_stake.insert(actor_id.to_string(), new_stake);
        Ok(())
    }

    /// Slash an actor's stake and apply a reputation penalty in one call.
    pub fn slash_and_reduce_reputation(
        &mut self,
        actor_id: &str,
        slash_amount: u128,
        reputation_penalty: i64,
    ) -> Result<(), SwapError> {
        self.slash_actor(actor_id, slash_amount)?;
        let current_rep = self.actor_reputation.get(actor_id).copied().unwrap_or(0);
        let new_rep = current_rep.saturating_sub(reputation_penalty).max(0);
        self.actor_reputation.insert(actor_id.to_string(), new_rep);
        Ok(())
    }

    /// Convenience: open a false-proof slashing case.
    pub fn record_false_proof(
        &mut self,
        actor_id: String,
        actor_type: SlashableActor,
        intent_id: u64,
        evidence: Vec<u8>,
    ) -> Result<u64, SwapError> {
        self.open_case(
            actor_id,
            actor_type,
            Some(intent_id),
            SlashReason::FalseProof(alloc::format!(
                "false proof submitted for intent {}",
                intent_id
            )),
            evidence,
            100,
        )
    }

    /// Convenience: open a missed-claim slashing case.
    pub fn record_missed_claim(
        &mut self,
        relayer_id: String,
        intent_id: u64,
        evidence: Vec<u8>,
    ) -> Result<u64, SwapError> {
        self.open_case(
            relayer_id,
            SlashableActor::Relayer,
            Some(intent_id),
            SlashReason::MissedAssignedClaim(alloc::format!(
                "relayer missed claim for intent {}",
                intent_id
            )),
            evidence,
            50,
        )
    }

    /// Convenience: open a missed-refund slashing case.
    pub fn record_missed_refund(
        &mut self,
        relayer_id: String,
        intent_id: u64,
        evidence: Vec<u8>,
    ) -> Result<u64, SwapError> {
        self.open_case(
            relayer_id,
            SlashableActor::Relayer,
            Some(intent_id),
            SlashReason::MissedAssignedRefund(alloc::format!(
                "relayer missed refund for intent {}",
                intent_id
            )),
            evidence,
            50,
        )
    }

    /// Convenience: open an RPC manipulation slashing case.
    pub fn record_rpc_manipulation(
        &mut self,
        actor_id: String,
        actor_type: SlashableActor,
        evidence: Vec<u8>,
    ) -> Result<u64, SwapError> {
        self.open_case(
            actor_id,
            actor_type,
            None,
            SlashReason::RpcManipulation("RPC response manipulation detected".into()),
            evidence,
            200,
        )
    }

    /// Count of active (Open + UnderReview) cases.
    pub fn total_active_cases(&self) -> usize {
        self.slash_cases
            .values()
            .filter(|r| {
                matches!(
                    r.status,
                    SlashCaseStatus::Open | SlashCaseStatus::UnderReview
                )
            })
            .count()
    }

    /// Sum of slash amounts for all active (Open + UnderReview) cases.
    pub fn active_slash_amount(&self) -> u128 {
        self.slash_cases
            .values()
            .filter(|r| {
                matches!(
                    r.status,
                    SlashCaseStatus::Open | SlashCaseStatus::UnderReview
                )
            })
            .map(|r| r.amount)
            .sum()
    }

    /// Build a summary of the current slashing state.
    pub fn slash_summary(&self) -> SlashSummary {
        let total_cases = self.slash_cases.len() as u64;
        let open_cases = self
            .slash_cases
            .values()
            .filter(|r| r.status == SlashCaseStatus::Open)
            .count() as u64;
        let resolved_cases = self
            .slash_cases
            .values()
            .filter(|r| r.status == SlashCaseStatus::Resolved)
            .count() as u64;
        let rejected_cases = self
            .slash_cases
            .values()
            .filter(|r| r.status == SlashCaseStatus::Rejected)
            .count() as u64;

        let total_slashed_amount: u128 = self
            .slash_cases
            .values()
            .filter(|r| r.status == SlashCaseStatus::Resolved)
            .map(|r| r.amount)
            .sum();

        let active_slash_amount = self.active_slash_amount();

        // Worst actors: reputation ascending (worst first).
        let mut worst_actors: Vec<(String, i64)> = self
            .actor_reputation
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        worst_actors.sort_by(|a, b| a.1.cmp(&b.1));

        SlashSummary {
            total_cases,
            open_cases,
            resolved_cases,
            rejected_cases,
            total_slashed_amount,
            active_slash_amount,
            worst_actors,
        }
    }
}

impl Default for SlashingEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helper: build a SlashingEngine with seeded stake and reputation
    // ------------------------------------------------------------------
    fn seeded_engine() -> SlashingEngine {
        let mut eng = SlashingEngine::new();
        eng.actor_stake.insert("alice".into(), 1000);
        eng.actor_stake.insert("bob".into(), 2000);
        eng.actor_stake.insert("carol".into(), 500);
        eng.actor_reputation.insert("alice".into(), 100);
        eng.actor_reputation.insert("bob".into(), 200);
        eng.actor_reputation.insert("carol".into(), 50);
        eng
    }

    fn dummy_evidence() -> Vec<u8> {
        vec![0u8; 16] // >= min_evidence_size (8)
    }

    // ----------------------------------------------------------
    // 1. Opening cases for all actor types
    // ----------------------------------------------------------
    #[test]
    fn test_open_case_relayer() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "relayer1".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::MissedAssignedClaim("missed claim 42".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        assert_eq!(id, 1);
        let record = eng.get_case(id).unwrap();
        assert_eq!(record.actor_id, "relayer1");
        assert_eq!(record.actor_type, SlashableActor::Relayer);
        assert_eq!(record.status, SlashCaseStatus::Open);
    }

    #[test]
    fn test_open_case_solver() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "solver1".into(),
                SlashableActor::Solver,
                None,
                SlashReason::StaleQuoteFillFailure("stale quote 99".into()),
                dummy_evidence(),
                200,
            )
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(eng.get_case(id).unwrap().actor_type, SlashableActor::Solver);
    }

    #[test]
    fn test_open_case_validator_prover() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "val1".into(),
                SlashableActor::ValidatorProver,
                None,
                SlashReason::FalseProof("invalid ZK proof".into()),
                dummy_evidence(),
                500,
            )
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(
            eng.get_case(id).unwrap().actor_type,
            SlashableActor::ValidatorProver
        );
    }

    #[test]
    fn test_open_case_watcher() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "watcher1".into(),
                SlashableActor::Watcher,
                None,
                SlashReason::CensorshipGriefing("withheld blocks".into()),
                dummy_evidence(),
                150,
            )
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(
            eng.get_case(id).unwrap().actor_type,
            SlashableActor::Watcher
        );
    }

    // ----------------------------------------------------------
    // 2. Opening cases for all slash reasons
    // ----------------------------------------------------------
    #[test]
    fn test_open_case_false_proof() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "a".into(),
                SlashableActor::Solver,
                Some(1),
                SlashReason::FalseProof("fake proof evidence".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::FalseProof(_)
        ));
    }

    #[test]
    fn test_open_case_missed_claim() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "b".into(),
                SlashableActor::Relayer,
                Some(2),
                SlashReason::MissedAssignedClaim("intent 2".into()),
                dummy_evidence(),
                50,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::MissedAssignedClaim(_)
        ));
    }

    #[test]
    fn test_open_case_missed_refund() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "c".into(),
                SlashableActor::Relayer,
                Some(3),
                SlashReason::MissedAssignedRefund("intent 3".into()),
                dummy_evidence(),
                50,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::MissedAssignedRefund(_)
        ));
    }

    #[test]
    fn test_open_case_stale_quote() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "d".into(),
                SlashableActor::Solver,
                Some(4),
                SlashReason::StaleQuoteFillFailure("quote 4".into()),
                dummy_evidence(),
                75,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::StaleQuoteFillFailure(_)
        ));
    }

    #[test]
    fn test_open_case_invalid_finality() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "e".into(),
                SlashableActor::ValidatorProver,
                Some(5),
                SlashReason::InvalidFinalityClaim("chain X".into()),
                dummy_evidence(),
                120,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::InvalidFinalityClaim(_)
        ));
    }

    #[test]
    fn test_open_case_rpc_manipulation() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "f".into(),
                SlashableActor::Watcher,
                None,
                SlashReason::RpcManipulation("fake RPC responses".into()),
                dummy_evidence(),
                200,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::RpcManipulation(_)
        ));
    }

    #[test]
    fn test_open_case_censorship_griefing() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "g".into(),
                SlashableActor::Watcher,
                None,
                SlashReason::CensorshipGriefing("censored txs".into()),
                dummy_evidence(),
                180,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::CensorshipGriefing(_)
        ));
    }

    #[test]
    fn test_open_case_fake_success() {
        let mut eng = SlashingEngine::new();
        let id = eng
            .open_case(
                "h".into(),
                SlashableActor::Relayer,
                Some(6),
                SlashReason::FakeSuccessReport("intent 6".into()),
                dummy_evidence(),
                300,
            )
            .unwrap();
        assert!(matches!(
            eng.get_case(id).unwrap().reason,
            SlashReason::FakeSuccessReport(_)
        ));
    }

    // ----------------------------------------------------------
    // 3. Resolving cases
    // ----------------------------------------------------------
    #[test]
    fn test_resolve_case_reduces_stake() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::MissedAssignedClaim("test".into()),
                dummy_evidence(),
                300,
            )
            .unwrap();
        assert_eq!(eng.get_actor_stake("alice"), 1000);
        eng.resolve_case(id).unwrap();
        assert_eq!(eng.get_actor_stake("alice"), 700);
        assert_eq!(eng.get_case(id).unwrap().status, SlashCaseStatus::Resolved);
        assert!(eng.get_case(id).unwrap().resolved_at.is_some());
    }

    #[test]
    fn test_resolve_case_under_review() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "bob".into(),
                SlashableActor::Solver,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        // Manually set to UnderReview
        let record = eng.slash_cases.get_mut(&id).unwrap();
        record.status = SlashCaseStatus::UnderReview;
        // Now resolve
        eng.resolve_case(id).unwrap();
        assert_eq!(eng.get_case(id).unwrap().status, SlashCaseStatus::Resolved);
    }

    // ----------------------------------------------------------
    // 4. Rejecting cases
    // ----------------------------------------------------------
    #[test]
    fn test_reject_case() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::CensorshipGriefing("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        let stake_before = eng.get_actor_stake("alice");
        eng.reject_case(id).unwrap();
        // Stake unchanged
        assert_eq!(eng.get_actor_stake("alice"), stake_before);
        assert_eq!(eng.get_case(id).unwrap().status, SlashCaseStatus::Rejected);
    }

    // ----------------------------------------------------------
    // 5. Insufficient evidence rejected
    // ----------------------------------------------------------
    #[test]
    fn test_insufficient_evidence_rejected() {
        let mut eng = SlashingEngine::new();
        let short_evidence = vec![0u8; 4]; // only 4 bytes < 8
        let result = eng.open_case(
            "alice".into(),
            SlashableActor::Relayer,
            None,
            SlashReason::FalseProof("test".into()),
            short_evidence,
            100,
        );
        assert!(result.is_err());
        match result.unwrap_err() {
            SwapError::InsufficientEvidence { minimum, actual } => {
                assert_eq!(minimum, 8);
                assert_eq!(actual, 4);
            }
            other => panic!("Expected InsufficientEvidence, got {:?}", other),
        }
    }

    // ----------------------------------------------------------
    // 6. Slashing reduces stake correctly
    // ----------------------------------------------------------
    #[test]
    fn test_slash_reduces_stake() {
        let mut eng = seeded_engine();
        assert_eq!(eng.get_actor_stake("alice"), 1000);
        eng.slash_actor("alice", 400).unwrap();
        assert_eq!(eng.get_actor_stake("alice"), 600);
    }

    // ----------------------------------------------------------
    // 7. Slashing with amount > stake reduces to 0
    // ----------------------------------------------------------
    #[test]
    fn test_slash_exceeding_stake_reduces_to_zero() {
        let mut eng = seeded_engine();
        assert_eq!(eng.get_actor_stake("carol"), 500);
        eng.slash_actor("carol", 9999).unwrap();
        assert_eq!(eng.get_actor_stake("carol"), 0);
    }

    // ----------------------------------------------------------
    // 8. Reputation penalty applied
    // ----------------------------------------------------------
    #[test]
    fn test_slash_reduces_reputation() {
        let mut eng = seeded_engine();
        assert_eq!(eng.get_actor_reputation("bob"), 200);
        eng.slash_and_reduce_reputation("bob", 100, 50).unwrap();
        assert_eq!(eng.get_actor_stake("bob"), 1900);
        assert_eq!(eng.get_actor_reputation("bob"), 150);
    }

    // ----------------------------------------------------------
    // 9. slash_and_reduce_reputation works
    // ----------------------------------------------------------
    #[test]
    fn test_slash_and_reduce_reputation_full() {
        let mut eng = seeded_engine();
        // alice: stake=1000, rep=100
        eng.slash_and_reduce_reputation("alice", 600, 80).unwrap();
        assert_eq!(eng.get_actor_stake("alice"), 400);
        assert_eq!(eng.get_actor_reputation("alice"), 20);

        // Reputation saturating at 0
        eng.slash_and_reduce_reputation("alice", 0, 100).unwrap();
        assert_eq!(eng.get_actor_reputation("alice"), 0);

        // Should not go negative
        eng.slash_and_reduce_reputation("alice", 0, 999).unwrap();
        assert_eq!(eng.get_actor_reputation("alice"), 0);
    }

    // ----------------------------------------------------------
    // 10. Cannot resolve already resolved case
    // ----------------------------------------------------------
    #[test]
    fn test_cannot_resolve_already_resolved() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        eng.resolve_case(id).unwrap();
        let result = eng.resolve_case(id);
        assert!(result.is_err());
        match result.unwrap_err() {
            SwapError::InvalidSlashStatus { slash_id, .. } => {
                assert_eq!(slash_id, id);
            }
            other => panic!("Expected InvalidSlashStatus, got {:?}", other),
        }
    }

    // ----------------------------------------------------------
    // 11. Cannot resolve already rejected case
    // ----------------------------------------------------------
    #[test]
    fn test_cannot_resolve_already_rejected() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        eng.reject_case(id).unwrap();
        let result = eng.resolve_case(id);
        assert!(result.is_err());
    }

    // ----------------------------------------------------------
    // 12. Cannot reject already resolved case
    // ----------------------------------------------------------
    #[test]
    fn test_cannot_reject_already_resolved() {
        let mut eng = seeded_engine();
        let id = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        eng.resolve_case(id).unwrap();
        let result = eng.reject_case(id);
        assert!(result.is_err());
    }

    // ----------------------------------------------------------
    // 13. get_actor_cases returns correct cases
    // ----------------------------------------------------------
    #[test]
    fn test_get_actor_cases() {
        let mut eng = seeded_engine();
        eng.open_case(
            "alice".into(),
            SlashableActor::Relayer,
            None,
            SlashReason::FalseProof("fp1".into()),
            dummy_evidence(),
            100,
        )
        .unwrap();
        eng.open_case(
            "alice".into(),
            SlashableActor::Relayer,
            None,
            SlashReason::MissedAssignedClaim("mc1".into()),
            dummy_evidence(),
            50,
        )
        .unwrap();
        eng.open_case(
            "bob".into(),
            SlashableActor::Solver,
            None,
            SlashReason::StaleQuoteFillFailure("sq1".into()),
            dummy_evidence(),
            75,
        )
        .unwrap();

        let alice_cases = eng.get_actor_cases("alice");
        assert_eq!(alice_cases.len(), 2);
        assert!(alice_cases.iter().all(|r| r.actor_id == "alice"));

        let bob_cases = eng.get_actor_cases("bob");
        assert_eq!(bob_cases.len(), 1);

        let unknown_cases = eng.get_actor_cases("unknown");
        assert!(unknown_cases.is_empty());
    }

    // ----------------------------------------------------------
    // 14. get_actor_reputation returns correct value
    // ----------------------------------------------------------
    #[test]
    fn test_get_actor_reputation() {
        let mut eng = SlashingEngine::new();
        assert_eq!(eng.get_actor_reputation("nonexistent"), 0);
        eng.actor_reputation.insert("alice".into(), 42);
        assert_eq!(eng.get_actor_reputation("alice"), 42);
    }

    // ----------------------------------------------------------
    // 15. get_actor_stake returns correct value
    // ----------------------------------------------------------
    #[test]
    fn test_get_actor_stake() {
        let mut eng = SlashingEngine::new();
        assert_eq!(eng.get_actor_stake("nonexistent"), 0);
        eng.actor_stake.insert("alice".into(), 999);
        assert_eq!(eng.get_actor_stake("alice"), 999);
    }

    // ----------------------------------------------------------
    // 16. total_active_cases count
    // ----------------------------------------------------------
    #[test]
    fn test_total_active_cases() {
        let mut eng = seeded_engine();
        assert_eq!(eng.total_active_cases(), 0);

        let id1 = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        assert_eq!(eng.total_active_cases(), 1);

        let id2 = eng
            .open_case(
                "bob".into(),
                SlashableActor::Solver,
                None,
                SlashReason::StaleQuoteFillFailure("test".into()),
                dummy_evidence(),
                75,
            )
            .unwrap();
        assert_eq!(eng.total_active_cases(), 2);

        eng.resolve_case(id1).unwrap();
        assert_eq!(eng.total_active_cases(), 1);

        eng.reject_case(id2).unwrap();
        assert_eq!(eng.total_active_cases(), 0);
    }

    // ----------------------------------------------------------
    // 17. active_slash_amount sum
    // ----------------------------------------------------------
    #[test]
    fn test_active_slash_amount() {
        let mut eng = seeded_engine();
        assert_eq!(eng.active_slash_amount(), 0);

        eng.open_case(
            "alice".into(),
            SlashableActor::Relayer,
            None,
            SlashReason::FalseProof("test".into()),
            dummy_evidence(),
            100,
        )
        .unwrap();
        eng.open_case(
            "bob".into(),
            SlashableActor::Solver,
            None,
            SlashReason::StaleQuoteFillFailure("test".into()),
            dummy_evidence(),
            250,
        )
        .unwrap();
        assert_eq!(eng.active_slash_amount(), 350);
    }

    // ----------------------------------------------------------
    // 18. slash_summary aggregation
    // ----------------------------------------------------------
    #[test]
    fn test_slash_summary() {
        let mut eng = seeded_engine();

        let id1 = eng
            .open_case(
                "alice".into(),
                SlashableActor::Relayer,
                None,
                SlashReason::FalseProof("test".into()),
                dummy_evidence(),
                100,
            )
            .unwrap();
        let id2 = eng
            .open_case(
                "bob".into(),
                SlashableActor::Solver,
                None,
                SlashReason::StaleQuoteFillFailure("test".into()),
                dummy_evidence(),
                200,
            )
            .unwrap();
        let _id3 = eng
            .open_case(
                "carol".into(),
                SlashableActor::Watcher,
                None,
                SlashReason::CensorshipGriefing("test".into()),
                dummy_evidence(),
                150,
            )
            .unwrap();

        // Resolve id1, reject id2
        eng.resolve_case(id1).unwrap();
        eng.reject_case(id2).unwrap();

        let summary = eng.slash_summary();
        assert_eq!(summary.total_cases, 3);
        assert_eq!(summary.open_cases, 1); // id3 still open
        assert_eq!(summary.resolved_cases, 1);
        assert_eq!(summary.rejected_cases, 1);
        assert_eq!(summary.total_slashed_amount, 100); // only id1 resolved
        assert_eq!(summary.active_slash_amount, 150); // id3 is open

        // Worst actors sorted by reputation ascending
        // carol=50, alice=100, bob=200
        assert_eq!(summary.worst_actors.len(), 3);
        assert_eq!(summary.worst_actors[0].0, "carol");
        assert_eq!(summary.worst_actors[1].0, "alice");
        assert_eq!(summary.worst_actors[2].0, "bob");
    }

    // ----------------------------------------------------------
    // 19. record_false_proof convenience
    // ----------------------------------------------------------
    #[test]
    fn test_record_false_proof() {
        let mut eng = seeded_engine();
        let id = eng
            .record_false_proof("alice".into(), SlashableActor::Solver, 42, dummy_evidence())
            .unwrap();
        let record = eng.get_case(id).unwrap();
        assert_eq!(record.actor_id, "alice");
        assert_eq!(record.actor_type, SlashableActor::Solver);
        assert_eq!(record.intent_id, Some(42));
        assert!(matches!(record.reason, SlashReason::FalseProof(_)));
        assert_eq!(record.amount, 100);
    }

    // ----------------------------------------------------------
    // 20. record_missed_claim convenience
    // ----------------------------------------------------------
    #[test]
    fn test_record_missed_claim() {
        let mut eng = seeded_engine();
        let id = eng
            .record_missed_claim("relayer1".into(), 77, dummy_evidence())
            .unwrap();
        let record = eng.get_case(id).unwrap();
        assert_eq!(record.actor_id, "relayer1");
        assert_eq!(record.actor_type, SlashableActor::Relayer);
        assert_eq!(record.intent_id, Some(77));
        assert!(matches!(record.reason, SlashReason::MissedAssignedClaim(_)));
        assert_eq!(record.amount, 50);
    }

    // ----------------------------------------------------------
    // 21. record_missed_refund convenience
    // ----------------------------------------------------------
    #[test]
    fn test_record_missed_refund() {
        let mut eng = seeded_engine();
        let id = eng
            .record_missed_refund("relayer2".into(), 88, dummy_evidence())
            .unwrap();
        let record = eng.get_case(id).unwrap();
        assert_eq!(record.actor_id, "relayer2");
        assert_eq!(record.actor_type, SlashableActor::Relayer);
        assert_eq!(record.intent_id, Some(88));
        assert!(matches!(
            record.reason,
            SlashReason::MissedAssignedRefund(_)
        ));
        assert_eq!(record.amount, 50);
    }

    // ----------------------------------------------------------
    // 22. record_rpc_manipulation convenience
    // ----------------------------------------------------------
    #[test]
    fn test_record_rpc_manipulation() {
        let mut eng = seeded_engine();
        let id = eng
            .record_rpc_manipulation("watcher1".into(), SlashableActor::Watcher, dummy_evidence())
            .unwrap();
        let record = eng.get_case(id).unwrap();
        assert_eq!(record.actor_id, "watcher1");
        assert_eq!(record.actor_type, SlashableActor::Watcher);
        assert!(record.intent_id.is_none());
        assert!(matches!(record.reason, SlashReason::RpcManipulation(_)));
        assert_eq!(record.amount, 200);
    }

    // ----------------------------------------------------------
    // 23. SlashNotFound error
    // ----------------------------------------------------------
    #[test]
    fn test_slash_not_found() {
        let mut eng = seeded_engine();
        let result = eng.resolve_case(999);
        assert!(result.is_err());
        match result.unwrap_err() {
            SwapError::SlashNotFound { slash_id } => {
                assert_eq!(slash_id, 999);
            }
            other => panic!("Expected SlashNotFound, got {:?}", other),
        }
    }

    // ----------------------------------------------------------
    // 24. Default engine initializes correctly
    // ----------------------------------------------------------
    #[test]
    fn test_default_engine() {
        let eng = SlashingEngine::default();
        assert!(eng.slash_cases.is_empty());
        assert!(eng.actor_reputation.is_empty());
        assert!(eng.actor_stake.is_empty());
        assert_eq!(eng.next_slash_id, 1);
        assert_eq!(eng.min_evidence_size, 8);
    }

    // ----------------------------------------------------------
    // 25. SlashableActor name() returns correct string
    // ----------------------------------------------------------
    #[test]
    fn test_slashable_actor_name() {
        assert_eq!(SlashableActor::Relayer.name(), "Relayer");
        assert_eq!(SlashableActor::Solver.name(), "Solver");
        assert_eq!(SlashableActor::ValidatorProver.name(), "ValidatorProver");
        assert_eq!(SlashableActor::Watcher.name(), "Watcher");
    }

    // ----------------------------------------------------------
    // 26. SlashReason code() returns correct codes
    // ----------------------------------------------------------
    #[test]
    fn test_slash_reason_code() {
        assert_eq!(SlashReason::FalseProof("x".into()).code(), "FALSE_PROOF");
        assert_eq!(
            SlashReason::MissedAssignedClaim("x".into()).code(),
            "MISSED_CLAIM"
        );
        assert_eq!(
            SlashReason::MissedAssignedRefund("x".into()).code(),
            "MISSED_REFUND"
        );
        assert_eq!(
            SlashReason::StaleQuoteFillFailure("x".into()).code(),
            "STALE_QUOTE"
        );
        assert_eq!(
            SlashReason::InvalidFinalityClaim("x".into()).code(),
            "INVALID_FINALITY"
        );
        assert_eq!(
            SlashReason::RpcManipulation("x".into()).code(),
            "RPC_MANIPULATION"
        );
        assert_eq!(
            SlashReason::CensorshipGriefing("x".into()).code(),
            "CENSORSHIP_GRIEFING"
        );
        assert_eq!(
            SlashReason::FakeSuccessReport("x".into()).code(),
            "FAKE_SUCCESS"
        );
    }

    // ----------------------------------------------------------
    // 27. SlashReason description() returns embedded string
    // ----------------------------------------------------------
    #[test]
    fn test_slash_reason_description() {
        let r = SlashReason::FalseProof("fake proof data".into());
        assert_eq!(r.description(), "fake proof data");
    }

    // ----------------------------------------------------------
    // 28. get_case returns None for non-existent
    // ----------------------------------------------------------
    #[test]
    fn test_get_case_nonexistent() {
        let eng = seeded_engine();
        assert!(eng.get_case(999).is_none());
    }
}
