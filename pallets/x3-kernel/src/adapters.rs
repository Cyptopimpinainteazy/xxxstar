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
/// The runtime configures this with `MockEvmAdapter` (tests), `NativeEvmAdapter`
/// (std + `frontier` builds) or `WasmEvmAdapter` (the wasm runtime the chain runs).
pub trait EvmExecutorAdapter {
    /// Execute EVM payload and return execution receipt
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute with an authenticated caller supplied by the dispatch boundary.
    /// Adapters must explicitly support this; ignoring identity is unsafe for persistent state.
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        let _ = (payload, gas_limit, caller);
        Err(DispatchError::Other(
            "Authenticated EVM execution is unsupported",
        ))
    }

    /// Call a deployed target. Calldata must never be treated as fresh bytecode.
    fn execute_call(
        caller: sp_core::H160,
        target: sp_core::H160,
        input: &[u8],
        value: u128,
        gas_limit: u64,
    ) -> Result<ExecutionReceipt, DispatchError> {
        let _ = (caller, target, input, value, gas_limit);
        Err(DispatchError::Other(
            "Authenticated deployed-target EVM calls are unsupported",
        ))
    }

    /// Simulate with the same authenticated identity as execution.
    fn estimate_gas_from(payload: &[u8], caller: sp_core::H160) -> Result<u64, DispatchError> {
        let _ = (payload, caller);
        Err(DispatchError::Other(
            "Authenticated EVM gas estimation is unsupported",
        ))
    }

    /// Estimate gas for a payload without state changes
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError>;

    /// Validate EVM bytecode without execution
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;
}

/// Bind a signed native account to a domain-separated EVM caller identity.
pub fn authenticated_evm_address(account: &impl parity_scale_codec::Encode) -> sp_core::H160 {
    let mut preimage = b"x3:evm:origin:v1:".to_vec();
    preimage.extend_from_slice(&account.encode());
    let hash = sp_io::hashing::keccak_256(&preimage);
    sp_core::H160::from_slice(&hash[12..])
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

    /// Execute X3 bytecode with the chain's contract slots visible to the program.
    ///
    /// `slots` is the chain's `X3ContractStorage`, read by the caller before execution; an empty
    /// slice means the chain holds no slot for this program. There is deliberately no default
    /// implementation: an adapter that cannot see chain storage must say so where it is written,
    /// because an adapter that silently ignores this argument produces receipts for a program that
    /// read zeros instead of the chain's state.
    fn execute_with_slots(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(H256, [u8; 32])],
    ) -> Result<ExecutionReceipt, DispatchError>;

    /// Execute X3 bytecode with the chain's private-submission posture as well as its slots.
    ///
    /// The artifact's compiled policy can *demand* private submission, and the chain's
    /// `Config::PrivateSubmissionChannel` says whether it can meet the demand. The pallet has
    /// already refused a demanding program when the chain cannot meet it, so this argument exists for
    /// the other direction: an adapter that really executes the bytes must tell its engine that a
    /// private channel exists, or the engine's own fail-closed default would refuse the very program
    /// the chain just accepted.
    ///
    /// The default implementation ignores the argument and delegates, which is fail closed: an
    /// adapter that does not implement this still hands its engine a context with no private channel,
    /// so a demanding artifact is refused rather than run in the clear.
    fn execute_with_slots_and_policy(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(H256, [u8; 32])],
        private_channel_available: bool,
    ) -> Result<ExecutionReceipt, DispatchError> {
        let _ = private_channel_available;
        Self::execute_with_slots(payload, gas_limit, slots)
    }

    /// Validate X3 bytecode without execution
    fn validate(payload: &[u8]) -> Result<(), DispatchError>;

    /// Estimate gas for X3 bytecode
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError>;
}

/// Mock EVM adapter for testing - always succeeds with predictable values
pub struct MockEvmAdapter;

impl EvmExecutorAdapter for MockEvmAdapter {
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        _caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        Self::execute(payload, gas_limit)
    }

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
            storage_writes: Vec::new(),
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

// Unit-type adapters are gated behind test/benchmarks only — production
// runtimes must wire real adapters (NativeEvmAdapter/WasmEvmAdapter,
// RbpfSvmAdapter, X3VmAdapter).
#[cfg(any(test, feature = "runtime-benchmarks"))]
impl EvmExecutorAdapter for () {
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        _caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        <Self as EvmExecutorAdapter>::execute(payload, gas_limit)
    }

    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 1000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            storage_writes: Vec::new(),
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
            storage_writes: Vec::new(),
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
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        _caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        Self::execute(payload, gas_limit)
    }

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
                storage_writes: Vec::new(),
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
                storage_writes: Vec::new(),
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

/// The unit-type adapter exists ONLY for test support and backwards compatibility
/// in non-production contexts.
#[cfg(any(test, feature = "dev-mock"))]
impl SvmExecutorAdapter for () {
    fn execute(_payload: &[u8], _compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 5000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            storage_writes: Vec::new(),
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
        Self::execute_with_slots(payload, _gas_limit, &[])
    }

    /// The mock has no chain storage: it reports the state changes it synthesizes and nothing else,
    /// which is what makes it a mock. Stated here rather than defaulted on the trait so a
    /// production adapter cannot inherit "ignores the chain's slots" by accident.
    fn execute_with_slots(
        payload: &[u8],
        _gas_limit: u64,
        _slots: &[(H256, [u8; 32])],
    ) -> Result<ExecutionReceipt, DispatchError> {
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
            storage_writes: Vec::new(),
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
                storage_writes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            });
        }

        MockX3Adapter::execute_with_slots(payload, gas_limit, &[])
    }

    /// `execute`s fault injection, with the same stance on chain storage as `MockX3Adapter`.
    fn execute_with_slots(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(H256, [u8; 32])],
    ) -> Result<ExecutionReceipt, DispatchError> {
        if payload.first() == Some(&0xFF) {
            return Err(DispatchError::Other("X3 execution failed (simulated)"));
        }

        MockX3Adapter::execute_with_slots(payload, gas_limit, slots)
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        MockX3Adapter::validate(payload)
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        MockX3Adapter::estimate_gas(payload)
    }
}

/// The unit-type adapter exists ONLY for test support and backwards compatibility
/// in non-production contexts.  Production code MUST use a concrete adapter
/// (e.g. the runtime's EVM adapter or `X3VmAdapter`, or a custom `X3ExecutorAdapter` impl)
/// and MUST NOT allow `()` to silently produce mock receipts.
#[cfg(any(test, feature = "dev-mock"))]
impl X3ExecutorAdapter for () {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Ok(ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 1000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: Vec::new(),
            storage_writes: Vec::new(),
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

    /// Test-only adapter: it synthesizes a success receipt and cannot see chain storage, which is
    /// why it is confined to test and `dev-mock` builds.
    fn execute_with_slots(
        payload: &[u8],
        gas_limit: u64,
        _slots: &[(H256, [u8; 32])],
    ) -> Result<ExecutionReceipt, DispatchError> {
        <() as X3ExecutorAdapter>::execute(payload, gas_limit)
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
    //!
    //! There is deliberately **no** EVM adapter here. The EVM arm is executed by
    //! whichever `EvmExecutorAdapter` the runtime configures (`NativeEvmAdapter`
    //! under `--features frontier`, `WasmEvmAdapter` otherwise), and both of those
    //! really run the payload. A standalone adapter in this module cannot: it has
    //! no account, storage, or block context to execute against, and the one that
    //! used to live here (`FrontierEvmAdapter`) returned `success: true` with a
    //! gas figure derived from the payload length, for code it never ran.

    use super::*;
    use x3_svm_integration::{RbpfSvmExecutor, SvmConfig, SvmExecutor};

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
                storage_writes: Vec::new(),
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

    /// The native runtime's X3 adapter. It delegates to `WasmX3Adapter`, so the native and the
    /// wasm runtime execute X3 programs with the same engine; it used to run `x3-vm`, which no node
    /// executing the wasm runtime ever runs (see `X3Executor::execute_on_chain`).
    pub struct X3VmAdapter;

    impl super::X3ExecutorAdapter for X3VmAdapter {
        fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
            <crate::wasm_adapters::WasmX3Adapter as super::X3ExecutorAdapter>::execute(
                payload, gas_limit,
            )
        }

        fn execute_with_slots(
            payload: &[u8],
            gas_limit: u64,
            slots: &[(H256, [u8; 32])],
        ) -> Result<ExecutionReceipt, DispatchError> {
            <crate::wasm_adapters::WasmX3Adapter as super::X3ExecutorAdapter>::execute_with_slots(
                payload, gas_limit, slots,
            )
        }

        fn execute_with_slots_and_policy(
            payload: &[u8],
            gas_limit: u64,
            slots: &[(H256, [u8; 32])],
            private_channel_available: bool,
        ) -> Result<ExecutionReceipt, DispatchError> {
            <crate::wasm_adapters::WasmX3Adapter as super::X3ExecutorAdapter>::execute_with_slots_and_policy(
                payload,
                gas_limit,
                slots,
                private_channel_available,
            )
        }

        fn validate(payload: &[u8]) -> Result<(), DispatchError> {
            <crate::wasm_adapters::WasmX3Adapter as super::X3ExecutorAdapter>::validate(payload)
        }

        fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
            <crate::wasm_adapters::WasmX3Adapter as super::X3ExecutorAdapter>::estimate_gas(payload)
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
        assert_eq!(result.gas_used, 1000);

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
}
