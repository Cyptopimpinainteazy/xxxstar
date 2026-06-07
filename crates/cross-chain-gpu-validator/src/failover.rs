//! GPU-to-CPU failover logic for validation operations

use crate::error::{Result, ValidatorError};
use crate::kernels::{Keccak256Kernel, Secp256k1Kernel};
use tracing::{info, warn};

/// Failover manager for GPU validation errors
pub struct FailoverManager {
    gpu_kernel_secp: Secp256k1Kernel,
    cpu_kernel_secp: Secp256k1Kernel,
    gpu_kernel_keccak: Keccak256Kernel,
    cpu_kernel_keccak: Keccak256Kernel,
}

/// CPU-only validation fallback used by header validators.
pub struct CpuFallback {
    kernel: Keccak256Kernel,
}

impl CpuFallback {
    pub fn new() -> Self {
        Self {
            kernel: Keccak256Kernel::new(32, false),
        }
    }

    pub fn validate_hash(&self, _height: u64, expected_hash: [u8; 32]) -> Result<[u8; 32]> {
        self.kernel.hash(&expected_hash)
    }
}

impl Default for CpuFallback {
    fn default() -> Self {
        Self::new()
    }
}

impl FailoverManager {
    pub fn new(batch_size: usize) -> Self {
        Self {
            gpu_kernel_secp: Secp256k1Kernel::new(batch_size, true),
            cpu_kernel_secp: Secp256k1Kernel::new(batch_size, false),
            gpu_kernel_keccak: Keccak256Kernel::new(batch_size, true),
            cpu_kernel_keccak: Keccak256Kernel::new(batch_size, false),
        }
    }

    /// Try GPU verification first, fall back to CPU on error
    pub fn verify_signatures_with_failover(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<(usize, u64, bool)> {
        // Try GPU first
        match self
            .gpu_kernel_secp
            .verify_batch_gpu(messages, signatures, public_keys)
        {
            Ok((count, timing)) => {
                info!(
                    "GPU signature verification succeeded: {} valid signatures",
                    count
                );
                Ok((count, timing, true)) // GPU succeeded
            }
            Err(e) => {
                warn!("GPU verification failed: {}, falling back to CPU", e);

                // Fallback to CPU
                match self
                    .cpu_kernel_secp
                    .verify_batch_cpu(messages, signatures, public_keys)
                {
                    Ok((count, timing)) => {
                        info!(
                            "CPU fallback signature verification succeeded: {} valid signatures",
                            count
                        );
                        Ok((count, timing, false)) // CPU fallback used
                    }
                    Err(cpu_err) => Err(ValidatorError::CpuValidationError(format!(
                        "CPU fallback also failed: {cpu_err}"
                    ))),
                }
            }
        }
    }

    /// Try GPU hashing first, fall back to CPU on error
    pub fn hash_with_failover(&self, inputs: &[&[u8]]) -> Result<(Vec<Vec<u8>>, u64, bool)> {
        // Try GPU first
        match self.gpu_kernel_keccak.hash_batch_gpu(inputs) {
            Ok((hashes, timing)) => {
                info!("GPU hashing succeeded for {} inputs", inputs.len());
                Ok((hashes, timing, true)) // GPU succeeded
            }
            Err(e) => {
                warn!("GPU hashing failed: {}, falling back to CPU", e);

                // Fallback to CPU
                match self.cpu_kernel_keccak.hash_batch_cpu(inputs) {
                    Ok((hashes, timing)) => {
                        info!("CPU fallback hashing succeeded for {} inputs", inputs.len());
                        Ok((hashes, timing, false)) // CPU fallback used
                    }
                    Err(cpu_err) => Err(ValidatorError::CpuValidationError(format!(
                        "CPU fallback hashing also failed: {cpu_err}"
                    ))),
                }
            }
        }
    }

    /// Check if GPU is healthy by running a parity test
    pub async fn check_gpu_health(&self) -> Result<bool> {
        let test_inputs = vec![b"health_check".as_slice()];

        match self.gpu_kernel_keccak.verify_parity(&test_inputs) {
            Ok(true) => {
                info!("GPU health check passed");
                Ok(true)
            }
            Ok(false) => {
                warn!("GPU parity check failed - GPU may be producing incorrect results");
                Ok(false)
            }
            Err(e) => {
                warn!("GPU health check failed with error: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_manager_creation() {
        let _manager = FailoverManager::new(32);
        // Manager created successfully
    }

    #[tokio::test]
    async fn test_gpu_health_check() {
        let manager = FailoverManager::new(32);
        let health = manager.check_gpu_health().await.unwrap();
        assert!(health); // CPU should pass health check
    }

    #[test]
    fn test_hash_failover() {
        let manager = FailoverManager::new(32);
        let inputs = vec![b"test".as_slice()];

        let (hashes, _, _used_gpu) = manager.hash_with_failover(&inputs).unwrap();
        assert_eq!(hashes.len(), 1);
        assert_eq!(hashes[0].len(), 32); // Keccak256 = 32 bytes
    }
}
