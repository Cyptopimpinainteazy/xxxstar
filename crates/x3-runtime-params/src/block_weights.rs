//! Block Weights Module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operation type for weight calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// Simple transfer
    Transfer,
    /// Token transfer
    TokenTransfer,
    /// Smart contract call
    ContractCall,
    /// Contract creation
    ContractCreate,
    /// Data storage
    DataStore,
    /// Cross-chain transfer
    CrossChain,
    /// VM execution
    VMExecute,
}

/// Transaction weight configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionWeight {
    pub operation: OperationType,
    pub base_weight: u64,
    pub data_weight: u64, // per byte
    pub compute_weight: u64,
}

impl TransactionWeight {
    /// Calculate total weight for a transaction
    pub fn calculate(&self, data_size: usize, compute_units: u64) -> u64 {
        self.base_weight 
            + (data_size as u64 * self.data_weight)
            + (compute_units * self.compute_weight)
    }
}

/// Block weight limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockWeights {
    /// Maximum weight per block
    pub max_block_weight: u64,
    /// Maximum transactions per block
    pub max_transactions: u32,
    /// Weight configurations by operation type
    pub weights: HashMap<OperationType, TransactionWeight>,
    /// Default compute budget per transaction
    pub default_compute_budget: u64,
}

impl Default for BlockWeights {
    fn default() -> Self {
        let mut weights = HashMap::new();
        
        weights.insert(OperationType::Transfer, TransactionWeight {
            operation: OperationType::Transfer,
            base_weight: 100,
            data_weight: 1,
            compute_weight: 0,
        });
        
        weights.insert(OperationType::TokenTransfer, TransactionWeight {
            operation: OperationType::TokenTransfer,
            base_weight: 200,
            data_weight: 1,
            compute_weight: 1,
        });
        
        weights.insert(OperationType::ContractCall, TransactionWeight {
            operation: OperationType::ContractCall,
            base_weight: 500,
            data_weight: 2,
            compute_weight: 10,
        });
        
        weights.insert(OperationType::ContractCreate, TransactionWeight {
            operation: OperationType::ContractCreate,
            base_weight: 1000,
            data_weight: 5,
            compute_weight: 20,
        });
        
        Self {
            max_block_weight: 60_000_000, // ~60M weight units
            max_transactions: 1200,
            weights,
            default_compute_budget: 200_000,
        }
    }
}

impl BlockWeights {
    /// High throughput configuration
    pub fn high_throughput() -> Self {
        let mut weights = Self::default();
        weights.max_block_weight = 120_000_000; // 2x default
        weights.max_transactions = 2400;
        weights
    }

    /// Validate weights
    pub fn validate(&self) -> Result<(), String> {
        if self.max_block_weight == 0 {
            return Err("max_block_weight must be > 0".into());
        }
        
        if self.max_transactions == 0 {
            return Err("max_transactions must be > 0".into());
        }
        
        for (op, weight) in &self.weights {
            if weight.base_weight == 0 {
                return Err(format!("base_weight for {:?} must be > 0", op));
            }
        }
        
        Ok(())
    }

    /// Get weight for operation
    pub fn get_weight(&self, op: OperationType) -> &TransactionWeight {
        self.weights.get(&op).unwrap_or(&self.weights[&OperationType::Transfer])
    }
}