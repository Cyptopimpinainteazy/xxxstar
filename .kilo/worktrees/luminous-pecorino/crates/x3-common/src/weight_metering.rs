//! Weight metering module for X3 Chain
//!
//! Provides unified weight metering for:
//! - **Compute units**: SVM-style compute unit metering
//! - **Gas**: EVM-style gas metering
//! - **Operation costs**: Fixed-cost operations for signing, hashing, etc.
//!
//! # Usage
//!
//! ```rust
//! use x3_common::weight_metering::{
//!     ComputeMeter, GasMeter, OperationCosts, WeightConfig,
//!     WeightResult, WeightError,
//! };
//!
//! // Create a compute meter with a limit
//! let mut meter = ComputeMeter::new(200_000);
//! meter.consume(10_000)?; // Consume 10,000 compute units
//!
//! // Create a gas meter with a limit
//! let mut gas_meter = GasMeter::new(1_000_000);
//! gas_meter.consume(50_000)?; // Consume 50,000 gas
//!
//! // Check operation costs
//! let costs = OperationCosts::default();
//! let cost = costs.signing_ed25519(); // Get ed25519 signing cost
//! ```

extern crate alloc;
use alloc::string::String;

use crate::KeyType;

/// Error type for weight metering operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeightError {
    /// Compute unit limit exceeded
    ComputeLimitExceeded(u64, u64),
    /// Gas limit exceeded
    GasLimitExceeded(u64, u64),
    /// Operation cost exceeded budget
    BudgetExceeded(u64, u64),
    /// Invalid weight configuration
    InvalidConfig(String),
}

impl sp_std::fmt::Display for WeightError {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        match self {
            WeightError::ComputeLimitExceeded(consumed, limit) => {
                write!(
                    f,
                    "Compute unit limit exceeded: consumed {} > limit {}",
                    consumed, limit
                )
            }
            WeightError::GasLimitExceeded(consumed, limit) => {
                write!(
                    f,
                    "Gas limit exceeded: consumed {} > limit {}",
                    consumed, limit
                )
            }
            WeightError::BudgetExceeded(consumed, budget) => {
                write!(
                    f,
                    "Budget exceeded: consumed {} > budget {}",
                    consumed, budget
                )
            }
            WeightError::InvalidConfig(msg) => {
                write!(f, "Invalid weight configuration: {}", msg)
            }
        }
    }
}

/// Result type for weight operations
pub type WeightResult<T> = Result<T, WeightError>;

/// Configuration for weight metering
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightConfig {
    /// Maximum compute units per execution
    pub max_compute_units: u64,
    /// Maximum gas per execution
    pub max_gas: u64,
    /// Maximum operation budget
    pub max_operation_budget: u64,
    /// Compute unit to gas conversion rate
    pub compute_to_gas_ratio: u64,
    /// Operation costs
    pub operation_costs: OperationCosts,
    /// Canonical fee for transaction processing
    /// This is the expected fee for a standard transaction on the chain
    pub canonical_fee: u64,
    /// Fee denominator for weight-to-fee conversion
    pub fee_denominator: u64,
}

impl Default for WeightConfig {
    fn default() -> Self {
        Self {
            max_compute_units: 200_000,
            max_gas: 1_000_000,
            max_operation_budget: 100_000,
            compute_to_gas_ratio: 10,
            operation_costs: OperationCosts::default(),
            canonical_fee: 1_000_000_000, // 1e9 (1 GWei in wei for EVM, or minimal unit for others)
            fee_denominator: 10,          // 10:1 weight-to-fee ratio
        }
    }
}

impl WeightConfig {
    /// Calculate the fee for a given weight
    /// Fee = (weight / fee_denominator) + canonical_fee
    pub fn calculate_fee(&self, weight: u64) -> u64 {
        (weight / self.fee_denominator).saturating_add(self.canonical_fee)
    }

    /// Verify that a fee meets the canonical fee requirement
    pub fn verify_canonical_fee(&self, fee: u64) -> bool {
        fee >= self.canonical_fee
    }

    /// Get the minimum fee for a given weight
    pub fn minimum_fee(&self, weight: u64) -> u64 {
        self.calculate_fee(weight)
    }
}

/// Operation costs for various cryptographic and computational operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCosts {
    /// ed25519 signing cost
    pub signing_ed25519: u64,
    /// secp256k1 signing cost
    pub signing_secp256k1: u64,
    /// sr25519 signing cost
    pub signing_sr25519: u64,
    /// ed25519 verification cost
    pub verify_ed25519: u64,
    /// secp256k1 verification cost
    pub verify_secp256k1: u64,
    /// sr25519 verification cost
    pub verify_sr25519: u64,
    /// SHA-256 hash cost
    pub hash_sha256: u64,
    /// Blake2b-256 hash cost
    pub hash_blake2b: u64,
    /// Keccak256 hash cost
    pub hash_keccak256: u64,
    /// EVM execution cost per instruction
    pub evm_instruction: u64,
    /// SVM execution cost per instruction
    pub svm_instruction: u64,
    /// Cross-VM bridge operation cost
    pub cross_vm_bridge: u64,
    /// Transaction base cost
    pub transaction_base: u64,
}

impl Default for OperationCosts {
    fn default() -> Self {
        Self {
            signing_ed25519: 100,
            signing_secp256k1: 150,
            signing_sr25519: 120,
            verify_ed25519: 80,
            verify_secp256k1: 120,
            verify_sr25519: 100,
            hash_sha256: 10,
            hash_blake2b: 5,
            hash_keccak256: 15,
            evm_instruction: 1,
            svm_instruction: 1,
            cross_vm_bridge: 5000,
            transaction_base: 1000,
        }
    }
}

impl OperationCosts {
    /// Get the cost of signing for a given key type
    pub fn signing(&self, key_type: KeyType) -> u64 {
        match key_type {
            KeyType::Ed25519 => self.signing_ed25519,
            KeyType::Secp256k1 => self.signing_secp256k1,
            KeyType::Sr25519 => self.signing_sr25519,
        }
    }

    /// Get the cost of verification for a given key type
    pub fn verify(&self, key_type: KeyType) -> u64 {
        match key_type {
            KeyType::Ed25519 => self.verify_ed25519,
            KeyType::Secp256k1 => self.verify_secp256k1,
            KeyType::Sr25519 => self.verify_sr25519,
        }
    }

    /// Get the cost of hashing for a given algorithm
    pub fn hash(&self, algorithm: HashAlgorithm) -> u64 {
        match algorithm {
            HashAlgorithm::Sha256 => self.hash_sha256,
            HashAlgorithm::Blake2b => self.hash_blake2b,
            HashAlgorithm::Keccak256 => self.hash_keccak256,
        }
    }
}

/// Hash algorithm identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256
    Sha256,
    /// Blake2b
    Blake2b,
    /// Keccak256
    Keccak256,
}

/// Compute unit meter for SVM-style execution
#[derive(Debug, Clone)]
pub struct ComputeMeter {
    /// Maximum compute units
    limit: u64,
    /// Units consumed
    consumed: u64,
}

impl ComputeMeter {
    /// Create a new compute meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self { limit, consumed: 0 }
    }

    /// Consume compute units; returns Err if over limit
    pub fn consume(&mut self, units: u64) -> WeightResult<()> {
        self.consumed = self.consumed.saturating_add(units);
        if self.consumed > self.limit {
            return Err(WeightError::ComputeLimitExceeded(self.consumed, self.limit));
        }
        Ok(())
    }

    /// Consume compute units with a budget check
    pub fn consume_with_budget(&mut self, units: u64, budget: u64) -> WeightResult<()> {
        let total = self.consumed.saturating_add(units);
        if total > budget {
            return Err(WeightError::BudgetExceeded(total, budget));
        }
        self.consumed = total;
        Ok(())
    }

    /// Get remaining units
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    /// Get consumed units
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Get the limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Reset the meter
    pub fn reset(&mut self) {
        self.consumed = 0;
    }
}

/// Gas meter for EVM-style execution
#[derive(Debug, Clone)]
pub struct GasMeter {
    /// Maximum gas
    limit: u64,
    /// Gas consumed
    consumed: u64,
}

impl GasMeter {
    /// Create a new gas meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self { limit, consumed: 0 }
    }

    /// Consume gas; returns Err if over limit
    pub fn consume(&mut self, gas: u64) -> WeightResult<()> {
        self.consumed = self.consumed.saturating_add(gas);
        if self.consumed > self.limit {
            return Err(WeightError::GasLimitExceeded(self.consumed, self.limit));
        }
        Ok(())
    }

    /// Consume gas with a budget check
    pub fn consume_with_budget(&mut self, gas: u64, budget: u64) -> WeightResult<()> {
        let total = self.consumed.saturating_add(gas);
        if total > budget {
            return Err(WeightError::BudgetExceeded(total, budget));
        }
        self.consumed = total;
        Ok(())
    }

    /// Get remaining gas
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    /// Get consumed gas
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Get the limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Reset the meter
    pub fn reset(&mut self) {
        self.consumed = 0;
    }
}

/// Unified weight meter that tracks both compute units and gas
#[derive(Debug, Clone)]
pub struct WeightMeter {
    /// Compute unit meter
    compute: ComputeMeter,
    /// Gas meter
    gas: GasMeter,
    /// Configuration
    config: WeightConfig,
}

impl WeightMeter {
    /// Create a new weight meter with the given config
    pub fn new(config: WeightConfig) -> Self {
        Self {
            compute: ComputeMeter::new(config.max_compute_units),
            gas: GasMeter::new(config.max_gas),
            config,
        }
    }

    /// Consume compute units
    pub fn consume_compute(&mut self, units: u64) -> WeightResult<()> {
        self.compute.consume(units)
    }

    /// Consume gas
    pub fn consume_gas(&mut self, gas: u64) -> WeightResult<()> {
        self.gas.consume(gas)
    }

    /// Consume an operation with the given key type
    pub fn consume_operation(&mut self, operation: Operation) -> WeightResult<()> {
        let cost = self.config.operation_costs.signing(operation.key_type());
        self.consume_compute(cost)
    }

    /// Consume a hash operation
    pub fn consume_hash(&mut self, algorithm: HashAlgorithm) -> WeightResult<()> {
        let cost = self.config.operation_costs.hash(algorithm);
        self.consume_compute(cost)
    }

    /// Convert compute units to gas
    pub fn compute_to_gas(&self, units: u64) -> u64 {
        units.saturating_mul(self.config.compute_to_gas_ratio)
    }

    /// Convert gas to compute units
    pub fn gas_to_compute(&self, gas: u64) -> u64 {
        gas.saturating_div(self.config.compute_to_gas_ratio)
    }

    /// Get the remaining compute units
    pub fn remaining_compute(&self) -> u64 {
        self.compute.remaining()
    }

    /// Get the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas.remaining()
    }

    /// Get the consumed compute units
    pub fn consumed_compute(&self) -> u64 {
        self.compute.consumed()
    }

    /// Get the consumed gas
    pub fn consumed_gas(&self) -> u64 {
        self.gas.consumed()
    }

    /// Get the config
    pub fn config(&self) -> &WeightConfig {
        &self.config
    }

    /// Reset the meter
    pub fn reset(&mut self) {
        self.compute.reset();
        self.gas.reset();
    }
}

/// Operation type for weight metering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Signing operation
    Sign(KeyType),
    /// Verification operation
    Verify(KeyType),
    /// Hash operation
    Hash(HashAlgorithm),
    /// EVM execution
    EvmExecution,
    /// SVM execution
    SvmExecution,
    /// Cross-VM bridge
    CrossVmBridge,
    /// Transaction
    Transaction,
}

impl Operation {
    /// Get the key type for signing/verification operations
    pub fn key_type(&self) -> KeyType {
        match self {
            Operation::Sign(key_type) | Operation::Verify(key_type) => *key_type,
            _ => KeyType::Ed25519, // Default for non-key operations
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_meter() {
        let mut meter = ComputeMeter::new(200_000);
        assert_eq!(meter.remaining(), 200_000);

        meter.consume(10_000).unwrap();
        assert_eq!(meter.remaining(), 190_000);
        assert_eq!(meter.consumed(), 10_000);

        meter.consume(190_000).unwrap();
        assert_eq!(meter.remaining(), 0);

        // Should fail when exceeding limit
        assert!(matches!(
            meter.consume(1),
            Err(WeightError::ComputeLimitExceeded(_, _))
        ));
    }

    #[test]
    fn test_gas_meter() {
        let mut meter = GasMeter::new(1_000_000);
        assert_eq!(meter.remaining(), 1_000_000);

        meter.consume(100_000).unwrap();
        assert_eq!(meter.remaining(), 900_000);
        assert_eq!(meter.consumed(), 100_000);

        meter.consume(900_000).unwrap();
        assert_eq!(meter.remaining(), 0);

        // Should fail when exceeding limit
        assert!(matches!(
            meter.consume(1),
            Err(WeightError::GasLimitExceeded(_, _))
        ));
    }

    #[test]
    fn test_weight_meter() {
        let config = WeightConfig::default();
        let mut meter = WeightMeter::new(config);

        meter.consume_compute(10_000).unwrap();
        meter.consume_gas(100_000).unwrap();

        assert_eq!(meter.consumed_compute(), 10_000);
        assert_eq!(meter.consumed_gas(), 100_000);

        // Test conversion
        assert_eq!(meter.compute_to_gas(100), 1000);
        assert_eq!(meter.gas_to_compute(1000), 100);
    }

    #[test]
    fn test_operation_costs() {
        let costs = OperationCosts::default();

        assert_eq!(costs.signing(KeyType::Ed25519), 100);
        assert_eq!(costs.signing(KeyType::Secp256k1), 150);
        assert_eq!(costs.signing(KeyType::Sr25519), 120);

        assert_eq!(costs.verify(KeyType::Ed25519), 80);
        assert_eq!(costs.verify(KeyType::Secp256k1), 120);
        assert_eq!(costs.verify(KeyType::Sr25519), 100);

        assert_eq!(costs.hash(HashAlgorithm::Sha256), 10);
        assert_eq!(costs.hash(HashAlgorithm::Blake2b), 5);
        assert_eq!(costs.hash(HashAlgorithm::Keccak256), 15);
    }

    #[test]
    fn test_weight_meter_budget() {
        let mut config = WeightConfig::default();
        config.max_operation_budget = 50_000;

        let mut meter = WeightMeter::new(config);

        // Should succeed within budget
        meter
            .consume_operation(Operation::Sign(KeyType::Ed25519))
            .unwrap();

        // Should fail when exceeding budget
        // (100 + 100 = 200, which is within the 50,000 budget)
        // Let's use a smaller budget
        config.max_operation_budget = 150;
        let mut meter = WeightMeter::new(config);

        meter
            .consume_operation(Operation::Sign(KeyType::Ed25519))
            .unwrap();
        assert!(matches!(
            meter.consume_operation(Operation::Sign(KeyType::Ed25519)),
            Err(WeightError::BudgetExceeded(_, _))
        ));
    }
}
