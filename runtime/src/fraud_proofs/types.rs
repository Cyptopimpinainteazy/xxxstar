//! Shared types for fraud-proof v0.
//!
//! Spec: `openspec/committee-reexec-fraudproofs-v0/witness.md`

#![allow(dead_code)]

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

// ── Proof-type tag ────────────────────────────────────────────────────────────

pub type ProofTypeTag = u8;

/// Scheduler-commitment mismatch proof (v0).
pub const PROOF_TYPE_SCHED_MISMATCH_V1: ProofTypeTag = 0x01;

// ── Block reference ───────────────────────────────────────────────────────────

#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
pub struct HeaderRef {
    pub block_number: u32,
    pub block_hash: H256,
}

// ── Fraud proof wire type ─────────────────────────────────────────────────────

/// The on-chain fraud proof submitted by a reporter.
///
/// `MAX_WITNESS_BYTES` is a const-generic bound matching the runtime constant.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct FraudProofV1<AccountId> {
    /// Must be `PROOF_TYPE_SCHED_MISMATCH_V1` in v0.
    pub proof_type: ProofTypeTag,
    /// Which block is disputed.
    pub header_ref: HeaderRef,
    /// `H(SCALE(tx_ids))` — reproduced by verifier.
    pub tx_set_commitment: H256,
    /// The scheduler_commitment that appears in the disputed block.
    pub claimed_scheduler_commitment: H256,
    /// Minimal bytes needed to deterministically recompute the commitment.
    /// Must be ≤ `MAX_WITNESS_BYTES`.
    pub reexec_witness: Vec<u8>,
    /// Expected (recomputed) hash the reporter claims.
    pub expected_hash: H256,
    /// The commitment actually observed in the block.
    pub observed_hash: H256,
    /// The reporter (must match `ensure_signed` origin).
    pub reporter: AccountId,
    /// Anti-replay nonce (domain separator; checked as part of proof_id).
    pub nonce: u64,
}

// ── Metadata the verifier needs about the disputed block ──────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
#[scale_info(skip_type_params(AccountId))]
pub struct DisputedBlockMeta<AccountId> {
    pub block_hash: H256,
    pub block_number: u32,
    pub rules_version: u32,
    pub scheduler_commitment: H256,
    pub proposer: AccountId,
}

// ── Event body ────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct FraudProofAcceptedRecord<AccountId, Balance> {
    pub proof_id: H256,
    pub block_hash: H256,
    pub proposer: AccountId,
    pub reporter: AccountId,
    pub slash_amount: Balance,
    pub reward: Balance,
}

// ── Trait abstractions for dependency injection ───────────────────────────────


/// Trait for querying the scheduler commitment for a given block.
///
/// Implemented by the sequencer pallet or any other pallet that stores
/// per-block scheduler commitments.
pub trait SchedulerCommitmentQuery {
    /// Return the scheduler commitment for the given block number, or `None`
    /// if the block is unknown or the commitment is not yet finalized.
    fn get_scheduler_commitment(block_number: u32) -> Option<H256>;
}

/// Trait for querying the block proposer (author) for a given block.
///
/// Implemented by the Aura, Babe, or any other consensus pallet.
pub trait ProposerQuery<AccountId> {
    /// Return the proposer account for the given block number, or `None`
    /// if the block is unknown.
    fn get_proposer(block_number: u32) -> Option<AccountId>;
}

// ── Default no-op implementations (for testing / when not wired) ──────────────

/// A no-op `SchedulerCommitmentQuery` that always returns `None`.
/// Useful as a default type parameter or in test configurations.
pub struct NoSchedulerCommitment;

impl SchedulerCommitmentQuery for NoSchedulerCommitment {
    fn get_scheduler_commitment(_block_number: u32) -> Option<H256> {
        None
    }
}

/// A no-op `ProposerQuery` that always returns `None`.
pub struct NoProposer;

impl<AccountId> ProposerQuery<AccountId> for NoProposer {
    fn get_proposer(_block_number: u32) -> Option<AccountId> {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::H256;

    /// A mock `SchedulerCommitmentQuery` that returns a fixed commitment.
    struct MockSchedulerCommitment;

    impl SchedulerCommitmentQuery for MockSchedulerCommitment {
        fn get_scheduler_commitment(block_number: u32) -> Option<H256> {
            if block_number == 0 || block_number > 100 {
                return None;
            }
            let mut h = [0u8; 32];
            h[..4].copy_from_slice(&block_number.to_le_bytes());
            Some(H256(h))
        }
    }

    /// A mock `ProposerQuery` that returns a fixed proposer.
    struct MockProposer;

    impl ProposerQuery<u64> for MockProposer {
        fn get_proposer(block_number: u32) -> Option<u64> {
            if block_number == 0 || block_number > 100 {
                return None;
            }
            Some(42u64)
        }
    }

    /// LOAD-META-001: SchedulerCommitmentQuery returns None for unknown blocks
    #[test]
    fn scheduler_commitment_none_for_unknown_block() {
        assert_eq!(
            NoSchedulerCommitment::get_scheduler_commitment(999),
            None
        );
    }

    /// LOAD-META-002: ProposerQuery returns None for unknown blocks
    #[test]
    fn proposer_none_for_unknown_block() {
        assert_eq!(NoProposer::get_proposer(999), None::<u64>);
    }

    /// LOAD-META-003: MockSchedulerCommitment returns deterministic values
    #[test]
    fn mock_scheduler_commitment_deterministic() {
        let c1 = MockSchedulerCommitment::get_scheduler_commitment(5);
        let c2 = MockSchedulerCommitment::get_scheduler_commitment(5);
        assert_eq!(c1, c2);
        assert!(c1.is_some());
    }

    /// LOAD-META-004: MockProposer returns deterministic values
    #[test]
    fn mock_proposer_deterministic() {
        let p1 = MockProposer::get_proposer(10);
        let p2 = MockProposer::get_proposer(10);
        assert_eq!(p1, p2);
        assert_eq!(p1, Some(42u64));
    }

}
