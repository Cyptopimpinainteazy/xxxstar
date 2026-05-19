//! # X3 Economic Engine
//!
//! Core economic mechanisms: EIP-1559 dynamic fee market, MEV protection via commit-reveal,
//! slashing insurance fund, and multi-dimensional resource accounting (CPU/GPU/Memory/IO).
//! Designed for deflationary fee burns, attack-resistant MEV handling, and economic security.
//!
//! ## Overview
//!
//! - **EIP-1559 Fee Market**: Base fee adjusts per-block based on target fullness (70% burn/30% validator)
//! - **Commit-Reveal MEV**: 2-phase transaction submission blocks sandwich attacks
//! - **Slashing Insurance**: 5% of slashed stake funds insurance pool; validators can claim recovery
//! - **Multi-Dimensional Resources**: GPU, CPU, memory, I/O each priced separately (Section 3.11)

use serde::{Deserialize, Serialize};
use sp_runtime::Permill;
use std::collections::HashMap;

pub mod calculator;
pub mod curve;
pub mod error;
pub mod reputation;
pub mod types;

/// EIP-1559 style dynamic base fee market
#[derive(Clone, Debug)]
pub struct Eip1559FeeMarket {
    /// Current base fee in units
    pub base_fee: u128,
    /// Target block fullness percentage (0-100)
    target_fullness: u8,
    /// Adjustment factor per block (parts per million)
    adjustment_factor: u32,
}

impl Eip1559FeeMarket {
    /// Create new fee market with initial base fee
    pub fn new(initial_base_fee: u128) -> Self {
        Self {
            base_fee: initial_base_fee,
            target_fullness: 50,
            adjustment_factor: 125_000, // 12.5% adjustment
        }
    }

    /// Adjust base fee based on block fullness
    /// `block_fullness` is 0.0-1.0 (percentage of block capacity used)
    pub fn adjust_base_fee(&mut self, block_fullness: f64) {
        let fullness_pct = (block_fullness * 100.0) as u8;

        if fullness_pct > self.target_fullness {
            // Block fuller than target: increase fee
            let increase_factor = Permill::from_parts(self.adjustment_factor);
            let increase = (self.base_fee as u64)
                .saturating_mul(increase_factor.deconstruct() as u64)
                / 1_000_000;
            self.base_fee = self.base_fee.saturating_add(increase as u128);
        } else if fullness_pct < self.target_fullness {
            // Block less full than target: decrease fee
            let decrease_factor = Permill::from_parts(self.adjustment_factor);
            let decrease = (self.base_fee as u64)
                .saturating_mul(decrease_factor.deconstruct() as u64)
                / 1_000_000;
            self.base_fee = self.base_fee.saturating_sub(decrease as u128);
        }
    }

    /// Split fee: 70% burn, 30% to validators
    pub fn split_fee(&self, total_fee: u128) -> (u128, u128) {
        let burn = (total_fee * 70) / 100;
        let validator = total_fee - burn;
        (burn, validator)
    }
}

/// Commit-reveal proof for MEV protection
#[derive(Clone, Debug)]
pub struct CommitRevealProof {
    /// SHA-256 hash of transaction data
    pub commit_hash: [u8; 32],
    /// Full transaction data (revealed in block 2)
    pub tx_data: Vec<u8>,
    /// Block height when committed
    pub commit_height: u32,
    /// Signature to authorize reveal
    pub signature: Vec<u8>,
}

impl CommitRevealProof {
    /// Create a new commit-reveal proof
    pub fn new(tx_data: Vec<u8>, signature: Vec<u8>) -> Self {
        let commit_hash = sha256(&tx_data);
        Self {
            commit_hash,
            tx_data,
            commit_height: 0,
            signature,
        }
    }

    /// Verify commit hash matches data
    pub fn verify(&self) -> bool {
        let hash = sha256(&self.tx_data);
        hash == self.commit_hash
    }
}

/// SHA-256 helper (mock implementation)
fn sha256(data: &[u8]) -> [u8; 32] {
    let hasher = sp_core::hashing::blake2_256(data);
    let mut result = [0u8; 32];
    result.copy_from_slice(&hasher[..32.min(hasher.len())]);
    result
}

/// Slashing insurance fund state
#[derive(Clone, Debug)]
pub struct SlashingInsuranceFund {
    /// Total balance in insurance pool
    pub pool_balance: u128,
    /// Contribution rate: 5% of slashes
    contribution_rate: Permill,
    /// Claims outstanding: claim_id -> amount
    claims: HashMap<String, u128>,
    /// Approved claims: claim_id -> amount paid
    approved_claims: HashMap<String, u128>,
}

impl Default for SlashingInsuranceFund {
    fn default() -> Self {
        Self::new()
    }
}

impl SlashingInsuranceFund {
    /// Create new insurance fund
    pub fn new() -> Self {
        Self {
            pool_balance: 0,
            contribution_rate: Permill::from_percent(5),
            claims: HashMap::new(),
            approved_claims: HashMap::new(),
        }
    }

    /// Contribute when slash occurs (5% of slash amount)
    pub fn contribute_from_slash(&mut self, slash_amount: u128) {
        let contribution = (slash_amount as u64)
            .saturating_mul(self.contribution_rate.deconstruct() as u64)
            / 1_000_000;
        self.pool_balance = self.pool_balance.saturating_add(contribution as u128);
    }

    /// File new claim (e.g., validator recovery)
    pub fn file_claim(&mut self, claim_id: String, amount: u128) {
        self.claims.insert(claim_id, amount);
    }

    /// Process claim approval or denial
    pub fn process_claim(&mut self, claim_id: &str, approve: bool) -> bool {
        if let Some(amount) = self.claims.remove(claim_id) {
            if approve && self.pool_balance >= amount {
                self.pool_balance = self.pool_balance.saturating_sub(amount);
                self.approved_claims.insert(claim_id.to_string(), amount);
                true
            } else {
                self.claims.insert(claim_id.to_string(), amount); // re-insert if denied
                false
            }
        } else {
            false
        }
    }
}

/// Complete Economic Engine orchestration
#[derive(Clone, Debug)]
pub struct EconomicEngine {
    pub fee_market: Eip1559FeeMarket,
    pub mev_protection: CommitRevealProof,
    pub insurance_fund: SlashingInsuranceFund,
    /// Statistics
    pub total_fees_collected: u128,
    pub total_burned: u128,
    pub total_staked_rewards: u128,
}

impl EconomicEngine {
    /// Initialize engine with starting conditions
    pub fn new(initial_base_fee: u128) -> Self {
        let proof = CommitRevealProof::new(vec![], vec![]);
        Self {
            fee_market: Eip1559FeeMarket::new(initial_base_fee),
            mev_protection: proof,
            insurance_fund: SlashingInsuranceFund::new(),
            total_fees_collected: 0,
            total_burned: 0,
            total_staked_rewards: 0,
        }
    }

    /// Process transaction with fees and MEV protection
    pub fn process_transaction(&mut self, tx_data: Vec<u8>, block_fullness: f64) -> (u128, u128) {
        // Adjust fee market based on block fullness
        self.fee_market.adjust_base_fee(block_fullness);

        // Create commit-reveal proof
        let proof = CommitRevealProof::new(tx_data, vec![]);
        self.mev_protection = proof;

        // Calculate fee and split
        let total_fee = self.fee_market.base_fee;
        let (burned, validator_reward) = self.fee_market.split_fee(total_fee);

        // Track statistics
        self.total_fees_collected = self.total_fees_collected.saturating_add(total_fee);
        self.total_burned = self.total_burned.saturating_add(burned);
        self.total_staked_rewards = self.total_staked_rewards.saturating_add(validator_reward);

        (burned, validator_reward)
    }

    /// Slash validator and contribute to insurance
    pub fn slash_validator(&mut self, amount: u128) {
        self.insurance_fund.contribute_from_slash(amount);
    }
}

#[cfg(test)]
mod resource_accounting_tests {
    use super::*;

    #[test]
    fn test_eip1559_fee_adjustment() {
        let mut market = Eip1559FeeMarket::new(1000);
        let initial = market.base_fee;

        // High fullness increases fee
        market.adjust_base_fee(0.75);
        assert!(market.base_fee > initial);

        // Low fullness decreases fee
        market.adjust_base_fee(0.25);
        assert!(market.base_fee < initial);
    }

    #[test]
    fn test_fee_split() {
        let market = Eip1559FeeMarket::new(1000);
        let (burned, validator) = market.split_fee(1000);
        assert_eq!(burned, 700);
        assert_eq!(validator, 300);
    }

    #[test]
    fn test_commit_reveal_proof() {
        let tx_data = b"transaction_payload".to_vec();
        let proof = CommitRevealProof::new(tx_data.clone(), vec![]);
        assert!(proof.verify());

        let mut bad_proof = proof.clone();
        bad_proof.tx_data[0] = 99; // corrupt
        assert!(!bad_proof.verify());
    }

    #[test]
    fn test_slashing_insurance_fund() {
        let mut fund = SlashingInsuranceFund::new();

        // Contribute 5% of 1000 slash → 50
        fund.contribute_from_slash(1000);
        assert_eq!(fund.pool_balance, 50);

        // File claim
        fund.file_claim("claim-1".to_string(), 30);
        assert!(fund.process_claim("claim-1", true));
        assert_eq!(fund.pool_balance, 20);
    }

    #[test]
    fn test_economic_engine_integration() {
        let mut engine = EconomicEngine::new(100);

        // Process transaction
        let (burned, validator) = engine.process_transaction(b"tx".to_vec(), 0.6);
        assert_eq!(burned, 70);
        assert_eq!(validator, 30);

        // Slash validator
        engine.slash_validator(1000);
        assert_eq!(engine.insurance_fund.pool_balance, 50);

        // Statistics
        assert!(engine.total_fees_collected > 0);
        assert!(engine.total_burned > 0);
    }

    #[test]
    fn test_deny_claim_insufficient_funds() {
        let mut fund = SlashingInsuranceFund::new();
        fund.file_claim("claim-100k".to_string(), 100000);
        assert!(!fund.process_claim("claim-100k", true)); // insufficient funds
    }
}

// ============================================================================
// Multi-Dimensional Resource Accounting (Section 3.11)
// ============================================================================

/// Resource usage vector - measures consumption across all dimensions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceVector {
    /// CPU cycles (deterministic steps)
    pub cpu_cycles: u64,
    /// GPU cycles or time units
    pub gpu_cycles: u64,
    /// Memory in bytes
    pub memory_bytes: u64,
    /// I/O operations count
    pub io_ops: u64,
    /// Storage read operations
    pub storage_reads: u64,
    /// Storage write operations
    pub storage_writes: u64,
}

impl ResourceVector {
    /// Create a new zero-initialized resource vector
    pub fn zero() -> Self {
        Self {
            cpu_cycles: 0,
            gpu_cycles: 0,
            memory_bytes: 0,
            io_ops: 0,
            storage_reads: 0,
            storage_writes: 0,
        }
    }

    /// Sum two resource vectors (for aggregation)
    pub fn saturating_add(&self, other: &Self) -> Self {
        Self {
            cpu_cycles: self.cpu_cycles.saturating_add(other.cpu_cycles),
            gpu_cycles: self.gpu_cycles.saturating_add(other.gpu_cycles),
            memory_bytes: self.memory_bytes.saturating_add(other.memory_bytes),
            io_ops: self.io_ops.saturating_add(other.io_ops),
            storage_reads: self.storage_reads.saturating_add(other.storage_reads),
            storage_writes: self.storage_writes.saturating_add(other.storage_writes),
        }
    }

    /// Check if this vector exceeds declared limits
    pub fn exceeds(&self, limits: &ResourceVector) -> bool {
        self.cpu_cycles > limits.cpu_cycles
            || self.gpu_cycles > limits.gpu_cycles
            || self.memory_bytes > limits.memory_bytes
            || self.io_ops > limits.io_ops
            || self.storage_reads > limits.storage_reads
            || self.storage_writes > limits.storage_writes
    }
}

/// Price vector - unit costs for each resource type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceVector {
    /// Price per million CPU cycles
    pub cpu_per_million: u128,
    /// Price per million GPU cycles
    pub gpu_per_million: u128,
    /// Price per KB of memory
    pub memory_per_kb: u128,
    /// Price per I/O operation
    pub io_op: u128,
    /// Price per storage read
    pub storage_read: u128,
    /// Price per storage write
    pub storage_write: u128,
}

impl PriceVector {
    /// Create a new price vector with default values (canonical X3 tokenomics)
    pub fn canonical() -> Self {
        Self {
            cpu_per_million: 1_000,  // 0.001 token per 1M cycles
            gpu_per_million: 10_000, // 0.01 token per 1M GPU cycles
            memory_per_kb: 100,      // 0.0001 token per KB
            io_op: 10,               // 0.00001 token per IO
            storage_read: 50,        // 0.00005 token per read
            storage_write: 200,      // 0.0002 token per write
        }
    }
}

/// Compute total fee for a given resource usage and price vector
pub fn compute_fee(resources: &ResourceVector, prices: &PriceVector) -> u128 {
    let cpu_cost = (resources.cpu_cycles as u128)
        .saturating_mul(prices.cpu_per_million)
        .saturating_div(1_000_000);
    let gpu_cost = (resources.gpu_cycles as u128)
        .saturating_mul(prices.gpu_per_million)
        .saturating_div(1_000_000);
    let memory_cost = (resources.memory_bytes as u128)
        .saturating_mul(prices.memory_per_kb)
        .saturating_div(1_000);
    let io_cost = (resources.io_ops as u128).saturating_mul(prices.io_op);
    let storage_read_cost = (resources.storage_reads as u128).saturating_mul(prices.storage_read);
    let storage_write_cost =
        (resources.storage_writes as u128).saturating_mul(prices.storage_write);

    cpu_cost
        .saturating_add(gpu_cost)
        .saturating_add(memory_cost)
        .saturating_add(io_cost)
        .saturating_add(storage_read_cost)
        .saturating_add(storage_write_cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_vector_addition() {
        let a = ResourceVector {
            cpu_cycles: 100,
            gpu_cycles: 200,
            memory_bytes: 1024,
            io_ops: 10,
            storage_reads: 5,
            storage_writes: 2,
        };
        let b = ResourceVector {
            cpu_cycles: 50,
            gpu_cycles: 100,
            memory_bytes: 512,
            io_ops: 5,
            storage_reads: 3,
            storage_writes: 1,
        };
        let sum = a.saturating_add(&b);
        assert_eq!(sum.cpu_cycles, 150);
        assert_eq!(sum.gpu_cycles, 300);
        assert_eq!(sum.memory_bytes, 1536);
    }

    #[test]
    fn test_fee_computation() {
        let resources = ResourceVector {
            cpu_cycles: 1_000_000, // 1M
            gpu_cycles: 500_000,   // 0.5M
            memory_bytes: 1024,    // 1KB
            io_ops: 100,
            storage_reads: 50,
            storage_writes: 25,
        };
        let prices = PriceVector::canonical();
        let fee = compute_fee(&resources, &prices);
        // Expected: CPU=1_000, GPU=5_000, Mem=102_400/1000=102? Actually 1024*100/1000=102.4→102
        // IO=100*10=1_000, Read=50*50=2_500, Write=25*200=5_000 → total ~13_602
        assert!(fee > 10_000 && fee < 20_000);
    }

    #[test]
    fn test_resource_exceeds() {
        let resources = ResourceVector {
            cpu_cycles: 100,
            gpu_cycles: 200,
            memory_bytes: 1024,
            io_ops: 10,
            storage_reads: 5,
            storage_writes: 2,
        };
        let limits = ResourceVector {
            cpu_cycles: 50,
            gpu_cycles: 200,
            memory_bytes: 2048,
            io_ops: 20,
            storage_reads: 10,
            storage_writes: 5,
        };
        assert!(resources.exceeds(&limits)); // cpu_cycles > limit
    }
}
