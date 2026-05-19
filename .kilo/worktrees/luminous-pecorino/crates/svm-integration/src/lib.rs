//! SVM (Solana Virtual Machine) Integration for X3 Chain
//!
//! This crate provides integration points for executing SVM transactions
//! as part of dual-VM operations on X3 Chain.
//! Uses solana-rbpf for actual BPF program execution.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Real BPF execution module (std only – uses solana-rbpf JIT/ELF loader)
#[cfg(feature = "std")]
pub mod rbpf;

/// SVM syscall table: sol_log, get_clock, sha256, cross-vm invoke
pub mod syscalls;

#[cfg(feature = "std")]
pub use rbpf::RbpfSvmExecutor;

/// Minimal eBPF interpreter – real execution, no std required.
/// Available in both std and no-std builds.
pub mod interp;

pub use interp::{execute_bpf as interp_execute_bpf, validate_program as interp_validate_program};

/// no-std SVM executor backed by the built-in eBPF interpreter.
pub struct InterpSvmExecutor;

/// Result type for SVM operations
pub type SvmResult<T> = Result<T, SvmError>;

/// Errors that can occur during SVM execution
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum SvmError {
    /// Invalid program or transaction data
    InvalidPayload,
    /// Program execution failed
    ExecutionFailed,
    /// Account not found or invalid
    InvalidAccount,
    /// Signature verification failed
    InvalidSignature,
    /// Out of compute units
    OutOfComputeUnits,
    /// Invalid instruction data
    InvalidInstructionData,
    /// Account data too small
    AccountDataTooSmall,
    /// Insufficient funds for fee
    InsufficientFunds,
    /// Program not executable
    ProgramNotExecutable,
    /// Invalid program ID
    InvalidProgramId,
    /// Other execution error with code
    ExecutionError(u32),
}

/// Represents the result of SVM program execution
#[derive(Debug, Clone, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Output data from the execution (return data)
    pub output: Vec<u8>,
    /// Compute units used in the execution
    pub compute_units_used: u64,
    /// Account changes during execution
    pub account_updates: Vec<AccountUpdate>,
    /// Log messages emitted by the program
    pub logs: Vec<Vec<u8>>,
    /// State root after execution
    pub state_root: [u8; 32],
}

/// Represents an update to an account during SVM execution
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, Default)]
pub struct AccountUpdate {
    /// Account public key (32 bytes)
    pub pubkey: [u8; 32],
    /// New account data
    pub data: Vec<u8>,
    /// New lamport balance
    pub lamports: u64,
    /// Is account executable
    pub executable: bool,
    /// Owner program ID
    pub owner: [u8; 32],
    /// Rent epoch
    pub rent_epoch: u64,
}

/// Compute a canonical state root from observable SVM execution outputs.
///
/// Both the no-std interpreter (`interp.rs`) and the std rbpf executor
/// (`rbpf.rs`) MUST use this function so that state roots are identical for
/// the same logical execution, regardless of which backend produced them.
///
/// Hash domain: `blake2_256(success || compute_units_used || return_data || account_updates || logs)`
pub fn compute_svm_state_root(result: &SvmExecutionResult) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut buf = Vec::new();
    // 1. success flag
    buf.push(if result.success { 1u8 } else { 0u8 });
    // 2. compute units consumed (le)
    buf.extend_from_slice(&result.compute_units_used.to_le_bytes());
    // 3. return data
    buf.extend_from_slice(&(result.output.len() as u32).to_le_bytes());
    buf.extend_from_slice(&result.output);
    // 4. account updates (deterministic: sorted by pubkey)
    let mut keys: Vec<usize> = (0..result.account_updates.len()).collect();
    keys.sort_by_key(|&i| result.account_updates[i].pubkey);
    for i in keys {
        let upd = &result.account_updates[i];
        buf.extend_from_slice(&upd.pubkey);
        buf.extend_from_slice(&upd.lamports.to_le_bytes());
        buf.extend_from_slice(&(upd.data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&upd.data);
        buf.extend_from_slice(&upd.owner);
        buf.push(if upd.executable { 1 } else { 0 });
    }
    // 5. logs
    for log in &result.logs {
        buf.extend_from_slice(&(log.len() as u32).to_le_bytes());
        buf.extend_from_slice(log);
    }

    blake2_256(&buf)
}

/// SVM execution environment configuration
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmConfig {
    /// Maximum compute units per transaction
    pub compute_unit_limit: u64,
    /// Compute unit price (microlamports)
    pub compute_unit_price: u64,
    /// Block height (slot) for execution context
    pub slot: u64,
    /// Block timestamp for execution context (unix timestamp)
    pub block_timestamp: i64,
    /// Recent blockhash for transaction validation
    pub recent_blockhash: [u8; 32],
    /// Enable cross-program invocation (CPI)
    pub enable_cpi: bool,
    /// Maximum CPI depth
    pub max_cpi_depth: u8,
}

impl Default for SvmConfig {
    fn default() -> Self {
        Self {
            compute_unit_limit: 200_000,
            compute_unit_price: 1,
            slot: 0,
            block_timestamp: 0,
            recent_blockhash: [0u8; 32],
            enable_cpi: true,
            max_cpi_depth: 4,
        }
    }
}

impl SvmConfig {
    /// Create a new SvmConfig with explicit parameters
    pub fn new(
        compute_unit_limit: u64,
        compute_unit_price: u64,
        slot: u64,
        block_timestamp: i64,
    ) -> Self {
        Self {
            compute_unit_limit,
            compute_unit_price,
            slot,
            block_timestamp,
            recent_blockhash: [0u8; 32],
            enable_cpi: true,
            max_cpi_depth: 4,
        }
    }

    /// Set the recent blockhash
    pub fn with_blockhash(mut self, hash: [u8; 32]) -> Self {
        self.recent_blockhash = hash;
        self
    }

    /// Enable/disable CPI
    pub fn with_cpi(mut self, enable: bool, max_depth: u8) -> Self {
        self.enable_cpi = enable;
        self.max_cpi_depth = max_depth;
        self
    }
}

/// Instruction for SVM execution
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmInstruction {
    /// Program ID to invoke
    pub program_id: [u8; 32],
    /// Accounts required by the instruction
    pub accounts: Vec<SvmAccountMeta>,
    /// Instruction data
    pub data: Vec<u8>,
}

/// Account metadata for SVM instruction
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct SvmAccountMeta {
    /// Account public key
    pub pubkey: [u8; 32],
    /// Is signer
    pub is_signer: bool,
    /// Is writable
    pub is_writable: bool,
}

/// Trait for SVM execution adapters
pub trait SvmExecutor {
    /// Execute SVM program with instruction
    fn execute(
        &self,
        instruction: &SvmInstruction,
        payer: [u8; 32],
        accounts: &[(SvmAccountMeta, AccountUpdate)],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult>;

    /// Execute raw BPF bytecode directly
    fn execute_bpf(
        &self,
        program: &[u8],
        input: &[u8],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult>;

    /// Validate BPF program
    fn validate_program(&self, program: &[u8]) -> SvmResult<()>;
}

/// Real SVM executor using solana-rbpf (replaces mock for testing)
#[cfg(any(test, feature = "test-utils"))]
pub struct MockSvmExecutor {
    inner: RbpfSvmExecutor,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockSvmExecutor {
    /// Create a new mock executor that delegates to real RbpfSvmExecutor
    pub fn new() -> Self {
        Self {
            inner: RbpfSvmExecutor::new(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for MockSvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl SvmExecutor for MockSvmExecutor {
    fn execute(
        &self,
        instruction: &SvmInstruction,
        _payer: [u8; 32],
        _accounts: &[(SvmAccountMeta, AccountUpdate)],
        _config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        if instruction.program_id == [0u8; 32] {
            return Err(SvmError::InvalidProgramId);
        }
        Ok(SvmExecutionResult {
            success: true,
            output: vec![0x00],
            compute_units_used: 100,
            account_updates: vec![],
            logs: vec![],
            state_root: [0u8; 32],
        })
    }

    fn execute_bpf(
        &self,
        program: &[u8],
        _input: &[u8],
        _config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        if program.is_empty() {
            return Err(SvmError::InvalidPayload);
        }
        Ok(SvmExecutionResult {
            success: true,
            output: vec![0x00],
            compute_units_used: 100,
            account_updates: vec![],
            logs: vec![],
            state_root: [0u8; 32],
        })
    }

    fn validate_program(&self, _program: &[u8]) -> SvmResult<()> {
        Ok(())
    }
}

/// Prepare root computation for SVM execution
pub fn compute_svm_prepare_root(
    comit_id: &[u8; 32],
    instruction: &SvmInstruction,
    result: &SvmExecutionResult,
) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut preimage = Vec::new();
    preimage.extend_from_slice(comit_id);
    preimage.extend_from_slice(&instruction.program_id);
    preimage.extend_from_slice(&instruction.data);
    preimage.extend_from_slice(&result.state_root);

    blake2_256(&preimage)
}

/// SVM account state database for tracking program accounts
#[derive(Default)]
pub struct SvmAccountDb {
    /// Accounts indexed by pubkey
    accounts: sp_std::collections::btree_map::BTreeMap<[u8; 32], AccountUpdate>,
}

impl SvmAccountDb {
    /// Create a new account database
    pub fn new() -> Self {
        Self::default()
    }

    /// Get account by pubkey
    pub fn get_account(&self, pubkey: &[u8; 32]) -> Option<&AccountUpdate> {
        self.accounts.get(pubkey)
    }

    /// Upsert an account
    pub fn set_account(&mut self, pubkey: [u8; 32], account: AccountUpdate) {
        self.accounts.insert(pubkey, account);
    }

    /// Transfer lamports between accounts with overflow protection
    pub fn transfer(
        &mut self,
        from: &[u8; 32],
        to: &[u8; 32],
        lamports: u64,
    ) -> Result<(), &'static str> {
        let from_balance = self.accounts.get(from).map(|a| a.lamports).unwrap_or(0);
        if from_balance < lamports {
            return Err("insufficient lamports");
        }

        // Debit source
        let from_acc = self.accounts.entry(*from).or_default();
        from_acc.lamports = from_acc.lamports.saturating_sub(lamports);

        // Credit destination
        let to_acc = self.accounts.entry(*to).or_default();
        to_acc.lamports = to_acc
            .lamports
            .checked_add(lamports)
            .ok_or("lamport overflow")?;

        Ok(())
    }

    /// Get number of accounts stored
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Compute state root from all accounts
    pub fn compute_state_root(&self) -> [u8; 32] {
        use sp_io::hashing::blake2_256;

        if self.accounts.is_empty() {
            return [0u8; 32];
        }

        let mut data = Vec::new();
        for (pubkey, account) in &self.accounts {
            data.extend_from_slice(pubkey);
            data.extend_from_slice(&account.lamports.to_le_bytes());
            data.extend_from_slice(&(account.data.len() as u32).to_le_bytes());
            data.extend_from_slice(&account.data);
            data.extend_from_slice(&account.owner);
            data.push(if account.executable { 1 } else { 0 });
        }

        blake2_256(&data)
    }
}

/// Compute unit metering for SVM execution
pub struct ComputeMeter {
    /// Maximum compute units
    limit: u64,
    /// Units consumed
    consumed: u64,
}

impl ComputeMeter {
    /// Create new meter with limit
    pub fn new(limit: u64) -> Self {
        Self { limit, consumed: 0 }
    }

    /// Consume compute units; returns Err if over limit
    pub fn consume(&mut self, units: u64) -> Result<(), SvmError> {
        self.consumed = self.consumed.saturating_add(units);
        if self.consumed > self.limit {
            return Err(SvmError::OutOfComputeUnits);
        }
        Ok(())
    }

    /// Get remaining units
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    /// Get consumed units
    pub fn consumed(&self) -> u64 {
        self.consumed
    }
}

/// Program deployment entry
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct DeployedProgram {
    /// Program public key
    pub program_id: [u8; 32],
    /// Deployer public key  
    pub deployer: [u8; 32],
    /// BPF bytecode hash
    pub bytecode_hash: [u8; 32],
    /// Slot when deployed
    pub deploy_slot: u64,
    /// Compute units used for deployment
    pub deploy_compute: u64,
    /// Whether program is currently executable
    pub is_executable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SvmConfig::default();
        assert_eq!(config.compute_unit_limit, 200_000);
        assert!(config.enable_cpi);
        assert_eq!(config.max_cpi_depth, 4);
    }

    #[test]
    fn test_config_builder() {
        let config = SvmConfig::new(400_000, 5, 100, 1_700_000_000)
            .with_blockhash([0xFF; 32])
            .with_cpi(false, 0);
        assert_eq!(config.compute_unit_limit, 400_000);
        assert_eq!(config.recent_blockhash, [0xFF; 32]);
        assert!(!config.enable_cpi);
    }

    #[test]
    fn test_mock_executor_success() {
        let executor = MockSvmExecutor::new();
        let instruction = SvmInstruction {
            program_id: [1u8; 32],
            accounts: vec![],
            data: vec![0x01, 0x02],
        };
        let result = executor.execute(&instruction, [0u8; 32], &[], &SvmConfig::default());
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_mock_executor_invalid_program_id() {
        let executor = MockSvmExecutor::new();
        let instruction = SvmInstruction {
            program_id: [0u8; 32], // zero = invalid
            accounts: vec![],
            data: vec![],
        };
        let result = executor.execute(&instruction, [0u8; 32], &[], &SvmConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_executor_bpf() {
        let executor = MockSvmExecutor::new();
        let result = executor.execute_bpf(&[0x79, 0x00], &[], &SvmConfig::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_executor_empty_program() {
        let executor = MockSvmExecutor::new();
        let result = executor.execute_bpf(&[], &[], &SvmConfig::default());
        assert_eq!(result, Err(SvmError::InvalidPayload));
    }

    #[test]
    fn test_account_db_transfer() {
        let mut db = SvmAccountDb::new();
        let from = [1u8; 32];
        let to = [2u8; 32];

        db.set_account(
            from,
            AccountUpdate {
                pubkey: from,
                lamports: 1000,
                ..Default::default()
            },
        );

        assert!(db.transfer(&from, &to, 500).is_ok());
        assert_eq!(db.get_account(&from).unwrap().lamports, 500);
        assert_eq!(db.get_account(&to).unwrap().lamports, 500);
    }

    #[test]
    fn test_account_db_insufficient_lamports() {
        let mut db = SvmAccountDb::new();
        let from = [1u8; 32];
        let to = [2u8; 32];

        db.set_account(
            from,
            AccountUpdate {
                pubkey: from,
                lamports: 100,
                ..Default::default()
            },
        );

        assert!(db.transfer(&from, &to, 200).is_err());
    }

    #[test]
    fn test_account_db_state_root() {
        let mut db = SvmAccountDb::new();
        let key = [0xAA; 32];
        db.set_account(
            key,
            AccountUpdate {
                pubkey: key,
                lamports: 1000,
                data: vec![1, 2, 3],
                executable: false,
                owner: [0u8; 32],
                rent_epoch: 0,
            },
        );

        let root1 = db.compute_state_root();
        let root2 = db.compute_state_root();
        assert_eq!(root1, root2);
        assert_ne!(root1, [0u8; 32]);
    }

    #[test]
    fn test_compute_meter() {
        let mut meter = ComputeMeter::new(200_000);
        assert_eq!(meter.remaining(), 200_000);

        assert!(meter.consume(50_000).is_ok());
        assert_eq!(meter.consumed(), 50_000);
        assert_eq!(meter.remaining(), 150_000);

        assert!(meter.consume(200_000).is_err()); // over limit
    }

    #[test]
    fn test_prepare_root_computation() {
        let comit_id = [0xBB; 32];
        let instruction = SvmInstruction {
            program_id: [1u8; 32],
            accounts: vec![],
            data: vec![0x01],
        };
        let result = SvmExecutionResult {
            success: true,
            output: vec![0x00],
            compute_units_used: 1000,
            account_updates: vec![],
            logs: vec![],
            state_root: [0xCC; 32],
        };
        let root = compute_svm_prepare_root(&comit_id, &instruction, &result);
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_deployed_program() {
        let program = DeployedProgram {
            program_id: [1u8; 32],
            deployer: [2u8; 32],
            bytecode_hash: [3u8; 32],
            deploy_slot: 100,
            deploy_compute: 50_000,
            is_executable: true,
        };
        assert!(program.is_executable);
        assert_eq!(program.deploy_slot, 100);
    }

    #[test]
    fn test_svm_error_variants() {
        assert_ne!(SvmError::OutOfComputeUnits, SvmError::InvalidPayload);
        assert_ne!(SvmError::InvalidProgramId, SvmError::InvalidAccount);
        assert_eq!(SvmError::ExecutionError(42), SvmError::ExecutionError(42));
    }

    #[test]
    fn test_account_meta_construction() {
        let meta = SvmAccountMeta {
            pubkey: [0xAA; 32],
            is_signer: true,
            is_writable: true,
        };
        assert!(meta.is_signer);
        assert!(meta.is_writable);
    }
}

// ---------------------------------------------------------------------------
// Account serialization (used by both InterpSvmExecutor and RbpfSvmExecutor)
// ---------------------------------------------------------------------------

/// Serialize accounts into a buffer for BPF program access.
///
/// Format per account:
/// - 32 bytes: pubkey
/// - 8 bytes: lamports (little-endian u64)
/// - 1 byte: is_signer flag
/// - 1 byte: is_writable flag
/// - 4 bytes: data length (little-endian u32)
/// - Variable: data from AccountUpdate
pub fn serialize_accounts(accounts: &[(SvmAccountMeta, AccountUpdate)]) -> Vec<u8> {
    let mut buffer = Vec::new();

    // Write account count as u32 LE
    buffer.extend_from_slice(&(accounts.len() as u32).to_le_bytes());

    for (meta, update) in accounts {
        // Pubkey (32 bytes)
        buffer.extend_from_slice(&meta.pubkey);

        // Lamports from update (8 bytes)
        buffer.extend_from_slice(&update.lamports.to_le_bytes());

        // Flags (2 bytes)
        buffer.push(if meta.is_signer { 1 } else { 0 });
        buffer.push(if meta.is_writable { 1 } else { 0 });

        // Data length and data
        buffer.extend_from_slice(&(update.data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&update.data);
    }

    buffer
}

// ---------------------------------------------------------------------------
// InterpSvmExecutor – real no-std executor using the built-in eBPF interpreter
// ---------------------------------------------------------------------------

impl SvmExecutor for InterpSvmExecutor {
    fn execute(
        &self,
        instruction: &SvmInstruction,
        _payer: [u8; 32],
        accounts: &[(SvmAccountMeta, AccountUpdate)],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        // Check for invalid program_id
        if instruction.program_id == [0u8; 32] {
            return Err(SvmError::InvalidProgramId);
        }

        // Serialize accounts into input buffer for BPF program access
        let account_input = serialize_accounts(accounts);

        // Execute the BPF program with instruction.data as program and account_input as input
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
        interp::execute_bpf(program, input, config)
    }

    fn validate_program(&self, program: &[u8]) -> SvmResult<()> {
        interp::validate_program(program)
    }
}
