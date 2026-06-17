//! Cross-chain GPU validator service for Solana and Ethereum
//!
//! This crate provides GPU-accelerated validation of signatures and hashes for EVM,
//! coupled with an atomic swap orchestrator for dual-chain commit/rollback semantics.

pub mod dashboard;
pub mod error;
pub mod evm_validator;
pub mod failover;
pub mod kernels;
pub mod orchestrator;
pub mod registry;
pub mod svm_validator;

use crate::evm_validator::EvmHeaderValidator;
use crate::svm_validator::SvmHeaderValidator;
use std::sync::Arc;

pub use error::ValidatorError;
pub use kernels::{Keccak256Kernel, Secp256k1Kernel};
pub use orchestrator::{AtomicSwapOrchestrator, SwapStatus};

/// Core validator types and traits
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SwapRequest {
    pub swap_id: String,
    pub evm_data: Vec<u8>,
    pub svm_data: Vec<u8>,
    pub timeout_secs: u64,
}

#[derive(Debug)]
pub enum ValidationError {
    GpuNotAvailable,
    InvalidBlockHeader,
    CpuFallbackFailed(String),
    DeterminismViolation,
}

/// Pending validation task sourced from the orchestrator.
#[derive(Debug, Clone)]
pub struct PendingValidationTask {
    pub swap_id: String,
    pub evm_data: Vec<u8>,
    pub svm_data: Vec<u8>,
}

/// Cross-chain validator that coordinates EVM and SVM header validation.
///
/// Holds GPU-accelerated validators for both chains.  An optional
/// `AtomicSwapOrchestrator` handle enables live proof-submission polling;
/// when `None`, `run_validation_loop()` is a diagnostics-only no-op.
pub struct CrossChainValidator {
    evm_validator: Arc<EvmHeaderValidator>,
    svm_validator: Arc<SvmHeaderValidator>,
    orchestrator: Option<Arc<AtomicSwapOrchestrator>>,
    protocol_version: u32,
}

impl CrossChainValidator {
    /// Create a new cross-chain validator with an optional orchestrator handle.
    ///
    /// Pass `Some(orch)` to enable live proof-submission in `run_validation_loop()`,
    /// or `None` for diagnostics-only operation.
    pub fn new(orchestrator: Option<Arc<AtomicSwapOrchestrator>>, protocol_version: u32) -> Self {
        Self {
            evm_validator: Arc::new(EvmHeaderValidator::new()),
            svm_validator: Arc::new(SvmHeaderValidator::new()),
            orchestrator,
            protocol_version,
        }
    }

    /// Live validation loop.
    ///
    /// When an orchestrator is configured, polls for pending swaps and routes
    /// each through `execute_atomic_swap`.  Without an orchestrator the loop
    /// performs a periodic health heartbeat until the caller shuts it down.
    pub async fn run_validation_loop(&self) -> Result<(), String> {
        log::info!(
            "CrossChainValidator loop started (protocol v{})",
            self.protocol_version
        );
        let poll_interval = tokio::time::Duration::from_secs(10);
        loop {
            if let Some(ref orch) = self.orchestrator {
                let pending = orch.pending_tasks_snapshot().await;
                for task in pending {
                    // Parse evm_block and svm_slot from the task's data payloads.
                    // EVM data: [block_hash: 32B][state_root: 32B][parent_hash: 32B]
                    //           [gas_limit: 8B LE][gas_used: 8B LE][timestamp: 8B LE]
                    // The evm_block is derived from the data or passed separately.
                    // SVM data: [blockhash: 32B][prev_blockhash: 32B] — svm_slot is separate.
                    //
                    // The record's evm_block/svm_slot fields from the registry are the
                    // authoritative block/slot numbers. We read them back from the registry
                    // rather than hard-coding zeros.
                    let evm_block = if let Ok(Some(record)) = orch.get_swap_internal(&task.swap_id).await {
                        record.evm_block
                    } else {
                        0
                    };
                    let svm_slot = if let Ok(Some(record)) = orch.get_swap_internal(&task.swap_id).await {
                        record.svm_slot
                    } else {
                        0
                    };
                    let _ = orch
                        .execute_atomic_swap(
                            task.swap_id,
                            evm_block,
                            svm_slot,
                            task.evm_data,
                            task.svm_data,
                        )
                        .await;
                }
            } else {
                log::debug!("CrossChainValidator: no orchestrator handle — heartbeat tick");
            }
            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Validate an EVM block header using the GPU-accelerated Keccak256 path.
    pub async fn validate_evm_header(
        &self,
        block_number: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_hash: [u8; 32],
        gas_limit: u64,
        gas_used: u64,
        timestamp: u64,
    ) -> Result<[u8; 32], String> {
        self.evm_validator
            .validate_header(
                block_number,
                block_hash,
                state_root,
                parent_hash,
                gas_limit,
                gas_used,
                timestamp,
            )
            .await
            .map_err(|e| format!("EVM validation error: {e:?}"))
    }

    /// Validate an SVM block header using the GPU-accelerated SHA-256 path.
    pub async fn validate_svm_header(
        &self,
        slot: u64,
        block_hash: [u8; 32],
        state_root: [u8; 32],
        parent_slot: u64,
        timestamp: u64,
        height: u64,
    ) -> Result<u64, String> {
        self.svm_validator
            .validate_header(slot, block_hash, state_root, parent_slot, timestamp, height)
            .await
            .map_err(|e| format!("SVM validation error: {e:?}"))
    }
}
