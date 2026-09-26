use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GpuClass {
    DataCenter, // e.g., A100, H100
    Consumer,   // e.g., RTX 4090
    Embedded,   // e.g., Jetson
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProofType {
    RecomputeA, // Re-run on CPU/GPU
    RedundantB, // N independent GPUs
    SpotCheckC, // Partial sampling
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GpuReceipt {
    pub kernel_hash: Hash,
    pub input_commitment: Hash,
    pub output_commitment: Hash,
    pub gpu_cycles_used: u64,
    pub device_class: GpuClass,
    pub executor: Address,
    pub proof_type: ProofType,
}

/// Domain tag of the canonical GPU receipt attestation message.
///
/// The validator that executes a bundle signs this message; `ProofAggregator::verify_attestations`
/// and every other verifier must rebuild the *same* bytes. It is a constant here so the signing and
/// verifying sides cannot drift, and it names the version so a future change can be a new tag
/// rather than a silent reinterpretation of old signatures.
pub const ATTESTATION_DOMAIN: &[u8] = b"x3-validator-attestation-v1";

/// Helper functions to validate `GpuReceipt` logic.
///
/// There is exactly one receipt attestation convention in this crate: 65-byte ed25519
/// `r || s || recovery` (see [`crate::crypto::SIGNATURE_LENGTH`]) over
/// [`GpuReceiptValidator::attestation_message`], checked against the *expected* validator key from
/// [`crate::proof_aggregator::ProofAggregator::register_validator_pubkey`].
///
/// The type used to carry a second convention — a 96-byte `pubkey || signature` blob verified over
/// `kernel_hash` alone — which had two defects: the "signature" carried its own public key, so any
/// caller could mint a passing signature for a key it had chosen itself, and only the kernel hash
/// was bound, so `output_commitment` (the field a validator is asked to be honest about) could be
/// rewritten under a valid signature. Nothing called it. It is gone rather than documented.
pub struct GpuReceiptValidator;

impl GpuReceiptValidator {
    /// The canonical bytes a validator signs to attest `receipt` inside a bundle.
    ///
    /// Deterministic and length-fixed: a domain tag, every commitment in the receipt, the bundle
    /// id, the finalized block anchor and the legs hash. Binding `finalized_block` means a replay
    /// that moves the header's block number invalidates the signature.
    pub fn attestation_message(
        receipt: &GpuReceipt,
        bundle_id: Hash,
        finalized_block: u64,
        legs_hash: Hash,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(ATTESTATION_DOMAIN);
        hasher.update(receipt.kernel_hash);
        hasher.update(receipt.input_commitment);
        hasher.update(receipt.output_commitment);
        hasher.update(receipt.executor);
        hasher.update(receipt.gpu_cycles_used.to_le_bytes());
        hasher.update(bundle_id);
        hasher.update(finalized_block.to_le_bytes());
        hasher.update(legs_hash);
        hasher.finalize().into()
    }

    /// Parse a signature in the crate's one convention: 65 bytes of `r || s || recovery`.
    ///
    /// `Err` names the length it saw, because "some bytes" is how a placeholder signature passes a
    /// structural check.
    pub fn parse_signature(
        signature: &[u8],
    ) -> Result<crate::crypto::SignatureOutput, crate::error::SwarmError> {
        if signature.len() != crate::crypto::SIGNATURE_LENGTH {
            return Err(crate::error::SwarmError::VerificationFailed(format!(
                "Invalid signature length: expected {}, got {}",
                crate::crypto::SIGNATURE_LENGTH,
                signature.len()
            )));
        }
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&signature[..32]);
        s.copy_from_slice(&signature[32..64]);
        Ok(crate::crypto::SignatureOutput::new(r, s, signature[64]))
    }

    /// Verify that `expected_pubkey` attested `receipt` inside this bundle.
    ///
    /// `expected_pubkey` is the key the caller resolved out of its own registry — never one read
    /// out of `signature`. Refuses a signature of the wrong length or of the wrong convention (the
    /// retired 96-byte `pubkey || signature` blob), a degenerate key, and any message that does not
    /// match — including one whose `output_commitment` has been rewritten under an
    /// otherwise-valid signature. The error names which check failed.
    pub fn verify_attestation_signature(
        receipt: &GpuReceipt,
        bundle_id: Hash,
        finalized_block: u64,
        legs_hash: Hash,
        signature: &[u8],
        expected_pubkey: &[u8; 33],
    ) -> Result<(), crate::error::SwarmError> {
        let signature = Self::parse_signature(signature)?;
        let msg = Self::attestation_message(receipt, bundle_id, finalized_block, legs_hash);
        if !signature.verify(&msg, expected_pubkey) {
            debug!("Rejecting GPU receipt attestation");
            return Err(crate::error::SwarmError::VerificationFailed(format!(
                "GPU receipt attestation does not verify for kernel hash 0x{}",
                hex::encode(receipt.kernel_hash)
            )));
        }
        Ok(())
    }

    pub fn slashable_mismatch(claimed: &GpuReceipt, actual_output: Hash) -> bool {
        claimed.output_commitment != actual_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::SigningKey;

    fn receipt() -> GpuReceipt {
        GpuReceipt {
            kernel_hash: [1u8; 32],
            input_commitment: [2u8; 32],
            output_commitment: [3u8; 32],
            gpu_cycles_used: 4,
            device_class: GpuClass::DataCenter,
            executor: [5u8; 32],
            proof_type: ProofType::RecomputeA,
        }
    }

    /// The validator's own key: what the verifier resolves out of its registry.
    fn key() -> SigningKey {
        SigningKey::from_secret_bytes([9u8; 32]).expect("a non-degenerate secret")
    }

    fn padded(pubkey: [u8; 32]) -> [u8; 33] {
        let mut out = [0u8; 33];
        out[..32].copy_from_slice(&pubkey);
        out
    }

    /// Sign the canonical message the way `Validator` does when it attests a bundle.
    fn attest(
        key: &SigningKey,
        receipt: &GpuReceipt,
        bundle_id: Hash,
        finalized_block: u64,
        legs_hash: Hash,
    ) -> Vec<u8> {
        let msg = GpuReceiptValidator::attestation_message(
            receipt,
            bundle_id,
            finalized_block,
            legs_hash,
        );
        key.sign(&msg).to_bytes()
    }

    #[test]
    fn a_valid_attestation_verifies() {
        let key = key();
        let (bundle_id, block, legs_hash) = ([7u8; 32], 11u64, [8u8; 32]);
        let receipt = receipt();
        let signature = attest(&key, &receipt, bundle_id, block, legs_hash);

        assert!(GpuReceiptValidator::verify_attestation_signature(
            &receipt,
            bundle_id,
            block,
            legs_hash,
            &signature,
            &padded(key.public_key_bytes()),
        )
        .is_ok());
    }

    /// The defect the retired `verify_signature` carried: it verified over `kernel_hash` alone, so
    /// a validator could attest one output and the proof could carry another.
    #[test]
    fn rewriting_the_output_commitment_breaks_the_signature() {
        let key = key();
        let (bundle_id, block, legs_hash) = ([7u8; 32], 11u64, [8u8; 32]);
        let receipt = receipt();
        let signature = attest(&key, &receipt, bundle_id, block, legs_hash);
        let expected = padded(key.public_key_bytes());

        assert!(GpuReceiptValidator::verify_attestation_signature(
            &receipt, bundle_id, block, legs_hash, &signature, &expected,
        )
        .is_ok());

        let mut forged = receipt.clone();
        forged.output_commitment = [0xAAu8; 32];
        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &forged, bundle_id, block, legs_hash, &signature, &expected,
            )
            .is_err(),
            "an edited output commitment must not verify under the original signature"
        );
    }

    /// And a signature is only good for the bundle and block it was made in.
    #[test]
    fn a_signature_does_not_transfer_to_another_block_or_bundle() {
        let key = key();
        let (bundle_id, block, legs_hash) = ([7u8; 32], 11u64, [8u8; 32]);
        let receipt = receipt();
        let signature = attest(&key, &receipt, bundle_id, block, legs_hash);
        let expected = padded(key.public_key_bytes());

        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &receipt,
                bundle_id,
                block + 1,
                legs_hash,
                &signature,
                &expected,
            )
            .is_err(),
            "a signature does not carry to another finalized block"
        );
        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &receipt,
                [0xFFu8; 32],
                block,
                legs_hash,
                &signature,
                &expected,
            )
            .is_err(),
            "a signature does not carry to another bundle"
        );
    }

    /// A second, weaker convention must not be able to satisfy the check.
    #[test]
    fn the_retired_ninety_six_byte_blob_and_a_hash_only_signature_are_refused() {
        let key = key();
        let (bundle_id, block, legs_hash) = ([7u8; 32], 11u64, [8u8; 32]);
        let receipt = receipt();
        let expected = padded(key.public_key_bytes());

        // `pubkey || signature` — the shape the deleted verifier accepted. Its own key rides along,
        // so this is precisely what a forger would hand a verifier that trusted the blob.
        let inner = attest(&key, &receipt, bundle_id, block, legs_hash);
        let mut blob = Vec::new();
        blob.extend_from_slice(&key.public_key_bytes());
        // The retired verifier read `sig[..64]` as an ed25519 signature, so this is byte-for-byte
        // the input shape it would have accepted: 32 bytes of self-asserted key plus 64 bytes.
        blob.extend_from_slice(&inner[..64]);
        assert_eq!(blob.len(), 96);
        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &receipt, bundle_id, block, legs_hash, &blob, &expected,
            )
            .is_err(),
            "a 96-byte pubkey||signature blob is not this crate's convention"
        );

        // A genuine signature over the kernel hash alone — the message the deleted verifier used.
        let hash_only = key.sign(&receipt.kernel_hash).to_bytes();
        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &receipt, bundle_id, block, legs_hash, &hash_only, &expected,
            )
            .is_err(),
            "attesting the kernel hash must not stand in for attesting the receipt"
        );
    }

    /// The key is an input, never something read out of the signature.
    #[test]
    fn another_key_does_not_verify_the_attestation() {
        let signer = key();
        let other = SigningKey::from_secret_bytes([0x11u8; 32]).expect("a non-degenerate secret");
        let (bundle_id, block, legs_hash) = ([7u8; 32], 11u64, [8u8; 32]);
        let receipt = receipt();
        let signature = attest(&signer, &receipt, bundle_id, block, legs_hash);

        assert!(
            GpuReceiptValidator::verify_attestation_signature(
                &receipt,
                bundle_id,
                block,
                legs_hash,
                &signature,
                &padded(other.public_key_bytes()),
            )
            .is_err(),
            "only the registered key may attest"
        );
    }
}
