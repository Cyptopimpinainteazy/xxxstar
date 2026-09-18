//! Validator attestation set tracking with cryptographically verified quorum.
//!
//! Audit finding P0-5: the previous implementation summed `weight` for any
//! attestation carrying a non-empty `signature` byte string, so a one-byte
//! payload such as `vec![1]` reached quorum with no cryptographic check at all.
//!
//! Attestations are now verified with Ed25519 before they are admitted to the
//! set. A rejected attestation never contributes weight, so quorum cannot be
//! reached with forged, truncated, zero-key, or mismatched-statement material.

use std::collections::{HashMap, HashSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Length of an Ed25519 public key, in bytes.
pub const PUBLIC_KEY_LEN: usize = 32;
/// Length of an Ed25519 signature, in bytes.
pub const SIGNATURE_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValidatorId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    pub validator: ValidatorId,
    /// The statement being attested to.
    pub statement_hash: [u8; 32],
    /// Ed25519 public key of the signing validator.
    pub public_key: [u8; PUBLIC_KEY_LEN],
    /// Ed25519 signature over `statement_hash`.
    pub signature: Vec<u8>,
    /// Voting weight contributed once the attestation verifies.
    pub weight: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    EmptySignature,
    DuplicateValidator,
    InvalidSignatureLength { got: usize },
    InvalidPublicKey,
    StatementHashMismatch,
    SignatureVerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationSet {
    statement_hash: [u8; 32],
    attestations: HashMap<ValidatorId, Attestation>,
    total_weight: u64,
}

impl AttestationSet {
    pub fn new(statement_hash: [u8; 32]) -> Self {
        Self {
            statement_hash,
            attestations: HashMap::new(),
            total_weight: 0,
        }
    }

    /// Verify and admit an attestation.
    ///
    /// Rejected attestations contribute no weight and are not stored.
    pub fn add_attestation(&mut self, attestation: Attestation) -> Result<(), AttestationError> {
        if self.attestations.contains_key(&attestation.validator) {
            return Err(AttestationError::DuplicateValidator);
        }

        if attestation.signature.is_empty() {
            return Err(AttestationError::EmptySignature);
        }

        if attestation.statement_hash != self.statement_hash {
            return Err(AttestationError::StatementHashMismatch);
        }

        if attestation.public_key.iter().all(|byte| *byte == 0) {
            return Err(AttestationError::InvalidPublicKey);
        }

        let verifying_key = VerifyingKey::from_bytes(&attestation.public_key)
            .map_err(|_| AttestationError::InvalidPublicKey)?;

        if attestation.signature.len() != SIGNATURE_LEN {
            return Err(AttestationError::InvalidSignatureLength {
                got: attestation.signature.len(),
            });
        }

        let signature = Signature::from_slice(&attestation.signature).map_err(|_| {
            AttestationError::InvalidSignatureLength {
                got: attestation.signature.len(),
            }
        })?;

        verifying_key
            .verify(&attestation.statement_hash, &signature)
            .map_err(|_| AttestationError::SignatureVerificationFailed)?;

        self.total_weight = self.total_weight.saturating_add(attestation.weight);
        self.attestations
            .insert(attestation.validator.clone(), attestation);
        Ok(())
    }

    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }

    pub fn unique_validators(&self) -> usize {
        self.attestations.len()
    }

    pub fn has_quorum(&self, required_weight: u64) -> bool {
        self.total_weight >= required_weight
    }

    pub fn validators(&self) -> HashSet<ValidatorId> {
        self.attestations.keys().cloned().collect()
    }

    pub fn statement_hash(&self) -> [u8; 32] {
        self.statement_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_attestation(name: &str, statement: [u8; 32], weight: u64, seed: u8) -> Attestation {
        let key = signing_key(seed);
        Attestation {
            validator: ValidatorId(name.to_string()),
            statement_hash: statement,
            public_key: key.verifying_key().to_bytes(),
            signature: key.sign(&statement).to_vec(),
            weight,
        }
    }

    #[test]
    fn accepts_valid_signature_and_counts_weight() {
        let mut set = AttestationSet::new([7; 32]);
        set.add_attestation(signed_attestation("alice", [7; 32], 40, 1))
            .unwrap();
        assert_eq!(set.total_weight(), 40);
        assert_eq!(set.unique_validators(), 1);
    }

    #[test]
    fn rejects_forged_signature() {
        // Signed by a different key than the one presented.
        let mut forged = signed_attestation("alice", [7; 32], 40, 2);
        forged.public_key = signing_key(3).verifying_key().to_bytes();

        let mut set = AttestationSet::new([7; 32]);
        assert_eq!(
            set.add_attestation(forged),
            Err(AttestationError::SignatureVerificationFailed)
        );
        assert_eq!(set.total_weight(), 0);
        assert!(!set.has_quorum(1));
    }

    #[test]
    fn rejects_tampered_statement_hash() {
        let mut tampered = signed_attestation("alice", [7; 32], 40, 4);
        tampered.statement_hash = [9; 32];

        let mut set = AttestationSet::new([7; 32]);
        assert_eq!(
            set.add_attestation(tampered),
            Err(AttestationError::StatementHashMismatch)
        );
        assert_eq!(set.total_weight(), 0);
    }

    #[test]
    fn rejects_short_signature() {
        let mut short = signed_attestation("alice", [7; 32], 40, 5);
        short.signature.truncate(SIGNATURE_LEN - 1);

        let mut set = AttestationSet::new([7; 32]);
        assert_eq!(
            set.add_attestation(short),
            Err(AttestationError::InvalidSignatureLength {
                got: SIGNATURE_LEN - 1
            })
        );
        assert_eq!(set.total_weight(), 0);
    }

    #[test]
    fn rejects_empty_signature() {
        let mut empty = signed_attestation("alice", [7; 32], 40, 6);
        empty.signature.clear();

        let mut set = AttestationSet::new([7; 32]);
        assert_eq!(
            set.add_attestation(empty),
            Err(AttestationError::EmptySignature)
        );
        assert_eq!(set.total_weight(), 0);
    }

    #[test]
    fn rejects_zero_public_key() {
        let mut zeroed = signed_attestation("alice", [7; 32], 40, 7);
        zeroed.public_key = [0u8; PUBLIC_KEY_LEN];

        let mut set = AttestationSet::new([7; 32]);
        assert_eq!(
            set.add_attestation(zeroed),
            Err(AttestationError::InvalidPublicKey)
        );
        assert_eq!(set.total_weight(), 0);
    }

    #[test]
    fn rejects_duplicate_validator() {
        let mut set = AttestationSet::new([7; 32]);
        set.add_attestation(signed_attestation("alice", [7; 32], 30, 8))
            .unwrap();
        let second = set.add_attestation(signed_attestation("alice", [7; 32], 20, 9));
        assert_eq!(second, Err(AttestationError::DuplicateValidator));
        assert_eq!(set.total_weight(), 30);
    }

    #[test]
    fn quorum_requires_verified_weights() {
        let mut set = AttestationSet::new([7; 32]);
        set.add_attestation(signed_attestation("alice", [7; 32], 40, 10))
            .unwrap();
        set.add_attestation(signed_attestation("bob", [7; 32], 35, 11))
            .unwrap();

        assert_eq!(set.total_weight(), 75);
        assert!(set.has_quorum(67));
        assert!(!set.has_quorum(80));

        // A forged attestation cannot push the set over the threshold.
        let mut forged = signed_attestation("carol", [7; 32], 100, 12);
        forged.public_key = signing_key(13).verifying_key().to_bytes();
        assert_eq!(
            set.add_attestation(forged),
            Err(AttestationError::SignatureVerificationFailed)
        );
        assert_eq!(set.total_weight(), 75);
    }
}
