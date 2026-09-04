//! # Dispute Resolution System
//!
//! Handles disputes between actors in atomic swaps: relayers claiming they
//! submitted on time, solvers contesting stale fills, validators challenging
//! proofs, and watchers reporting manipulation.
//!
//! ## Resolution paths
//! - **Relayer dispute**: Did the relayer submit a claim before timeout?
//! - **Solver dispute**: Did the solver fill with a stale quote?
//! - **Proof dispute**: Is the finality or transfer proof valid?
//! - **Watcher dispute**: Did a watcher censor or fabricate events?

use crate::error::SwapError;
use crate::slashing::{SlashReason, SlashableActor, SlashingEngine};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Kinds of disputes that can be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeKind {
    /// Disagreement about whether a claim was submitted before timeout.
    ClaimSubmittedOnTime,
    /// Disagreement about whether a refund was submitted before a competing claim.
    RefundVsClaimRace,
    /// Disagreement about whether a solver filled with a stale quote.
    StaleQuoteFill,
    /// Disagreement about the validity of a finality proof.
    InvalidFinalityProof,
    /// Disagreement about whether a transfer proof is valid.
    InvalidTransferProof,
    /// Disagreement about RPC censorship or manipulation.
    RpcManipulation,
    /// Disagreement about a faked success report.
    FakeSuccessReport,
}

impl DisputeKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            DisputeKind::ClaimSubmittedOnTime => "claim_on_time",
            DisputeKind::RefundVsClaimRace => "refund_vs_claim_race",
            DisputeKind::StaleQuoteFill => "stale_quote_fill",
            DisputeKind::InvalidFinalityProof => "invalid_finality_proof",
            DisputeKind::InvalidTransferProof => "invalid_transfer_proof",
            DisputeKind::RpcManipulation => "rpc_manipulation",
            DisputeKind::FakeSuccessReport => "fake_success_report",
        }
    }

    /// The slashing reason that corresponds to this dispute kind.
    pub fn slash_reason(&self, detail: String) -> SlashReason {
        match self {
            DisputeKind::ClaimSubmittedOnTime => SlashReason::MissedAssignedClaim(detail),
            DisputeKind::RefundVsClaimRace => SlashReason::MissedAssignedRefund(detail),
            DisputeKind::StaleQuoteFill => SlashReason::StaleQuoteFillFailure(detail),
            DisputeKind::InvalidFinalityProof => SlashReason::InvalidFinalityClaim(detail),
            DisputeKind::InvalidTransferProof => SlashReason::FalseProof(detail),
            DisputeKind::RpcManipulation => SlashReason::RpcManipulation(detail),
            DisputeKind::FakeSuccessReport => SlashReason::FakeSuccessReport(detail),
        }
    }
}

/// Status of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeStatus {
    /// Dispute has been filed but not yet reviewed.
    Filed,
    /// Dispute is under active arbitration.
    UnderArbitration,
    /// Dispute resolved in favor of the plaintiff.
    ResolvedForPlaintiff,
    /// Dispute resolved in favor of the defendant.
    ResolvedForDefendant,
    /// Dispute dismissed (no merit).
    Dismissed,
}

/// Evidence bundle submitted with a dispute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeEvidence {
    /// Hash of the transaction in question.
    pub tx_hash: Option<String>,
    /// Block number or slot at which the disputed action occurred.
    pub block_number: Option<u64>,
    /// Raw proof data (finality proof, transfer proof, etc.).
    pub proof_data: Vec<u8>,
    /// Witness signatures or attestations.
    pub witness_attestations: Vec<String>,
    /// Free-form description of the dispute.
    pub description: String,
}

/// A dispute record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeRecord {
    /// Unique dispute identifier.
    pub dispute_id: u64,
    /// Intent ID this dispute relates to (if any).
    pub intent_id: Option<u64>,
    /// Kind of dispute.
    pub kind: DisputeKind,
    /// Actor ID of the party filing the dispute.
    pub plaintiff_id: String,
    /// Actor type of the plaintiff.
    pub plaintiff_type: SlashableActor,
    /// Actor ID of the party being disputed.
    pub defendant_id: String,
    /// Actor type of the defendant.
    pub defendant_type: SlashableActor,
    /// Evidence submitted.
    pub evidence: DisputeEvidence,
    /// Current status.
    pub status: DisputeStatus,
    /// Timestamp when filed.
    pub filed_at: u64,
    /// Timestamp when resolved.
    pub resolved_at: Option<u64>,
    /// Resolution description.
    pub resolution: Option<String>,
}

/// Summary statistics for the dispute system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisputeSummary {
    /// Total disputes ever filed.
    pub total_disputes: u64,
    /// Currently filed / under arbitration.
    pub active_disputes: u64,
    /// Resolved for plaintiff.
    pub resolved_for_plaintiff: u64,
    /// Resolved for defendant.
    pub resolved_for_defendant: u64,
    /// Dismissed.
    pub dismissed: u64,
    /// Total amount slashed from dispute resolutions.
    pub total_slashed_from_disputes: u128,
}

/// Dispute resolution engine.
#[derive(Debug, Clone)]
pub struct DisputeEngine {
    /// Dispute records keyed by dispute_id.
    pub disputes: BTreeMap<u64, DisputeRecord>,
    /// Next dispute ID.
    pub next_dispute_id: u64,
    /// Link to the slashing engine for penalty enforcement.
    pub slashing_engine: SlashingEngine,
}

impl DisputeEngine {
    /// Create a new dispute engine.
    pub fn new() -> Self {
        Self {
            disputes: BTreeMap::new(),
            next_dispute_id: 1,
            slashing_engine: SlashingEngine::new(),
        }
    }

    /// Create a new dispute engine sharing an existing slashing engine.
    pub fn with_slashing_engine(slashing_engine: SlashingEngine) -> Self {
        Self {
            disputes: BTreeMap::new(),
            next_dispute_id: 1,
            slashing_engine,
        }
    }

    /// File a new dispute.
    ///
    /// Returns the dispute ID on success.
    pub fn file_dispute(
        &mut self,
        intent_id: Option<u64>,
        kind: DisputeKind,
        plaintiff_id: String,
        plaintiff_type: SlashableActor,
        defendant_id: String,
        defendant_type: SlashableActor,
        evidence: DisputeEvidence,
    ) -> Result<u64, SwapError> {
        let dispute_id = self.next_dispute_id;
        self.next_dispute_id += 1;

        let record = DisputeRecord {
            dispute_id,
            intent_id,
            kind,
            plaintiff_id,
            plaintiff_type,
            defendant_id,
            defendant_type,
            evidence,
            status: DisputeStatus::Filed,
            filed_at: 0,
            resolved_at: None,
            resolution: None,
        };

        self.disputes.insert(dispute_id, record);
        Ok(dispute_id)
    }

    /// Resolve a dispute in favor of the plaintiff.
    ///
    /// Slashes the defendant using the slashing engine with the dispute kind's
    /// corresponding slash reason.
    pub fn resolve_for_plaintiff(
        &mut self,
        dispute_id: u64,
        slash_amount: u128,
        resolution: String,
    ) -> Result<(), SwapError> {
        let record = self
            .disputes
            .get_mut(&dispute_id)
            .ok_or(SwapError::DisputeNotFound { dispute_id })?;

        if !matches!(
            record.status,
            DisputeStatus::Filed | DisputeStatus::UnderArbitration
        ) {
            return Err(SwapError::InvalidDisputeStatus {
                dispute_id,
                reason: alloc::format!("cannot resolve dispute in status {:?}", record.status),
            });
        }

        let slash_reason = record
            .kind
            .slash_reason(alloc::format!("dispute {} resolution", dispute_id));

        // Open and immediately resolve a slashing case against the defendant.
        let slash_id = self.slashing_engine.open_case(
            record.defendant_id.clone(),
            record.defendant_type,
            record.intent_id,
            slash_reason,
            record.evidence.proof_data.clone(),
            slash_amount,
        )?;
        self.slashing_engine.resolve_case(slash_id)?;

        record.status = DisputeStatus::ResolvedForPlaintiff;
        record.resolved_at = Some(1);
        record.resolution = Some(resolution);
        Ok(())
    }

    /// Resolve a dispute in favor of the defendant (no slash applied).
    pub fn resolve_for_defendant(
        &mut self,
        dispute_id: u64,
        resolution: String,
    ) -> Result<(), SwapError> {
        let record = self
            .disputes
            .get_mut(&dispute_id)
            .ok_or(SwapError::DisputeNotFound { dispute_id })?;

        if !matches!(
            record.status,
            DisputeStatus::Filed | DisputeStatus::UnderArbitration
        ) {
            return Err(SwapError::InvalidDisputeStatus {
                dispute_id,
                reason: alloc::format!("cannot resolve dispute in status {:?}", record.status),
            });
        }

        record.status = DisputeStatus::ResolvedForDefendant;
        record.resolved_at = Some(1);
        record.resolution = Some(resolution);
        Ok(())
    }

    /// Dismiss a dispute (no merit, no slash).
    pub fn dismiss_dispute(&mut self, dispute_id: u64, reason: String) -> Result<(), SwapError> {
        let record = self
            .disputes
            .get_mut(&dispute_id)
            .ok_or(SwapError::DisputeNotFound { dispute_id })?;

        if !matches!(
            record.status,
            DisputeStatus::Filed | DisputeStatus::UnderArbitration
        ) {
            return Err(SwapError::InvalidDisputeStatus {
                dispute_id,
                reason: alloc::format!("cannot dismiss dispute in status {:?}", record.status),
            });
        }

        record.status = DisputeStatus::Dismissed;
        record.resolved_at = Some(1);
        record.resolution = Some(reason);
        Ok(())
    }

    /// Move a dispute to under-arbitration status.
    pub fn take_under_arbitration(&mut self, dispute_id: u64) -> Result<(), SwapError> {
        let record = self
            .disputes
            .get_mut(&dispute_id)
            .ok_or(SwapError::DisputeNotFound { dispute_id })?;

        if record.status != DisputeStatus::Filed {
            return Err(SwapError::InvalidDisputeStatus {
                dispute_id,
                reason: alloc::format!(
                    "dispute must be in Filed status to take under arbitration, got {:?}",
                    record.status
                ),
            });
        }

        record.status = DisputeStatus::UnderArbitration;
        Ok(())
    }

    /// Get a dispute by ID.
    pub fn get_dispute(&self, dispute_id: u64) -> Option<&DisputeRecord> {
        self.disputes.get(&dispute_id)
    }

    /// Get all disputes involving an actor (as plaintiff or defendant).
    pub fn get_actor_disputes(&self, actor_id: &str) -> Vec<&DisputeRecord> {
        self.disputes
            .values()
            .filter(|r| r.plaintiff_id == actor_id || r.defendant_id == actor_id)
            .collect()
    }

    /// Count of active (Filed + UnderArbitration) disputes.
    pub fn active_dispute_count(&self) -> u64 {
        self.disputes
            .values()
            .filter(|r| {
                matches!(
                    r.status,
                    DisputeStatus::Filed | DisputeStatus::UnderArbitration
                )
            })
            .count() as u64
    }

    /// Build a summary of the dispute system.
    pub fn summary(&self) -> DisputeSummary {
        let total = self.disputes.len() as u64;
        let active = self.active_dispute_count();
        let mut for_plaintiff = 0u64;
        let mut for_defendant = 0u64;
        let mut dismissed = 0u64;

        for record in self.disputes.values() {
            match record.status {
                DisputeStatus::ResolvedForPlaintiff => for_plaintiff += 1,
                DisputeStatus::ResolvedForDefendant => for_defendant += 1,
                DisputeStatus::Dismissed => dismissed += 1,
                _ => {}
            }
        }

        let slash_summary = self.slashing_engine.slash_summary();
        DisputeSummary {
            total_disputes: total,
            active_disputes: active,
            resolved_for_plaintiff: for_plaintiff,
            resolved_for_defendant: for_defendant,
            dismissed,
            total_slashed_from_disputes: slash_summary.total_slashed_amount,
        }
    }
}

impl Default for DisputeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_evidence() -> DisputeEvidence {
        DisputeEvidence {
            tx_hash: Some("0xabc123".into()),
            block_number: Some(100),
            proof_data: vec![0u8; 16],
            witness_attestations: vec!["witness1".into()],
            description: "test dispute".into(),
        }
    }

    #[test]
    fn test_file_dispute() {
        let mut engine = DisputeEngine::new();
        let id = engine
            .file_dispute(
                Some(42),
                DisputeKind::ClaimSubmittedOnTime,
                "relayer1".into(),
                SlashableActor::Relayer,
                "relayer2".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();
        assert_eq!(id, 1);
        let record = engine.get_dispute(id).unwrap();
        assert_eq!(record.plaintiff_id, "relayer1");
        assert_eq!(record.defendant_id, "relayer2");
        assert_eq!(record.kind, DisputeKind::ClaimSubmittedOnTime);
        assert_eq!(record.status, DisputeStatus::Filed);
    }

    #[test]
    fn test_resolve_for_plaintiff_slashes_defendant() {
        let mut engine = DisputeEngine::new();
        // Seed the slashing engine with stake for the defendant
        engine
            .slashing_engine
            .actor_stake
            .insert("bad_relayer".into(), 1000);

        let id = engine
            .file_dispute(
                Some(1),
                DisputeKind::ClaimSubmittedOnTime,
                "good_relayer".into(),
                SlashableActor::Relayer,
                "bad_relayer".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();

        engine
            .resolve_for_plaintiff(id, 300, "defendant missed the claim window".into())
            .unwrap();

        let record = engine.get_dispute(id).unwrap();
        assert_eq!(record.status, DisputeStatus::ResolvedForPlaintiff);
        assert_eq!(engine.slashing_engine.get_actor_stake("bad_relayer"), 700);
    }

    #[test]
    fn test_resolve_for_defendant_no_slash() {
        let mut engine = DisputeEngine::new();
        engine
            .slashing_engine
            .actor_stake
            .insert("honest_relayer".into(), 1000);

        let id = engine
            .file_dispute(
                Some(2),
                DisputeKind::RefundVsClaimRace,
                "alice".into(),
                SlashableActor::Relayer,
                "honest_relayer".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();

        engine
            .resolve_for_defendant(id, "defendant acted correctly".into())
            .unwrap();

        assert_eq!(
            engine.slashing_engine.get_actor_stake("honest_relayer"),
            1000
        );
        let record = engine.get_dispute(id).unwrap();
        assert_eq!(record.status, DisputeStatus::ResolvedForDefendant);
    }

    #[test]
    fn test_dismiss_dispute() {
        let mut engine = DisputeEngine::new();
        let id = engine
            .file_dispute(
                None,
                DisputeKind::RpcManipulation,
                "watcher1".into(),
                SlashableActor::Watcher,
                "watcher2".into(),
                SlashableActor::Watcher,
                dummy_evidence(),
            )
            .unwrap();

        engine
            .dismiss_dispute(id, "no evidence of manipulation".into())
            .unwrap();
        let record = engine.get_dispute(id).unwrap();
        assert_eq!(record.status, DisputeStatus::Dismissed);
    }

    #[test]
    fn test_take_under_arbitration() {
        let mut engine = DisputeEngine::new();
        let id = engine
            .file_dispute(
                Some(3),
                DisputeKind::InvalidFinalityProof,
                "validator1".into(),
                SlashableActor::ValidatorProver,
                "validator2".into(),
                SlashableActor::ValidatorProver,
                dummy_evidence(),
            )
            .unwrap();

        engine.take_under_arbitration(id).unwrap();
        assert_eq!(
            engine.get_dispute(id).unwrap().status,
            DisputeStatus::UnderArbitration
        );
    }

    #[test]
    fn test_cannot_resolve_twice() {
        let mut engine = DisputeEngine::new();
        let id = engine
            .file_dispute(
                Some(4),
                DisputeKind::FakeSuccessReport,
                "a".into(),
                SlashableActor::Watcher,
                "b".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();

        engine
            .resolve_for_plaintiff(id, 100, "guilty".into())
            .unwrap();
        let result = engine.resolve_for_plaintiff(id, 100, "double jeopardy".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_summary() {
        let mut engine = DisputeEngine::new();
        engine
            .slashing_engine
            .actor_stake
            .insert("defendant".into(), 500);

        let id1 = engine
            .file_dispute(
                Some(10),
                DisputeKind::ClaimSubmittedOnTime,
                "p1".into(),
                SlashableActor::Relayer,
                "defendant".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();
        let _id2 = engine
            .file_dispute(
                Some(11),
                DisputeKind::StaleQuoteFill,
                "p2".into(),
                SlashableActor::Solver,
                "d2".into(),
                SlashableActor::Solver,
                dummy_evidence(),
            )
            .unwrap();
        let _id3 = engine
            .file_dispute(
                None,
                DisputeKind::RpcManipulation,
                "p3".into(),
                SlashableActor::Watcher,
                "d3".into(),
                SlashableActor::Watcher,
                dummy_evidence(),
            )
            .unwrap();

        engine
            .resolve_for_plaintiff(id1, 200, "guilty".into())
            .unwrap();
        engine.dismiss_dispute(2, "no merit".into()).unwrap();

        let summary = engine.summary();
        assert_eq!(summary.total_disputes, 3);
        assert_eq!(summary.active_disputes, 1);
        assert_eq!(summary.resolved_for_plaintiff, 1);
        assert_eq!(summary.dismissed, 1);
        assert_eq!(summary.total_slashed_from_disputes, 200);
    }

    #[test]
    fn test_get_actor_disputes() {
        let mut engine = DisputeEngine::new();
        let _id1 = engine
            .file_dispute(
                Some(1),
                DisputeKind::ClaimSubmittedOnTime,
                "alice".into(),
                SlashableActor::Relayer,
                "bob".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();
        let _id2 = engine
            .file_dispute(
                Some(2),
                DisputeKind::RefundVsClaimRace,
                "carol".into(),
                SlashableActor::Relayer,
                "alice".into(),
                SlashableActor::Relayer,
                dummy_evidence(),
            )
            .unwrap();

        let alice_disputes = engine.get_actor_disputes("alice");
        assert_eq!(alice_disputes.len(), 2); // plaintiff in id1, defendant in id2

        let bob_disputes = engine.get_actor_disputes("bob");
        assert_eq!(bob_disputes.len(), 1);

        let nobody_disputes = engine.get_actor_disputes("nobody");
        assert!(nobody_disputes.is_empty());
    }

    #[test]
    fn test_dispute_kind_slash_reason_mapping() {
        assert!(matches!(
            DisputeKind::ClaimSubmittedOnTime.slash_reason("test".into()),
            SlashReason::MissedAssignedClaim(_)
        ));
        assert!(matches!(
            DisputeKind::RefundVsClaimRace.slash_reason("test".into()),
            SlashReason::MissedAssignedRefund(_)
        ));
        assert!(matches!(
            DisputeKind::StaleQuoteFill.slash_reason("test".into()),
            SlashReason::StaleQuoteFillFailure(_)
        ));
        assert!(matches!(
            DisputeKind::InvalidFinalityProof.slash_reason("test".into()),
            SlashReason::InvalidFinalityClaim(_)
        ));
        assert!(matches!(
            DisputeKind::InvalidTransferProof.slash_reason("test".into()),
            SlashReason::FalseProof(_)
        ));
        assert!(matches!(
            DisputeKind::RpcManipulation.slash_reason("test".into()),
            SlashReason::RpcManipulation(_)
        ));
        assert!(matches!(
            DisputeKind::FakeSuccessReport.slash_reason("test".into()),
            SlashReason::FakeSuccessReport(_)
        ));
    }
}
