//! Types for the Proof-Carrying Agent pallet.
//!
//! Defines proof types, agent action envelopes, and verification outcomes
//! for agents that submit proofs alongside their on-chain actions.

use alloc::vec::Vec;
use parity_scale_codec::DecodeWithMemTracking;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

/// The kind of proof an agent can carry with an action.
#[derive(
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub enum ProofKind {
    /// Zero-knowledge proof (e.g., Groth16, PLONK)
    ZkSnark,
    /// Formal verification proof (e.g., Coq, K framework)
    FormalVerification,
    /// Deterministic replay proof (re-execute and compare state)
    ReplayProof,
    /// Validator quorum attestation
    ValidatorAttestation,
    /// Fraud proof (submitted by challenger)
    FraudProof,
    /// Execution trace proof (full trace of VM execution)
    ExecutionTrace,
    /// Custom proof type (extensible)
    Custom(u8),
}

/// The status of a proof submission lifecycle.
#[derive(
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub enum ProofStatus {
    /// Proof submitted, pending verification
    Pending,
    /// Proof verified successfully
    Verified,
    /// Proof verification failed
    Failed,
    /// Proof expired (timeout reached)
    Expired,
    /// Proof challenged by another agent
    Challenged,
}

/// An action that an agent wants to execute, carrying a proof.
///
/// NOTE: This type is NOT stored directly in storage maps that require
/// `MaxEncodedLen`. It is used transiently during submission. The stored
/// version is `VerifiedAction`.
#[allow(dead_code)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, DecodeWithMemTracking)]
pub struct ProofCarryingAction<AccountId, BlockNumber> {
    /// The agent submitting the action
    pub agent: AccountId,
    /// The action payload (opaque — interpreted by the target pallet)
    pub action_payload: Vec<u8>,
    /// The proof payload (opaque — verified by the verification router)
    pub proof_payload: Vec<u8>,
    /// The kind of proof being submitted
    pub proof_kind: ProofKind,
    /// Target pallet index (which pallet the action is for)
    pub target_pallet: u8,
    /// Target call index (which extrinsic in the target pallet)
    pub target_call: u8,
    /// Deadline block — proof must be verified by this block
    pub deadline: BlockNumber,
    /// Nonce for replay protection
    pub nonce: u64,
}

/// A verified action record stored on-chain.
///
/// NOTE: `MaxEncodedLen` is NOT derived because `Vec<u8>` fields don't implement it.
/// The `#[pallet::without_storage_info]` attribute on the pallet struct removes
/// the `MaxEncodedLen` requirement for storage items.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, DecodeWithMemTracking)]
pub struct VerifiedAction<AccountId, BlockNumber> {
    /// Unique action ID (hash of the action)
    pub action_id: [u8; 32],
    /// The agent who submitted
    pub agent: AccountId,
    /// The action payload
    pub action_payload: Vec<u8>,
    /// The proof payload
    pub proof_payload: Vec<u8>,
    /// The proof kind
    pub proof_kind: ProofKind,
    /// Target pallet
    pub target_pallet: u8,
    /// Target call
    pub target_call: u8,
    /// Current status
    pub status: ProofStatus,
    /// Block when submitted
    pub submitted_at: BlockNumber,
    /// Block when verified (None if not yet verified)
    pub verified_at: Option<BlockNumber>,
    /// Verification outcome reason
    pub verification_reason: Vec<u8>,
    /// Nonce
    pub nonce: u64,
}

/// Summary of an agent's proof submission statistics.
#[derive(
    Default,
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub struct AgentProofStats {
    /// Total proofs submitted
    pub total_submitted: u64,
    /// Proofs verified successfully
    pub total_verified: u64,
    /// Proofs that failed verification
    pub total_failed: u64,
    /// Proofs that expired
    pub total_expired: u64,
    /// Proofs that were challenged
    pub total_challenged: u64,
}

/// A challenge to a verified proof.
///
/// NOTE: `MaxEncodedLen` is NOT derived because `Vec<u8>` and `u128` fields
/// don't implement it. The `#[pallet::without_storage_info]` attribute on the
/// pallet struct removes the `MaxEncodedLen` requirement for storage items.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, DecodeWithMemTracking)]
pub struct ProofChallenge<AccountId, BlockNumber> {
    /// The action being challenged
    pub action_id: [u8; 32],
    /// The challenger agent
    pub challenger: AccountId,
    /// The challenge reason
    pub reason: Vec<u8>,
    /// Block when challenged
    pub challenged_at: BlockNumber,
    /// Stake deposited by challenger
    pub challenge_stake: u128,
    /// Resolution (None = unresolved)
    pub resolution: Option<ChallengeResolution>,
}

/// Resolution of a proof challenge.
#[derive(
    Encode,
    Decode,
    Clone,
    PartialEq,
    Eq,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    DecodeWithMemTracking,
)]
pub enum ChallengeResolution {
    /// Challenge upheld — original proof was invalid
    Upheld,
    /// Challenge dismissed — original proof was valid
    Dismissed,
    /// Challenge expired without resolution
    Expired,
}

/// Configuration for proof verification deadlines.
///
/// NOTE: `MaxEncodedLen` is NOT derived because `min_challenge_stake` uses `u128`
/// which doesn't implement it. The `#[pallet::without_storage_info]` attribute
/// on the pallet struct removes the `MaxEncodedLen` requirement for storage items.
#[derive(
    Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, DecodeWithMemTracking,
)]
pub struct ProofConfig {
    /// Maximum blocks before a pending proof expires
    pub max_pending_blocks: u32,
    /// Maximum blocks for a challenge window
    pub challenge_window: u32,
    /// Minimum stake required to challenge a proof
    pub min_challenge_stake: u128,
    /// Maximum proofs per agent per epoch
    pub max_proofs_per_epoch: u32,
}
