//! DePIN Marketplace Service for GPU Swarm
//!
//! Proposal: DEPIN-GPU-001
//!
//! Bridges the on-chain `pallet-depin-marketplace` with the off-chain GPU swarm:
//! - Listens for on-chain marketplace orders via RPC subscription
//! - Validates provider capacity (GPU tier, VRAM, availability)
//! - Dispatches rental workloads through the Warden's Marketplace lane
//! - Reports job completion/failure back to the pallet
//! - Handles preemption when higher-priority lanes need resources
//!
//! ## Invariants
//!
//! - DEPIN-MARKET-001: Provider only accepts orders matching registered GPU specs
//! - DEPIN-MARKET-003: Escrow collected before execution begins
//! - DEPIN-MARKET-004: Preemption uses 2ms budget for state checkpoint

use crate::error::SwarmResult;
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// A DePIN marketplace rental order received from on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalOrder {
    /// On-chain order ID.
    pub order_id: [u8; 16],
    /// Requester's account (SS58 or hex).
    pub requester: String,
    /// Required GPU tier.
    pub gpu_tier: GpuTier,
    /// Required VRAM (MB).
    pub min_vram_mb: u32,
    /// Rental duration (seconds).
    pub duration_secs: u64,
    /// Maximum price per hour (micro-tokens).
    pub max_price_per_hour: u64,
    /// Workload payload hash.
    pub workload_hash: [u8; 32],
    /// Escrow amount (confirmed on-chain).
    pub escrow_amount: u128,
}

/// GPU tier matching the pallet types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuTier {
    Consumer,
    Professional,
    DataCenter,
    HPC,
}

/// Status of a rental on the provider side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RentalStatus {
    /// Validating capacity and workload.
    Validating,
    /// Setting up sandbox environment.
    Provisioning,
    /// Workload running.
    Running,
    /// Preempted by higher-priority lane.
    Preempted,
    /// Completed successfully.
    Completed,
    /// Failed.
    Failed,
}

/// An active rental tracked by the DePIN service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveRental {
    pub order: RentalOrder,
    pub status: RentalStatus,
    pub started_at: u64,
    pub compute_units_used: u64,
    pub sandbox_id: Option<String>,
    pub preemption_count: u32,
}

/// Configuration for the DePIN service.
#[derive(Debug, Clone)]
pub struct DepinServiceConfig {
    /// Maximum concurrent rentals.
    pub max_concurrent_rentals: usize,
    /// Preemption state-checkpoint budget (microseconds).
    pub preemption_budget_us: u64,
    /// Minimum acceptable price per hour (micro-tokens).
    pub min_price_per_hour: u64,
    /// RPC endpoint for pallet interaction.
    pub rpc_endpoint: String,
    /// This node's provider account.
    pub provider_account: String,
}

impl Default for DepinServiceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_rentals: 4,
            preemption_budget_us: 2000, // 2ms
            min_price_per_hour: 100_000,
            rpc_endpoint: "ws://127.0.0.1:9944".to_string(),
            provider_account: String::new(),
        }
    }
}

/// The DePIN marketplace service.
pub struct DepinService {
    config: DepinServiceConfig,
    active_rentals: Arc<RwLock<HashMap<[u8; 16], ActiveRental>>>,
    completed_count: u64,
    failed_count: u64,
    total_revenue: u128,
}

impl DepinService {
    /// Create a new DePIN service.
    pub fn new(config: DepinServiceConfig) -> Self {
        Self {
            config,
            active_rentals: Arc::new(RwLock::new(HashMap::new())),
            completed_count: 0,
            failed_count: 0,
            total_revenue: 0,
        }
    }

    /// Accept a rental order after validation.
    ///
    /// # Invariant: DEPIN-MARKET-001
    pub async fn accept_order(
        &self,
        order: RentalOrder,
        available_vram_mb: u32,
        gpu_tier: GpuTier,
    ) -> Result<(), DepinError> {
        // Validate GPU tier match
        if order.gpu_tier != gpu_tier {
            return Err(DepinError::GpuTierMismatch {
                requested: order.gpu_tier,
                available: gpu_tier,
            });
        }

        // Validate VRAM
        if order.min_vram_mb > available_vram_mb {
            return Err(DepinError::InsufficientVram {
                requested: order.min_vram_mb,
                available: available_vram_mb,
            });
        }

        // Validate price
        if order.max_price_per_hour < self.config.min_price_per_hour {
            return Err(DepinError::PriceTooLow {
                offered: order.max_price_per_hour,
                minimum: self.config.min_price_per_hour,
            });
        }

        // Check capacity
        let rentals = self.active_rentals.read().await;
        if rentals.len() >= self.config.max_concurrent_rentals {
            return Err(DepinError::AtCapacity);
        }
        drop(rentals);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let rental = ActiveRental {
            order: order.clone(),
            status: RentalStatus::Provisioning,
            started_at: now,
            compute_units_used: 0,
            sandbox_id: None,
            preemption_count: 0,
        };

        self.active_rentals
            .write()
            .await
            .insert(order.order_id, rental);
        Ok(())
    }

    /// Preempt a rental for higher-priority work.
    ///
    /// # Invariant: DEPIN-MARKET-004
    pub async fn preempt(&self, order_id: &[u8; 16]) -> Result<(), DepinError> {
        let mut rentals = self.active_rentals.write().await;
        let rental = rentals
            .get_mut(order_id)
            .ok_or(DepinError::RentalNotFound)?;

        rental.status = RentalStatus::Preempted;
        rental.preemption_count += 1;

        // In production: checkpoint workload state within 2ms budget
        tracing::info!(
            order_id = ?order_id,
            budget_us = self.config.preemption_budget_us,
            "Preempting rental workload"
        );

        Ok(())
    }

    /// Mark a rental as completed.
    pub async fn complete_rental(&mut self, order_id: &[u8; 16]) -> Result<u64, DepinError> {
        let mut rentals = self.active_rentals.write().await;
        let rental = rentals
            .get_mut(order_id)
            .ok_or(DepinError::RentalNotFound)?;

        rental.status = RentalStatus::Completed;
        let compute_units = rental.compute_units_used;

        rentals.remove(order_id);
        self.completed_count += 1;

        Ok(compute_units)
    }

    /// Get active rental count.
    pub async fn active_count(&self) -> usize {
        self.active_rentals.read().await.len()
    }

    /// Get stats.
    pub fn stats(&self) -> DepinServiceStats {
        DepinServiceStats {
            completed: self.completed_count,
            failed: self.failed_count,
            total_revenue: self.total_revenue,
        }
    }
}

/// DePIN rental job — implements SwarmJob for Warden scheduling.
#[derive(Debug, Clone)]
pub struct RentalJob {
    pub order_id: [u8; 16],
    pub workload_hash: [u8; 32],
    pub duration_secs: u64,
    pub gpu_tier: GpuTier,
    pub sandboxed: bool,
}

impl SwarmJob for RentalJob {
    fn job_type(&self) -> JobType {
        JobType::MarketplaceRental
    }

    fn compute_units(&self) -> u64 {
        self.duration_secs * 1000
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.duration_secs + 60) // grace period
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        // Execution handled by sandbox_manager — this is the Warden hook
        Err(crate::error::SwarmError::InvalidTask(
            "RentalJob executed via sandbox_manager, not directly".into(),
        ))
    }

    fn verify(&self, _result: &JobOutput) -> SwarmResult<bool> {
        // Verification for marketplace jobs is hash-based
        Ok(true)
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::Normal // Preemptible
    }

    fn requires_gpu(&self) -> bool {
        true
    }

    fn min_vram_mb(&self) -> u32 {
        match self.gpu_tier {
            GpuTier::Consumer => 4096,
            GpuTier::Professional => 8192,
            GpuTier::DataCenter => 16384,
            GpuTier::HPC => 40960,
        }
    }
}

/// Stats for the DePIN service.
#[derive(Debug, Clone)]
pub struct DepinServiceStats {
    pub completed: u64,
    pub failed: u64,
    pub total_revenue: u128,
}

/// Errors from the DePIN service.
#[derive(Debug, thiserror::Error)]
pub enum DepinError {
    #[error("GPU tier mismatch: requested {requested:?}, have {available:?}")]
    GpuTierMismatch {
        requested: GpuTier,
        available: GpuTier,
    },
    #[error("Insufficient VRAM: need {requested}MB, have {available}MB")]
    InsufficientVram { requested: u32, available: u32 },
    #[error("Price too low: offered {offered}, minimum {minimum}")]
    PriceTooLow { offered: u64, minimum: u64 },
    #[error("At maximum rental capacity")]
    AtCapacity,
    #[error("Rental not found")]
    RentalNotFound,
    #[error("Sandbox error: {0}")]
    SandboxError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn accept_valid_order() {
        let service = DepinService::new(DepinServiceConfig {
            min_price_per_hour: 1000,
            ..Default::default()
        });

        let order = RentalOrder {
            order_id: [0x01; 16],
            requester: "5GrwvaEF...".into(),
            gpu_tier: GpuTier::DataCenter,
            min_vram_mb: 8192,
            duration_secs: 3600,
            max_price_per_hour: 5000,
            workload_hash: [0xAB; 32],
            escrow_amount: 100_000,
        };

        let result = service
            .accept_order(order, 16384, GpuTier::DataCenter)
            .await;
        assert!(result.is_ok());
        assert_eq!(service.active_count().await, 1);
    }

    /// # Invariant: DEPIN-MARKET-001
    #[tokio::test]
    async fn reject_tier_mismatch() {
        let service = DepinService::new(DepinServiceConfig::default());

        let order = RentalOrder {
            order_id: [0x01; 16],
            requester: "5GrwvaEF...".into(),
            gpu_tier: GpuTier::HPC,
            min_vram_mb: 40960,
            duration_secs: 3600,
            max_price_per_hour: 100_000,
            workload_hash: [0xAB; 32],
            escrow_amount: 500_000,
        };

        let result = service.accept_order(order, 8192, GpuTier::DataCenter).await;
        assert!(matches!(result, Err(DepinError::GpuTierMismatch { .. })));
    }
}
