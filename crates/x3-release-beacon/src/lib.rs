//! Signed X3 block beacons used to order cross-domain settlement releases.
//!
//! A beacon is not accepted merely because a caller names a block. Authorized
//! validators sign a domain-separated statement binding their attestor id, the
//! finalized block height and the finalized block hash. A policy verifies the
//! configured threshold and counts each authorized key at most once.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use thiserror::Error;

#[cfg(feature = "service")]
pub mod exchange;
#[cfg(feature = "service")]
pub mod publisher;

/// Ed25519 public key length for an X3 release-order attestor.
pub const RELEASE_BEACON_PUBLIC_KEY_LEN: usize = 32;
/// Ed25519 signature length for an X3 release-order attestor.
pub const RELEASE_BEACON_SIGNATURE_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReleaseBeaconError {
    #[error("release beacon policy is empty, duplicated, malformed, or below its threshold")]
    InvalidPolicy,
    #[error("release beacon signature has length {got}; expected {expected}")]
    InvalidSignatureLength { got: usize, expected: usize },
    #[error("release beacon public key is malformed")]
    InvalidPublicKey,
    #[error(
        "release beacon {block_number}:{block_hash} has {valid} valid authorized attestations; policy requires {required}"
    )]
    ThresholdNotMet {
        block_number: u64,
        block_hash: String,
        valid: usize,
        required: u32,
    },
    #[error("release beacon attestor id must not be empty")]
    EmptyAttestorId,
    #[error("release beacon key is not authorized")]
    UnauthorizedAttestor,
    #[error("release beacon signature verification failed")]
    InvalidSignature,
    #[error("release beacon exchange error: {0}")]
    Exchange(String),
}

/// One authorized attestor's signature over the finalized X3 release beacon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconAttestation {
    pub attestor_id: String,
    pub public_key: [u8; RELEASE_BEACON_PUBLIC_KEY_LEN],
    pub signature: Vec<u8>,
}

/// Threshold evidence that an X3 block is the release ordering beacon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconEvidence {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub attestations: Vec<ReleaseBeaconAttestation>,
}

/// Policy-owned attestor keys and threshold for an X3 release beacon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBeaconPolicy {
    required_signatures: u32,
    authorized_attestors: Vec<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
}

impl ReleaseBeaconPolicy {
    pub fn new(
        required_signatures: u32,
        authorized_attestors: Vec<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
    ) -> Result<Self, ReleaseBeaconError> {
        let unique: BTreeSet<_> = authorized_attestors.iter().copied().collect();
        if required_signatures == 0
            || authorized_attestors.is_empty()
            || unique.len() != authorized_attestors.len()
            || authorized_attestors
                .iter()
                .any(|key| key.iter().all(|byte| *byte == 0))
            || unique.len() < required_signatures as usize
        {
            return Err(ReleaseBeaconError::InvalidPolicy);
        }
        Ok(Self {
            required_signatures,
            authorized_attestors,
        })
    }

    pub fn required_signatures(&self) -> u32 {
        self.required_signatures
    }

    pub fn authorized_attestors(&self) -> &[[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]] {
        &self.authorized_attestors
    }

    /// Verify the threshold and authenticate every counted attestation.
    pub fn verify(
        &self,
        evidence: &ReleaseBeaconEvidence,
    ) -> Result<VerifiedReleaseBeacon, ReleaseBeaconError> {
        let mut valid = BTreeSet::new();
        for attestation in &evidence.attestations {
            if !self.authorized_attestors.contains(&attestation.public_key) {
                continue;
            }
            if verify_attestation(
                &attestation.attestor_id,
                evidence.block_number,
                evidence.block_hash,
                &attestation.public_key,
                &attestation.signature,
            )
            .is_ok()
            {
                valid.insert(attestation.public_key);
            }
        }
        if valid.len() < self.required_signatures as usize {
            return Err(ReleaseBeaconError::ThresholdNotMet {
                block_number: evidence.block_number,
                block_hash: hex::encode(evidence.block_hash),
                valid: valid.len(),
                required: self.required_signatures,
            });
        }
        Ok(VerifiedReleaseBeacon {
            block_number: evidence.block_number,
            block_hash: evidence.block_hash,
        })
    }
}

/// A release ordering beacon that passed the configured threshold policy.
///
/// Fields are private and there is no public constructor, so release paths
/// cannot accept a raw caller-chosen X3 block hash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedReleaseBeacon {
    block_number: u64,
    block_hash: [u8; 32],
}

impl VerifiedReleaseBeacon {
    pub fn block_number(&self) -> u64 {
        self.block_number
    }

    pub fn block_hash(&self) -> [u8; 32] {
        self.block_hash
    }
}

/// Statement signed by an X3 release-order attestor.
pub fn release_beacon_statement_hash(
    attestor_id: &str,
    block_number: u64,
    block_hash: [u8; 32],
) -> [u8; 32] {
    fn component(hasher: &mut Sha256, bytes: &[u8]) {
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }

    let mut hasher = Sha256::new();
    component(&mut hasher, b"X3:CROSS_DOMAIN_RELEASE_BEACON:V1");
    component(&mut hasher, attestor_id.trim().as_bytes());
    hasher.update(block_number.to_be_bytes());
    hasher.update(block_hash);
    hasher.finalize().into()
}

/// Verify one Ed25519 attestation over an exact beacon statement.
pub fn verify_attestation(
    attestor_id: &str,
    block_number: u64,
    block_hash: [u8; 32],
    public_key: &[u8; RELEASE_BEACON_PUBLIC_KEY_LEN],
    signature: &[u8],
) -> Result<(), ReleaseBeaconError> {
    if attestor_id.trim().is_empty() {
        return Err(ReleaseBeaconError::EmptyAttestorId);
    }
    if public_key.iter().all(|byte| *byte == 0) {
        return Err(ReleaseBeaconError::InvalidPublicKey);
    }
    let key =
        VerifyingKey::from_bytes(public_key).map_err(|_| ReleaseBeaconError::InvalidPublicKey)?;
    if signature.len() != RELEASE_BEACON_SIGNATURE_LEN {
        return Err(ReleaseBeaconError::InvalidSignatureLength {
            got: signature.len(),
            expected: RELEASE_BEACON_SIGNATURE_LEN,
        });
    }
    let signature = Signature::from_slice(signature).map_err(|_| {
        ReleaseBeaconError::InvalidSignatureLength {
            got: RELEASE_BEACON_SIGNATURE_LEN,
            expected: RELEASE_BEACON_SIGNATURE_LEN,
        }
    })?;
    let statement = release_beacon_statement_hash(attestor_id.trim(), block_number, block_hash);
    key.verify_strict(&statement, &signature)
        .map_err(|_| ReleaseBeaconError::InvalidSignature)
}

/// Produce one attestation with a validator-held key.
pub fn sign_attestation(
    attestor_id: &str,
    signing_key: &SigningKey,
    block_number: u64,
    block_hash: [u8; 32],
) -> Result<ReleaseBeaconAttestation, ReleaseBeaconError> {
    if attestor_id.trim().is_empty() {
        return Err(ReleaseBeaconError::EmptyAttestorId);
    }
    let statement = release_beacon_statement_hash(attestor_id.trim(), block_number, block_hash);
    Ok(ReleaseBeaconAttestation {
        attestor_id: attestor_id.trim().to_string(),
        public_key: signing_key.verifying_key().to_bytes(),
        signature: signing_key.sign(&statement).to_bytes().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn evidence(block_hash: [u8; 32]) -> ReleaseBeaconEvidence {
        ReleaseBeaconEvidence {
            block_number: 77,
            block_hash,
            attestations: (1..=3)
                .map(|seed| {
                    sign_attestation(&format!("validator-{seed}"), &key(seed), 77, block_hash)
                        .unwrap()
                })
                .collect(),
        }
    }

    #[test]
    fn threshold_counts_distinct_authorized_keys_once() {
        let policy = ReleaseBeaconPolicy::new(
            2,
            (1..=3)
                .map(|seed| key(seed).verifying_key().to_bytes())
                .collect(),
        )
        .unwrap();
        let mut evidence = evidence([0x91; 32]);
        evidence.attestations[1] = evidence.attestations[0].clone();
        evidence.attestations[2] = evidence.attestations[0].clone();

        match policy.verify(&evidence) {
            Err(ReleaseBeaconError::ThresholdNotMet { valid: 1, .. }) => {}
            other => panic!("duplicate key must count once: {other:?}"),
        }
    }

    #[test]
    fn forged_and_unauthorized_attestations_do_not_count() {
        let policy = ReleaseBeaconPolicy::new(
            2,
            (1..=3)
                .map(|seed| key(seed).verifying_key().to_bytes())
                .collect(),
        )
        .unwrap();
        let mut evidence = evidence([0x91; 32]);
        evidence.attestations[0].signature = key(9).sign(b"wrong").to_bytes().to_vec();
        evidence.attestations[1].public_key = key(9).verifying_key().to_bytes();
        assert!(policy.verify(&evidence).is_err());
    }

    #[test]
    fn height_hash_and_attestor_id_are_bound() {
        let signing_key = key(1);
        let signed = sign_attestation("validator-1", &signing_key, 77, [0x91; 32]).unwrap();
        assert!(verify_attestation(
            "validator-1",
            77,
            [0x91; 32],
            &signed.public_key,
            &signed.signature,
        )
        .is_ok());
        assert!(verify_attestation(
            "validator-2",
            77,
            [0x91; 32],
            &signed.public_key,
            &signed.signature,
        )
        .is_err());
        assert!(verify_attestation(
            "validator-1",
            78,
            [0x91; 32],
            &signed.public_key,
            &signed.signature,
        )
        .is_err());
        assert!(verify_attestation(
            "validator-1",
            77,
            [0x92; 32],
            &signed.public_key,
            &signed.signature,
        )
        .is_err());
    }
}
