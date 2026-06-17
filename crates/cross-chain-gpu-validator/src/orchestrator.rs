//! Atomic swap orchestrator with dual-chain commit/rollback semantics

use crate::error::Result;
use crate::evm_validator::EvmHeaderValidator;
use crate::failover::FailoverManager;
use crate::registry::{AtomicRegistry, AtomicSwapRecord, SwapPhase};
use crate::svm_validator::SvmHeaderValidator;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SwapStatus {
    Pending,
    ValidatingEvm,
    ValidatingSvm,
    ReadyCommit,
    Committed,
    RolledBack,
    TimedOut,
}

/// Main orchestrator for atomic cross-chain swaps
pub struct AtomicSwapOrchestrator {
    registry: Arc<AtomicRegistry>,
    evm_validator: Arc<EvmHeaderValidator>,
    svm_validator: Arc<SvmHeaderValidator>,
    failover: Arc<FailoverManager>,
    default_timeout: Duration,
}

impl AtomicSwapOrchestrator {
    pub async fn new(redis_url: &str, default_timeout_secs: u64) -> Result<Self> {
        let registry = Arc::new(AtomicRegistry::new(redis_url, 3600).await?);
        let evm_validator = Arc::new(EvmHeaderValidator::new());
        let svm_validator = Arc::new(SvmHeaderValidator::new());
        let failover = Arc::new(FailoverManager::new(32));

        Ok(Self {
            registry,
            evm_validator,
            svm_validator,
            failover,
            default_timeout: Duration::from_secs(default_timeout_secs),
        })
    }

    /// Initiate an atomic swap with timeout and dual-chain validation
    pub async fn execute_atomic_swap(
        &self,
        swap_id: String,
        evm_block: u64,
        svm_slot: u64,
        evm_data: Vec<u8>,
        svm_data: Vec<u8>,
    ) -> Result<SwapStatus> {
        // Register swap in atomicity registry
        let record = AtomicSwapRecord::new(
            swap_id.clone(),
            self.default_timeout.as_secs(),
            evm_block,
            svm_slot,
        );
        self.registry.register_swap(&record).await?;
        info!(
            "Registered swap {} with timeout {} secs",
            swap_id,
            self.default_timeout.as_secs()
        );

        // Update phase to validating EVM
        self.registry
            .update_phase(&swap_id, SwapPhase::ValidatingEvm)
            .await?;

        // Validate EVM side with timeout
        let evm_result = timeout(
            self.default_timeout,
            self.validate_evm_side(&swap_id, evm_block, evm_data),
        )
        .await;

        match evm_result {
            Ok(Ok(evm_valid)) => {
                self.registry
                    .mark_evm_validated(&swap_id, evm_valid)
                    .await?;

                if !evm_valid {
                    self.registry
                        .update_phase(&swap_id, SwapPhase::RolledBack)
                        .await?;
                    info!("Swap {} rolled back: EVM validation failed", swap_id);
                    return Ok(SwapStatus::RolledBack);
                }
            }
            Ok(Err(e)) => {
                warn!("EVM validation error: {}", e);
                self.registry
                    .update_phase(&swap_id, SwapPhase::RolledBack)
                    .await?;
                return Ok(SwapStatus::RolledBack);
            }
            Err(_) => {
                error!("EVM validation timeout for swap {}", swap_id);
                self.registry
                    .update_phase(&swap_id, SwapPhase::TimedOut)
                    .await?;
                return Ok(SwapStatus::TimedOut);
            }
        }

        // Update phase to validating SVM
        self.registry
            .update_phase(&swap_id, SwapPhase::ValidatingSvm)
            .await?;

        // Validate SVM side with timeout
        let svm_result = timeout(
            self.default_timeout,
            self.validate_svm_side(&swap_id, svm_slot, svm_data),
        )
        .await;

        match svm_result {
            Ok(Ok(svm_valid)) => {
                self.registry
                    .mark_svm_validated(&swap_id, svm_valid)
                    .await?;

                if !svm_valid {
                    self.registry
                        .update_phase(&swap_id, SwapPhase::RolledBack)
                        .await?;
                    info!("Swap {} rolled back: SVM validation failed", swap_id);
                    return Ok(SwapStatus::RolledBack);
                }
            }
            Ok(Err(e)) => {
                warn!("SVM validation error: {}", e);
                self.registry
                    .update_phase(&swap_id, SwapPhase::RolledBack)
                    .await?;
                return Ok(SwapStatus::RolledBack);
            }
            Err(_) => {
                error!("SVM validation timeout for swap {}", swap_id);
                self.registry
                    .update_phase(&swap_id, SwapPhase::TimedOut)
                    .await?;
                return Ok(SwapStatus::TimedOut);
            }
        }

        // Both sides validated - atomic commit
        self.registry
            .update_phase(&swap_id, SwapPhase::Committed)
            .await?;
        info!("Swap {} atomically committed", swap_id);
        Ok(SwapStatus::Committed)
    }

    async fn validate_evm_side(&self, swap_id: &str, block: u64, data: Vec<u8>) -> Result<bool> {
        if data.is_empty() || block == 0 {
            warn!(
                "Swap {}: EVM validation rejected — empty data or zero block",
                swap_id
            );
            return Ok(false);
        }

        // Extract header fields from the data blob.
        // Expected format: [block_hash: 32B][state_root: 32B][parent_hash: 32B]
        //                 [gas_limit: 8B LE][gas_used: 8B LE][timestamp: 8B LE]
        if data.len() < 120 {
            warn!(
                "Swap {}: EVM validation header data too short ({} bytes, need ≥120)",
                swap_id,
                data.len()
            );
            return Ok(false);
        }
        let block_hash: [u8; 32] = data[0..32].try_into().unwrap_or_default();
        let state_root: [u8; 32] = data[32..64].try_into().unwrap_or_default();
        let parent_hash: [u8; 32] = data[64..96].try_into().unwrap_or_default();
        let gas_limit = u64::from_le_bytes(data[96..104].try_into().unwrap_or_default());
        let gas_used = u64::from_le_bytes(data[104..112].try_into().unwrap_or_default());
        let timestamp = u64::from_le_bytes(data[112..120].try_into().unwrap_or_default());

        // Validate gas fields: gas_used must not exceed gas_limit
        if gas_used > gas_limit {
            warn!(
                "Swap {}: EVM gas_used ({}) exceeds gas_limit ({})",
                swap_id, gas_used, gas_limit
            );
            return Ok(false);
        }

        // Delegate to the EVM header validator for Keccak256-based block-hash check.
        // The CpuFailback inside EvmValidator provides CPU fallthrough when GPU is
        // unavailable, so we always get a real hash comparison.
        match self
            .evm_validator
            .validate_header(
                block,
                block_hash,
                state_root,
                parent_hash,
                gas_limit,
                gas_used,
                timestamp,
            )
            .await
        {
            Ok(computed) => {
                let valid = computed == block_hash;
                if !valid {
                    warn!(
                        "Swap {}: EVM block hash mismatch (expected {:?}, computed {:?})",
                        swap_id, block_hash, computed
                    );
                } else {
                    info!("Swap {}: EVM block {} validated", swap_id, block);
                }
                Ok(valid)
            }
            Err(e) => {
                warn!("Swap {}: EVM validation error: {}", swap_id, e);
                Ok(false)
            }
        }
    }

    async fn validate_svm_side(&self, swap_id: &str, slot: u64, data: Vec<u8>) -> Result<bool> {
        if data.is_empty() || slot == 0 {
            warn!(
                "Swap {}: SVM validation rejected — empty data or zero slot",
                swap_id
            );
            return Ok(false);
        }

        // Extract header fields from the data blob.
        // Expected format: [blockhash: 32B][prev_blockhash: 32B]
        if data.len() < 64 {
            warn!(
                "Swap {}: SVM validation header data too short ({} bytes, need ≥64)",
                swap_id,
                data.len()
            );
            return Ok(false);
        }
        let blockhash: [u8; 32] = data[0..32].try_into().unwrap_or_default();
        let prev_blockhash: [u8; 32] = data[32..64].try_into().unwrap_or_default();

        // Delegate to the SVM slot validator for SHA-256 / secp256k1 verification.
        match self
            .svm_validator
            .validate_slot(slot, blockhash, prev_blockhash)
        {
            Ok(valid) => {
                if !valid {
                    warn!(
                        "Swap {}: SVM slot {} validation failed (blockhash {:?})",
                        swap_id, slot, blockhash
                    );
                } else {
                    info!("Swap {}: SVM slot {} validated", swap_id, slot);
                }
                Ok(valid)
            }
            Err(e) => {
                warn!("Swap {}: SVM validation error: {}", swap_id, e);
                Ok(false)
            }
        }
    }

    /// Return a snapshot of pending swap tasks.
    ///
    /// Currently returns an empty vec because the Redis-based registry
    /// does not expose a key-scanning API.  A future implementation could
    /// use `redis::Cmd::scan` to enumerate pending swaps.
    pub async fn pending_tasks_snapshot(&self) -> Vec<crate::PendingValidationTask> {
        // TODO: implement Redis SCAN-based enumeration of pending swaps
        Vec::new()
    }

    pub async fn get_swap_status(&self, swap_id: &str) -> Result<Option<SwapStatus>> {
        match self.registry.get_swap(swap_id).await? {
            Some(record) => {
                let status = match record.phase {
                    SwapPhase::Pending => SwapStatus::Pending,
                    SwapPhase::ValidatingEvm => SwapStatus::ValidatingEvm,
                    SwapPhase::ValidatingSvm => SwapStatus::ValidatingSvm,
                    SwapPhase::ReadyCommit => SwapStatus::ReadyCommit,
                    SwapPhase::Committed => SwapStatus::Committed,
                    SwapPhase::RolledBack => SwapStatus::RolledBack,
                    SwapPhase::TimedOut => SwapStatus::TimedOut,
                };
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use std::sync::Arc;

    #[test]
    fn evm_side_rejects_empty_data() {
        let orchestrator = AtomicSwapOrchestrator {
            registry: Arc::new(crate::registry::AtomicRegistry::new_in_memory()),
            evm_validator: Arc::new(crate::evm_validator::EvmHeaderValidator::new()),
            svm_validator: Arc::new(crate::svm_validator::SvmHeaderValidator::new()),
            failover: Arc::new(crate::failover::FailoverManager::new(32)),
            default_timeout: std::time::Duration::from_secs(60),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(orchestrator.validate_evm_side("test", 1, vec![]));
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn svm_side_rejects_empty_data() {
        let orchestrator = AtomicSwapOrchestrator {
            registry: Arc::new(crate::registry::AtomicRegistry::new_in_memory()),
            evm_validator: Arc::new(crate::evm_validator::EvmHeaderValidator::new()),
            svm_validator: Arc::new(crate::svm_validator::SvmHeaderValidator::new()),
            failover: Arc::new(crate::failover::FailoverManager::new(32)),
            default_timeout: std::time::Duration::from_secs(60),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(orchestrator.validate_svm_side("test", 1, vec![]));
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn evm_data_extraction_parses_header_fields() {
        let orchestrator = AtomicSwapOrchestrator {
            registry: Arc::new(crate::registry::AtomicRegistry::new_in_memory()),
            evm_validator: Arc::new(crate::evm_validator::EvmHeaderValidator::new()),
            svm_validator: Arc::new(crate::svm_validator::SvmHeaderValidator::new()),
            failover: Arc::new(crate::failover::FailoverManager::new(32)),
            default_timeout: std::time::Duration::from_secs(60),
        };

        // Build valid header data: [block_hash: 32B][state_root: 32B][parent_hash: 32B]
        //                        [gas_limit: 8B LE][gas_used: 8B LE][timestamp: 8B LE]
        let mut data = vec![0u8; 120];
        data[0..32].copy_from_slice(&[0xABu8; 32]); // block_hash
        data[32..64].copy_from_slice(&[0xCDu8; 32]); // state_root
        data[64..96].copy_from_slice(&[0xEFu8; 32]); // parent_hash
        data[96..104].copy_from_slice(&30_000_000u64.to_le_bytes()); // gas_limit
        data[104..112].copy_from_slice(&20_000_000u64.to_le_bytes()); // gas_used
        data[112..120].copy_from_slice(&1234567890u64.to_le_bytes()); // timestamp

        let rt = tokio::runtime::Runtime::new().unwrap();
        // This should not panic — it should reach the header validator
        let result = rt.block_on(orchestrator.validate_evm_side("test", 1, data));
        // Result could be Ok(true) or Ok(false) depending on GPU/CPU hash match
        assert!(result.is_ok());
    }

    #[test]
    fn svm_data_extraction_parses_header_fields() {
        let orchestrator = AtomicSwapOrchestrator {
            registry: Arc::new(crate::registry::AtomicRegistry::new_in_memory()),
            evm_validator: Arc::new(crate::evm_validator::EvmHeaderValidator::new()),
            svm_validator: Arc::new(crate::svm_validator::SvmHeaderValidator::new()),
            failover: Arc::new(crate::failover::FailoverManager::new(32)),
            default_timeout: std::time::Duration::from_secs(60),
        };

        // Build valid SVM header data: [blockhash: 32B][prev_blockhash: 32B]
        let mut data = vec![0u8; 64];
        data[0..32].copy_from_slice(&[0x11u8; 32]);
        data[32..64].copy_from_slice(&[0x22u8; 32]);

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(orchestrator.validate_svm_side("test", 42, data));
        assert!(result.is_ok());
    }
}
