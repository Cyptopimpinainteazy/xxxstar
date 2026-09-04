//! Court integration for the X3 settlement engine.
//!
//! Wires the deterministic replay court into the settlement engine so that
//! failed atomic bundles can be disputed and adjudicated.
//!
//! ## Flow
//!
//! 1. `dispute_settlement` — file a dispute referencing a settlement intent + proof chain
//! 2. `resolve_dispute` — court replays execution, returns verdict
//! 3. On fraud verdict: reverse settlement, release escrow back to submitter
//! 4. On invalid challenge: slashes challenger's bond

use sp_core::H256;
use sp_runtime::DispatchResult;

/// Minimal dispute descriptor embedded in the settlement engine.
#[derive(Debug, Clone, PartialEq, Eq, codec::Encode, codec::Decode)]
pub struct SettlementDispute {
    /// The settlement intent being disputed.
    pub intent_id: H256,
    /// The bundle execution that produced the contested result.
    pub bundle_id: H256,
    /// Account that filed the dispute.
    pub challenger: [u8; 32],
    /// Account that executed the bundle (respondent).
    pub respondent: [u8; 32],
    /// Current dispute state.
    pub state: DisputeState,
    /// Reason for the dispute.
    pub reason: DisputeReason,
    /// Block number when the dispute was filed.
    pub filed_at: u64,
}

/// Dispute lifecycle state within the settlement engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, codec::Encode, codec::Decode)]
pub enum DisputeState {
    /// Dispute filed, awaiting court replay.
    Filed,
    /// Court replay completed, verdict pending.
    AwaitingVerdict,
    /// Verdict rendered — settlement reversed or confirmed.
    Resolved,
}

/// Reasons a settlement can be disputed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, codec::Encode, codec::Decode)]
pub enum DisputeReason {
    /// Bundle execution produced wrong receipt root.
    ReceiptMismatch,
    /// Bundle violated declared access set.
    AccessSetViolation,
    /// Executor submitted false finality certificate.
    FalseFinalityCert,
    /// Execution diverged from deterministic replay.
    ExecutionDivergence,
}

/// Integration trait that the settlement engine calls to resolve disputes.
///
/// Production runtimes wire this to `x3_court::Court::adjudicate`.
pub trait CourtAdapter {
    /// Submit a dispute for deterministic replay adjudication.
    /// Returns `Ok(true)` if the respondent is found fraudulent (verdict against them),
    /// `Ok(false)` if the challenge is invalid (challenger penalized),
    /// `Err` if the court cannot process the dispute.
    fn adjudicate(
        bundle_id: H256,
        challenger: &[u8; 32],
        respondent: &[u8; 32],
        reason: DisputeReason,
    ) -> Result<bool, &'static str>;
}

/// No-op court adapter for testing / pre-integration.
/// All disputes are immediately dismissed.
pub struct NoopCourtAdapter;
impl CourtAdapter for NoopCourtAdapter {
    fn adjudicate(
        _bundle_id: H256,
        _challenger: &[u8; 32],
        _respondent: &[u8; 32],
        _reason: DisputeReason,
    ) -> Result<bool, &'static str> {
        Ok(false) // Dismiss all challenges
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_adapter_dismisses_all() {
        let result = NoopCourtAdapter::adjudicate(
            H256::repeat_byte(0x01),
            &[1u8; 32],
            &[2u8; 32],
            DisputeReason::ReceiptMismatch,
        );
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn dispute_reason_encodes() {
        let encoded = codec::Encode::encode(&DisputeReason::ExecutionDivergence);
        assert!(!encoded.is_empty());
        let decoded: DisputeReason = codec::Decode::decode(&mut &encoded[..]).unwrap();
        assert_eq!(decoded, DisputeReason::ExecutionDivergence);
    }
}
