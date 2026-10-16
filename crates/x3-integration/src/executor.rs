//! X3 VM Executor for X3 Kernel
//!
//! This module provides the main executor that bridges the X3 VM with
//! the X3 Kernel pallet's execution interface.

#[cfg(not(feature = "std"))]
use alloc::{format, vec};

use sp_core::H256;

use crate::error::{X3IntegrationError, X3Result};
use crate::types::{X3ExecutionLog, X3StorageWrite};
use crate::types::{X3ExecutionReceipt, X3GasConfig, X3Value};

#[cfg(feature = "std")]
use x3_vm::{BytecodeModule, VMConfig, Verifier, VerifyOptions, VM};

/// Call depth the chain allows one execution (the figure the runtime has always enforced).
const ON_CHAIN_MAX_CALL_DEPTH: usize = 64;
/// Operand stack bound the chain allows one execution.
const ON_CHAIN_MAX_STACK_SIZE: usize = 1024;
/// Fixed part of the pre-execution gas estimate.
const ON_CHAIN_BASE_GAS: u64 = 21_000;
/// Per-byte part of the pre-execution gas estimate.
const ON_CHAIN_GAS_PER_BYTE: u64 = 10;

/// Configuration for X3 executor
#[derive(Clone, Debug)]
pub struct X3ExecutorConfig {
    /// Maximum gas allowed for execution
    pub gas_limit: u64,
    /// Maximum call stack depth
    pub max_call_depth: usize,
    /// Maximum operand stack size
    pub max_stack_size: usize,
    /// Enable execution tracing (debug only)
    pub trace: bool,
    /// Gas cost configuration
    pub gas_config: X3GasConfig,
    /// Allow debug opcodes (NEVER for on-chain)
    pub allow_debug_ops: bool,
    /// Whether this execution context can offer a private submission channel.
    ///
    /// An artifact may *demand* private submission in its own header (the compiled policy,
    /// AGENTS.md §11). This is the context's half of the contract: when it is `false`, a module
    /// carrying the demand is refused by the engine rather than executed in the clear. It defaults
    /// to `false` everywhere, including `simulation()`, because "nobody told me whether a private
    /// channel exists" must resolve to the refusal and not to the permissive answer.
    pub allow_private_submission: bool,
}

impl Default for X3ExecutorConfig {
    fn default() -> Self {
        Self {
            gas_limit: 1_000_000,
            max_call_depth: 64,
            max_stack_size: 1024,
            trace: false,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: false,
            allow_private_submission: false,
        }
    }
}

impl X3ExecutorConfig {
    /// Create config for on-chain execution (strict)
    pub fn on_chain() -> Self {
        Self {
            gas_limit: 500_000,
            max_call_depth: 32,
            max_stack_size: 512,
            trace: false,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: false,
            allow_private_submission: false,
        }
    }

    /// Create config for off-chain simulation (permissive)
    pub fn simulation() -> Self {
        Self {
            gas_limit: 10_000_000,
            max_call_depth: 128,
            max_stack_size: 4096,
            trace: true,
            gas_config: X3GasConfig::default(),
            allow_debug_ops: true,
            allow_private_submission: false,
        }
    }

    /// Set gas limit
    pub fn with_gas_limit(mut self, limit: u64) -> Self {
        self.gas_limit = limit;
        self
    }

    /// Declare that this context really does offer a private submission channel.
    ///
    /// The runtime's equivalent is `pallet_private_execution::Enabled`; a caller that sets this
    /// without such a channel is asserting something the chain does not have, which is why nothing
    /// sets it by default.
    pub fn with_private_submission_available(mut self) -> Self {
        self.allow_private_submission = true;
        self
    }
}

/// Main X3 executor
pub struct X3Executor;

impl X3Executor {
    /// Execute X3 bytecode and return execution receipt
    ///
    /// This is the main entry point for on-chain X3 execution:
    /// 1. Verify bytecode (mandatory)
    /// 2. Create VM instance
    /// 3. Execute function
    /// 4. Collect state changes and logs
    /// 5. Return receipt
    #[cfg(feature = "std")]
    pub fn execute(
        bytecode: &[u8],
        args: &[X3Value],
        config: X3ExecutorConfig,
    ) -> X3Result<X3ExecutionReceipt> {
        Self::execute_with_slots(bytecode, args, config, &[])
    }

    /// Verify a standalone artifact's detached attestation, then execute it.
    ///
    /// For an artifact that arrived **out of band** — a `.x3b` from a release page, a file copied
    /// between machines — rather than as the argument of a signed extrinsic. On the chain path the
    /// transaction's own signature already covers the program's bytes, so this adds nothing there;
    /// off it, nothing did (X3-LANG-009). The envelope's checksum is a corruption check anyone can
    /// recompute over a forged body, so a loader that trusts it is trusting arithmetic, not a key.
    ///
    /// The attestation is checked **before** the bytes reach the verifier or the engine: an
    /// artifact nobody the caller trusts has vouched for is not parsed, not verified and not run.
    #[cfg(feature = "std")]
    pub fn execute_attested(
        bytecode: &[u8],
        attestation: &x3_common::artifact::ArtifactAttestation,
        registry: &x3_common::artifact::ArtifactKeyRegistry,
        args: &[X3Value],
        config: X3ExecutorConfig,
    ) -> X3Result<X3ExecutionReceipt> {
        attestation
            .verify(bytecode, registry)
            .map_err(|refusal| X3IntegrationError::UnattestedArtifact(format!("{refusal}")))?;
        Self::execute(bytecode, args, config)
    }

    /// Execute X3 bytecode with the chain's contract slots visible to the program.
    ///
    /// `seeds` is the chain's `X3ContractStorage`: the 32-byte slot keys and the tagged payloads a
    /// previous execution persisted. They are loaded as inherited state (the VM does not journal
    /// them and the receipt does not report them), which is what makes a *second* comit able to
    /// read what the first one wrote. Before this existed the executor was handed no prior state, so
    /// `evm_sload` answered EVM's zero for every slot and contract state was write-only.
    #[cfg(feature = "std")]
    pub fn execute_with_slots(
        bytecode: &[u8],
        args: &[X3Value],
        config: X3ExecutorConfig,
        seeds: &[([u8; 32], [u8; 32])],
    ) -> X3Result<X3ExecutionReceipt> {
        // Step 1: Verify bytecode
        let verify_opts = if config.allow_debug_ops {
            VerifyOptions::default()
        } else {
            VerifyOptions::on_chain()
        };
        Verifier::verify_module_bytes(bytecode, &verify_opts)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;

        // Step 2: Create the VM with the limits this call asked for.
        //
        // `VM::from_bytes` builds the VM with `VMConfig::default()`, so every field of
        // `X3ExecutorConfig` was thrown away here: `on_chain()` (500_000 gas) and
        // `simulation()` (10_000_000) both ran under the VM's own 1_000_000, and a caller
        // that asked for *less* got more. The gas limit is the bound the caller is charged
        // against and the one the pallet reports, so it has to be the bound the VM stops at.
        // Measured before this fix: a 100-gas limit admitted a 2_000-instruction program and
        // returned `success = true, gas_used = 2002`
        // (`crates/x3-integration/tests/gas_accounting.rs`).
        let module = BytecodeModule::from_bytes(bytecode)
            .map_err(|e| X3IntegrationError::InvalidBytecode(format!("{:?}", e)))?;
        // The compiled policy, before anything runs: the artifact says what it requires and this
        // configuration says what the context can offer. Between the two, the demand is honoured —
        // a program that asked to be submitted privately must not be executed in the clear, and the
        // std engine has to agree with `mini_x3` about that or the same bytes would be refused on a
        // block and accepted off it.
        if module
            .features
            .has(x3_backend::bc_format::FeatureFlags::PRIVATE_SUBMISSION_REQUIRED)
            && !config.allow_private_submission
        {
            return Err(X3IntegrationError::ExecutionFailed(
                "private submission required by the compiled policy, but this context offers no \
                 private channel"
                    .into(),
            ));
        }
        let mut vm = VM::with_config_and_seeds(
            module,
            VMConfig {
                gas_limit: config.gas_limit,
                max_call_depth: config.max_call_depth,
                max_stack_size: config.max_stack_size,
                trace: config.trace,
            },
            seeds,
        )
        .map_err(|e| X3IntegrationError::ExecutionFailed(format!("{:?}", e)))?;

        // Step 3: Convert arguments to VM values
        let vm_args: Vec<x3_vm::Value> = args
            .iter()
            .map(|arg| match arg {
                X3Value::I64(v) => x3_vm::Value::I64(*v),
                X3Value::F64Bits(bits) => x3_vm::Value::F64(f64::from_bits(*bits)),
                X3Value::Bool(v) => x3_vm::Value::Bool(*v),
                X3Value::Bytes(v) => x3_vm::Value::Bytes(v.clone()),
                X3Value::Address(v) => x3_vm::Value::Bytes(v.clone()),
                X3Value::Unit => x3_vm::Value::Unit,
            })
            .collect();

        // Step 4: Execute (function 0 = main/entry)
        let result = vm.call_function(0, &vm_args);

        // Step 5: Build receipt
        let gas_used = vm.gas_used();

        match result {
            Ok(exec_result) => {
                let return_data = match exec_result.value {
                    Some(x3_vm::Value::I64(v)) => v.to_le_bytes().to_vec(),
                    Some(x3_vm::Value::F64(v)) => v.to_bits().to_le_bytes().to_vec(),
                    Some(x3_vm::Value::Bool(v)) => vec![v as u8],
                    Some(x3_vm::Value::Bytes(v)) => v,
                    Some(x3_vm::Value::String(s)) => s.into_bytes(),
                    Some(x3_vm::Value::Unit) => vec![],
                    Some(x3_vm::Value::Addr(a)) => a.to_le_bytes().to_vec(),
                    None => vec![],
                };

                // Drain the slot journal into the receipt's storage channel.
                //
                // This is the only place an X3VM slot write becomes visible to the chain: the VM's
                // `VmStorage` is in-memory and is dropped when this function returns. A write that
                // an atomic window rolled back is already gone from the journal (`VmStorage::rollback`
                // truncates it to the snapshot's length), so a reverted window reports no change
                // rather than a half-applied one.
                let storage_writes = vm
                    .drain_storage_journal()
                    .into_iter()
                    .map(|write| X3StorageWrite {
                        key: H256::from(write.key),
                        old_value: write.old_value,
                        new_value: write.new_value,
                    })
                    .collect();

                // The events the program emitted, in order. This field was `vec![]` with a note
                // saying collection was "deferred to runtime integration" — the buffer existed,
                // nothing filled it, and `emit` was refused at lowering, so a program whose
                // receipt is supposed to carry an event produced a receipt without one.
                // `drain_events` returns only the committed ones, so a rolled-back atomic
                // window's events are not here, exactly as its slot writes are not.
                let logs = vm
                    .drain_events()
                    .into_iter()
                    .map(|event| X3ExecutionLog {
                        topic: event
                            .topics
                            .first()
                            .map(|topic| H256::from(*topic))
                            .unwrap_or_default(),
                        data: event.data,
                    })
                    .collect();

                Ok(X3ExecutionReceipt {
                    success: true,
                    gas_used,
                    return_data,
                    logs,
                    state_changes: vec![], // Hostcall state change collection deferred to runtime integration
                    storage_writes,
                    function_index: 0,
                    // The VM counts these; the field used to say counting needed instrumentation it
                    // already had, so every receipt reported zero instructions (TICKET-130).
                    instructions_executed: vm.instruction_count(),
                })
            }
            Err(vm_err) => {
                // Check for gas exhaustion
                if gas_used >= config.gas_limit {
                    return Err(X3IntegrationError::GasExhausted {
                        used: gas_used,
                        limit: config.gas_limit,
                    });
                }

                Ok(X3ExecutionReceipt {
                    success: false,
                    gas_used,
                    return_data: format!("{:?}", vm_err).into_bytes(),
                    logs: vec![],
                    state_changes: vec![],
                    // Fail closed: a failed execution's partial writes must never be applied, so a
                    // failure reports no slot writes even though the VM journaled some before it
                    // failed. `journal` is deliberately not drained here.
                    storage_writes: vec![],
                    function_index: 0,
                    // The VM counts these; the field used to say counting needed instrumentation it
                    // already had, so every receipt reported zero instructions (TICKET-130).
                    instructions_executed: vm.instruction_count(),
                })
            }
        }
    }

    /// Execute X3BC bytecode (no_std: the chain's engine, see `execute_on_chain`)
    #[cfg(not(feature = "std"))]
    pub fn execute(
        bytecode: &[u8],
        _args: &[X3Value],
        config: X3ExecutorConfig,
    ) -> X3Result<X3ExecutionReceipt> {
        Self::execute_with_slots(bytecode, _args, config, &[])
    }

    /// Execute X3BC bytecode on the engine a block runs, with the chain's slots visible.
    ///
    /// Same channel as the `std` arm above: the seeds are the chain's `X3ContractStorage`, loaded
    /// as inherited state rather than as writes, so a program can read a slot a previous comit
    /// persisted and a store over one reports the chain's value as its `old_value`.
    #[cfg(not(feature = "std"))]
    pub fn execute_with_slots(
        bytecode: &[u8],
        _args: &[X3Value],
        config: X3ExecutorConfig,
        seeds: &[([u8; 32], [u8; 32])],
    ) -> X3Result<X3ExecutionReceipt> {
        Self::execute_on_chain(
            bytecode,
            config.gas_limit,
            seeds,
            config.allow_private_submission,
        )
    }

    /// Execute a program the way the chain does, in **every** build.
    ///
    /// This is the one engine: `x3-vm`, built without `std` inside the wasm runtime and with it in
    /// the native runtime, tests and tools. It used to be `mini_x3`, a second, smaller interpreter
    /// that existed only because `x3-vm` needed `std`; the two had different gas schedules and
    /// disagreed on opcodes, so a simulation was a prediction about a different engine. With
    /// `x3-vm` building `no_std` there is nothing left for a second engine to do.
    ///
    /// `seeds` are the chain's contract slots, loaded as inherited state (not journaled, so the
    /// receipt does not report them), and `private_channel_available` is whether the caller can
    /// honour an artifact that demands private submission.
    ///
    /// Contract, unchanged from the engine it replaces:
    /// - a module the on-chain verifier refuses is `VerificationFailed`, and nothing runs;
    /// - an artifact demanding private submission where none is offered is `ExecutionFailed`;
    /// - running out of gas is an unsuccessful receipt charged the whole limit;
    /// - any other fault is an unsuccessful receipt naming it, with no slot writes and no events,
    ///   however many the program journaled before the fault.
    pub fn execute_on_chain(
        bytecode: &[u8],
        gas_limit: u64,
        seeds: &[([u8; 32], [u8; 32])],
        private_channel_available: bool,
    ) -> X3Result<X3ExecutionReceipt> {
        Self::execute_on_chain_with_context(
            bytecode,
            gas_limit,
            seeds,
            private_channel_available,
            &x3_vm::BlockContext::unknown(),
        )
    }

    /// `execute_on_chain`, with what the host knows about the call.
    ///
    /// The context is what `ctx_sender`, `ctx_block_height`, `ctx_timestamp`, `ctx_value` and
    /// `ctx_chain_id` read. A field left `None` is refused by the opcode that reads it, so a caller
    /// supplies only what it actually knows; `execute_on_chain` supplies nothing.
    pub fn execute_on_chain_with_context(
        bytecode: &[u8],
        gas_limit: u64,
        seeds: &[([u8; 32], [u8; 32])],
        private_channel_available: bool,
        context: &x3_vm::BlockContext,
    ) -> X3Result<X3ExecutionReceipt> {
        use x3_vm::{BytecodeModule as Module, VMConfig as Config, VMErrorKind, Value, VM as Vm};

        Self::verify_on_chain(bytecode)?;
        let module = Module::from_bytes(bytecode)
            .map_err(|e| X3IntegrationError::InvalidBytecode(format!("{:?}", e)))?;

        // The artifact's compiled policy, before anything runs. A program that asked not to be
        // public must not be executed in the clear, whichever route brought it here.
        if module
            .features
            .has(x3_backend::bc_format::FeatureFlags::PRIVATE_SUBMISSION_REQUIRED)
            && !private_channel_available
        {
            return Err(X3IntegrationError::ExecutionFailed(
                "private submission required by the compiled policy, but this context offers no \
                 private channel"
                    .into(),
            ));
        }

        let mut vm = Vm::with_config_and_seeds(
            module,
            Config {
                gas_limit,
                max_call_depth: ON_CHAIN_MAX_CALL_DEPTH,
                max_stack_size: ON_CHAIN_MAX_STACK_SIZE,
                trace: false,
            },
            seeds,
        )
        .map_err(|e| X3IntegrationError::ExecutionFailed(format!("{:?}", e)))?;

        vm.set_context(*context);
        // The runtime calls the entry function with no arguments; the verifier has already
        // refused an entry that declares any.
        let result = vm.call_function(0, &[]);
        let instructions_executed = vm.instruction_count();

        match result {
            Ok(exec) => {
                let return_data = match exec.value {
                    Some(Value::I64(v)) => v.to_le_bytes().to_vec(),
                    Some(Value::F64(v)) => v.to_bits().to_le_bytes().to_vec(),
                    Some(Value::Bool(v)) => vec![v as u8],
                    Some(Value::Bytes(v)) => v,
                    Some(Value::String(s)) => s.into_bytes(),
                    Some(Value::Addr(a)) => a.to_le_bytes().to_vec(),
                    Some(Value::Unit) | None => vec![],
                };
                // A write a rolled-back window abandoned is already gone from the journal, so
                // this is exactly the state change the execution made.
                let storage_writes = vm
                    .drain_storage_journal()
                    .into_iter()
                    .map(|write| X3StorageWrite {
                        key: H256::from(write.key),
                        old_value: write.old_value,
                        new_value: write.new_value,
                    })
                    .collect();
                // Committed events only, in order: a rolled-back window's events are not here.
                let logs = vm
                    .drain_events()
                    .into_iter()
                    .map(|event| X3ExecutionLog {
                        topic: event
                            .topics
                            .first()
                            .map(|topic| H256::from(*topic))
                            .unwrap_or_default(),
                        data: event.data,
                    })
                    .collect();
                Ok(X3ExecutionReceipt {
                    success: true,
                    gas_used: vm.gas_used(),
                    return_data,
                    logs,
                    state_changes: vec![],
                    storage_writes,
                    function_index: 0,
                    instructions_executed,
                })
            }
            Err(error) if matches!(error.kind, VMErrorKind::GasLimitExceeded) => {
                Ok(X3ExecutionReceipt {
                    success: false,
                    gas_used: gas_limit,
                    return_data: b"gas exhausted".to_vec(),
                    logs: vec![],
                    state_changes: vec![],
                    storage_writes: vec![],
                    function_index: 0,
                    instructions_executed,
                })
            }
            // A validated program that faults while running (a division by zero, a rolled-back
            // atomic window, an assertion) is an unsuccessful receipt naming the fault, charged
            // the gas it consumed up to the fault. Fail closed: it carries no slot writes and no
            // events, so nothing a failed execution journaled can reach chain state.
            Err(error) => Ok(X3ExecutionReceipt {
                success: false,
                gas_used: vm.gas_used().min(gas_limit),
                return_data: format!("{:?}", error).into_bytes(),
                logs: vec![],
                state_changes: vec![],
                storage_writes: vec![],
                function_index: 0,
                instructions_executed,
            }),
        }
    }

    /// Validate a program the way the chain does, in every build (see `execute_on_chain`).
    ///
    /// `x3-vm`'s verifier with on-chain options: structure, linkage, control flow, atomic balance,
    /// the gas budget, no debug, GPU or floating-point opcodes, no opcode the engine cannot run,
    /// and an entry function that takes no arguments.
    pub fn verify_on_chain(bytecode: &[u8]) -> X3Result<()> {
        x3_vm::Verifier::verify_module_bytes(bytecode, &x3_vm::VerifyOptions::on_chain())
            .map(|_| ())
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))
    }

    /// The chain's up-front gas estimate for a program, in every build (see `execute_on_chain`).
    ///
    /// A size-based bound (EIP-2028 style), unchanged from the engine this replaced, so the
    /// weight a call is pre-charged does not move with this change.
    pub fn estimate_gas_on_chain(bytecode: &[u8]) -> u64 {
        ON_CHAIN_BASE_GAS
            .saturating_add((bytecode.len() as u64).saturating_mul(ON_CHAIN_GAS_PER_BYTE))
    }

    /// Verify bytecode without execution
    #[cfg(feature = "std")]
    pub fn verify(bytecode: &[u8], allow_debug_ops: bool) -> X3Result<()> {
        let opts = if allow_debug_ops {
            VerifyOptions::default()
        } else {
            VerifyOptions::on_chain()
        };
        Verifier::verify_module_bytes(bytecode, &opts)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;
        Ok(())
    }

    /// Verify bytecode (no_std: the chain's verifier, see `verify_on_chain`)
    #[cfg(not(feature = "std"))]
    pub fn verify(bytecode: &[u8], _allow_debug_ops: bool) -> X3Result<()> {
        Self::verify_on_chain(bytecode)
    }

    /// Estimate gas for bytecode execution
    #[cfg(feature = "std")]
    pub fn estimate_gas(bytecode: &[u8]) -> X3Result<u64> {
        // Use verifier's gas estimation
        let report = x3_verifier::Verifier::new(x3_verifier::SafetyRules::default())
            .verify_bytecode(bytecode)
            .map_err(|e| X3IntegrationError::VerificationFailed(format!("{:?}", e)))?;

        // Get gas estimate from report, or use conservative fallback
        let gas_estimate = report
            .gas_report
            .map(|gr| gr.total_gas)
            .unwrap_or_else(|| (bytecode.len() as u64) * 10);

        Ok(gas_estimate)
    }

    /// Estimate gas (no_std: the chain's estimate, see `estimate_gas_on_chain`)
    #[cfg(not(feature = "std"))]
    pub fn estimate_gas(bytecode: &[u8]) -> X3Result<u64> {
        Ok(Self::estimate_gas_on_chain(bytecode))
    }

    /// Compute code hash for bytecode
    pub fn code_hash(bytecode: &[u8]) -> H256 {
        H256::from(sp_io::hashing::blake2_256(bytecode))
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_defaults() {
        let config = X3ExecutorConfig::default();
        assert_eq!(config.gas_limit, 1_000_000);
        assert!(!config.allow_debug_ops);
    }

    #[test]
    fn test_on_chain_config() {
        let config = X3ExecutorConfig::on_chain();
        assert_eq!(config.gas_limit, 500_000);
        assert!(!config.allow_debug_ops);
        assert!(!config.trace);
    }

    #[test]
    fn test_simulation_config() {
        let config = X3ExecutorConfig::simulation();
        assert_eq!(config.gas_limit, 10_000_000);
        assert!(config.allow_debug_ops);
        assert!(config.trace);
    }
}
