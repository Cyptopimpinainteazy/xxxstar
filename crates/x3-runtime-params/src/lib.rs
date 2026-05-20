//! X3 Chain Runtime Parameters
//!
//! This module provides comprehensive runtime parameter management for X3 Chain,
//! including block weights, gas limits, and network performance tuning.
//!
//! ## Key Components
//!
//! - **BlockWeights**: Transaction execution weights
//! - **GasLimits**: Maximum gas per block and transaction
//! - **NetworkParams**: Network throughput and connection parameters
//! - **ConsensusParams**: Finality and PoH parameters

pub mod block_weights;
pub mod gas_limits;
pub mod network_params;
pub mod consensus_params;
pub mod tuning;
pub mod benchmarks;

pub use block_weights::{BlockWeights, TransactionWeight, OperationType};
pub use gas_limits::{GasLimits, ComputeBudget};
pub use network_params::NetworkParams;
pub use consensus_params::ConsensusParams;
pub use tuning::{RuntimeTuner, TuningProfile};
pub use benchmarks::{BenchmarkResults, LoadScenario, PerformanceMetrics};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Main runtime parameters container
#[derive(Debug, Clone)]
pub struct RuntimeParameters {
    pub block_weights: BlockWeights,
    pub gas_limits: GasLimits,
    pub network: NetworkParams,
    pub consensus: ConsensusParams,
}

impl Default for RuntimeParameters {
    fn default() -> Self {
        Self {
            block_weights: BlockWeights::default(),
            gas_limits: GasLimits::default(),
            network: NetworkParams::default(),
            consensus: ConsensusParams::default(),
        }
    }
}

impl RuntimeParameters {
    /// Create optimized parameters for high throughput
    pub fn high_throughput() -> Self {
        Self {
            block_weights: BlockWeights::high_throughput(),
            gas_limits: GasLimits::high_throughput(),
            network: NetworkParams::high_throughput(),
            consensus: ConsensusParams::default(),
        }
    }

    /// Create optimized parameters for low latency
    pub fn low_latency() -> Self {
        Self {
            block_weights: BlockWeights::default(),
            gas_limits: GasLimits::default(),
            network: NetworkParams::low_latency(),
            consensus: ConsensusParams::low_latency(),
        }
    }

    /// Create optimized parameters for archival nodes
    pub fn archival() -> Self {
        Self {
            block_weights: BlockWeights::default(),
            gas_limits: GasLimits::default(),
            network: NetworkParams::archival(),
            consensus: ConsensusParams::default(),
        }
    }

    /// Validate all parameters
    pub fn validate(&self) -> Result<(), String> {
        self.block_weights.validate()?;
        self.gas_limits.validate()?;
        self.network.validate()?;
        self.consensus.validate()?;
        Ok(())
    }
}

/// Runtime parameter manager with hot-reloading support
pub struct RuntimeParameterManager {
    params: Arc<RwLock<RuntimeParameters>>,
    tuning: Arc<RuntimeTuner>,
}

impl RuntimeParameterManager {
    /// Create new parameter manager
    pub fn new(params: RuntimeParameters) -> Self {
        let tuning = Arc::new(RuntimeTuner::new(params.clone()));
        
        Self {
            params: Arc::new(RwLock::new(params)),
            tuning,
        }
    }

    /// Get current parameters
    pub fn get_params(&self) -> RuntimeParameters {
        self.params.read().clone()
    }

    /// Update parameters
    pub fn update_params(&self, params: RuntimeParameters) {
        if let Err(e) = params.validate() {
            warn!("Invalid parameters: {}", e);
            return;
        }
        
        *self.params.write() = params.clone();
        self.tuning.apply_tuning(&params);
        info!("Runtime parameters updated");
    }

    /// Get tuning manager
    pub fn tuner(&self) -> &RuntimeTuner {
        &self.tuning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params() {
        let params = RuntimeParameters::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_high_throughput_params() {
        let params = RuntimeParameters::high_throughput();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_parameter_manager() {
        let params = RuntimeParameters::default();
        let manager = RuntimeParameterManager::new(params);
        
        let current = manager.get_params();
        assert!(current.validate().is_ok());
    }
}