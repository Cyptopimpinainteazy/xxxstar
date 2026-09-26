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
        // The kernel hands this adapter SCALE-encoded packets, and a packet is not EVM code: its first
        // byte is the enum discriminant `0x00`, which is `STOP`, so running it halts immediately and
        // reports success. Refusing is the only honest answer until the payload convention is settled.
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "EVM payload is a SCALE-encoded Packet, not bytecode: this adapter cannot execute it",
            ));
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
        // X3-LANG-004: the kernel now asks *this* component whether a payload is valid, so the
        // refusal has to live here as well as in `execute`. `validate_evm` on its own would accept
        // a packet — it only checks non-empty, EIP-170 size and the `0xEF` prefix — and the kernel
        // would then take a packet as validated bytecode.
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "EVM payload is a SCALE-encoded Packet, not bytecode: this adapter cannot execute it",
            ));
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
        // Same reason as the EVM arm above: a packet is not eBPF, so it cannot be executed here.
        // Today this path fails anyway (`SVM execution failed`); refusing by name makes the reason
        // the payload convention rather than a decoding accident.
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "SVM payload is a SCALE-encoded Packet, not a program: this adapter cannot execute it",
            ));
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
        // Same as the EVM arm above: `interp_validate_program` happens to reject the 124-byte packet
        // as malformed, but the reason has to be named rather than left to a length accident.
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "SVM payload is a SCALE-encoded Packet, not a program: this adapter cannot execute it",
            ));
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

#[cfg(test)]
mod an_accepted_evm_payload_is_refused {
    use super::*;
    use crate::adapters::EvmExecutorAdapter;

    /// An accepted EVM payload must be executed or refused — never reported as a success.
    ///
    /// The regression test for the false success found on 2026-09-26. The kernel validates a
    /// non-empty EVM payload by decoding it as a `Packet` and checking the EVM domain bit, while this
    /// adapter — the one a live chain is compiled with — used to hand those same bytes to
    /// `mini_evm::execute_evm` as EVM code. A SCALE-encoded `Packet::Evm(..)` begins with its enum
    /// discriminant `0x00`, which is `STOP`, so the interpreter halted on byte 0, charged base gas
    /// and reported success, and the kernel persisted a receipt for an operation that never ran.
    ///
    /// ```text
    /// (before the fix) payload 124 bytes, head [00, 00, 6b, cb, 44, 6f, 34, 8c]
    ///                  execute -> Ok((success: true, gas_used: 22576))
    ///                  validate -> Ok(())
    /// ```
    ///
    /// The adapter refuses a packet by name now, and the payload convention is settled the other
    /// way (X3-LANG-004, 2026-09-26): a payload **is** the artifact the adapter executes, so the
    /// fixture here is an explicit packet — the shape the kernel used to accept — while the tests
    /// below run the bytecode an EVM payload is supposed to be. Reports:
    /// `.ai/reports/evm-payload-never-executed-20260926.md`,
    /// `.ai/reports/evm-svm-payload-convention-20260926.md`.
    #[test]
    fn an_accepted_evm_payload_is_refused_rather_than_reported_as_success() {
        use x3_packet_schema::{EvmPacket, Packet};
        let payload = Packet::Evm(EvmPacket::Call {
            contract: [0x11u8; 20],
            function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
            args: vec![0xAAu8; 64],
            value: x3_packet_schema::U256::from(0),
        })
        .encode();
        assert!(
            !payload.is_empty(),
            "the fixture has to be a payload the kernel accepts"
        );
        assert!(
            crate::packet_adapters::deserialize_packet(&payload).is_ok(),
            "the fixture has to be a packet, which is what the old validation accepted"
        );
        assert_eq!(
            payload.first().copied(),
            Some(0x00),
            "SCALE puts the enum discriminant first, and `0x00` is EVM STOP"
        );

        match WasmEvmAdapter::execute(&payload, 6_000_000) {
            Err(DispatchError::Other(msg)) => assert!(
                msg.contains("Packet, not bytecode"),
                "the refusal has to name the reason, got: {msg}"
            ),
            Err(other) => panic!("expected the named refusal, got: {other:?}"),
            Ok(receipt) => panic!(
                "the adapter must not report success for a payload it cannot execute: \
                 success={}, gas_used={}",
                receipt.success, receipt.gas_used
            ),
        }
        // `validate` is the gate the kernel actually calls now, so it has to refuse by the same
        // name — otherwise a packet would pass validation and only fail later, at execution.
        match WasmEvmAdapter::validate(&payload) {
            Err(DispatchError::Other(msg)) => assert!(
                msg.contains("Packet, not bytecode"),
                "`validate` has to name the reason, got: {msg}"
            ),
            other => panic!("`validate` has to refuse the packet, got: {other:?}"),
        }
    }

    /// The SVM arm, same boundary: a packet is not a program, and the refusal says so.
    #[test]
    fn an_accepted_svm_payload_is_refused_by_name() {
        use x3_packet_schema::{Packet, SvmPacket};
        let payload = Packet::Svm(SvmPacket::Invoke {
            program_id: [0x22u8; 32],
            accounts: Vec::new(),
            data: vec![0xBBu8; 64],
        })
        .encode();
        assert!(crate::packet_adapters::deserialize_packet(&payload).is_ok());
        match WasmSvmAdapter::execute(&payload, 500_000) {
            Err(DispatchError::Other(msg)) => assert!(
                msg.contains("Packet, not a program"),
                "the refusal has to name the reason, got: {msg}"
            ),
            Err(other) => panic!("expected the named refusal, got: {other:?}"),
            Ok(receipt) => panic!(
                "the adapter must not report success for a payload it cannot execute: \
                 success={}, compute_units_used={}",
                receipt.success, receipt.gas_used
            ),
        }
        match WasmSvmAdapter::validate(&payload) {
            Err(DispatchError::Other(msg)) => assert!(
                msg.contains("Packet, not a program"),
                "`validate` has to name the reason, got: {msg}"
            ),
            other => panic!("`validate` has to refuse the packet, got: {other:?}"),
        }
    }

    /// The other direction, measured in the same run: the artifact an EVM payload is supposed to
    /// be — bytecode — is executed. Without this, the refusals above could be satisfied by an
    /// adapter that refuses everything.
    #[test]
    fn evm_bytecode_that_is_not_a_packet_executes() {
        let code = crate::test_helpers::wrap_evm_payload(&[0xAAu8; 64]);
        assert!(
            !crate::packet_adapters::payload_is_packet(&code),
            "the fixture has to be bytecode, not a packet"
        );
        let receipt = WasmEvmAdapter::execute(&code, 6_000_000).expect("bytecode has to execute");
        assert!(
            receipt.success,
            "`PUSH1 0; PUSH1 0; RETURN` halts successfully"
        );
        assert!(receipt.gas_used > 0, "execution has to charge for the work");
    }

    /// The SVM arm of the same direction: an eBPF program runs.
    #[test]
    fn svm_program_that_is_not_a_packet_executes() {
        let program = crate::test_helpers::wrap_svm_payload(&[0xBBu8; 8]);
        assert!(
            !crate::packet_adapters::payload_is_packet(&program),
            "the fixture has to be a program, not a packet"
        );
        let receipt =
            <WasmSvmAdapter as crate::adapters::SvmExecutorAdapter>::execute(&program, 500_000)
                .expect("an eBPF program has to execute");
        assert!(
            receipt.success,
            "`MOV64_IMM r0, …; EXIT` returns successfully"
        );
    }
}
