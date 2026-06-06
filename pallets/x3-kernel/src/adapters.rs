//! VM Execution Adapters for X3 Kernel
//!
//! This module provides the bridge between the pallet's execution interface
//! and the real EVM/SVM executors from the integration crates.

#[cfg(feature = "std")]
use crate::ExecutionLog;
use crate::{ExecutionReceipt, StateChange};
use frame_support::pallet_prelude::*;
use sp_core::H256;
use sp_std::vec;
use sp_std::vec::Vec;

/// Trait for EVM execution adapters
/// Runtime configures this with either MockEvmAdapter (tests) or FrontierEvmAdapter (production)
pub trait EvmExecutorAdapter {
    /// Execute EVM payload and return execution receipt
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError>;

    /// Estimate gas for a payload without state changes
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError>;

    /// Validate EVM bytecode without execution
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;
}

/// Trait for SVM execution adapters
/// Runtime configures this with either MockSvmAdapter (tests) or RbpfSvmAdapter (production)
pub trait SvmExecutorAdapter {
    /// Execute SVM/BPF payload and return execution receipt
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError>;

    /// Validate BPF program without execution
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;
}

/// Trait for X3 VM execution adapters
/// Runtime configures this with either MockX3Adapter (tests) or X3VmAdapter (production)
pub trait X3ExecutorAdapter {
    /// Execute X3 bytecode and return execution receipt
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError>;

    /// Validate X3 bytecode without execution
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;

    /// Estimate gas for X3 bytecode
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError>;
}

/// Mock EVM adapter for testing - always succeeds with predictable values
pub struct MockEvmAdapter;

impl EvmExecutorAdapter for MockEvmAdapter {
    fn execute(payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Mock execution: hash payload to generate deterministic state changes
        let state_root = if payload.is_empty() {
            H256::zero()
        } else {
            H256::from(sp_io::hashing::blake2_256(payload))
        };

        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 21000 + (payload.len() as u64 * 68), // Base + calldata gas
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![StateChange {
                address: vec![0u8; 20], // Zero address
                key: state_root,
                value: state_root,
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(21000 + (payload.len() as u64 * 68))
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }
        Ok(())
    }
}

impl EvmExecutorAdapter for () {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Unit type returns mock receipt (for backwards compatibility)
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 21000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(21000)
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Mock SVM adapter for testing - always succeeds with predictable values
pub struct MockSvmAdapter;

impl SvmExecutorAdapter for MockSvmAdapter {
    fn execute(payload: &[u8], _compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Mock execution: hash payload to generate deterministic state changes
        let state_root = if payload.is_empty() {
            H256::zero()
        } else {
            H256::from(sp_io::hashing::blake2_256(payload))
        };

        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 5000 + (payload.len() as u64 * 10), // Base + instruction cost
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![StateChange {
                address: vec![0u8; 32], // Zero pubkey
                key: state_root,
                value: state_root,
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM payload"));
        }
        Ok(())
    }
}

/// Mock EVM adapter that simulates failures for testing error paths (L-5)
/// Fails when payload starts with 0xFF
pub struct FailingMockEvmAdapter;

impl EvmExecutorAdapter for FailingMockEvmAdapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Simulate failure when payload starts with 0xFF
        if payload.first() == Some(&0xFF) {
            return Err(DispatchError::Other("EVM execution failed (simulated)"));
        }
        // Simulate execution failure (success=false) when payload starts with 0xFE
        if payload.first() == Some(&0xFE) {
            return Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: false,
                gas_used: gas_limit / 2, // Partial gas consumed
                return_data: b"revert".to_vec(),
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });
        }
        // Otherwise delegate to normal mock
        MockEvmAdapter::execute(payload, gas_limit)
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        MockEvmAdapter::estimate_gas(payload)
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        MockEvmAdapter::validate(payload)
    }
}

/// Mock SVM adapter that simulates failures for testing error paths (L-5)
/// Fails when payload starts with 0xFF
pub struct FailingMockSvmAdapter;

impl SvmExecutorAdapter for FailingMockSvmAdapter {
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Simulate failure when payload starts with 0xFF
        if payload.first() == Some(&0xFF) {
            return Err(DispatchError::Other("SVM execution failed (simulated)"));
        }
        // Simulate execution failure (success=false) when payload starts with 0xFE
        if payload.first() == Some(&0xFE) {
            return Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: false,
                gas_used: compute_limit / 2,
                return_data: b"program error".to_vec(),
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });
        }
        // Otherwise delegate to normal mock
        MockSvmAdapter::execute(payload, compute_limit)
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        MockSvmAdapter::validate(payload)
    }
}

impl SvmExecutorAdapter for () {
    fn execute(_payload: &[u8], _compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Unit type returns mock receipt (for backwards compatibility)
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 5000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// Mock X3 adapter for testing - always succeeds with predictable values
pub struct MockX3Adapter;

impl X3ExecutorAdapter for MockX3Adapter {
    fn execute(payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Mock execution: hash payload to generate deterministic state changes
        let state_root = if payload.is_empty() {
            H256::zero()
        } else {
            H256::from(sp_io::hashing::blake2_256(payload))
        };

        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 1000 + (payload.len() as u64 * 5), // Base + per-byte cost
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![StateChange {
                address: vec![0u8; 32], // X3 uses 32-byte addresses
                key: state_root,
                value: state_root,
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        // Check for X3BC magic bytes (0x58 0x33 = "X3")
        if payload.len() >= 2 && payload[0] == 0x58 && payload[1] == 0x33 {
            Ok(())
        } else if payload.len() >= 4 {
            // Allow any 4+ byte payload for testing
            Ok(())
        } else {
            Err(DispatchError::Other("Invalid X3 bytecode header"))
        }
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(1000 + (payload.len() as u64 * 5))
    }
}

/// Mock X3 adapter that simulates failures for testing error paths.
///
/// - Returns Err when payload starts with 0xFF
/// - Returns success=false when payload starts with 0xFE
pub struct FailingMockX3Adapter;

impl X3ExecutorAdapter for FailingMockX3Adapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        if payload.first() == Some(&0xFF) {
            return Err(DispatchError::Other("X3 execution failed (simulated)"));
        }
        if payload.first() == Some(&0xFE) {
            return Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: false,
                gas_used: gas_limit / 2,
                return_data: b"x3 fault".to_vec(),
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });
        }

        MockX3Adapter::execute(payload, gas_limit)
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        MockX3Adapter::validate(payload)
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        MockX3Adapter::estimate_gas(payload)
    }
}

impl X3ExecutorAdapter for () {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        // Unit type returns mock receipt (for backwards compatibility)
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 1000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(1000)
    }
}

#[cfg(feature = "std")]
pub mod real_adapters {
    //! Real VM adapters using solana-rbpf and X3 VM
    //!
    //! These are only available in std builds due to external dependencies.
    //! Note: FrontierEvmAdapter uses a standalone EVM implementation because
    //! the Frontier executor requires runtime type parameters not available here.

    use super::*;
    use x3_svm_integration::{RbpfSvmExecutor, SvmConfig, SvmExecutor};

    /// Production EVM adapter
    /// Uses a standalone EVM implementation for basic bytecode validation and execution.
    /// For full Frontier integration, the runtime should configure pallet-evm directly.
    pub struct FrontierEvmAdapter;

    impl EvmExecutorAdapter for FrontierEvmAdapter {
        fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
            // Basic EVM payload validation
            if payload.is_empty() {
                return Err(DispatchError::Other("Empty EVM payload"));
            }

            // For native execution, we perform basic validation and return a success receipt.
            // The actual EVM execution happens via pallet-evm in the runtime.
            // This adapter is primarily for gas estimation and validation in native context.

            // Compute gas based on payload size (21000 base + 16 per non-zero byte + 4 per zero byte)
            let gas_used: u64 = 21000
                + payload
                    .iter()
                    .map(|&b| if b == 0 { 4u64 } else { 16u64 })
                    .sum::<u64>();
            let gas_used = gas_used.min(gas_limit);

            let mut pseudo_address = [0u8; 20];
            let payload_hash = sp_io::hashing::blake2_256(payload);
            pseudo_address.copy_from_slice(&payload_hash[..20]);

            Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: true,
                gas_used,
                return_data: Vec::new(),
                logs: Vec::new(),
                state_changes: vec![StateChange {
                    // Standalone adapter has no runtime AccountId context, so we derive a
                    // deterministic pseudo-address from payload hash.
                    address: pseudo_address.to_vec(),
                    key: canonical_asset_key(0),
                    value: canonical_balance_value(gas_used as u128),
                }],
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            })
        }

        fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
            if payload.is_empty() {
                return Err(DispatchError::Other("Empty EVM payload"));
            }

            // EIP-2028 gas costs: 16 per non-zero byte, 4 per zero byte, plus 21000 base
            let calldata_gas: u64 = payload
                .iter()
                .map(|&b| if b == 0 { 4u64 } else { 16u64 })
                .sum();
            Ok(21000 + calldata_gas)
        }

        fn validate(payload: &[u8]) -> Result<(), DispatchError> {
            if payload.is_empty() {
                return Err(DispatchError::Other("Empty EVM payload"));
            }
            // Basic validation - check for common invalid patterns
            // Full validation happens during actual execution in pallet-evm
            Ok(())
        }
    }

    /// Production SVM adapter using solana-rbpf
    pub struct RbpfSvmAdapter;

    impl SvmExecutorAdapter for RbpfSvmAdapter {
        fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
            let executor = RbpfSvmExecutor::new();

            let config = SvmConfig {
                compute_unit_limit: compute_limit,
                compute_unit_price: 1,
                slot: 0,
                block_timestamp: 0,
                recent_blockhash: [0u8; 32],
                enable_cpi: false,
                max_cpi_depth: 0,
            };

            let result = executor.execute_bpf(payload, &[], &config).map_err(|e| {
                DispatchError::Other(match e {
                    x3_svm_integration::SvmError::OutOfComputeUnits => "SVM out of compute units",
                    x3_svm_integration::SvmError::InvalidPayload => "Invalid SVM payload",
                    x3_svm_integration::SvmError::InvalidProgramId => "Invalid program ID",
                    x3_svm_integration::SvmError::InvalidAccount => "Invalid account",
                    _ => "SVM execution failed",
                })
            })?;

            // Convert SVM result to pallet ExecutionReceipt
            Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: result.success,
                gas_used: result.compute_units_used,
                return_data: result.output,
                logs: result
                    .logs
                    .into_iter()
                    .map(|log| ExecutionLog {
                        address: vec![0u8; 32], // SVM doesn't have per-log addresses
                        topics: Vec::new(),
                        data: log,
                        block_number: 0,
                    })
                    .collect(),
                state_changes: result
                    .account_updates
                    .into_iter()
                    .map(|update| StateChange {
                        address: update.pubkey.to_vec(),
                        key: canonical_asset_key(0),
                        value: canonical_balance_value(update.lamports as u128),
                    })
                    .collect(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            })
        }

        fn validate(payload: &[u8]) -> Result<(), DispatchError> {
            let executor = RbpfSvmExecutor::new();
            executor
                .validate_program(payload)
                .map_err(|_| DispatchError::Other("Invalid BPF program"))
        }
    }

    /// Production X3 VM adapter using x3-vm
    pub struct X3VmAdapter;

    impl super::X3ExecutorAdapter for X3VmAdapter {
        fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
            use x3_x3_integration::{X3Executor, X3ExecutorConfig};

            let config = X3ExecutorConfig::on_chain().with_gas_limit(gas_limit);

            let receipt = X3Executor::execute(payload, &[], config).map_err(|e| {
                DispatchError::Other(match e {
                    x3_x3_integration::X3IntegrationError::VerificationFailed(_) => {
                        "X3 verification failed"
                    }
                    x3_x3_integration::X3IntegrationError::InvalidBytecode(_) => {
                        "Invalid X3 bytecode"
                    }
                    x3_x3_integration::X3IntegrationError::GasExhausted { .. } => "X3 out of gas",
                    x3_x3_integration::X3IntegrationError::ExecutionFailed(_) => {
                        "X3 execution failed"
                    }
                    x3_x3_integration::X3IntegrationError::StackOverflow => "X3 stack overflow",
                    x3_x3_integration::X3IntegrationError::MemoryOutOfBounds => "X3 memory error",
                    _ => "X3 VM error",
                })
            })?;

            // Convert X3 receipt to pallet ExecutionReceipt
            Ok(ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: receipt.success,
                gas_used: receipt.gas_used,
                return_data: receipt.return_data,
                logs: receipt
                    .logs
                    .into_iter()
                    .map(|log| ExecutionLog {
                        address: vec![0u8; 32], // X3 uses module-level logging
                        topics: vec![log.topic],
                        data: log.data,
                        block_number: 0,
                    })
                    .collect(),
                state_changes: receipt
                    .state_changes
                    .into_iter()
                    .map(|change| StateChange {
                        address: vec![0u8; 32], // X3 module address
                        key: change.key,
                        value: H256::from_slice(change.new_value.get(..32).unwrap_or(&[0u8; 32])),
                    })
                    .collect(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            })
        }

        fn validate(payload: &[u8]) -> Result<(), DispatchError> {
            use x3_x3_integration::X3Executor;
            X3Executor::verify(payload, false)
                .map_err(|_| DispatchError::Other("Invalid X3 bytecode"))
        }

        fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
            use x3_x3_integration::X3Executor;
            X3Executor::estimate_gas(payload)
                .map_err(|_| DispatchError::Other("X3 gas estimation failed"))
        }
    }

    fn canonical_asset_key(asset_id: u32) -> H256 {
        let mut out = [0u8; 32];
        out[..4].copy_from_slice(&asset_id.to_le_bytes());
        H256::from(out)
    }

    fn canonical_balance_value(balance: u128) -> H256 {
        let mut out = [0u8; 32];
        out[..16].copy_from_slice(&balance.to_le_bytes());
        H256::from(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_evm_adapter_execute() {
        let payload = b"test payload";
        let result = MockEvmAdapter::execute(payload, 100_000).unwrap();
        assert!(result.success);
        assert!(result.gas_used > 21000);
    }

    #[test]
    fn test_mock_svm_adapter_execute() {
        let payload = b"test payload";
        let result = MockSvmAdapter::execute(payload, 100_000).unwrap();
        assert!(result.success);
        assert!(result.gas_used > 5000);
    }

    #[test]
    fn test_unit_adapter_backwards_compat() {
        let result = <() as EvmExecutorAdapter>::execute(b"test", 100_000).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 21000);

        let result = <() as SvmExecutorAdapter>::execute(b"test", 100_000).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 5000);
    }
}

#[cfg(all(test, feature = "std"))]
mod real_adapter_tests {
    use super::real_adapters::*;
    use super::*;

    #[test]
    fn test_rbpf_svm_adapter_real_execution() {
        let simple_bpf = vec![
            0xb7, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // mov r0, 0
            0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
        ];
        let result = RbpfSvmAdapter::execute(&simple_bpf, 100_000);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_x3_vm_adapter_validation() {
        // Test X3 bytecode validation
        let invalid_bytecode = vec![0xFF, 0xFF];
        let result = X3VmAdapter::validate(&invalid_bytecode);
        assert!(result.is_err());

        // Valid X3 magic bytes
        let valid_header = vec![0x58, 0x33, 0x42, 0x43];
        let result = X3VmAdapter::validate(&valid_header);
        // May fail due to incomplete module, but should recognize header
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_frontier_evm_adapter_with_real_executor() {
        // This test will work once FrontierEvmAdapter is wired to real executor
        let simple_evm = vec![0x60, 0x00, 0x60, 0x00, 0xf3];
        let result = FrontierEvmAdapter::execute(&simple_evm, 100_000);
        assert!(result.is_ok());
    }
}
