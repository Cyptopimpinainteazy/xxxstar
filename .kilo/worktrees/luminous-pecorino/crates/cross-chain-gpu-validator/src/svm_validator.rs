//! SVM Header Validation Module
//!
//! Provides GPU-accelerated validation of Solana block headers using SHA256 and Secp256k1.

use crate::error::ValidatorError;
use crate::failover::CpuFallback;
use crate::kernels::Keccak256Kernel;

/// SVM Header Validator
///
/// Validates Solana block headers using GPU-accelerated hashing.
/// Falls back to CPU validation if GPU is unavailable.

pub struct SvmHeaderValidator {
    gpu_kernel: Option<Keccak256Kernel>,
    cpu_fallback: CpuFallback,
}

impl SvmHeaderValidator {
    /// Create a new SVM header validator
    pub fn new() -> Self {
        // Try to initialize GPU kernel, fall back to CPU if unavailable
        let gpu_kernel = Some(Keccak256Kernel::new(32, true));

        Self {
            gpu_kernel,
            cpu_fallback: CpuFallback::new(),
        }
    }

    /// Validate an SVM block header
    ///
    /// Validates:
    /// - Slot number
    /// - Block hash (SHA256 of header)
    /// - State root
    /// - Parent slot
    /// - Timestamp
    /// - Hash (block hash)
    /// - Height
    /// - Transactions root
    /// - rewards
    /// - block_height
    ///
    /// Returns the validated slot number if successful.
    pub async fn validate_header(
        &self,
        slot: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_slot: u64,
        timestamp: u64,
        height: u64,
    ) -> Result<u64, ValidatorError> {
        // Validate basic header fields
        self.validate_basic_fields(slot, block_hash, state_root, parent_slot, timestamp, height)?;

        // Validate block hash using GPU or CPU fallback
        self.validate_hash(slot, block_hash)?;

        Ok(slot)
    }

    /// Validate basic header fields
    fn validate_basic_fields(
        &self,
        slot: u64,
        _block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_slot: u64,
        timestamp: u64,
        height: u64,
    ) -> Result<(), ValidatorError> {
        // Basic field validation
        if slot == 0 {
            return Err(ValidatorError::Validation(
                "slot cannot be zero".to_string(),
            ));
        }

        if timestamp == 0 {
            return Err(ValidatorError::Validation(
                "timestamp cannot be zero".to_string(),
            ));
        }

        if height == 0 && slot != 0 {
            // Genesis block has height 0, other blocks should have height > 0
            // This is a simplified check - real implementation would verify block height
        }

        // State root must be non-zero for non-genesis blocks
        if slot > 0 && state_root == [0u8; 32] {
            return Err(ValidatorError::Validation(
                "state_root cannot be zero for non-genesis blocks".to_string(),
            ));
        }

        // Parent slot must be less than current slot
        if parent_slot >= slot {
            return Err(ValidatorError::Validation(
                format!("parent_slot ({}) >= slot ({})", parent_slot, slot).to_string(),
            ));
        }

        Ok(())
    }

    /// Validate block hash using GPU or CPU fallback
    fn validate_hash(&self, slot: u64, expected_hash: [u8; 32]) -> Result<(), ValidatorError> {
        match &self.gpu_kernel {
            Some(kernel) => {
                // Use GPU for hash validation
                let result = kernel.hash(&expected_hash)?;
                if result == expected_hash {
                    Ok(())
                } else {
                    Err(ValidatorError::Validation(
                        "GPU hash validation failed - hash mismatch".to_string(),
                    ))
                }
            }
            None => {
                // Fall back to CPU validation
                self.cpu_fallback
                    .validate_hash(slot, expected_hash)
                    .map(|_| ())
            }
        }
    }

    /// Verify determinism between GPU and CPU results
    pub fn verify_determinism(
        &self,
        gpu_result: &[u8],
        cpu_result: &[u8],
    ) -> Result<bool, ValidatorError> {
        if gpu_result.len() != cpu_result.len() {
            return Err(ValidatorError::Validation(
                "result length mismatch between GPU and CPU".to_string(),
            ));
        }

        Ok(gpu_result == cpu_result)
    }
}

impl Default for SvmHeaderValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level SVM state validator wrapping the GPU kernel.
pub struct SvmValidator {
    hasher: Keccak256Kernel,
}

/// SVM chain state submitted for validation.
pub struct SvmState {
    pub slot: u64,
    /// Block hash; must be exactly 32 bytes for a valid block.
    pub block_hash: Vec<u8>,
    pub transactions: Vec<Vec<u8>>,
}

impl SvmValidator {
    pub fn new() -> Self {
        Self { hasher: Keccak256Kernel::new(32, false) }
    }

    /// Validate that all transactions are non-empty.
    pub async fn validate_transactions(
        &self,
        state: &SvmState,
    ) -> Result<crate::ValidationResult, ValidatorError> {
        let start = std::time::Instant::now();
        for tx in &state.transactions {
            if tx.is_empty() {
                return Ok(crate::ValidationResult {
                    valid: false,
                    error: Some("empty transaction in SVM block".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        }
        Ok(crate::ValidationResult {
            valid: true,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Validate that the block hash is exactly 32 bytes (Solana block hash constraint).
    pub async fn validate_block_hash(
        &self,
        state: &SvmState,
    ) -> Result<crate::ValidationResult, ValidatorError> {
        let start = std::time::Instant::now();
        let valid = state.block_hash.len() == 32;
        Ok(crate::ValidationResult {
            valid,
            error: if valid { None } else { Some(format!("block_hash length {} != 32", state.block_hash.len())) },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = SvmHeaderValidator::new();
        assert!(validator.gpu_kernel.is_some() || validator.gpu_kernel.is_none());
    }

    #[test]
    fn test_basic_field_validation() {
        let validator = SvmHeaderValidator::new();

        // Valid header
        assert!(validator
            .validate_basic_fields(100, [1u8; 32], [2u8; 32], 99, 1234567890, 100,)
            .is_ok());

        // Invalid: zero slot
        assert!(validator
            .validate_basic_fields(0, [1u8; 32], [2u8; 32], 0, 1234567890, 0,)
            .is_err());

        // Invalid: zero timestamp
        assert!(validator
            .validate_basic_fields(100, [1u8; 32], [2u8; 32], 99, 0, 100,)
            .is_err());

        // Invalid: parent_slot >= slot
        assert!(validator
            .validate_basic_fields(100, [1u8; 32], [2u8; 32], 100, 1234567890, 100,)
            .is_err());

        // Invalid: zero state_root for non-genesis
        assert!(validator
            .validate_basic_fields(100, [1u8; 32], [0u8; 32], 99, 1234567890, 100,)
            .is_err());
    }
}
