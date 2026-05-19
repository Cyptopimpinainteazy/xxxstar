//! WASM/no-std VM adapters backed by inline interpreters.
//!
//! `WasmEvmAdapter`  — uses `x3_evm_integration::mini_evm`
//! `WasmSvmAdapter`  — uses `x3_svm_integration::interp_execute_bpf`
//! `WasmX3Adapter`   — uses `x3_x3_integration::X3Executor` (mini_x3 in no-std)
//!
//! These are available in both std and no-std builds; the underlying
//! integration crates choose their fast (native) or mini (WASM) path
//! based on their own feature flags.

use crate::adapters::{EvmExecutorAdapter, SvmExecutorAdapter, X3ExecutorAdapter};
use crate::ExecutionReceipt;
use frame_support::pallet_prelude::DispatchError;
#[allow(unused_imports)]
use parity_scale_codec::Encode as _;
use sp_std::vec::Vec;

// ---------------------------------------------------------------------------
// WasmEvmAdapter
// ---------------------------------------------------------------------------

/// EVM adapter powered by `mini_evm` — runs the SputnikVM interpreter.
pub struct WasmEvmAdapter;

impl EvmExecutorAdapter for WasmEvmAdapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }
        let evm_config = x3_evm_integration::EvmConfig {
            gas_limit,
            ..x3_evm_integration::EvmConfig::default()
        };
        x3_evm_integration::mini_evm::execute_evm(
            payload,
            sp_core::H160::zero(), // caller — pallet fills in from origin
            sp_core::U256::zero(), // value — pallet fills in from extrinsic
            &evm_config,
        )
        .map(|res| ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: res.success,
            gas_used: res.gas_used,
            return_data: res.output,
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
        .map_err(|_| DispatchError::Other("EVM execution failed"))
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }
        x3_evm_integration::mini_evm::estimate_gas_evm(payload)
            .map_err(|_| DispatchError::Other("EVM gas estimation failed"))
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty EVM payload"));
        }
        x3_evm_integration::mini_evm::validate_evm(payload)
            .map_err(|_| DispatchError::Other("EVM validation failed"))
    }
}

// ---------------------------------------------------------------------------
// WasmSvmAdapter
// ---------------------------------------------------------------------------

/// SVM adapter powered by the inline eBPF interpreter (`interp.rs`).
pub struct WasmSvmAdapter;

impl SvmExecutorAdapter for WasmSvmAdapter {
    fn execute(payload: &[u8], compute_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM payload"));
        }
        let config = x3_svm_integration::SvmConfig {
            compute_unit_limit: compute_limit,
            compute_unit_price: 1,
            slot: 0,
            block_timestamp: 0,
            recent_blockhash: [0u8; 32],
            enable_cpi: false,
            max_cpi_depth: 0,
        };
        x3_svm_integration::interp_execute_bpf(payload, &[], &config)
            .map(|res| {
                let state_changes: Vec<crate::StateChange> = res
                    .account_updates
                    .iter()
                    .map(|update| {
                        // Persist account data via the pallet storage key so
                        // stateful SVM programs retain state across calls.
                        // We compute the raw storage key for SvmAccountData to
                        // avoid importing the pallet type in this no_std crate.
                        if !update.data.is_empty() {
                            // Key layout: twox128("X3Kernel") || twox128("SvmAccountData")
                            //             || blake2_128(pubkey) || pubkey
                            let pallet_hash = sp_io::hashing::twox_128(b"X3Kernel");
                            let item_hash = sp_io::hashing::twox_128(b"SvmAccountData");
                            let key_hash = sp_io::hashing::blake2_128(&update.pubkey);
                            let mut full_key = Vec::with_capacity(80);
                            full_key.extend_from_slice(&pallet_hash);
                            full_key.extend_from_slice(&item_hash);
                            full_key.extend_from_slice(&key_hash);
                            full_key.extend_from_slice(&update.pubkey);
                            // SCALE-encode BoundedVec<u8>: compact length prefix + bytes
                            let mut encoded = Vec::with_capacity(update.data.len() + 4);
                            parity_scale_codec::Encode::encode_to(&update.data, &mut encoded);
                            sp_io::storage::set(&full_key, &encoded);
                        }
                        // Map lamport balance change to a canonical StateChange.
                        let asset_key = {
                            let out = [0u8; 32];
                            // CANONICAL_NATIVE_ASSET_ID = 0u128 LE
                            sp_core::H256::from(out)
                        };
                        let balance_value = {
                            let mut out = [0u8; 32];
                            out[..8].copy_from_slice(&update.lamports.to_le_bytes());
                            sp_core::H256::from(out)
                        };
                        crate::StateChange {
                            address: update.pubkey.to_vec(),
                            key: asset_key,
                            value: balance_value,
                        }
                    })
                    .collect();
                ExecutionReceipt {
                    version: crate::EXECUTION_RECEIPT_VERSION,
                    success: res.success,
                    gas_used: res.compute_units_used,
                    return_data: res.output,
                    logs: Vec::new(),
                    state_changes,
                    protocol_version: 1,
                    migration_history: Vec::new(),
                    compatibility_flags: 0,
                    from: Vec::new(),
                    to: Vec::new(),
                    value: 0,
                }
            })
            .map_err(|_| DispatchError::Other("SVM execution failed"))
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty SVM payload"));
        }
        x3_svm_integration::interp_validate_program(payload)
            .map_err(|_| DispatchError::Other("SVM validation failed"))
    }
}

// ---------------------------------------------------------------------------
// WasmX3Adapter
// ---------------------------------------------------------------------------

/// X3 adapter backed by `mini_x3` in no-std and `x3-vm` in std.
pub struct WasmX3Adapter;

impl X3ExecutorAdapter for WasmX3Adapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        let config = x3_x3_integration::X3ExecutorConfig {
            gas_limit,
            ..Default::default()
        };
        x3_x3_integration::X3Executor::execute(payload, &[], config)
            .map(|rec| ExecutionReceipt {
                version: crate::EXECUTION_RECEIPT_VERSION,
                success: rec.success,
                gas_used: rec.gas_used,
                return_data: rec.return_data,
                logs: Vec::new(),
                state_changes: Vec::new(),
                protocol_version: 1,
                migration_history: Vec::new(),
                compatibility_flags: 0,
                from: Vec::new(),
                to: Vec::new(),
                value: 0,
            })
            .map_err(|_| DispatchError::Other("X3 execution failed"))
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        x3_x3_integration::X3Executor::verify(payload, false)
            .map_err(|_| DispatchError::Other("X3 validation failed"))
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        x3_x3_integration::X3Executor::estimate_gas(payload)
            .map_err(|_| DispatchError::Other("X3 gas estimation failed"))
    }
}
