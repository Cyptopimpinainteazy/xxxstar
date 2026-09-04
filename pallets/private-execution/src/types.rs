//! Types for the Private Execution pallet.
//!
//! Proposal: PRIV-ENCLAVE-003

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

/// Maximum length for GPU model name.
pub const MAX_GPU_MODEL_LEN: u32 = 128;
/// Maximum length for attestation report blob.
pub const MAX_ATTESTATION_LEN: u32 = 4096;
/// Maximum length for encrypted payload.
pub const MAX_ENCRYPTED_PAYLOAD_LEN: u32 = 1_048_576; // 1 MB
/// Maximum length for encrypted state diff.
pub const MAX_STATE_DIFF_LEN: u32 = 524_288; // 512 KB
/// Maximum length for ZK proof.
pub const MAX_ZK_PROOF_LEN: u32 = 65_536; // 64 KB

/// Enclave attestation status.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum EnclaveStatus {
    /// Attestation verified, accepting private TXs.
    Verified,
    /// Attestation needs refresh.
    Expired,
    /// Failed attestation or revoked.
    Revoked,
}

/// Private transaction status.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum PrivateTxStatus {
    /// In encrypted mempool, waiting for execution.
    Pending,
    /// Being executed inside enclave.
    Executing,
    /// State diff committed on chain.
    Committed,
    /// ZK proof verified (if applicable).
    Verified,
    /// Execution failed inside enclave.
    Failed,
}

/// Attestation record for a confidential validator.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
#[scale_info(skip_type_params(T))]
pub struct EnclaveAttestation<T: frame_system::Config> {
    /// Validator account.
    pub validator: T::AccountId,
    /// GPU model name.
    pub gpu_model: BoundedVec<u8, ConstU32<MAX_GPU_MODEL_LEN>>,
    /// Raw attestation report from NVIDIA CC / AMD SEV-SNP.
    pub attestation_report: BoundedVec<u8, ConstU32<MAX_ATTESTATION_LEN>>,
    /// Enclave's ephemeral encryption public key (X25519).
    pub enclave_public_key: [u8; 32],
    /// Block when attestation was last refreshed.
    pub last_refreshed: BlockNumberFor<T>,
    /// Current status.
    pub status: EnclaveStatus,
}

/// Record of a private transaction.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
#[scale_info(skip_type_params(T))]
pub struct PrivateTxRecord<T: frame_system::Config> {
    /// Transaction hash.
    pub tx_hash: sp_core::H256,
    /// Sender account (can be pseudonymous).
    pub sender: T::AccountId,
    /// Encrypted transaction payload (AES-256-GCM).
    pub encrypted_payload: BoundedVec<u8, ConstU32<MAX_ENCRYPTED_PAYLOAD_LEN>>,
    /// Fee commitment (Pedersen commitment to the fee amount).
    pub fee_commitment: sp_core::H256,
    /// Total fee paid (base + premium).
    pub fee_paid: u128,
    /// Current status.
    pub status: PrivateTxStatus,
    /// Block when submitted.
    pub submitted_at: BlockNumberFor<T>,
    /// Confidential validator that executed this TX.
    pub executed_by: Option<T::AccountId>,
}

/// An encrypted state diff committed on-chain.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct EncryptedDiff {
    /// Transaction hash this diff belongs to.
    pub tx_hash: sp_core::H256,
    /// Encrypted state changes (encrypted to chain key).
    pub encrypted_state_changes: BoundedVec<u8, ConstU32<MAX_STATE_DIFF_LEN>>,
    /// Pedersen commitment to the plaintext diff.
    pub commitment: sp_core::H256,
    /// Optional ZK validity proof.
    pub zk_proof: Option<BoundedVec<u8, ConstU32<MAX_ZK_PROOF_LEN>>>,
    /// Signature from enclave attestation key (Ed25519).
    pub enclave_signature: [u8; 64],
    /// Block when committed.
    pub committed_at: u32,
}

// NOTE: EncryptedDiff uses concrete types (u32 for block number) since it's stored
// in a BoundedVec and needs to be T-independent. In production, parameterize properly.
