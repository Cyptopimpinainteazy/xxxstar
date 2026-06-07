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
