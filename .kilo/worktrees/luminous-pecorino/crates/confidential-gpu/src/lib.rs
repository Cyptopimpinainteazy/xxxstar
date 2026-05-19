//! # Confidential GPU Runtime
//!
//! Proposal: PRIV-ENCLAVE-003
//!
//! Provides the low-level interface to NVIDIA Confidential Computing enclaves.
//! Manages:
//! - Enclave initialization and attestation
//! - Threshold key share management (Pedersen DKG)
//! - Secure execution of transactions inside GPU TEEs
//! - Attestation refresh and key rotation
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                 Confidential GPU Runtime                     │
//! │                                                              │
//! │  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐  │
//! │  │   Enclave    │  │  Attestation  │  │   Threshold      │  │
//! │  │   Manager    │  │   Verifier    │  │   DKG Shares     │  │
//! │  └──────┬───────┘  └───────┬───────┘  └────────┬─────────┘  │
//! │         │                  │                   │            │
//! │         └──────────────────┼───────────────────┘            │
//! │                            │                                │
//! │                   NVIDIA CC / TEE Boundary                  │
//! │                            │                                │
//! │                    GPU Enclave Execution                    │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Invariants
//!
//! - PRIV-EXEC-001: TX content never exposed outside enclave
//! - PRIV-EXEC-004: Attestation verified before joining confidential set
//! - PRIV-EXEC-006: Finality latency overhead ≤1ms

#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    non_snake_case,
    unexpected_cfgs,
    unused_parens,
    non_camel_case_types,
    clippy::all
)]
pub mod attestation;
pub mod enclave;
pub mod threshold;

/// Status of the confidential GPU runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatus {
    /// Not initialized.
    Uninitialized,
    /// Initializing enclave.
    Initializing,
    /// Enclave ready, waiting for DKG.
    WaitingForDkg,
    /// Fully operational.
    Active,
    /// Error state.
    Error,
    /// Shutting down.
    ShuttingDown,
}

/// Configuration for the confidential GPU runtime.
#[derive(Debug, Clone)]
pub struct ConfidentialGpuConfig {
    /// GPU device index.
    pub device_index: u32,
    /// GPU model name.
    pub gpu_model: String,
    /// Whether to enable NVIDIA Confidential Computing.
    /// Falls back to simulation mode if false.
    pub enable_cc: bool,
    /// Attestation refresh interval (seconds).
    pub attestation_refresh_secs: u64,
    /// Maximum concurrent transactions in enclave.
    pub max_concurrent_txs: u32,
    /// Validator index in the DKG committee.
    pub validator_index: u32,
    /// Threshold t for t-of-n decryption.
    pub threshold: u32,
    /// Total committee size n.
    pub committee_size: u32,
}

impl Default for ConfidentialGpuConfig {
    fn default() -> Self {
        Self {
            device_index: 0,
            gpu_model: "NVIDIA H100".to_string(),
            enable_cc: false, // Default to simulation
            attestation_refresh_secs: 3600,
            max_concurrent_txs: 16,
            validator_index: 0,
            threshold: 3,
            committee_size: 5,
        }
    }
}

/// The main confidential GPU runtime.
pub struct ConfidentialGpuRuntime {
    config: ConfidentialGpuConfig,
    status: RuntimeStatus,
    enclave: enclave::EnclaveManager,
    attestation: attestation::AttestationManager,
    dkg: threshold::DkgManager,
    executions_completed: u64,
    total_latency_ns: u64,
}

impl ConfidentialGpuRuntime {
    /// Create a new confidential GPU runtime.
    pub fn new(config: ConfidentialGpuConfig) -> Self {
        Self {
            enclave: enclave::EnclaveManager::new(config.device_index, config.enable_cc),
            attestation: attestation::AttestationManager::new(
                config.gpu_model.clone(),
                config.attestation_refresh_secs,
            ),
            dkg: threshold::DkgManager::new(
                config.validator_index,
                config.threshold,
                config.committee_size,
            ),
            status: RuntimeStatus::Uninitialized,
            executions_completed: 0,
            total_latency_ns: 0,
            config,
        }
    }

    /// Initialize the enclave and generate attestation.
    ///
    /// # Invariant: PRIV-EXEC-004
    pub fn initialize(&mut self) -> Result<(), ConfidentialGpuError> {
        self.status = RuntimeStatus::Initializing;

        // Initialize GPU enclave
        self.enclave.initialize()?;

        // Generate attestation report
        self.attestation.generate_report(&self.enclave)?;

        self.status = RuntimeStatus::WaitingForDkg;
        Ok(())
    }

    /// Participate in DKG ceremony.
    pub fn participate_in_dkg(
        &mut self,
        peer_commitments: &[threshold::DkgCommitment],
    ) -> Result<threshold::DkgShare, ConfidentialGpuError> {
        let share = self.dkg.participate(peer_commitments)?;

        if self.dkg.is_complete() {
            self.status = RuntimeStatus::Active;
        }

        Ok(share)
    }

    /// Execute a transaction inside the enclave.
    ///
    /// # Invariant: PRIV-EXEC-001, PRIV-EXEC-006
    pub fn execute_private_tx(
        &mut self,
        encrypted_tx: &[u8],
        decryption_shares: &[private_mempool::DecryptionShare],
    ) -> Result<EnclaveTxResult, ConfidentialGpuError> {
        if self.status != RuntimeStatus::Active {
            return Err(ConfidentialGpuError::NotActive);
        }

        let start = std::time::Instant::now();

        // Step 1: Combine decryption shares inside enclave
        let shared_secret = self.dkg.combine_decryption_shares(decryption_shares)?;

        // Step 2: Decrypt TX inside enclave
        let plaintext = self
            .enclave
            .decrypt_in_enclave(encrypted_tx, &shared_secret)?;

        // Step 3: Execute TX in enclave
        let execution_result = self.enclave.execute_in_enclave(&plaintext)?;

        // Step 4: Encrypt state diff
        let encrypted_diff = self
            .enclave
            .encrypt_state_diff(&execution_result.state_changes)?;

        // Step 5: Sign with enclave key
        let signature = self.attestation.sign_with_enclave_key(&encrypted_diff)?;

        let elapsed = start.elapsed();
        self.executions_completed += 1;
        self.total_latency_ns += elapsed.as_nanos() as u64;

        Ok(EnclaveTxResult {
            encrypted_state_diff: encrypted_diff,
            commitment: execution_result.commitment,
            zk_proof: execution_result.zk_proof,
            enclave_signature: signature,
            execution_time_ns: elapsed.as_nanos() as u64,
        })
    }

    /// Get attestation report for on-chain registration.
    pub fn attestation_report(&self) -> Result<Vec<u8>, ConfidentialGpuError> {
        self.attestation.current_report()
    }

    /// Get enclave public key.
    pub fn enclave_public_key(&self) -> Result<[u8; 32], ConfidentialGpuError> {
        self.attestation.enclave_public_key()
    }

    /// Get runtime status.
    pub fn status(&self) -> RuntimeStatus {
        self.status
    }

    /// Get runtime statistics.
    pub fn stats(&self) -> RuntimeStats {
        RuntimeStats {
            status: self.status,
            executions_completed: self.executions_completed,
            avg_latency_ns: if self.executions_completed > 0 {
                self.total_latency_ns / self.executions_completed
            } else {
                0
            },
            dkg_complete: self.dkg.is_complete(),
            gpu_model: self.config.gpu_model.clone(),
        }
    }
}

/// Result of executing a private transaction.
#[derive(Debug, Clone)]
pub struct EnclaveTxResult {
    /// Encrypted state diff.
    pub encrypted_state_diff: Vec<u8>,
    /// Pedersen commitment to the plaintext diff.
    pub commitment: [u8; 32],
    /// Optional ZK validity proof.
    pub zk_proof: Option<Vec<u8>>,
    /// Signature from the enclave attestation key.
    pub enclave_signature: [u8; 64],
    /// Execution time in nanoseconds.
    pub execution_time_ns: u64,
}

/// Runtime statistics.
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub status: RuntimeStatus,
    pub executions_completed: u64,
    pub avg_latency_ns: u64,
    pub dkg_complete: bool,
    pub gpu_model: String,
}

/// Errors from the confidential GPU runtime.
#[derive(Debug, thiserror::Error)]
pub enum ConfidentialGpuError {
    #[error("Enclave not active")]
    NotActive,

    #[error("Enclave initialization failed: {0}")]
    InitFailed(String),

    #[error("Attestation error: {0}")]
    AttestationFailed(String),

    #[error("DKG error: {0}")]
    DkgFailed(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("GPU device error: {0}")]
    DeviceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_lifecycle() {
        let mut runtime = ConfidentialGpuRuntime::new(ConfidentialGpuConfig::default());
        assert_eq!(runtime.status(), RuntimeStatus::Uninitialized);

        runtime.initialize().unwrap();
        assert_eq!(runtime.status(), RuntimeStatus::WaitingForDkg);
    }

    #[test]
    fn stats_tracking() {
        let runtime = ConfidentialGpuRuntime::new(ConfidentialGpuConfig::default());
        let stats = runtime.stats();
        assert_eq!(stats.executions_completed, 0);
        assert_eq!(stats.gpu_model, "NVIDIA H100");
    }
}
