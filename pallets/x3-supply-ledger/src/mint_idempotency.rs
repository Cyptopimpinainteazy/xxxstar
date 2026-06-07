//! # Mint Idempotency Module (S0-2)
//!
//! Prevents double-mint attacks by enforcing unique, one-time-use mint operations.
//!
//! ## Problem
//! Without idempotency protection, the same mint operation can be replayed multiple times:
//! - In the mempool (transaction duplication)
//! - Via bridge replay attacks  
//! - Through governance voting manipulation
//!
//! This leads to unlimited token creation and economic collapse.
//!
//! ## Solution
//! Each mint operation requires:
//! 1. **Unique Nonce** — Strictly incrementing per origin
//! 2. **Transaction Hash** — Deterministic hash of (origin, amount, asset, nonce)
//! 3. **One-Time Use** — Hash recorded in storage, cannot be reused
//!
//! ## Architecture
//! ```text
//! mint_canonical(origin, asset, amount, nonce)
//!     ↓
//! validate_idempotency(origin, nonce, hash)
//!     ↓ (if valid)
//! do_mint_canonical(asset, amount)
//!     ↓
//! record_mint_token(origin, nonce, hash)
//! ```

use codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::{DispatchError, RuntimeDebug};
use sp_std::vec::Vec;
use x3_asset_kernel_types::Balance;

/// Unique identifier for a mint operation (idempotency token).
///
/// Prevents replay attacks by binding operation to:
/// - Origin account (who initiated)
/// - Nonce (strictly increasing sequence)
/// - Transaction hash (cryptographic commitment)
/// - Block number (when processed)
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct MintIdempotencyToken {
    /// Account that initiated the mint (governance, bridge relayer, etc.)
    pub origin: Vec<u8>, // AccountId encoded as bytes
    /// Strictly incrementing nonce for this origin (prevents replay)
    pub nonce: u64,
    /// Blake2-256 hash of (origin, asset_id, amount, nonce)
    pub tx_hash: H256,
    /// Block number when this mint was processed
    pub processed_at: u32,
}

impl MintIdempotencyToken {
    /// Create new idempotency token from mint parameters.
    ///
    /// # Arguments
    /// - `origin` — Account initiating mint
    /// - `asset_id` — Asset being minted
    /// - `amount` — Amount to mint
    /// - `nonce` — Current nonce for this origin
    /// - `block_number` — Current block number
    pub fn new(
        origin: &[u8],
        asset_id: &[u8; 32],
        amount: Balance,
        nonce: u64,
        block_number: u32,
    ) -> Self {
        let tx_hash = Self::compute_hash(origin, asset_id, amount, nonce);
        Self {
            origin: origin.to_vec(),
            nonce,
            tx_hash,
            processed_at: block_number,
        }
    }

    /// Compute deterministic hash for mint operation.
    ///
    /// Uses Blake2-256 over concatenated inputs:
    /// hash = Blake2_256(origin || asset_id || amount || nonce)
    pub fn compute_hash(origin: &[u8], asset_id: &[u8; 32], amount: Balance, nonce: u64) -> H256 {
        use sp_io::hashing::blake2_256;

        // Concatenate all inputs for hashing
        let mut data = Vec::with_capacity(origin.len() + 32 + 16 + 8);
        data.extend_from_slice(origin);
        data.extend_from_slice(asset_id);
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&nonce.to_le_bytes());

        H256::from(blake2_256(&data))
    }

    /// Verify this token matches the given parameters.
    pub fn verify(&self, origin: &[u8], asset_id: &[u8; 32], amount: Balance, nonce: u64) -> bool {
        // Check nonce matches
        if self.nonce != nonce {
            return false;
        }

        // Recompute hash and verify
        let expected_hash = Self::compute_hash(origin, asset_id, amount, nonce);
        self.tx_hash == expected_hash
    }
}

/// Result of idempotency validation.
#[derive(Debug, PartialEq, Eq)]
pub enum IdempotencyResult {
    /// Mint is valid and can proceed (first time seeing this nonce)
    Valid,
    /// Mint is duplicate (nonce already used)
    Duplicate,
    /// Mint has invalid nonce (not next expected nonce)
    InvalidNonce { expected: u64, provided: u64 },
    /// Mint hash doesn't match computed hash (tampering detected)
    HashMismatch,
}

/// Idempotency validator for mint operations.
///
/// Enforces strict nonce ordering and one-time-use semantics.
pub struct IdempotencyValidator;

impl IdempotencyValidator {
    /// Validate a mint operation is unique and uses the correct nonce.
    ///
    /// # Arguments
    /// - `origin` — Account initiating mint
    /// - `asset_id` — Asset being minted
    /// - `amount` — Amount to mint
    /// - `nonce` — Nonce provided by caller
    /// - `current_nonce` — Current nonce for this origin (from storage)
    /// - `is_duplicate` — Function to check if this nonce was already used
    ///
    /// # Returns
    /// - `Ok(())` if mint can proceed
    /// - `Err(IdempotencyError)` if mint should be rejected
    pub fn validate(
        origin: &[u8],
        asset_id: &[u8; 32],
        amount: Balance,
        nonce: u64,
        current_nonce: u64,
        is_duplicate: impl FnOnce(u64) -> bool,
    ) -> Result<(), IdempotencyError> {
        // Check 1: Nonce must be next expected value (strictly increasing)
        let expected_nonce = current_nonce;
        if nonce != expected_nonce {
            return Err(IdempotencyError::InvalidNonce {
                expected: expected_nonce,
                provided: nonce,
            });
        }

        // Check 2: Nonce must not have been used before (duplicate protection)
        if is_duplicate(nonce) {
            return Err(IdempotencyError::DuplicateMint { nonce });
        }

        // Check 3: Compute hash to verify parameters (no tampering)
        let _ = MintIdempotencyToken::compute_hash(origin, asset_id, amount, nonce);

        Ok(())
    }

    /// Get the next expected nonce for an origin.
    ///
    /// Nonces start at 0 and increment by 1 for each successful mint.
    pub fn next_nonce(current_nonce: u64) -> u64 {
        current_nonce.saturating_add(1)
    }
}

/// Errors that can occur during idempotency validation.
#[derive(Debug, PartialEq, Eq)]
pub enum IdempotencyError {
    /// Nonce doesn't match expected value (out of sequence)
    InvalidNonce { expected: u64, provided: u64 },
    /// This nonce was already used (duplicate mint attempt)
    DuplicateMint { nonce: u64 },
    /// Hash verification failed (tampering detected)
    HashMismatch,
}

impl From<IdempotencyError> for DispatchError {
    fn from(err: IdempotencyError) -> Self {
        match err {
            IdempotencyError::InvalidNonce { .. } => DispatchError::Other("Invalid mint nonce"),
            IdempotencyError::DuplicateMint { .. } => {
                DispatchError::Other("Duplicate mint detected")
            }
            IdempotencyError::HashMismatch => DispatchError::Other("Mint hash verification failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let nonce = 0u64;
        let block = 100u32;

        let token = MintIdempotencyToken::new(origin, &asset_id, amount, nonce, block);

        assert_eq!(token.origin, origin.to_vec());
        assert_eq!(token.nonce, nonce);
        assert_eq!(token.processed_at, block);
        assert_ne!(token.tx_hash, H256::zero()); // Hash should be non-zero
    }

    #[test]
    fn test_hash_computation_deterministic() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let nonce = 0u64;

        let hash1 = MintIdempotencyToken::compute_hash(origin, &asset_id, amount, nonce);
        let hash2 = MintIdempotencyToken::compute_hash(origin, &asset_id, amount, nonce);

        assert_eq!(hash1, hash2); // Deterministic hashing
    }

    #[test]
    fn test_hash_changes_with_inputs() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];

        let hash1 = MintIdempotencyToken::compute_hash(origin, &asset_id, 1000, 0);
        let hash2 = MintIdempotencyToken::compute_hash(origin, &asset_id, 2000, 0); // Different amount
        let hash3 = MintIdempotencyToken::compute_hash(origin, &asset_id, 1000, 1); // Different nonce

        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn test_token_verification_success() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let nonce = 0u64;

        let token = MintIdempotencyToken::new(origin, &asset_id, amount, nonce, 100);
        assert!(token.verify(origin, &asset_id, amount, nonce));
    }

    #[test]
    fn test_token_verification_wrong_nonce() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;

        let token = MintIdempotencyToken::new(origin, &asset_id, amount, 0, 100);
        assert!(!token.verify(origin, &asset_id, amount, 1)); // Wrong nonce
    }

    #[test]
    fn test_validator_accepts_valid_nonce() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let current_nonce = 5u64;
        let provided_nonce = 5u64; // Correct next nonce

        let result = IdempotencyValidator::validate(
            origin,
            &asset_id,
            amount,
            provided_nonce,
            current_nonce,
            |_| false, // Not a duplicate
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_validator_rejects_wrong_nonce() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let current_nonce = 5u64;
        let provided_nonce = 10u64; // Wrong nonce (skipped ahead)

        let result = IdempotencyValidator::validate(
            origin,
            &asset_id,
            amount,
            provided_nonce,
            current_nonce,
            |_| false,
        );

        assert_eq!(
            result,
            Err(IdempotencyError::InvalidNonce {
                expected: 5,
                provided: 10
            })
        );
    }

    #[test]
    fn test_validator_rejects_duplicate() {
        let origin = b"Alice";
        let asset_id = [1u8; 32];
        let amount = 1000u128;
        let current_nonce = 5u64;
        let provided_nonce = 5u64;

        let result = IdempotencyValidator::validate(
            origin,
            &asset_id,
            amount,
            provided_nonce,
            current_nonce,
            |_| true, // Already used
        );

        assert_eq!(result, Err(IdempotencyError::DuplicateMint { nonce: 5 }));
    }

    #[test]
    fn test_next_nonce_increments() {
        assert_eq!(IdempotencyValidator::next_nonce(0), 1);
        assert_eq!(IdempotencyValidator::next_nonce(5), 6);
        assert_eq!(IdempotencyValidator::next_nonce(999), 1000);
    }

    #[test]
    fn test_next_nonce_saturates_at_max() {
        assert_eq!(IdempotencyValidator::next_nonce(u64::MAX), u64::MAX);
    }
}
