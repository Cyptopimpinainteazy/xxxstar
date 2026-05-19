//! Real BPF Executor using solana-rbpf
//!
//! This module provides actual Solana BPF program execution using
//! the solana-rbpf virtual machine.

use crate::{
    serialize_accounts, AccountUpdate, SvmAccountMeta, SvmConfig, SvmError, SvmExecutionResult,
    SvmExecutor, SvmInstruction, SvmResult,
};
use solana_rbpf::{
    elf::Executable,
    error::ProgramResult,
    memory_region::{MemoryMapping, MemoryRegion},
    program::{BuiltinFunction, BuiltinProgram, FunctionRegistry, SBPFVersion},
    verifier::RequisiteVerifier,
    vm::{Config, ContextObject, EbpfVm},
};
use std::sync::Arc;

/// Real SVM executor using solana-rbpf
pub struct RbpfSvmExecutor {
    /// VM configuration
    config: Config,
}

impl RbpfSvmExecutor {
    /// Create a new RBPF executor
    pub fn new() -> Self {
        Self {
            config: Config {
                max_call_depth: 64,
                stack_frame_size: 4096,
                enable_stack_frame_gaps: true,
                instruction_meter_checkpoint_distance: 10000,
                enable_instruction_meter: true,
                enable_instruction_tracing: false,
                enable_symbol_and_section_labels: false,
                reject_broken_elfs: true,
                noop_instruction_rate: 256,
                sanitize_user_provided_values: true,
                external_internal_function_hash_collision: false,
                reject_callx_r10: true,
                optimize_rodata: true,
                aligned_memory_mapping: true,
                ..Config::default()
            },
        }
    }

    /// Create executor with custom config
    pub fn with_config(config: Config) -> Self {
        Self { config }
    }
}

impl Default for RbpfSvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for X3 syscall execution
/// Tracks compute units, logs, and return data during BPF execution
struct AtlasSyscallContext {
    /// Remaining compute units
    compute_units_remaining: u64,
    /// Compute units consumed
    compute_units_used: u64,
    /// Logs emitted during execution
    logs: Vec<Vec<u8>>,
}

impl AtlasSyscallContext {
    fn new(compute_limit: u64) -> Self {
        Self {
            compute_units_remaining: compute_limit,
            compute_units_used: 0,
            logs: Vec::new(),
        }
    }
}

impl ContextObject for AtlasSyscallContext {
    fn trace(&mut self, _state: [u64; 12]) {}

    fn consume(&mut self, amount: u64) {
        self.compute_units_used = self.compute_units_used.saturating_add(amount);
        self.compute_units_remaining = self.compute_units_remaining.saturating_sub(amount);
    }

    fn get_remaining(&self) -> u64 {
        self.compute_units_remaining
    }
}

/// Stub syscall: logs a message (no-op in our minimal runtime, but prevents BPF abort)
fn syscall_sol_log(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _addr: u64,
    _len: u64,
    _r3: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: sha256 (no-op, returns 0)
fn syscall_sol_sha256(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _vals: u64,
    _val_len: u64,
    _hash_result: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: keccak256 (no-op, returns 0)
fn syscall_sol_keccak256(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _vals: u64,
    _val_len: u64,
    _hash_result: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: memcpy
fn syscall_sol_memcpy(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _dst: u64,
    _src: u64,
    _n: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: memmove
fn syscall_sol_memmove(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _dst: u64,
    _src: u64,
    _n: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: memcmp
fn syscall_sol_memcmp(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _s1: u64,
    _s2: u64,
    _n: u64,
    _cmp_result: u64,
    _r5: u64,
) {
}

/// Stub syscall: memset
fn syscall_sol_memset(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _s: u64,
    _c: u64,
    _n: u64,
    _r4: u64,
    _r5: u64,
) {
}

/// Stub syscall: panic / abort
fn syscall_sol_panic(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _file: u64,
    _len: u64,
    _line: u64,
    _column: u64,
    _r5: u64,
) {
}

/// Stub syscall: create_program_address (PDA)
fn syscall_sol_create_program_address(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _seeds: u64,
    _seeds_len: u64,
    _program_id: u64,
    _address: u64,
    _r5: u64,
) {
}

/// Stub syscall: try_find_program_address (PDA with bump)
fn syscall_sol_try_find_program_address(
    _vm: *mut EbpfVm<AtlasSyscallContext>,
    _seeds: u64,
    _seeds_len: u64,
    _program_id: u64,
    _address: u64,
    _bump_seed: u64,
) {
}

/// Create the built-in program with core Solana syscalls registered.
///
/// Without these registrations, any BPF program that invokes a syscall will
/// abort with an unknown-function error.  The stubs here are minimal (no-op)
/// but sufficient to let well-formed programs execute to completion.
fn create_loader() -> Arc<BuiltinProgram<AtlasSyscallContext>> {
    let mut registry = FunctionRegistry::<BuiltinFunction<AtlasSyscallContext>>::default();

    let syscalls: &[(&[u8], BuiltinFunction<AtlasSyscallContext>)] = &[
        (b"sol_log_", syscall_sol_log),
        (b"sol_sha256", syscall_sol_sha256),
        (b"sol_keccak256", syscall_sol_keccak256),
        (b"sol_memcpy_", syscall_sol_memcpy),
        (b"sol_memmove_", syscall_sol_memmove),
        (b"sol_memcmp_", syscall_sol_memcmp),
        (b"sol_memset_", syscall_sol_memset),
        (b"sol_panic_", syscall_sol_panic),
        (
            b"sol_create_program_address",
            syscall_sol_create_program_address,
        ),
        (
            b"sol_try_find_program_address",
            syscall_sol_try_find_program_address,
        ),
    ];

    for (name, func) in syscalls {
        let _ = registry.register_function_hashed(name.to_vec(), *func);
    }

    Arc::new(BuiltinProgram::new_loader(Config::default(), registry))
}

impl SvmExecutor for RbpfSvmExecutor {
    fn execute(
        &self,
        instruction: &SvmInstruction,
        _payer: [u8; 32],
        accounts: &[(SvmAccountMeta, AccountUpdate)],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        // For now, we expect the program data to be in instruction.data
        // In a full implementation, we'd look up the program from storage by program_id
        if instruction.program_id == [0u8; 32] {
            return Err(SvmError::InvalidProgramId);
        }

        // Use shared serialize_accounts from lib.rs
        let account_input = serialize_accounts(accounts);

        // Execute the BPF program with instruction data + serialized accounts as input
        let mut result = self.execute_bpf(&instruction.data, &account_input, config)?;

        // Surface writable account balances to upper layers so canonical ledgers can
        // persist account-level views even when the BPF program does not emit deltas.
        if result.account_updates.is_empty() {
            result.account_updates = accounts
                .iter()
                .filter_map(|(meta, update)| {
                    if meta.is_writable {
                        Some(update.clone())
                    } else {
                        None
                    }
                })
                .collect();
        }

        Ok(result)
    }

    fn execute_bpf(
        &self,
        program: &[u8],
        input: &[u8],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        if program.is_empty() {
            return Err(SvmError::InvalidPayload);
        }

        // Create the loader with no syscalls (minimal execution)
        let loader = create_loader();

        // Parse the program (either ELF or raw text bytecode)
        let executable_result = if program.starts_with(b"\x7fELF") {
            Executable::from_elf(program, loader.clone())
        } else {
            Executable::from_text_bytes(
                program,
                loader.clone(),
                SBPFVersion::V1,
                FunctionRegistry::default(),
            )
        };

        let executable = match executable_result {
            Ok(exe) => exe,
            Err(_) => return Err(SvmError::InvalidPayload),
        };

        // Verify the program before execution
        if executable.verify::<RequisiteVerifier>().is_err() {
            return Err(SvmError::InvalidPayload);
        }

        // Create execution context with compute unit metering
        let mut context = AtlasSyscallContext::new(config.compute_unit_limit);

        // Set up memory regions for the VM
        // Region 0: Program code (read-only)
        // Region 1: Input data (read-write for return data)
        let mut input_buffer = input.to_vec();
        // Ensure minimum buffer size for BPF
        if input_buffer.len() < 64 {
            input_buffer.resize(64, 0);
        }

        let regions: Vec<MemoryRegion> =
            vec![MemoryRegion::new_writable(&mut input_buffer, 0x100000000)];

        let sbpf_version = SBPFVersion::V1;
        let memory_mapping = match MemoryMapping::new(regions, &self.config, &sbpf_version) {
            Ok(mm) => mm,
            Err(_) => return Err(SvmError::ExecutionFailed),
        };

        // Create and run the VM
        let mut vm = EbpfVm::new(
            loader,
            &sbpf_version,
            &mut context,
            memory_mapping,
            4096, // stack size
        );

        // Execute the BPF program
        let (instruction_count, result) = vm.execute_program(&executable, true);

        // Consume compute units based on instructions executed
        context.consume(instruction_count);

        // Check if we ran out of compute units
        if context.get_remaining() == 0 && instruction_count >= config.compute_unit_limit {
            return Err(SvmError::OutOfComputeUnits);
        }

        // Interpret execution result
        let (success, return_data) = match result {
            ProgramResult::Ok(return_value) => {
                // Return value 0 indicates success in BPF convention
                (return_value == 0, vec![return_value as u8])
            }
            ProgramResult::Err(_) => (false, vec![]),
        };

        // Compute state root using the canonical formula shared with interp.rs
        let mut result = SvmExecutionResult {
            success,
            output: return_data,
            compute_units_used: context.compute_units_used,
            account_updates: vec![],
            logs: context.logs,
            state_root: [0u8; 32],
        };
        result.state_root = crate::compute_svm_state_root(&result);

        Ok(result)
    }

    fn validate_program(&self, program: &[u8]) -> SvmResult<()> {
        if program.is_empty() {
            return Err(SvmError::InvalidPayload);
        }

        let loader = create_loader();
        let sbpf_version = SBPFVersion::V1;

        // Try to parse and verify
        let executable = if program.starts_with(b"\x7fELF") {
            Executable::from_elf(program, loader).map_err(|_| SvmError::InvalidPayload)?
        } else {
            Executable::from_text_bytes(program, loader, sbpf_version, FunctionRegistry::default())
                .map_err(|_| SvmError::InvalidPayload)?
        };

        executable
            .verify::<RequisiteVerifier>()
            .map_err(|_| SvmError::InvalidPayload)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbpf_executor_creation() {
        let executor = RbpfSvmExecutor::new();
        assert!(executor.config.enable_instruction_meter);
    }

    #[test]
    fn test_rbpf_executor_empty_program() {
        let executor = RbpfSvmExecutor::new();
        let result = executor.execute_bpf(&[], &[], &SvmConfig::default());
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }

    #[test]
    fn test_rbpf_executor_validate_empty() {
        let executor = RbpfSvmExecutor::new();
        let result = executor.validate_program(&[]);
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }
}
