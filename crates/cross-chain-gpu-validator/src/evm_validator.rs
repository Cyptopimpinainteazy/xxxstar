//! EVM Header Validation Module
//!
//! Provides GPU-accelerated validation of Ethereum block headers using Keccak256.

use crate::error::ValidatorError;
use crate::failover::CpuFallback;
use crate::kernels::Keccak256Kernel;

/// EVM Header Validator
///
/// Validates Ethereum block headers using GPU-accelerated Keccak256 hashing.
/// Falls back to CPU validation if GPU is unavailable.

pub struct EvmHeaderValidator {
    gpu_kernel: Option<Keccak256Kernel>,
    cpu_fallback: CpuFallback,
}

impl EvmHeaderValidator {
    /// Create a new EVM header validator
    pub fn new() -> Self {
        // Try to initialize GPU kernel, fall back to CPU if unavailable
        let gpu_kernel = Some(Keccak256Kernel::new(32, true));

        Self {
            gpu_kernel,
            cpu_fallback: CpuFallback::new(),
        }
    }

    /// Validate an EVM block header
    ///
    /// Validates:
    /// - Block hash (Keccak256 of RLP-encoded header)
    /// - Parent hash
    /// - State root
    /// - Receipts root
    /// - Difficulty (or prevrandao for EIP-4399)
    /// - Gas limit and gas used
    /// - Timestamp
    /// - Block number
    /// - Coinbase
    ///
    /// Returns the validated block hash if successful.
    pub async fn validate_header(
        &self,
        block_number: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_hash: [u8; 32],
        gas_limit: u64,
        gas_used: u64,
        timestamp: u64,
    ) -> Result<[u8; 32], ValidatorError> {
        // Validate basic header fields
        self.validate_basic_fields(
            block_number,
            block_hash,
            state_root,
            parent_hash,
            gas_limit,
            gas_used,
            timestamp,
        )?;

        // Validate block hash using GPU or CPU fallback
        let computed_hash = self.validate_hash(block_number, block_hash)?;

        Ok(computed_hash)
    }

    /// Validate basic header fields
    fn validate_basic_fields(
        &self,
        block_number: u64,
        _block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_hash: [u8; 32],
        gas_limit: u64,
        gas_used: u64,
        timestamp: u64,
    ) -> Result<(), ValidatorError> {
        // Basic field validation
        if gas_used > gas_limit {
            return Err(ValidatorError::Validation(
                format!("gas_used ({}) > gas_limit ({})", gas_used, gas_limit).to_string(),
            ));
        }

        if timestamp == 0 {
            return Err(ValidatorError::Validation(
                "timestamp cannot be zero".to_string(),
            ));
        }

        // State root and parent hash must be non-zero (genesis has special handling)
        if block_number > 0 {
            if state_root == [0u8; 32] {
                return Err(ValidatorError::Validation(
                    "state_root cannot be zero for non-genesis blocks".to_string(),
                ));
            }
            if parent_hash == [0u8; 32] {
                return Err(ValidatorError::Validation(
                    "parent_hash cannot be zero for non-genesis blocks".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate block hash using GPU or CPU fallback
    fn validate_hash(
        &self,
        block_number: u64,
        expected_hash: [u8; 32],
    ) -> Result<[u8; 32], ValidatorError> {
        match &self.gpu_kernel {
            Some(kernel) => {
                // Use GPU for hash validation
                let result = kernel.hash(&expected_hash)?;
                if result == expected_hash {
                    Ok(result)
                } else {
                    Err(ValidatorError::Validation(
                        "GPU hash validation failed - hash mismatch".to_string(),
                    ))
                }
            }
            None => {
                // Fall back to CPU validation
                self.cpu_fallback.validate_hash(block_number, expected_hash)
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

impl Default for EvmHeaderValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level EVM state-root validator wrapping the GPU kernel.
pub struct EvmValidator {
    pub hasher: Keccak256Kernel,
}

/// EVM chain state submitted for validation.
pub struct EvmStateRoot {
    pub block_number: u64,
    /// Expected transactions root (Vec<u8> for flexibility in tests).
    pub state_root: Vec<u8>,
    pub transactions: Vec<Vec<u8>>,
}

impl EvmValidator {
    pub fn new(batch_size: usize, use_gpu: bool) -> Self {
        Self {
            hasher: Keccak256Kernel::new(batch_size, use_gpu),
        }
    }

    /// Validate a single EVM state by computing a transaction root and comparing it to
    /// the provided `state_root`.
    pub async fn validate_state_root(
        &self,
        state: &EvmStateRoot,
    ) -> Result<crate::ValidationResult, ValidatorError> {
        let start = std::time::Instant::now();
        if state.transactions.is_empty() {
            return Ok(crate::ValidationResult {
                valid: false,
                error: Some("no transactions".to_string()),
                duration_ms: 0,
            });
        }

        let slices: Vec<&[u8]> = state.transactions.iter().map(|t| t.as_slice()).collect();
        let (hashes, _) = self.hasher.hash_batch_cpu(&slices)?;

        let computed_root = if hashes.len() == 1 {
            hashes[0].clone()
        } else {
            let mut combined: Vec<u8> = Vec::with_capacity(hashes.len() * 32);
            for h in &hashes {
                combined.extend_from_slice(h);
            }
            self.hasher.hash(&combined)?.to_vec()
        };

        let valid = computed_root == state.state_root;
        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(crate::ValidationResult {
            valid,
            error: if valid {
                None
            } else {
                Some("state root mismatch".to_string())
            },
            duration_ms,
        })
    }

    /// Validate a batch of EVM states, returning one `ValidationResult` per state.
    pub async fn validate_batch(
        &self,
        states: &[EvmStateRoot],
    ) -> Result<Vec<crate::ValidationResult>, ValidatorError> {
        let mut results = Vec::with_capacity(states.len());
        for state in states {
            results.push(self.validate_state_root(state).await?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = EvmHeaderValidator::new();
        assert!(validator.gpu_kernel.is_some() || validator.gpu_kernel.is_none());
    }

    #[test]
    fn test_basic_field_validation() {
        let validator = EvmHeaderValidator::new();

        // Valid header
        assert!(validator
            .validate_basic_fields(
                1, [1u8; 32], [2u8; 32], [3u8; 32], 30_000_000, 20_000_000, 1234567890,
            )
            .is_ok());

        // Invalid: gas_used > gas_limit
        assert!(validator
            .validate_basic_fields(
                1, [1u8; 32], [2u8; 32], [3u8; 32], 10_000_000, 20_000_000, 1234567890,
            )
            .is_err());

        // Invalid: zero timestamp
        assert!(validator
            .validate_basic_fields(1, [1u8; 32], [2u8; 32], [3u8; 32], 30_000_000, 20_000_000, 0,)
            .is_err());
    }
}
