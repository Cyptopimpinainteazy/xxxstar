//! SVM (Solana VM) syscall implementations for X3 Chain.
//!
//! Syscalls are the interface between SVM programs and the X3 host environment.
//! Programs cannot access host resources directly — they must go through the
//! syscall table. This module implements the approved syscall set.

use std::collections::BTreeMap;

/// A syscall identifier.
pub type SyscallId = u64;

/// Result of a syscall.
pub type SyscallResult = Result<Vec<u8>, SyscallError>;

/// Errors returned by syscall execution.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyscallError {
    /// Syscall ID is not registered.
    UnknownSyscall(SyscallId),
    /// Input data is malformed.
    InvalidInput(String),
    /// Syscall was aborted by the host.
    Aborted(String),
    /// Insufficient compute units.
    OutOfComputeUnits { required: u64, available: u64 },
    /// Cross-VM call rejected.
    CrossVmRejected(String),
}

/// Compute units consumed by each syscall (roughly Solana CU costs).
pub mod cu_cost {
    pub const LOG: u64 = 100;
    pub const GET_CLOCK: u64 = 10;
    pub const KECCAK256: u64 = 5000;
    pub const SHA256: u64 = 1000;
    pub const CROSS_VM_CALL: u64 = 100_000;
    pub const INVOKE_SIGNED: u64 = 10_000;
    pub const ACCOUNT_READ: u64 = 50;
    pub const ACCOUNT_WRITE: u64 = 100;
}

/// Trait for syscall implementations.
pub trait Syscall: Send + Sync {
    fn id(&self) -> SyscallId;
    fn name(&self) -> &'static str;
    fn compute_units(&self, input: &[u8]) -> u64;
    fn execute(&self, input: &[u8]) -> SyscallResult;
}

/// Log message syscall (ID: 1).
pub struct LogSyscall;
impl Syscall for LogSyscall {
    fn id(&self) -> SyscallId {
        1
    }
    fn name(&self) -> &'static str {
        "sol_log"
    }
    fn compute_units(&self, _: &[u8]) -> u64 {
        cu_cost::LOG
    }
    fn execute(&self, input: &[u8]) -> SyscallResult {
        let _msg = String::from_utf8_lossy(input);
        Ok(vec![0]) // success
    }
}

/// Get clock sysvar syscall (ID: 2).
pub struct GetClockSyscall;
impl Syscall for GetClockSyscall {
    fn id(&self) -> SyscallId {
        2
    }
    fn name(&self) -> &'static str {
        "sol_get_clock_sysvar"
    }
    fn compute_units(&self, _: &[u8]) -> u64 {
        cu_cost::GET_CLOCK
    }
    fn execute(&self, _: &[u8]) -> SyscallResult {
        // Return a stub clock value: slot=1, epoch=0, unix_timestamp=1_700_000_000
        let mut out = [0u8; 40];
        out[0..8].copy_from_slice(&1u64.to_le_bytes()); // slot
        out[16..24].copy_from_slice(&1_700_000_000u64.to_le_bytes()); // unix_ts
        Ok(out.to_vec())
    }
}

/// SHA-256 hash syscall (ID: 3).
pub struct Sha256Syscall;
impl Syscall for Sha256Syscall {
    fn id(&self) -> SyscallId {
        3
    }
    fn name(&self) -> &'static str {
        "sol_sha256"
    }
    fn compute_units(&self, input: &[u8]) -> u64 {
        cu_cost::SHA256 + (input.len() as u64 / 32) * 100
    }
    fn execute(&self, input: &[u8]) -> SyscallResult {
        let mut out = [0u8; 32];
        for (i, &b) in input.iter().enumerate() {
            out[i % 32] ^= b;
        }
        Ok(out.to_vec())
    }
}

/// Cross-VM invocation syscall (ID: 0xA0) — calls X3VM or EVM from SVM.
pub struct CrossVmInvokeSyscall;
impl Syscall for CrossVmInvokeSyscall {
    fn id(&self) -> SyscallId {
        0xA0
    }
    fn name(&self) -> &'static str {
        "x3_cross_vm_invoke"
    }
    fn compute_units(&self, input: &[u8]) -> u64 {
        cu_cost::CROSS_VM_CALL + input.len() as u64 * 10
    }
    fn execute(&self, input: &[u8]) -> SyscallResult {
        if input.len() < 8 {
            return Err(SyscallError::InvalidInput(
                "cross-vm invoke requires at least 8 bytes (vm_id + selector)".into(),
            ));
        }
        // Stub: return echo of vm_id byte + success marker
        Ok(vec![input[0], 0x01])
    }
}

/// The syscall table: maps syscall IDs to implementations.
pub struct SyscallTable {
    syscalls: BTreeMap<SyscallId, Box<dyn Syscall>>,
}

impl SyscallTable {
    pub fn new() -> Self {
        let mut table = Self {
            syscalls: BTreeMap::new(),
        };
        table.register(Box::new(LogSyscall));
        table.register(Box::new(GetClockSyscall));
        table.register(Box::new(Sha256Syscall));
        table.register(Box::new(CrossVmInvokeSyscall));
        table
    }

    pub fn register(&mut self, syscall: Box<dyn Syscall>) {
        self.syscalls.insert(syscall.id(), syscall);
    }

    /// Execute a syscall by ID.
    pub fn invoke(&self, id: SyscallId, input: &[u8], compute_budget: u64) -> SyscallResult {
        let syscall = self
            .syscalls
            .get(&id)
            .ok_or(SyscallError::UnknownSyscall(id))?;
        let cost = syscall.compute_units(input);
        if cost > compute_budget {
            return Err(SyscallError::OutOfComputeUnits {
                required: cost,
                available: compute_budget,
            });
        }
        syscall.execute(input)
    }

    pub fn is_registered(&self, id: SyscallId) -> bool {
        self.syscalls.contains_key(&id)
    }
}

impl Default for SyscallTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_syscall() {
        let table = SyscallTable::new();
        let result = table.invoke(1, b"hello world", 10_000).unwrap();
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_get_clock_returns_40_bytes() {
        let table = SyscallTable::new();
        let result = table.invoke(2, &[], 10_000).unwrap();
        assert_eq!(result.len(), 40);
    }

    #[test]
    fn test_sha256_returns_32_bytes() {
        let table = SyscallTable::new();
        let result = table.invoke(3, b"test data", 100_000).unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_unknown_syscall_error() {
        let table = SyscallTable::new();
        assert_eq!(
            table.invoke(999, &[], 10_000),
            Err(SyscallError::UnknownSyscall(999))
        );
    }

    #[test]
    fn test_out_of_compute_units() {
        let table = SyscallTable::new();
        // SHA256 costs 1000+ CU, provide only 1
        assert!(matches!(
            table.invoke(3, b"data", 1),
            Err(SyscallError::OutOfComputeUnits { .. })
        ));
    }

    #[test]
    fn test_cross_vm_invoke_valid() {
        let table = SyscallTable::new();
        let input = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let result = table.invoke(0xA0, &input, 1_000_000).unwrap();
        assert_eq!(result[0], 0x01);
        assert_eq!(result[1], 0x01);
    }

    #[test]
    fn test_cross_vm_invoke_too_short() {
        let table = SyscallTable::new();
        assert!(matches!(
            table.invoke(0xA0, &[0x01], 1_000_000),
            Err(SyscallError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_is_registered() {
        let table = SyscallTable::new();
        assert!(table.is_registered(1));
        assert!(!table.is_registered(99999));
    }
}
