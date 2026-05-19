//! Gas estimation RPC endpoint
//!
//! Provides `x3_estimateGas` for EVM-compatible gas estimation
//! using forked execution (no state changes).
//!
//! ## Abuse Controls
//!
//! All public RPC entry-points enforce hard limits on input sizes to prevent
//! DoS via oversized calldata or excessive batch sizes.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Abuse-control limits (prevent DoS)
// ---------------------------------------------------------------------------

/// Maximum UTF-8 length for human-readable display messages (hardware wallet screen).
pub const MAX_DISPLAY_MESSAGE_LEN: usize = 256;
/// Maximum calldata length (bytes) accepted by estimation and call endpoints.
/// Matches EIP-2028 practical limits and prevents O(n) gas-loop DoS.
pub const MAX_CALLDATA_LEN: usize = 128_000; // 128 KB
/// Maximum number of transactions in a single batch estimate call.
pub const MAX_BATCH_SIZE: usize = 50;
/// Maximum string length for address fields (`from` / `to`).
pub const MAX_ADDRESS_LEN: usize = 128;

/// Transaction representation for RPC
#[derive(Clone, Debug)]
pub struct RPCTransaction {
    pub from: String,
    pub to: Option<String>,
    pub value: u128,
    pub data: Vec<u8>,
    pub gas_price: u64,
    pub max_fee_per_gas: Option<u64>,
}

/// Estimation result
#[derive(Clone, Debug)]
pub struct GasEstimation {
    pub gas_used: u64,
    pub gas_limit: u64,
    pub execution_time_ms: u64,
    pub status: ExecutionStatus,
    pub revert_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ExecutionStatus {
    Success,
    Reverted,
    OutOfGas,
    InvalidOpcode,
}

/// Forked execution context (isolated state)
#[derive(Clone)]
pub struct ForkContext {
    /// Block height to fork at
    pub fork_height: u32,
    /// Snapshot of state (account → storage)
    pub state_snapshot: HashMap<String, AccountState>,
}

#[derive(Clone)]
pub struct AccountState {
    pub nonce: u64,
    pub balance: u128,
    pub code: Vec<u8>,
    pub storage: HashMap<[u8; 32], [u8; 32]>,
}

/// Gas estimator engine
pub struct GasEstimator {
    pub opcodes: HashMap<u8, OpcodeCost>,
    pub fork_context: Option<ForkContext>,
}

#[derive(Clone)]
pub struct OpcodeCost {
    pub name: String,
    pub base_cost: u64,
    pub dynamic_cost: fn(u64) -> u64, // Function that calculates additional cost
}

impl GasEstimator {
    pub fn new() -> Self {
        let mut opcodes = HashMap::new();

        // Common EVM opcodes and their costs
        opcodes.insert(0x00, OpcodeCost::new("STOP", 0));
        opcodes.insert(0x01, OpcodeCost::new("ADD", 3));
        opcodes.insert(0x02, OpcodeCost::new("MUL", 5));
        opcodes.insert(0x03, OpcodeCost::new("SUB", 3));
        opcodes.insert(0x04, OpcodeCost::new("DIV", 5));
        opcodes.insert(0x20, OpcodeCost::new("SHA3", 30));
        opcodes.insert(0x54, OpcodeCost::new("SLOAD", 100));
        opcodes.insert(0x55, OpcodeCost::new("SSTORE", 20000));

        Self {
            opcodes,
            fork_context: None,
        }
    }

    /// Set fork context for state-aware estimation
    pub fn set_fork_context(&mut self, context: ForkContext) {
        self.fork_context = Some(context);
    }

    /// Estimate gas for a transaction (forked execution)
    pub fn estimate_gas(&self, tx: &RPCTransaction) -> GasEstimation {
        let start = std::time::Instant::now();

        // Intrinsic gas (data + call overhead)
        let intrinsic_gas = self.calculate_intrinsic_gas(tx);

        // Execute in fork context
        let (execution_gas, status, revert_reason) = self.execute_in_fork(tx);

        let total_gas = intrinsic_gas + execution_gas;
        let gas_limit = (total_gas as f64 * 1.25) as u64; // 25% safety margin

        GasEstimation {
            gas_used: total_gas,
            gas_limit,
            execution_time_ms: start.elapsed().as_millis() as u64,
            status,
            revert_reason,
        }
    }

    /// Calculate intrinsic gas (transaction overhead + data)
    fn calculate_intrinsic_gas(&self, tx: &RPCTransaction) -> u64 {
        let mut gas = 21_000u64; // Base transaction cost

        // Data cost: 4 gas per zero byte, 16 per non-zero
        for byte in &tx.data {
            gas += if *byte == 0 { 4 } else { 16 };
        }

        // Contract creation cost
        if tx.to.is_none() {
            gas += 32_000;
        }

        gas
    }

    /// Execute transaction in isolated fork context using runtime API dry-run
    fn execute_in_fork(&self, tx: &RPCTransaction) -> (u64, ExecutionStatus, Option<String>) {
        // Fork-based fallback
        if let Some(fork) = &self.fork_context {
            if let Some(account) = fork.state_snapshot.get(&tx.to.clone().unwrap_or_default()) {
                if !account.code.is_empty() {
                    let execution_gas = self.simulate_opcodes(&account.code);
                    return (execution_gas, ExecutionStatus::Success, None);
                }
            }
        }

        // Heuristic fallback: EIP-2028 compliant (saturating to avoid overflow)
        let estimated_gas = tx.data.iter().fold(21_000u64, |acc, &b| {
            acc.saturating_add(if b == 0 { 4 } else { 16 })
        });

        if estimated_gas > 30_000_000 {
            (
                0,
                ExecutionStatus::OutOfGas,
                Some("Gas limit exceeded".to_string()),
            )
        } else {
            (estimated_gas, ExecutionStatus::Success, None)
        }
    }

    /// Simulate opcode execution
    fn simulate_opcodes(&self, bytecode: &[u8]) -> u64 {
        let mut gas = 0u64;

        let mut pc = 0;
        while pc < bytecode.len() {
            let opcode = bytecode[pc];

            if let Some(cost) = self.opcodes.get(&opcode) {
                gas += cost.base_cost;
            } else {
                // Unknown opcode: assume 3 gas (PUSH, etc.)
                gas += 3;
            }

            pc += 1;
        }

        gas
    }

    /// Batch estimate (multiple txs)
    pub fn estimate_batch(&self, txs: &[RPCTransaction]) -> Vec<GasEstimation> {
        txs.iter().map(|tx| self.estimate_gas(tx)).collect()
    }

    /// Estimate with custom gas limit
    pub fn estimate_with_limit(
        &self,
        tx: &RPCTransaction,
        limit: u64,
    ) -> Result<GasEstimation, String> {
        let est = self.estimate_gas(tx);

        if est.gas_used > limit {
            return Err(format!(
                "Estimated gas {} exceeds limit {}",
                est.gas_used, limit
            ));
        }

        Ok(est)
    }
}

impl OpcodeCost {
    pub fn new(name: &str, cost: u64) -> Self {
        Self {
            name: name.to_string(),
            base_cost: cost,
            dynamic_cost: |_| 0,
        }
    }
}

/// RPC Server implementation (simulation-only).
///
/// This estimator is intended for off-chain tooling and developer simulation.
/// Production node RPC serving should use the runtime-backed Frontier path in
/// `node::rpc_frontier::create_frontier_stub`.
#[deprecated(
    note = "GasEstimationRPC is simulation-only; use node::rpc_frontier::create_frontier_stub for canonical runtime-backed RPC"
)]
pub struct GasEstimationRPC {
    estimator: GasEstimator,
}

impl GasEstimationRPC {
    pub fn new() -> Self {
        Self {
            estimator: GasEstimator::new(),
        }
    }

    /// x3_estimateGas RPC method
    pub fn estimate_gas(&self, tx: &RPCTransaction) -> Result<GasEstimation, String> {
        if tx.from.is_empty() {
            return Err("Missing 'from' field".to_string());
        }
        if tx.from.len() > MAX_ADDRESS_LEN {
            return Err(format!(
                "'from' address too long: {} chars (max {})",
                tx.from.len(),
                MAX_ADDRESS_LEN
            ));
        }
        if let Some(ref to) = tx.to {
            if to.len() > MAX_ADDRESS_LEN {
                return Err(format!(
                    "'to' address too long: {} chars (max {})",
                    to.len(),
                    MAX_ADDRESS_LEN
                ));
            }
        }
        if tx.data.len() > MAX_CALLDATA_LEN {
            return Err(format!(
                "calldata too large: {} bytes (max {})",
                tx.data.len(),
                MAX_CALLDATA_LEN
            ));
        }
        Ok(self.estimator.estimate_gas(tx))
    }

    /// x3_estimateGasMany (batch)
    pub fn estimate_gas_many(&self, txs: &[RPCTransaction]) -> Result<Vec<GasEstimation>, String> {
        if txs.is_empty() {
            return Err("Empty transaction list".to_string());
        }

        if txs.len() > MAX_BATCH_SIZE {
            return Err(format!(
                "Batch too large: {} transactions (max {})",
                txs.len(),
                MAX_BATCH_SIZE
            ));
        }
        // Validate each tx in the batch
        for (i, tx) in txs.iter().enumerate() {
            if tx.data.len() > MAX_CALLDATA_LEN {
                return Err(format!(
                    "Transaction {} calldata too large: {} bytes (max {})",
                    i,
                    tx.data.len(),
                    MAX_CALLDATA_LEN
                ));
            }
        }
        Ok(self.estimator.estimate_batch(txs))
    }

    /// x3_call (simulate without state change)
    ///
    /// NOTE: This method does not execute against live runtime state and must
    /// not be used as a production RPC call path.
    pub fn call(&self, tx: &RPCTransaction) -> Result<Vec<u8>, String> {
        // Validate input before simulation
        if tx.data.len() > MAX_CALLDATA_LEN {
            return Err(format!(
                "calldata too large: {} bytes (max {})",
                tx.data.len(),
                MAX_CALLDATA_LEN
            ));
        }
        let est = self.estimator.estimate_gas(tx);

        if matches!(est.status, ExecutionStatus::Success) {
            // Deterministic simulation output: echo calldata as the synthetic return.
            Ok(tx.data.clone())
        } else {
            Err("Call reverted".to_string())
        }
    }

    /// Internal simulation-only base fee hint.
    ///
    /// Kept private so it is not exposed as a public production-facing API.
    fn gas_price(&self) -> u64 {
        100_000_000_000u64 // 100 Wei simulation baseline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimator_creation() {
        let estimator = GasEstimator::new();
        assert!(!estimator.opcodes.is_empty());
    }

    #[test]
    fn test_intrinsic_gas_transfer() {
        let estimator = GasEstimator::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 1_000_000_000_000_000_000u128,
            data: vec![],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let intrinsic = estimator.calculate_intrinsic_gas(&tx);
        assert_eq!(intrinsic, 21_000); // No data, so just base
    }

    #[test]
    fn test_intrinsic_gas_with_data() {
        let estimator = GasEstimator::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 0,
            data: vec![0x00; 10], // 10 zero bytes
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let intrinsic = estimator.calculate_intrinsic_gas(&tx);
        assert_eq!(intrinsic, 21_000 + 40); // 10 * 4 gas per zero byte
    }

    #[test]
    fn test_contract_creation_cost() {
        let estimator = GasEstimator::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: None, // Contract creation
            value: 0,
            data: vec![0x60, 0x60], // Example bytecode
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let intrinsic = estimator.calculate_intrinsic_gas(&tx);
        assert!(intrinsic > 21_000); // Should include contract creation overhead
    }

    #[test]
    fn test_estimate_gas() {
        let estimator = GasEstimator::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 100,
            data: vec![0x01; 32],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let est = estimator.estimate_gas(&tx);
        assert!(est.gas_used > 0);
        assert!(est.gas_limit > est.gas_used); // Limit should have safety margin
    }

    #[test]
    fn test_gas_limit_safety_margin() {
        let estimator = GasEstimator::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 0,
            data: vec![],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let est = estimator.estimate_gas(&tx);
        // Gas limit should be ~25% higher than used
        let expected_margin = (est.gas_used as f64 * 0.25) as u64;
        assert!(est.gas_limit >= est.gas_used + expected_margin - 1);
    }

    #[test]
    fn test_batch_estimation() {
        let estimator = GasEstimator::new();
        let txs = vec![
            RPCTransaction {
                from: "0xabc".to_string(),
                to: Some("0xdef".to_string()),
                value: 0,
                data: vec![],
                gas_price: 20_000_000_000,
                max_fee_per_gas: None,
            };
            3
        ];

        let results = estimator.estimate_batch(&txs);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_rpc_estimate_gas() {
        let rpc = GasEstimationRPC::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 0,
            data: vec![],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let result = rpc.estimate_gas(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rpc_missing_from() {
        let rpc = GasEstimationRPC::new();
        let tx = RPCTransaction {
            from: "".to_string(),
            to: Some("0xdef".to_string()),
            value: 0,
            data: vec![],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let result = rpc.estimate_gas(&tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_rpc_call_simulation() {
        let rpc = GasEstimationRPC::new();
        let tx = RPCTransaction {
            from: "0xabc".to_string(),
            to: Some("0xdef".to_string()),
            value: 0,
            data: vec![],
            gas_price: 20_000_000_000,
            max_fee_per_gas: None,
        };

        let result = rpc.call(&tx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rpc_gas_price() {
        let rpc = GasEstimationRPC::new();
        let price = rpc.gas_price();
        assert!(price > 0);
    }
}
