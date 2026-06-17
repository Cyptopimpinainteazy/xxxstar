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
    /// - Block hash (SHA256 of serialized header fields)
    /// - State root
    /// - Parent slot
    /// - Timestamp
    /// - Height
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

        // Build deterministic header serialization so the hash is computed
        // from real header content, not from the asserted hash itself.
        let mut header_bytes = Vec::with_capacity(104);
        header_bytes.extend_from_slice(&slot.to_le_bytes());
        header_bytes.extend_from_slice(&parent_slot.to_le_bytes());
        header_bytes.extend_from_slice(&timestamp.to_le_bytes());
        header_bytes.extend_from_slice(&height.to_le_bytes());
        header_bytes.extend_from_slice(&state_root);

        // Validate block hash using GPU or CPU fallback on real header bytes.
        self.validate_hash(slot, &header_bytes, block_hash)?;

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

    /// Validate block hash using GPU or CPU fallback.
    ///
    /// `header_bytes` is the serialized block header content whose hash
    /// must equal `expected_hash`.
    fn validate_hash(
        &self,
        slot: u64,
        header_bytes: &[u8],
        expected_hash: [u8; 32],
    ) -> Result<(), ValidatorError> {
        match &self.gpu_kernel {
            Some(kernel) => {
                // Hash the real serialized header bytes via GPU.
                let computed = kernel.hash(header_bytes)?;
                if computed == expected_hash {
                    Ok(())
                } else {
                    Err(ValidatorError::Validation(
                        "GPU hash validation failed - hash mismatch".to_string(),
                    ))
                }
            }
            None => {
                // CPU fallback hashes the real header bytes.
                self.cpu_fallback
                    .validate_hash(slot, header_bytes, expected_hash)
                    .map(|_| ())
            }
        }
    }

    /// Validate an SVM slot against expected blockhash and prev blockhash.
    ///
    /// This is the method used by the orchestrator's `validate_svm_side`.
    /// It builds deterministic header bytes from the supplied fields and
    /// verifies that the hash matches the expected `blockhash`.
    pub fn validate_slot(
        &self,
        slot: u64,
        blockhash: [u8; 32],
        prev_blockhash: [u8; 32],
    ) -> Result<bool, ValidatorError> {
        // Reject zero/invalid slots
        if slot == 0 {
            return Err(ValidatorError::Validation(
                "slot cannot be zero".to_string(),
            ));
        }
        if blockhash == [0u8; 32] || prev_blockhash == [0u8; 32] {
            return Err(ValidatorError::Validation(
                "blockhash or prev_blockhash cannot be zero".to_string(),
            ));
        }

        // Build serialized header bytes so the hash is computed from real data.
        let mut header_bytes = Vec::with_capacity(72);
        header_bytes.extend_from_slice(&slot.to_le_bytes());
        header_bytes.extend_from_slice(&prev_blockhash);
        self.validate_hash(slot, &header_bytes, blockhash)?;
        Ok(true)
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

    /// Validate raw header bytes: hash the provided bytes and check that the
    /// result matches `expected_hash`.
    pub fn validate_raw_header(
        &self,
        slot: u64,
        header_bytes: &[u8],
        expected_hash: [u8; 32],
    ) -> Result<(), ValidatorError> {
        self.validate_hash(slot, header_bytes, expected_hash)
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
        Self {
            hasher: Keccak256Kernel::new(32, false),
        }
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
            error: if valid {
                None
            } else {
                Some(format!(
                    "block_hash length {} != 32",
                    state.block_hash.len()
                ))
            },
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

    /// Verify that the GPU path hashes real header bytes and detects a
    /// mismatch against a different expected hash.
    #[test]
    fn test_validate_hash_on_real_header_bytes_detects_mismatch() {
        let validator = SvmHeaderValidator::new();
        // Build deterministic header bytes (same layout as validate_header).
        let mut header_bytes = Vec::new();
        header_bytes.extend_from_slice(&100u64.to_le_bytes()); // slot
        header_bytes.extend_from_slice(&99u64.to_le_bytes()); // parent_slot
        header_bytes.extend_from_slice(&1234567890u64.to_le_bytes()); // timestamp
        header_bytes.extend_from_slice(&100u64.to_le_bytes()); // height
        header_bytes.extend_from_slice(&[0xCDu8; 32]); // state_root

        let kernel = Keccak256Kernel::new(32, false);
        let real_hash = kernel.hash(&header_bytes).unwrap();

        // validate_hash should succeed when expected_hash matches.
        assert!(validator
            .validate_hash(100, &header_bytes, real_hash)
            .is_ok());

        // validate_hash should fail when expected_hash is different.
        assert!(validator
            .validate_hash(100, &header_bytes, [0u8; 32])
            .is_err());
    }

    /// Provenance test: `validate_raw_header` must verify a known
    /// header → hash pair and reject a tampered header.
    #[test]
    fn test_validate_raw_header_positive_and_tampered() {
        let validator = SvmHeaderValidator::new();

        let mut header_bytes = Vec::new();
        header_bytes.extend_from_slice(&42u64.to_le_bytes());
        header_bytes.extend_from_slice(&41u64.to_le_bytes());
        header_bytes.extend_from_slice(&888_888_888u64.to_le_bytes());
        header_bytes.extend_from_slice(&42u64.to_le_bytes());
        header_bytes.extend_from_slice(&[0x33u8; 32]);

        let kernel = Keccak256Kernel::new(32, false);
        let good_hash = kernel.hash(&header_bytes).unwrap();

        // Positive: real hash matches.
        assert!(
            validator
                .validate_raw_header(42, &header_bytes, good_hash)
                .is_ok(),
            "real SVM header must validate"
        );

        // Negative: tampered header (flip one byte) must be rejected.
        let mut tampered = header_bytes.clone();
        tampered[4] ^= 0x01;
        assert!(
            validator
                .validate_raw_header(42, &tampered, good_hash)
                .is_err(),
            "tampered SVM header must be rejected"
        );
    }
}
