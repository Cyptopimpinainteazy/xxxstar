//! Gas Limits Module

use serde::{Deserialize, Serialize};

/// Compute budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeBudget {
    /// Maximum compute units per transaction
    pub max_compute_units: u64,
    /// Maximum compute units per block
    pub max_block_compute_units: u64,
    /// Base compute units
    pub base_compute_units: u64,
    /// Compute unit price (micro-lamports)
    pub compute_unit_price: u64,
    /// Memory limit in KB
    pub memory_limit_kb: u64,
}

impl Default for ComputeBudget {
    fn default() -> Self {
        Self {
            max_compute_units: 1_400_000,
            max_block_compute_units: 50_000_000,
            base_compute_units: 500,
            compute_unit_price: 1,
            memory_limit_kb: 256,
        }
    }
}

/// Gas limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasLimits {
    /// Maximum gas per transaction
    pub max_gas_per_tx: u64,
    /// Maximum gas per block
    pub max_gas_per_block: u64,
    /// Minimum gas price
    pub min_gas_price: u64,
    /// Gas price bump percentage for priority
    pub priority_gas_bump_percent: u32,
    /// Compute budget
    pub compute: ComputeBudget,
}

impl Default for GasLimits {
    fn default() -> Self {
        Self {
            max_gas_per_tx: 21_000_000, // 21M gas (similar to EIP-1559)
            max_gas_per_block: 1_000_000_000, // 1B gas per block
            min_gas_price: 1, // 1 lamport
            priority_gas_bump_percent: 10,
            compute: ComputeBudget::default(),
        }
    }
}

impl GasLimits {
    /// High throughput configuration
    pub fn high_throughput() -> Self {
        Self {
            max_gas_per_tx: 42_000_000,
            max_gas_per_block: 2_000_000_000,
            min_gas_price: 1,
            priority_gas_bump_percent: 5,
            compute: ComputeBudget {
                max_compute_units: 2_800_000,
                max_block_compute_units: 100_000_000,
                ..Default::default()
            },
        }
    }

    /// Calculate priority fee
    pub fn calculate_priority_fee(&self, base_fee: u64) -> u64 {
        (base_fee * self.priority_gas_bump_percent as u64) / 100
    }

    /// Validate gas limits
    pub fn validate(&self) -> Result<(), String> {
        if self.max_gas_per_tx == 0 {
            return Err("max_gas_per_tx must be > 0".into());
        }
        
        if self.max_gas_per_block < self.max_gas_per_tx {
            return Err("max_gas_per_block must be >= max_gas_per_tx".into());
        }
        
        if self.compute.max_compute_units > self.compute.max_block_compute_units {
            return Err("max_compute_units must be <= max_block_compute_units".into());
        }
        
        Ok(())
    }
}