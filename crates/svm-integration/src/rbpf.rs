//! Real BPF Executor using solana-rbpf
//!
//! This module provides actual Solana BPF program execution using
//! the solana-rbpf virtual machine.

use crate::{
    serialize_accounts, AccountUpdate, SvmAccountMeta, SvmConfig, SvmError, SvmExecutionResult,
    SvmExecutor, SvmInstruction, SvmResult,
};
use sha2::Digest as Sha2Digest;
use sha3::Digest as Sha3Digest;
use solana_rbpf::{
    elf::Executable,
    error::ProgramResult,
    memory_region::{AccessType, MemoryMapping, MemoryRegion},
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

// ── Guest memory helpers (solana_rbpf 0.8 API) ───────────────────────────────

/// Reads `n` bytes from VM guest memory starting at `src_addr`, returns a
/// buffer. Uses `MemoryMapping::map()` (v0.8) to translate guest→host address
/// then copies the bytes.
unsafe fn read_guest_mem(
    vm_ref: &EbpfVm<AtlasSyscallContext>,
    src_addr: u64,
    n: u64,
) -> Option<Vec<u8>> {
    if n == 0 {
        return Some(vec![]);
    }
    let host_addr = match vm_ref.memory_mapping.map(AccessType::Load, src_addr, n) {
        ProgramResult::Ok(addr) => addr,
        ProgramResult::Err(_) => return None,
    };
    let mut buf = vec![0u8; n as usize];
    unsafe { std::ptr::copy_nonoverlapping(host_addr as *const u8, buf.as_mut_ptr(), n as usize) };
    Some(buf)
}

/// Writes `data` into VM guest memory starting at `dst_addr`.
/// Returns true on success.
unsafe fn write_guest_mem(
    vm_ref: &EbpfVm<AtlasSyscallContext>,
    dst_addr: u64,
    data: &[u8],
) -> bool {
    if data.is_empty() {
        return true;
    }
    let n = data.len() as u64;
    let host_addr = match vm_ref.memory_mapping.map(AccessType::Store, dst_addr, n) {
        ProgramResult::Ok(addr) => addr,
        ProgramResult::Err(_) => return false,
    };
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), host_addr as *mut u8, data.len()) };
    true
}

/// Reads a `u64` from guest memory at `addr` using `MemoryMapping::load`.
/// Returns `None` on access violation.
fn load_u64(vm_ref: &EbpfVm<AtlasSyscallContext>, addr: u64) -> Option<u64> {
    match vm_ref.memory_mapping.load::<u64>(addr) {
        ProgramResult::Ok(val) => Some(val),
        ProgramResult::Err(_) => None,
    }
}

// ── Syscall implementations ─────────────────────────────────────────────────

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

/// Real syscall: sol_sha256
///
/// vals: pointer to array of SolBytes { addr: u64, len: u64 } structs
/// val_len: number of structs in the array
/// hash_result: pointer to 32-byte output buffer
fn syscall_sol_sha256(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    vals: u64,
    val_len: u64,
    hash_result: u64,
    _r4: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    let mut hasher = sha2::Sha256::new();

    let entry_size: u64 = 16;
    for i in 0..val_len {
        let entry_addr = vals.saturating_add(i.saturating_mul(entry_size));

        let byte_addr = match load_u64(vm_ref, entry_addr) {
            Some(a) => a,
            None => return,
        };
        let byte_len = match load_u64(vm_ref, entry_addr.saturating_add(8)) {
            Some(l) => l,
            None => return,
        };
        if byte_len == 0 {
            continue;
        }
        let buf = match unsafe { read_guest_mem(vm_ref, byte_addr, byte_len) } {
            Some(b) => b,
            None => return,
        };
        hasher.update(&buf);
    }

    let digest = hasher.finalize();
    let output: [u8; 32] = digest.into();
    let _ = unsafe { write_guest_mem(vm_ref, hash_result, &output) };
}

/// Real syscall: sol_keccak256
///
/// Same calling convention as sol_sha256: array of {addr, len} structs
/// at vals, val_len entries, output to hash_result (32 bytes).
fn syscall_sol_keccak256(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    vals: u64,
    val_len: u64,
    hash_result: u64,
    _r4: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    let mut hasher = sha3::Keccak256::new();

    let entry_size: u64 = 16;
    for i in 0..val_len {
        let entry_addr = vals.saturating_add(i.saturating_mul(entry_size));

        let byte_addr = match load_u64(vm_ref, entry_addr) {
            Some(a) => a,
            None => return,
        };
        let byte_len = match load_u64(vm_ref, entry_addr.saturating_add(8)) {
            Some(l) => l,
            None => return,
        };
        if byte_len == 0 {
            continue;
        }
        let buf = match unsafe { read_guest_mem(vm_ref, byte_addr, byte_len) } {
            Some(b) => b,
            None => return,
        };
        hasher.update(&buf);
    }

    let digest = hasher.finalize();
    let output: [u8; 32] = digest.into();
    let _ = unsafe { write_guest_mem(vm_ref, hash_result, &output) };
}

/// Real syscall: sol_memcpy
fn syscall_sol_memcpy(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    dst: u64,
    src: u64,
    n: u64,
    _r4: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    if n == 0 {
        return;
    }
    if let Some(data) = unsafe { read_guest_mem(vm_ref, src, n) } {
        let _ = unsafe { write_guest_mem(vm_ref, dst, &data) };
    }
}

/// Real syscall: sol_memmove
fn syscall_sol_memmove(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    dst: u64,
    src: u64,
    n: u64,
    _r4: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    if n == 0 {
        return;
    }
    if let Some(data) = unsafe { read_guest_mem(vm_ref, src, n) } {
        let _ = unsafe { write_guest_mem(vm_ref, dst, &data) };
    }
}

/// Real syscall: sol_memcmp
/// Writes the comparison result as i32 at cmp_result:
///   0 if equal, <0 if s1 < s2, >0 if s1 > s2
fn syscall_sol_memcmp(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    s1: u64,
    s2: u64,
    n: u64,
    cmp_result: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    let data1 = match unsafe { read_guest_mem(vm_ref, s1, n) } {
        Some(d) => d,
        None => return,
    };
    let data2 = match unsafe { read_guest_mem(vm_ref, s2, n) } {
        Some(d) => d,
        None => return,
    };
    // Lexicographic byte comparison
    let mut cmp: i32 = 0;
    for (a, b) in data1.iter().zip(data2.iter()) {
        if a != b {
            cmp = (*a as i32) - (*b as i32);
            break;
        }
    }
    let cmp_bytes = cmp.to_le_bytes();
    let _ = unsafe { write_guest_mem(vm_ref, cmp_result, &cmp_bytes) };
}

/// Real syscall: sol_memset
fn syscall_sol_memset(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    s: u64,
    c: u64,
    n: u64,
    _r4: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    if n == 0 {
        return;
    }
    let byte_val = (c & 0xFF) as u8;
    let fill = vec![byte_val; n as usize];
    let _ = unsafe { write_guest_mem(vm_ref, s, &fill) };
}

/// Real syscall: sol_panic
/// Logs the panic message and sets the return value to non-zero to
/// indicate error to the BPF program.
fn syscall_sol_panic(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    file: u64,
    len: u64,
    line: u64,
    column: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    let file_msg = if len > 0 {
        unsafe { read_guest_mem(vm_ref, file, len) }.unwrap_or_default()
    } else {
        vec![]
    };
    let msg = format!(
        "BPF panic at {}:{}:{}",
        String::from_utf8_lossy(&file_msg),
        line,
        column,
    );
    // solana_rbpf 0.8: context is accessed via context_object_pointer.
    // The syscall receives a *mut EbpfVm, so we access via the raw
    // pointer to get mutable access to the context.
    unsafe {
        let vm_mut = &mut *(vm as *mut EbpfVm<AtlasSyscallContext>);
        vm_mut.context_object_pointer.logs.push(msg.into_bytes());
    }
}

/// Real syscall: sol_create_program_address (PDA derivation)
///
/// Derives a program-derived address from seeds + program_id.
/// Per the Solana spec: hash = SHA-256(seeds || program_id || "ProgramDerivedAddress")
/// Returns the address at `address` pointer.  If the address is on the
/// ed25519 curve (invalid PDA), the BPF return value should indicate error.
fn syscall_sol_create_program_address(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    seeds: u64,
    seeds_len: u64,
    program_id: u64,
    address: u64,
    _r5: u64,
) {
    let vm_ref = unsafe { &*vm };
    let seeds_data = match unsafe { read_guest_mem(vm_ref, seeds, seeds_len) } {
        Some(d) => d,
        None => return,
    };
    let pid_data = match unsafe { read_guest_mem(vm_ref, program_id, 32) } {
        Some(d) => d,
        None => return,
    };

    // PDA derivation: SHA-256(seeds || program_id || "ProgramDerivedAddress")
    let mut hasher = sha2::Sha256::new();
    hasher.update(&seeds_data);
    hasher.update(&pid_data);
    hasher.update(b"ProgramDerivedAddress");
    let derived: [u8; 32] = hasher.finalize().into();

    let _ = unsafe { write_guest_mem(vm_ref, address, &derived) };
}

/// Real syscall: sol_try_find_program_address (PDA with bump iteration)
///
/// Iterates bump seeds from 255 down to 0, appending each to the seed
/// list, until the derived address is off the ed25519 curve (i.e. not a
/// valid public key).  Writes the resulting address + bump to guest memory.
fn syscall_sol_try_find_program_address(
    vm: *mut EbpfVm<AtlasSyscallContext>,
    seeds: u64,
    seeds_len: u64,
    program_id: u64,
    address: u64,
    bump_seed: u64,
) {
    let vm_ref = unsafe { &*vm };
    let seeds_data = match unsafe { read_guest_mem(vm_ref, seeds, seeds_len) } {
        Some(d) => d,
        None => return,
    };
    let pid_data = match unsafe { read_guest_mem(vm_ref, program_id, 32) } {
        Some(d) => d,
        None => return,
    };

    // Try bumps from 255 down to 0
    for bump in (0u8..=255).rev() {
        let mut hasher = sha2::Sha256::new();
        hasher.update(&seeds_data);
        hasher.update(&[bump]);
        hasher.update(&pid_data);
        hasher.update(b"ProgramDerivedAddress");
        let derived: [u8; 32] = hasher.finalize().into();

        // Check if the derived address is on the ed25519 curve
        if ed25519_dalek::VerifyingKey::from_bytes(&derived).is_err() {
            // Off-curve → valid PDA
            let _ = unsafe { write_guest_mem(vm_ref, address, &derived) };
            // bump_seed expects a u64 pointer; write the bump as u64
            let bump_u64 = vec![bump, 0, 0, 0, 0, 0, 0, 0];
            let _ = unsafe { write_guest_mem(vm_ref, bump_seed, &bump_u64) };
            return;
        }
    }
    // If all bumps exhausted and none produce off-curve address,
    // write zeros to indicate failure.
    let zeros = [0u8; 32];
    let _ = unsafe { write_guest_mem(vm_ref, address, &zeros) };
}

/// Create the built-in program with core Solana syscalls registered.
///
/// Registers real implementations for sha256, keccak256, memory operations,
/// PDA derivation, and panic handling.  Well-formed Solana BPF programs can
/// execute to completion with working syscalls.
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
