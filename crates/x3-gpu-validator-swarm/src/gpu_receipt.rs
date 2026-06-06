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
    pub fn verify_signature(receipt: &GpuReceipt, _signature: &[u8]) -> bool {
        // GPU executor signature validation stub
        debug!(
            "Verifying GPU Executor signature for receipt {:?}",
            receipt.kernel_hash
        );
        true
    }

    pub fn slashable_mismatch(claimed: &GpuReceipt, actual_output: Hash) -> bool {
        claimed.output_commitment != actual_output
    }
}
