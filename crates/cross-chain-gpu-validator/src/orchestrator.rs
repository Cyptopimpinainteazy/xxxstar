//! Atomic swap orchestrator with dual-chain commit/rollback semantics

use crate::error::Result;
use crate::evm_validator::EvmValidator;
use crate::failover::FailoverManager;
use crate::registry::{AtomicRegistry, AtomicSwapRecord, SwapPhase};
use crate::svm_validator::SvmValidator;
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
    _evm_validator: Arc<EvmValidator>,
    _svm_validator: Arc<SvmValidator>,
    _failover: Arc<FailoverManager>,
    default_timeout: Duration,
}

impl AtomicSwapOrchestrator {
    pub async fn new(redis_url: &str, default_timeout_secs: u64) -> Result<Self> {
        let registry = Arc::new(AtomicRegistry::new(redis_url, 3600).await?);
        let evm_validator = Arc::new(EvmValidator::new(32, false));
        let svm_validator = Arc::new(SvmValidator::new());
        let failover = Arc::new(FailoverManager::new(32));

        Ok(Self {
            registry,
            _evm_validator: evm_validator,
            _svm_validator: svm_validator,
            _failover: failover,
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

    async fn validate_evm_side(&self, _swap_id: &str, block: u64, data: Vec<u8>) -> Result<bool> {
        // Simple validation: non-empty data and valid block number
        if data.is_empty() || block == 0 {
            return Ok(false);
        }
        Ok(true)
    }

    async fn validate_svm_side(&self, _swap_id: &str, slot: u64, data: Vec<u8>) -> Result<bool> {
        // Simple validation: non-empty data and valid slot number
        if data.is_empty() || slot == 0 {
            return Ok(false);
        }
        Ok(true)
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

    #[tokio::test]
    #[ignore] // Requires Redis running
    async fn test_orchestrator_creation() {
        let _result = AtomicSwapOrchestrator::new("redis://localhost:6379", 60).await;
        // Would succeed if Redis is running
    }
}
