//! Runtime API for the X3 Keyring pallet.

use sp_api::decl_runtime_apis;
use sp_runtime::DispatchResult;

decl_runtime_apis! {
    /// API for querying keyring verification status.
    pub trait X3KeyringApi<AccountId, Balance, BlockNumber> {
        /// Check if a keyring has been verified.
        fn is_keyring_verified(keyring_id: [u8; 32]) -> bool;

        /// Get the verification result for a keyring.
        fn get_verification(keyring_id: [u8; 32]) -> Option<VerificationResult>;

        /// Check if an account is a registered and active attestor.
        fn is_attestor(account: AccountId) -> bool;

        /// Get the attestor record for an account.
        fn get_attestor(account: AccountId) -> Option<AttestorInfo<Balance, BlockNumber>>;

        /// Get the total number of registered attestors.
        fn total_attestors() -> u64;

        /// Get the total number of proofs submitted.
        fn total_proofs_submitted() -> u64;

        /// Get the total number of verified proofs.
        fn total_proofs_verified() -> u64;
    }
}

/// Verification result structure for runtime API responses.
#[derive(codec::Encode, codec::Decode, sp_runtime::RuntimeDebug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VerificationResult {
    /// Whether the keyring was verified.
    pub verified: bool,
    /// The verified keyring identifier.
    pub keyring_id: [u8; 32],
    /// Number of attestors who confirmed.
    pub confirmation_count: u32,
    /// Timestamp of verification (Unix millis).
    pub timestamp: u64,
}

/// Attestor information structure for runtime API responses.
#[derive(codec::Encode, codec::Decode, sp_runtime::RuntimeDebug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AttestorInfo<Balance, BlockNumber> {
    /// Attestor account identifier.
    pub account: [u8; 32],
    /// Stake amount.
    pub stake: Balance,
    /// Total proofs verified.
    pub proofs_verified: u64,
    /// Total proofs rejected.
    pub proofs_rejected: u64,
    /// Reputation score (0-100).
    pub reputation: u8,
    /// Whether the attestor is active.
    pub active: bool,
}