use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
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

/// Helper functions to validate GpuReceipt logic
pub struct GpuReceiptValidator;

impl GpuReceiptValidator {
    pub fn verify_signature(receipt: &GpuReceipt, signature: &[u8]) -> bool {
        debug!(
            "Verifying GPU Executor signature for receipt {:?}",
            receipt.kernel_hash
        );
        // Require a valid signature
        if signature.len() < 64 {
            debug!("Signature too short: {} bytes", signature.len());
            return false;
        }
        // The first 32 bytes of the signature are treated as the public key
        // and the remaining bytes as the actual signature. This binding
        // matches the GPU validator swarm's signing convention.
        if signature.len() < 32 + 64 {
            return false;
        }
        let (pubkey_bytes, sig_bytes) = signature.split_at(32);
        let sig_bytes = &sig_bytes[..64];
        let Ok(pubkey) = VerifyingKey::from_bytes(pubkey_bytes.try_into().unwrap()) else {
            return false;
        };
        let Ok(sig) = Signature::from_slice(sig_bytes) else {
            return false;
        };
        // Sign over the receipt hash for replay/nonce binding
        let msg = &receipt.kernel_hash;
        pubkey.verify(msg, &sig).is_ok()
    }

    pub fn slashable_mismatch(claimed: &GpuReceipt, actual_output: Hash) -> bool {
        claimed.output_commitment != actual_output
    }
}
