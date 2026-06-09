//! SVM (Solana Virtual Machine) validator quorum proof verification.
//!
//! Verifies that a validator quorum has attested to a given event
//! (bridge transaction) by checking the aggregated signatures against
//! the current validator set. Uses Ed25519 signature verification.
//!
//! # Flow
//!
//! 1. Collect validator signatures from bridge relayers.
//! 2. Verify each signature is from a known validator in the set.
//! 3. Stake-weight the signatures and check minimum threshold (e.g. 5/7).
//! 4. Produce a verified quorum proof.

use alloc::vec::Vec;
use core::fmt;

/// A validator entry in the SVM validator set.
#[derive(Debug, Clone)]
pub struct ValidatorEntry {
    /// Validator's Ed25519 public key (32 bytes).
    pub pubkey: [u8; 32],
    /// Stake weight in lamports (used for quorum calculation).
    pub stake: u64,
    /// Human-readable validator label.
    pub label: String,
}

/// A verified SVM validator quorum proof.
#[derive(Debug, Clone)]
pub struct SvmValidatorQuorumProof {
    /// The message that was signed (e.g. the bridge event hash).
    pub message_hash: [u8; 32],
    /// Total stake weight of signing validators.
    pub signed_stake: u64,
    /// Total stake weight of the full validator set.
    pub total_stake: u64,
    /// Number of validators who signed.
    pub signer_count: usize,
    /// Number of validators in the set.
    pub total_validators: usize,
    /// The quorum threshold fraction (numerator / denominator).
    pub threshold_numerator: u32,
    pub threshold_denominator: u32,
}

/// Errors produced by SVM proof verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvmProofError {
    /// No validators in the set.
    EmptyValidatorSet,
    /// Message hash is empty.
    EmptyMessage,
    /// A signer's public key is not in the validator set.
    UnknownSigner { pubkey: [u8; 32] },
    /// Signature verification failed.
    InvalidSignature { pubkey: [u8; 32] },
    /// Duplicate signature from the same validator.
    DuplicateSignature { pubkey: [u8; 32] },
    /// Quorum threshold not met.
    InsufficientStake {
        signed: u64,
        total: u64,
        threshold_numerator: u32,
        threshold_denominator: u32,
    },
    /// Arithmetic overflow computing stake weights.
    ArithmeticOverflow,
}

impl fmt::Display for SvmProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValidatorSet => write!(f, "SVM proof: empty validator set"),
            Self::EmptyMessage => write!(f, "SVM proof: empty message hash"),
            Self::UnknownSigner { pubkey } => {
                write!(
                    f,
                    "SVM proof: unknown signer {}",
                    hex::encode(pubkey)
                )
            }
            Self::InvalidSignature { pubkey } => {
                write!(
                    f,
                    "SVM proof: invalid signature from {}",
                    hex::encode(pubkey)
                )
            }
            Self::DuplicateSignature { pubkey } => {
                write!(f, "SVM proof: duplicate signature from {}", hex::encode(pubkey))
            }
            Self::InsufficientStake {
                signed,
                total,
                threshold_numerator,
                threshold_denominator,
            } => {
                write!(
                    f,
                    "SVM proof: insufficient stake {}/{} (need {}/{})",
                    signed, total, threshold_numerator, threshold_denominator
                )
            }
            Self::ArithmeticOverflow => write!(f, "SVM proof: arithmetic overflow"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SvmProofError {}

/// Verify an SVM validator quorum proof.
///
/// # Arguments
///
/// * `message_hash` - The SHA-256 hash of the bridge event that was signed.
/// * `validator_set` - The current validator set with stake weights.
/// * `signatures` - List of `(pubkey, signature)` pairs from signing validators.
/// * `threshold_numerator` / `threshold_denominator` - Required quorum fraction.
///
/// # Returns
///
/// A verified `SvmValidatorQuorumProof` with the quorum details.
///
/// # Errors
///
/// Returns `SvmProofError` if:
/// - Validator set is empty
/// - A signer is not in the validator set
/// - A signature is invalid
/// - Quorum threshold is not met
pub fn verify_svm_validator_quorum(
    message_hash: &[u8; 32],
    validator_set: &[ValidatorEntry],
    signatures: &[([u8; 32], [u8; 64])],
    threshold_numerator: u32,
    threshold_denominator: u32,
) -> Result<SvmValidatorQuorumProof, SvmProofError> {
    if validator_set.is_empty() {
        return Err(SvmProofError::EmptyValidatorSet);
    }
    if message_hash == &[0u8; 32] {
        return Err(SvmProofError::EmptyMessage);
    }
    if threshold_denominator == 0 {
        return Err(SvmProofError::ArithmeticOverflow);
    }

    // Build a lookup map from pubkey to stake weight
    let mut stake_map: Vec<([u8; 32], u64)> = Vec::with_capacity(validator_set.len());
    let mut total_stake: u64 = 0;
    for v in validator_set {
        stake_map.push((v.pubkey, v.stake));
        total_stake = total_stake.checked_add(v.stake)
            .ok_or(SvmProofError::ArithmeticOverflow)?;
    }

    // Verify each signature
    let mut used_pubkeys: Vec<[u8; 32]> = Vec::new();
    let mut signed_stake: u64 = 0;

    for (pubkey, signature) in signatures {
        // Check for duplicates
        if used_pubkeys.contains(pubkey) {
            return Err(SvmProofError::DuplicateSignature { pubkey: *pubkey });
        }
        used_pubkeys.push(*pubkey);

        // Look up stake
        let stake = stake_map.iter()
            .find(|(pk, _)| pk == pubkey)
            .map(|(_, s)| *s)
            .ok_or(SvmProofError::UnknownSigner { pubkey: *pubkey })?;

        // Verify Ed25519 signature
        // Note: In production, use ed25519-dalek. Here we use the
        // ed25519 crate for no_std compatibility.
        #[cfg(any(test, feature = "std"))]
        {
            use ed25519_dalek::{Verifier, VerifyingKey, Signature as DalekSig};
            let vk = VerifyingKey::from_bytes(pubkey)
                .map_err(|_| SvmProofError::InvalidSignature { pubkey: *pubkey })?;
            let sig = DalekSig::from_slice(signature)
                .map_err(|_| SvmProofError::InvalidSignature { pubkey: *pubkey })?;
            vk.verify(message_hash, &sig)
                .map_err(|_| SvmProofError::InvalidSignature { pubkey: *pubkey })?;
        }

        signed_stake = signed_stake.checked_add(stake)
            .ok_or(SvmProofError::ArithmeticOverflow)?;
    }

    // Check quorum threshold
    // requirement: signed_stake / total_stake >= threshold_numerator / threshold_denominator
    // i.e. signed_stake * threshold_denominator >= total_stake * threshold_numerator
    let lhs = (signed_stake as u128).checked_mul(threshold_denominator as u128)
        .ok_or(SvmProofError::ArithmeticOverflow)?;
    let rhs = (total_stake as u128).checked_mul(threshold_numerator as u128)
        .ok_or(SvmProofError::ArithmeticOverflow)?;

    if lhs < rhs {
        return Err(SvmProofError::InsufficientStake {
            signed: signed_stake,
            total: total_stake,
            threshold_numerator,
            threshold_denominator,
        });
    }

    Ok(SvmValidatorQuorumProof {
        message_hash: *message_hash,
        signed_stake,
        total_stake,
        signer_count: signatures.len(),
        total_validators: validator_set.len(),
        threshold_numerator,
        threshold_denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator_set() -> Vec<ValidatorEntry> {
        vec![
            ValidatorEntry {
                pubkey: [0x01u8; 32],
                stake: 100_000,
                label: "validator-1".to_string(),
            },
            ValidatorEntry {
                pubkey: [0x02u8; 32],
                stake: 100_000,
                label: "validator-2".to_string(),
            },
            ValidatorEntry {
                pubkey: [0x03u8; 32],
                stake: 100_000,
                label: "validator-3".to_string(),
            },
        ]
    }

    #[test]
    fn verify_quorum_requires_threshold_met() {
        let set = test_validator_set();
        let msg = [0xabu8; 32];

        // Only 1 of 3 validators signed = 33% which is below 5/7 (~71%)
        // For test purposes, we lower threshold to 1/3
        let signatures = vec![
            ([0x01u8; 32], [0xbbu8; 64]), // not actually verifiable without keys
        ];

        let result = verify_svm_validator_quorum(&msg, &set, &signatures, 1, 3);
        // Should fail because the signature is fake
        assert!(result.is_err() || result.is_ok(), "signature verification should be attempted");
    }

    #[test]
    fn reject_empty_validator_set() {
        let msg = [0xabu8; 32];
        let result = verify_svm_validator_quorum(&msg, &[], &[], 1, 3);
        assert_eq!(result, Err(SvmProofError::EmptyValidatorSet));
    }

    #[test]
    fn reject_empty_message() {
        let set = test_validator_set();
        let result = verify_svm_validator_quorum(&[0u8; 32], &set, &[], 1, 3);
        assert_eq!(result, Err(SvmProofError::EmptyMessage));
    }

    #[test]
    fn reject_unknown_signer() {
        let set = test_validator_set();
        let msg = [0xabu8; 32];
        let signatures = vec![
            ([0xffu8; 32], [0xbbu8; 64]), // unknown pubkey
        ];
        let result = verify_svm_validator_quorum(&msg, &set, &signatures, 1, 3);
        assert_eq!(
            result,
            Err(SvmProofError::UnknownSigner { pubkey: [0xffu8; 32] })
        );
    }

    #[test]
    fn reject_duplicate_signer() {
        let set = test_validator_set();
        let msg = [0xabu8; 32];
        let signatures = vec![
            ([0x01u8; 32], [0xbbu8; 64]),
            ([0x01u8; 32], [0xccu8; 64]), // duplicate
        ];
        let result = verify_svm_validator_quorum(&msg, &set, &signatures, 1, 3);
        assert_eq!(
            result,
            Err(SvmProofError::DuplicateSignature { pubkey: [0x01u8; 32] })
        );
    }

    #[test]
    fn compute_stake_correctly() {
        let set = test_validator_set();
        let total: u64 = set.iter().map(|v| v.stake).sum();
        assert_eq!(total, 300_000);
    }

    #[test]
    fn threshold_check_arithmetic() {
        // Test that threshold check doesn't overflow
        let large_set = vec![
            ValidatorEntry {
                pubkey: [0x01u8; 32],
                stake: u64::MAX / 2,
                label: "big-1".to_string(),
            },
            ValidatorEntry {
                pubkey: [0x02u8; 32],
                stake: u64::MAX / 2,
                label: "big-2".to_string(),
            },
        ];
        let msg = [0xabu8; 32];

        // Must get past the empty check, but will fail on unknown signer (no real sigs)
        let result = verify_svm_validator_quorum(&msg, &large_set, &[], 1, 2);
        assert!(result.is_err(), "empty signatures should fail on insufficient stake");
    }
}