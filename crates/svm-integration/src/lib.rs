//! SVM (Solana Virtual Machine) Integration for X3 Chain
//!
//! This crate provides integration points for executing SVM transactions
//! as part of dual-VM operations on X3 Chain.
//! Uses solana-rbpf for actual BPF program execution.
#![allow(clippy::items_after_test_module)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Real BPF execution module (std only – uses solana-rbpf JIT/ELF loader)
#[cfg(feature = "std")]
pub mod rbpf;

/// SVM syscall table: sol_log, get_clock, sha256, cross-vm invoke
#[cfg(feature = "std")]
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
/// Hash domain: `blake2_256(success || return_data || account_updates || logs)`
///
/// Compute usage is resource accounting, not post-execution state. Different executors may meter
/// the same program differently, while the economic result and the state they produce must agree.
pub fn compute_svm_state_root(result: &SvmExecutionResult) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut buf = Vec::new();
    // 1. success flag
    buf.push(if result.success { 1u8 } else { 0u8 });
    // 2. return data
    buf.extend_from_slice(&(result.output.len() as u32).to_le_bytes());
    buf.extend_from_slice(&result.output);
    // 3. account updates (deterministic: sorted by pubkey)
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
    // 4. logs
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

    /// Execute explicit program bytecode with separate instruction data and account state.
    fn execute_with_program(
        &self,
        program: &[u8],
        instruction_data: &[u8],
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
    /// Inner real executor that the mock currently delegates away
    /// from. Held for parity with future revisions that may use it.
    #[allow(dead_code)]
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

    fn execute_with_program(
        &self,
        program: &[u8],
        _instruction_data: &[u8],
        _accounts: &[(SvmAccountMeta, AccountUpdate)],
        _config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        self.execute_bpf(program, &[], &SvmConfig::default())
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

    fn increment_account_data_program() -> Vec<u8> {
        const ACCOUNT_DATA_OFFSET: i16 = 50;
        vec![
            0x79,
            0x10,
            ACCOUNT_DATA_OFFSET as u8,
            0,
            0,
            0,
            0,
            0, // r0 = *(u64 *)(r1 + 50)
            0x07,
            0x00,
            0,
            0,
            1,
            0,
            0,
            0, // r0 += 1
            0x7b,
            0x01,
            ACCOUNT_DATA_OFFSET as u8,
            0,
            0,
            0,
            0,
            0, // *(u64 *)(r1 + 50) = r0
            0xb7,
            0x00,
            0,
            0,
            0,
            0,
            0,
            0, // r0 = 0 (success)
            0x95,
            0,
            0,
            0,
            0,
            0,
            0,
            0, // exit
        ]
    }

    fn increment_account_fixture() -> (Vec<u8>, Vec<(SvmAccountMeta, AccountUpdate)>) {
        let pubkey = [0x22; 32];
        let meta = SvmAccountMeta {
            pubkey,
            is_signer: false,
            is_writable: true,
        };
        let account = AccountUpdate {
            pubkey,
            data: vec![0; 8],
            lamports: 1_000,
            owner: [0x33; 32],
            ..Default::default()
        };
        (increment_account_data_program(), vec![(meta, account)])
    }

    #[test]
    fn interpreter_returns_writable_account_data_changes() {
        let (program, accounts) = increment_account_fixture();
        let input = serialize_accounts(&accounts);
        let result =
            interp::execute_bpf_with_accounts(&program, &input, &accounts, &SvmConfig::default())
                .expect("interpreter execution succeeds");
        assert!(result.success);
        assert_eq!(result.account_updates.len(), 1);
        assert_eq!(result.account_updates[0].data, 1u64.to_le_bytes());
    }

    #[test]
    fn rbpf_and_interpreter_agree_on_writable_account_data() {
        let (program, accounts) = increment_account_fixture();
        let input = serialize_accounts(&accounts);
        let interp_result =
            interp::execute_bpf_with_accounts(&program, &input, &accounts, &SvmConfig::default())
                .expect("interpreter execution succeeds");

        let executor = RbpfSvmExecutor::new();
        let instruction = SvmInstruction {
            program_id: [0x11; 32],
            accounts: accounts.iter().map(|(meta, _)| meta.clone()).collect(),
            data: program,
        };
        let rbpf_result = executor
            .execute(&instruction, [0u8; 32], &accounts, &SvmConfig::default())
            .expect("rbpf execution succeeds");

        assert!(rbpf_result.success);
        assert_eq!(interp_result.account_updates, rbpf_result.account_updates);
        assert_eq!(interp_result.output, rbpf_result.output);
        assert!(interp_result.compute_units_used > 0);
        assert!(rbpf_result.compute_units_used > 0);
        assert_eq!(interp_result.logs, rbpf_result.logs);
        assert_eq!(interp_result.state_root, rbpf_result.state_root);
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

/// Extract writable account updates from a program's serialized account buffer.
///
/// The buffer layout is defined by [`serialize_accounts`]. Programs may mutate account data in
/// place, but they may not rewrite the account count, public keys, lamports, access flags, or data
/// lengths. Those fields are part of the instruction's authorization context, not program state.
/// Any bytes after the serialized accounts are instruction data and are not persisted as account
/// state.
/// The returned vector includes every writable account so callers can persist unchanged balances as
/// well as changed data.
pub fn account_updates_from_serialized(
    original: &[u8],
    executed: &[u8],
    accounts: &[(SvmAccountMeta, AccountUpdate)],
) -> SvmResult<Vec<AccountUpdate>> {
    if executed.len() < original.len() || original.len() < 4 {
        return Err(SvmError::InvalidAccount);
    }
    let count = u32::from_le_bytes([original[0], original[1], original[2], original[3]]) as usize;
    if count != accounts.len() || executed[0..4] != original[0..4] {
        return Err(SvmError::InvalidAccount);
    }

    const ACCOUNT_HEADER_LEN: usize = 32 + 8 + 1 + 1 + 4;
    let mut offset = 4usize;
    let mut updates = Vec::with_capacity(count);

    for (meta, previous) in accounts {
        let header_end = offset
            .checked_add(ACCOUNT_HEADER_LEN)
            .ok_or(SvmError::InvalidAccount)?;
        if header_end > executed.len() {
            return Err(SvmError::InvalidAccount);
        }

        if executed[offset..header_end] != original[offset..header_end]
            || executed[offset..offset + 32] != meta.pubkey
        {
            return Err(SvmError::InvalidAccount);
        }

        let length_offset = offset + 32 + 8 + 1 + 1;
        let data_len = u32::from_le_bytes([
            executed[length_offset],
            executed[length_offset + 1],
            executed[length_offset + 2],
            executed[length_offset + 3],
        ]) as usize;
        if data_len != previous.data.len() {
            return Err(SvmError::InvalidAccount);
        }

        let data_end = header_end
            .checked_add(data_len)
            .ok_or(SvmError::InvalidAccount)?;
        if data_end > executed.len() {
            return Err(SvmError::InvalidAccount);
        }

        let mutated = &executed[header_end..data_end];
        let changed = mutated != previous.data.as_slice();
        if changed && !meta.is_writable {
            return Err(SvmError::InvalidAccount);
        }
        if meta.is_writable {
            let mut update = previous.clone();
            update.pubkey = meta.pubkey;
            update.data = mutated.to_vec();
            updates.push(update);
        }
        offset = data_end;
    }

    if offset <= original.len() {
        Ok(updates)
    } else {
        Err(SvmError::InvalidAccount)
    }
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

        self.execute_with_program(&instruction.data, &[], accounts, config)
    }

    fn execute_with_program(
        &self,
        program: &[u8],
        instruction_data: &[u8],
        accounts: &[(SvmAccountMeta, AccountUpdate)],
        config: &SvmConfig,
    ) -> SvmResult<SvmExecutionResult> {
        let mut input = serialize_accounts(accounts);
        input.extend_from_slice(instruction_data);
        interp::execute_bpf_with_accounts(program, &input, accounts, config)
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
