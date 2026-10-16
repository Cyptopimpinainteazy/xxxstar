//! WASM/no-std VM adapters backed by inline interpreters.
//!
//! `WasmEvmAdapter`  — uses `x3_evm_integration::mini_evm`
//! `WasmSvmAdapter`  — uses `x3_svm_integration::interp_execute_bpf`
//! `WasmX3Adapter`   — uses `x3_x3_integration::X3Executor::execute_on_chain` (`x3-vm`, every build)
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

impl WasmEvmAdapter {
    fn execute_with_config(
        payload: &[u8],
        evm_config: &x3_evm_integration::EvmConfig,
        caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
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
        x3_evm_integration::mini_evm::execute_evm_persistent(
            payload,
            caller,
            sp_core::U256::zero(), // No implicit native-ledger value transfer
            evm_config,
        )
        .map(|res| ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: res.success,
            gas_used: res.gas_used,
            return_data: res.output,
            logs: res
                .logs
                .into_iter()
                .map(|log| crate::ExecutionLog {
                    address: log.address.as_bytes().to_vec(),
                    topics: log.topics,
                    data: log.data,
                    block_number: evm_config.block_number,
                })
                .collect(),
            state_changes: Vec::new(),
            storage_writes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: caller.as_bytes().to_vec(),
            to: Vec::new(),
            value: 0,
        })
        .map_err(|_| DispatchError::Other("EVM execution failed"))
    }
}

impl EvmExecutorAdapter for WasmEvmAdapter {
    fn execute(payload: &[u8], _gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Self::validate(payload)?;
        Err(DispatchError::Other(
            "EVM execution requires an authenticated caller",
        ))
    }
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        Self::execute_with_config(
            payload,
            &x3_evm_integration::EvmConfig {
                gas_limit,
                ..Default::default()
            },
            caller,
        )
    }
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        Self::validate(payload)?;
        Err(DispatchError::Other(
            "EVM gas estimation requires an authenticated caller",
        ))
    }
    fn estimate_gas_from(payload: &[u8], caller: sp_core::H160) -> Result<u64, DispatchError> {
        Self::validate(payload)?;
        x3_evm_integration::mini_evm::estimate_gas_evm_persistent(
            payload,
            caller,
            sp_core::U256::zero(),
            &Default::default(),
        )
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

/// Production context for the same content-addressed EVM adapter.
pub struct ContextualWasmEvmAdapter<T>(core::marker::PhantomData<T>);
impl<T: crate::Config> ContextualWasmEvmAdapter<T> {
    fn config(gas_limit: u64) -> x3_evm_integration::EvmConfig {
        use frame_support::traits::UnixTime;
        use sp_runtime::traits::SaturatedConversion;
        x3_evm_integration::EvmConfig {
            gas_limit,
            block_number: frame_system::Pallet::<T>::block_number().saturated_into(),
            block_timestamp: <pallet_timestamp::Pallet<T> as UnixTime>::now().as_secs(),
            ..Default::default()
        }
    }
}
impl<T: crate::Config> EvmExecutorAdapter for ContextualWasmEvmAdapter<T> {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        WasmEvmAdapter::execute(payload, gas_limit)
    }
    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        WasmEvmAdapter::execute_with_config(payload, &Self::config(gas_limit), caller)
    }
    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        WasmEvmAdapter::estimate_gas(payload)
    }
    fn estimate_gas_from(payload: &[u8], caller: sp_core::H160) -> Result<u64, DispatchError> {
        Self::validate(payload)?;
        use frame_support::traits::Get;
        x3_evm_integration::mini_evm::estimate_gas_evm_persistent(
            payload,
            caller,
            sp_core::U256::zero(),
            &Self::config(T::DefaultEvmGasLimit::get()),
        )
        .map_err(|_| DispatchError::Other("EVM gas estimation failed"))
    }
    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        WasmEvmAdapter::validate(payload)
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
                    storage_writes: Vec::new(),
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

/// The runtime's X3 adapter: `X3Executor`'s on-chain engine, the same code in every build.
///
/// `execute_on_chain` runs `x3-vm` in the wasm runtime and the native one alike, so blocks, the
/// native runtime's tests and benchmarks, and every off-chain simulation execute the same engine.
/// (It used to run `mini_x3`, a separate interpreter, because `x3-vm` needed `std`.)
/// `X3VmAdapter`, the native runtime's adapter, delegates here as well.
pub struct WasmX3Adapter;

impl X3ExecutorAdapter for WasmX3Adapter {
    fn execute(payload: &[u8], gas_limit: u64) -> Result<ExecutionReceipt, DispatchError> {
        Self::execute_with_slots(payload, gas_limit, &[])
    }

    fn execute_with_slots(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(sp_core::H256, [u8; 32])],
    ) -> Result<ExecutionReceipt, DispatchError> {
        // No posture supplied: the fail-closed default. A program whose compiled policy demands
        // private submission is refused by the engine rather than executed in the clear.
        Self::execute_with_slots_and_policy(payload, gas_limit, slots, false)
    }

    fn execute_with_slots_and_policy(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(sp_core::H256, [u8; 32])],
        private_channel_available: bool,
    ) -> Result<ExecutionReceipt, DispatchError> {
        // No context supplied: every context opcode refuses (fail closed).
        Self::execute_with_context(
            payload,
            gas_limit,
            slots,
            private_channel_available,
            &x3_x3_integration::X3BlockContext::unknown(),
        )
    }

    fn execute_with_context(
        payload: &[u8],
        gas_limit: u64,
        slots: &[(sp_core::H256, [u8; 32])],
        private_channel_available: bool,
        context: &x3_x3_integration::X3BlockContext,
    ) -> Result<ExecutionReceipt, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        // The chain's slots in the executor's own shape. Loading them is what makes a program able
        // to read a slot an earlier comit persisted; without them every `evm_sload` on a block
        // answered EVM's zero.
        let seeds: Vec<([u8; 32], [u8; 32])> = slots
            .iter()
            .map(|(key, value)| (key.to_fixed_bytes(), *value))
            .collect();
        // The engine refuses an artifact that demands private submission unless the pallet, which
        // holds `Config::PrivateSubmissionChannel`, says a private channel exists.
        x3_x3_integration::X3Executor::execute_on_chain_with_context(
            payload,
            gas_limit,
            &seeds,
            private_channel_available,
            context,
        )
        .map(|rec| ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: rec.success,
            gas_used: rec.gas_used,
            return_data: rec.return_data,
            logs: Vec::new(),
            state_changes: Vec::new(),
            // Carry the executor's slot channel through unchanged. The engine journals the slot
            // writes it made, and they must not be dropped on the way to the pallet or a
            // contract's state would silently vanish between execution and the receipt the
            // chain stores.
            storage_writes: rec
                .storage_writes
                .into_iter()
                .map(|write| crate::StorageWrite {
                    key: write.key,
                    old_value: write.old_value,
                    new_value: write.new_value,
                })
                .collect(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
        .map_err(|error| {
            DispatchError::Other(match error {
                x3_x3_integration::X3IntegrationError::VerificationFailed(_) => {
                    "X3 verification failed"
                }
                _ => "X3 execution failed",
            })
        })
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        x3_x3_integration::X3Executor::verify_on_chain(payload)
            .map_err(|_| DispatchError::Other("Invalid X3 bytecode"))
    }

    fn estimate_gas(payload: &[u8]) -> Result<u64, DispatchError> {
        if payload.is_empty() {
            return Err(DispatchError::Other("Empty X3 payload"));
        }
        Ok(x3_x3_integration::X3Executor::estimate_gas_on_chain(
            payload,
        ))
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
        let receipt = sp_io::TestExternalities::default().execute_with(|| {
            WasmEvmAdapter::execute_from(&code, 6_000_000, sp_core::H160::zero())
                .expect("bytecode has to execute")
        });
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

    /// The production adapter retains a slot across independent interpreter instances.
    #[test]
    fn the_chains_evm_keeps_state_between_executions() {
        sp_io::TestExternalities::default().execute_with(|| {
            // Increment slot zero, then return it. Identical code means the same
            // content-addressed contract on both calls.
            let code = [
                0x60, 0, 0x54, 0x60, 1, 0x01, 0x80, 0x60, 0, 0x55, 0x60, 0, 0x52, 0x60, 32, 0x60,
                0, 0xf3,
            ];
            let root_before = sp_io::storage::root(sp_runtime::StateVersion::V1);
            let refused: Result<(), DispatchError> =
                frame_support::storage::with_storage_layer(|| {
                    let receipt =
                        WasmEvmAdapter::execute_from(&code, 100_000, sp_core::H160::zero())?;
                    assert_eq!(receipt.return_data[31], 1);
                    // Model the next VM failing after successful EVM execution.
                    Err(DispatchError::Other("second VM refused"))
                });
            assert!(refused.is_err());
            assert_eq!(
                root_before,
                sp_io::storage::root(sp_runtime::StateVersion::V1)
            );
            let estimated_first = WasmEvmAdapter::estimate_gas_from(&code, sp_core::H160::zero())
                .expect("simulate first call");
            assert_eq!(
                root_before,
                sp_io::storage::root(sp_runtime::StateVersion::V1)
            );
            let first = WasmEvmAdapter::execute_from(&code, 100_000, sp_core::H160::zero())
                .expect("first call");
            assert_eq!(estimated_first, first.gas_used);
            let root_after_first = sp_io::storage::root(sp_runtime::StateVersion::V1);
            let estimated_second = WasmEvmAdapter::estimate_gas_from(&code, sp_core::H160::zero())
                .expect("simulate second call");
            assert_eq!(
                root_after_first,
                sp_io::storage::root(sp_runtime::StateVersion::V1)
            );
            let second = WasmEvmAdapter::execute_from(&code, 100_000, sp_core::H160::zero())
                .expect("second call");
            assert_eq!(estimated_second, second.gas_used);
            assert_eq!(first.return_data[31], 1);
            assert_eq!(second.return_data[31], 2);
            assert!(
                second.gas_used < first.gas_used,
                "SSTORE uses the persisted original value"
            );
        });
    }
    #[test]
    fn runtime_context_uses_chain_block_number_and_timestamp() {
        crate::mock::new_test_ext().execute_with(|| {
            crate::mock::System::set_block_number(123);
            pallet_timestamp::Pallet::<crate::mock::Test>::set_timestamp(456_000);
            let code = [
                0x43, 0x60, 0, 0x52, 0x42, 0x60, 32, 0x52, 0x60, 64, 0x60, 0, 0xf3,
            ];
            let receipt = ContextualWasmEvmAdapter::<crate::mock::Test>::execute_from(
                &code,
                100_000,
                sp_core::H160::zero(),
            )
            .unwrap();
            assert_eq!(
                sp_core::U256::from_big_endian(&receipt.return_data[..32]),
                sp_core::U256::from(123)
            );
            assert_eq!(
                sp_core::U256::from_big_endian(&receipt.return_data[32..]),
                sp_core::U256::from(456)
            );
        });
    }
}
