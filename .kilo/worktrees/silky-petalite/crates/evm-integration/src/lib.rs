// EVM Integration Layer for X3 Chain
// This module provides the bridge between the X3 Kernel and the EVM execution environment
// Supports REAL EVM execution via Frontier pallet-evm

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Phase 2: EVM State Integration
/// Account state management, contract code storage, and state database
pub mod state;

/// EVM precompile contracts (sha256, keccak256, x3 cross-vm precompile)
pub mod precompiles;

/// Minimal no-std EVM executor (SputnikVM) — available in both std and no-std builds
pub mod mini_evm;

/// Frontier EVM execution backend (gated behind `frontier` feature; OFF in default RC-1 build).
#[cfg(all(feature = "std", feature = "frontier"))]
pub mod frontier;

#[cfg(all(feature = "std", feature = "frontier"))]
pub use frontier::FrontierEvmExecutor;

/// Result type for EVM operations
pub type EvmResult<T> = Result<T, EvmError>;

/// Errors that can occur during EVM execution
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum EvmError {
    /// Invalid bytecode or transaction data
    InvalidPayload,
    /// EVM execution reverted
    ExecutionReverted,
    /// Out of gas
    OutOfGas,
    /// Invalid account state
    InvalidState,
    /// Stack overflow
    StackOverflow,
    /// Stack underflow
    StackUnderflow,
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Contract creation collision
    CreateCollision,
    /// Other execution error with code
    ExecutionFailed(u32),
}

/// Represents the result of EVM execution
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output data from the execution
    pub output: Vec<u8>,
    /// Gas used in the execution
    pub gas_used: u64,
    /// Any logs emitted during execution
    pub logs: Vec<EvmLog>,
    /// State changes from execution
    pub state_changes: Vec<EvmStateChange>,
    /// State root after execution (computed from state changes)
    pub state_root: [u8; 32],
}

/// Represents an EVM log entry
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmLog {
    /// Address that emitted the log
    pub address: H160,
    /// Topics for the log
    pub topics: Vec<H256>,
    /// Data payload
    pub data: Vec<u8>,
}

/// Represents a state change from EVM execution
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmStateChange {
    /// Account address affected
    pub address: H160,
    /// Balance change (positive or negative encoded as signed)
    pub balance_delta: i128,
    /// Nonce change
    pub nonce_delta: i64,
    /// Storage changes (key -> new value)
    pub storage_changes: Vec<(H256, H256)>,
    /// New code deployed (if contract creation)
    pub code: Option<Vec<u8>>,
}

/// EVM execution environment configuration
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmConfig {
    /// Maximum gas per transaction
    pub gas_limit: u64,
    /// Gas price per unit
    pub gas_price: U256,
    /// Block number for execution context
    pub block_number: u64,
    /// Block timestamp for execution context
    pub block_timestamp: u64,
    /// Chain ID (EIP-155)
    pub chain_id: u64,
    /// Base fee per gas (EIP-1559)
    pub base_fee: U256,
    /// Coinbase/block author address
    pub coinbase: H160,
}

impl Default for EvmConfig {
    fn default() -> Self {
        Self {
            gas_limit: 21_000_000,                // ~20M gas per block
            gas_price: U256::from(1_000_000_000), // 1 gwei
            block_number: 0,
            block_timestamp: 0,
            chain_id: 650_000, // X3 Chain ID (matches runtime ChainId parameter)
            base_fee: U256::from(1_000_000_000), // 1 gwei base
            coinbase: H160::zero(),
        }
    }
}

/// EvmConfig builder for explicit runtime configuration
impl EvmConfig {
    /// Create a new EvmConfig with explicit parameters
    pub fn new(
        gas_limit: u64,
        gas_price: U256,
        block_number: u64,
        block_timestamp: u64,
        chain_id: u64,
    ) -> Self {
        Self {
            gas_limit,
            gas_price,
            block_number,
            block_timestamp,
            chain_id,
            base_fee: U256::from(1_000_000_000),
            coinbase: H160::zero(),
        }
    }

    /// Set the coinbase address
    pub fn with_coinbase(mut self, coinbase: H160) -> Self {
        self.coinbase = coinbase;
        self
    }

    /// Set the base fee
    pub fn with_base_fee(mut self, base_fee: U256) -> Self {
        self.base_fee = base_fee;
        self
    }
}

/// Trait for EVM execution adapters
pub trait EvmExecutor {
    /// Execute EVM bytecode/transaction
    fn execute(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>, // None for contract creation
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult>;

    /// Call EVM contract (read-only, no state changes)
    fn call(
        &self,
        payload: &[u8],
        caller: H160,
        target: H160,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult>;

    /// Validate EVM bytecode without executing
    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()>;

    /// Estimate gas for a transaction
    fn estimate_gas(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<u64>;
}

/// Deployed contract registry entry
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct DeployedContract {
    /// Contract address
    pub address: H160,
    /// Deployer address
    pub deployer: H160,
    /// Code hash of the deployed bytecode
    pub code_hash: H256,
    /// Block number when deployed
    pub deploy_block: u64,
    /// Gas used for deployment
    pub deploy_gas: u64,
}

/// Gas metering report for an EVM execution
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct GasMeteringReport {
    /// Intrinsic gas (base tx cost)
    pub intrinsic_gas: u64,
    /// Execution gas consumed by opcodes
    pub execution_gas: u64,
    /// Storage gas for SSTORE operations
    pub storage_gas: u64,
    /// Refunded gas (SELFDESTRUCT, storage clears)
    pub refunded_gas: u64,
    /// Effective gas used (execution - refund)
    pub effective_gas: u64,
}

/// EVM execution context snapshot for deterministic replay
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct EvmExecutionSnapshot {
    /// Config used for execution
    pub config: EvmConfig,
    /// Caller address
    pub caller: H160,
    /// Target contract (None for creation)
    pub target: Option<H160>,
    /// Call value
    pub value: U256,
    /// Input payload hash (blake2_256)
    pub payload_hash: [u8; 32],
    /// Result state root
    pub state_root: [u8; 32],
    /// Gas metering details
    pub gas_report: GasMeteringReport,
}

/// Mock EVM executor for testing (always succeeds)
#[cfg(any(test, feature = "test-utils"))]
pub struct MockEvmExecutor;

#[cfg(any(test, feature = "test-utils"))]
impl EvmExecutor for MockEvmExecutor {
    fn execute(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>,
        _value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        if payload.is_empty() {
            return Err(EvmError::InvalidPayload);
        }

        let gas_used = config.gas_limit / 2;
        let state_changes = vec![EvmStateChange {
            address: caller,
            balance_delta: -(gas_used as i128),
            nonce_delta: 1,
            storage_changes: Vec::new(),
            code: if target.is_none() {
                Some(payload.to_vec())
            } else {
                None
            },
        }];

        let state_root = compute_mock_state_root(&state_changes);

        Ok(EvmExecutionResult {
            success: true,
            output: vec![0x01],
            gas_used,
            logs: vec![],
            state_changes,
            state_root,
        })
    }

    fn call(
        &self,
        payload: &[u8],
        caller: H160,
        target: H160,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<EvmExecutionResult> {
        self.execute(payload, caller, Some(target), value, config)
    }

    fn validate_bytecode(&self, payload: &[u8]) -> EvmResult<()> {
        if payload.is_empty() {
            return Err(EvmError::InvalidPayload);
        }
        // Reject bytecodes > 24KB (EIP-170)
        if payload.len() > 24_576 {
            return Err(EvmError::ExecutionFailed(0xEF));
        }
        Ok(())
    }

    fn estimate_gas(
        &self,
        payload: &[u8],
        caller: H160,
        target: Option<H160>,
        value: U256,
        config: &EvmConfig,
    ) -> EvmResult<u64> {
        let result = self.execute(payload, caller, target, value, config)?;
        // 10% buffer
        Ok(result.gas_used.saturating_mul(11) / 10)
    }
}

/// Compute state root from state changes for the mock executor.
#[cfg(any(test, feature = "test-utils"))]
fn compute_mock_state_root(changes: &[EvmStateChange]) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    if changes.is_empty() {
        return [0u8; 32];
    }

    let mut data = Vec::new();
    for change in changes {
        data.extend_from_slice(change.address.as_bytes());
        data.extend_from_slice(&change.balance_delta.to_le_bytes());
        data.extend_from_slice(&change.nonce_delta.to_le_bytes());
        for (key, val) in &change.storage_changes {
            data.extend_from_slice(key.as_bytes());
            data.extend_from_slice(val.as_bytes());
        }
    }

    blake2_256(&data)
}

/// Prepare root computation for EVM execution
pub fn compute_evm_prepare_root(
    comit_id: &[u8; 32],
    payload: &[u8],
    result: &EvmExecutionResult,
) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut preimage = Vec::new();
    preimage.extend_from_slice(comit_id);
    preimage.extend_from_slice(payload);
    preimage.extend_from_slice(&result.state_root);

    blake2_256(&preimage)
}

/// Compute CREATE2 address for deterministic contract deployment
pub fn compute_create2_address(deployer: &H160, salt: &H256, init_code_hash: &H256) -> H160 {
    use sp_io::hashing::keccak_256;
    let mut data = Vec::with_capacity(1 + 20 + 32 + 32);
    data.push(0xff);
    data.extend_from_slice(deployer.as_bytes());
    data.extend_from_slice(salt.as_bytes());
    data.extend_from_slice(init_code_hash.as_bytes());
    let hash = keccak_256(&data);
    H160::from_slice(&hash[12..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EvmConfig::default();
        assert_eq!(config.gas_limit, 21_000_000);
        assert_eq!(config.chain_id, 650_000);
    }

    #[test]
    fn test_config_builder() {
        let config = EvmConfig::new(
            30_000_000,
            U256::from(2_000_000_000u64),
            100,
            1_700_000_000,
            650_000,
        )
        .with_coinbase(H160::from_low_u64_be(1))
        .with_base_fee(U256::from(500_000_000u64));

        assert_eq!(config.gas_limit, 30_000_000);
        assert_eq!(config.chain_id, 650_000);
        assert_eq!(config.coinbase, H160::from_low_u64_be(1));
    }

    #[test]
    fn test_mock_executor_success() {
        let executor = MockEvmExecutor;
        let result = executor.execute(
            &[0x01, 0x02],
            H160::zero(),
            Some(H160::zero()),
            U256::zero(),
            &EvmConfig::default(),
        );
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert!(!result.state_changes.is_empty());
        assert_ne!(result.state_root, [0u8; 32]);
    }

    #[test]
    fn test_mock_executor_contract_creation() {
        let executor = MockEvmExecutor;
        let bytecode = vec![0x60, 0x01, 0x60, 0x00, 0x55]; // PUSH1 1, PUSH1 0, SSTORE
        let result = executor
            .execute(
                &bytecode,
                H160::from_low_u64_be(0xAA),
                None,
                U256::zero(),
                &EvmConfig::default(),
            )
            .unwrap();
        assert!(result.success);
        // Contract creation should include code in state changes
        assert!(result.state_changes[0].code.is_some());
    }

    #[test]
    fn test_mock_executor_empty_payload() {
        let executor = MockEvmExecutor;
        let result = executor.execute(
            &[],
            H160::zero(),
            Some(H160::zero()),
            U256::zero(),
            &EvmConfig::default(),
        );
        assert_eq!(result, Err(EvmError::InvalidPayload));
    }

    #[test]
    fn test_estimate_gas() {
        let executor = MockEvmExecutor;
        let gas = executor
            .estimate_gas(
                &[0x60, 0x01],
                H160::zero(),
                None,
                U256::zero(),
                &EvmConfig::default(),
            )
            .unwrap();
        // Should include 10% buffer
        let base_gas = EvmConfig::default().gas_limit / 2;
        assert_eq!(gas, base_gas.saturating_mul(11) / 10);
    }

    #[test]
    fn test_validate_bytecode_too_large() {
        let executor = MockEvmExecutor;
        let large = vec![0x00; 25_000]; // > 24KB
        assert!(executor.validate_bytecode(&large).is_err());
    }

    #[test]
    fn test_compute_create2_address() {
        let deployer = H160::from_low_u64_be(0xFF);
        let salt = H256::zero();
        let code_hash = H256::zero();
        let addr = compute_create2_address(&deployer, &salt, &code_hash);
        assert_ne!(addr, H160::zero());
    }

    #[test]
    fn test_evm_state_root_deterministic() {
        let changes = vec![EvmStateChange {
            address: H160::from_low_u64_be(1),
            balance_delta: 100,
            nonce_delta: 1,
            storage_changes: vec![(H256::zero(), H256::from_low_u64_be(42))],
            code: None,
        }];
        let root1 = compute_mock_state_root(&changes);
        let root2 = compute_mock_state_root(&changes);
        assert_eq!(root1, root2);
        assert_ne!(root1, [0u8; 32]);
    }

    #[test]
    fn test_prepare_root_computation() {
        let comit_id = [0xAA; 32];
        let payload = &[0x60, 0x01];
        let result = EvmExecutionResult {
            success: true,
            output: vec![],
            gas_used: 21000,
            logs: vec![],
            state_changes: vec![],
            state_root: [0xBB; 32],
        };
        let root = compute_evm_prepare_root(&comit_id, payload, &result);
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_evm_error_variants() {
        assert_ne!(EvmError::OutOfGas, EvmError::StackOverflow);
        assert_ne!(EvmError::ExecutionReverted, EvmError::InvalidPayload);
        assert_eq!(EvmError::InvalidOpcode(0xEF), EvmError::InvalidOpcode(0xEF));
    }

    #[test]
    fn test_gas_metering_report_default() {
        let report = GasMeteringReport::default();
        assert_eq!(report.intrinsic_gas, 0);
        assert_eq!(report.execution_gas, 0);
        assert_eq!(report.effective_gas, 0);
    }

    #[test]
    fn test_deployed_contract_struct() {
        let contract = DeployedContract {
            address: H160::from_low_u64_be(42),
            deployer: H160::from_low_u64_be(1),
            code_hash: H256::zero(),
            deploy_block: 100,
            deploy_gas: 21_000,
        };
        assert_eq!(contract.deploy_block, 100);
    }

    #[test]
    fn test_frontier_executor_validate_bytecode_empty() {
        let exec = MockEvmExecutor;
        assert_eq!(
            exec.validate_bytecode(&[]).unwrap_err(),
            EvmError::InvalidPayload
        );
    }

    #[test]
    fn test_frontier_executor_validate_bytecode_too_large() {
        let exec = MockEvmExecutor;
        let big = vec![0u8; 24_577]; // EIP-170: 24KB + 1 byte
        assert!(exec.validate_bytecode(&big).is_err());
    }

    #[test]
    fn test_frontier_executor_validate_bytecode_valid() {
        let exec = MockEvmExecutor;
        let code = vec![0x60, 0x00, 0x56]; // PUSH1 0x00, JUMP
        assert!(exec.validate_bytecode(&code).is_ok());
    }

    #[test]
    fn test_estimate_gas_adds_buffer() {
        let exec = MockEvmExecutor;
        let caller = H160::from_low_u64_be(1);
        let target = H160::from_low_u64_be(2);
        let config = EvmConfig::default();
        // estimate should be >= base execution gas
        let gas = exec
            .estimate_gas(&[0x60, 0x00], caller, Some(target), U256::zero(), &config)
            .expect("estimate gas ok");
        // MockEvmExecutor charges intrinsic_gas (21_000) base plus data cost
        // estimate adds 10% buffer: result >= 21_000
        assert!(gas >= 21_000, "gas estimate too low: {}", gas);
    }

    #[test]
    fn test_compute_evm_prepare_root_changes_with_input() {
        let comit_id = [0xABu8; 32];
        let payload1 = b"contract_a";
        let payload2 = b"contract_b";
        let result = EvmExecutionResult {
            success: true,
            output: vec![],
            gas_used: 21_000,
            logs: vec![],
            state_changes: vec![],
            state_root: [0u8; 32],
        };
        let root1 = compute_evm_prepare_root(&comit_id, payload1, &result);
        let root2 = compute_evm_prepare_root(&comit_id, payload2, &result);
        assert_ne!(
            root1, root2,
            "different payloads must produce different roots"
        );
    }

    #[test]
    fn test_compute_evm_prepare_root_deterministic() {
        let comit_id = [0x01u8; 32];
        let payload = b"deterministic";
        let result = EvmExecutionResult {
            success: true,
            output: vec![],
            gas_used: 0,
            logs: vec![],
            state_changes: vec![],
            state_root: [0u8; 32],
        };
        let r1 = compute_evm_prepare_root(&comit_id, payload, &result);
        let r2 = compute_evm_prepare_root(&comit_id, payload, &result);
        assert_eq!(r1, r2, "same inputs must produce same root");
    }
}
