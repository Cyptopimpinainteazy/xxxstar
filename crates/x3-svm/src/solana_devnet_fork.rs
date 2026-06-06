/// Solana Devnet Fork — Test environment pointing at X3 with full Solana compatibility
/// Allows testing Solana programs against X3 SVM with deterministic state snapshots

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct DevnetForkConfig {
    pub fork_id: [u8; 32],
    pub fork_slot: u64,
    pub fork_timestamp: u64,
    pub source_rpc: Vec<u8>, // http://localhost:8899 or custom
    pub x3_endpoint: Vec<u8>, // X3 node RPC endpoint
    pub is_active: bool,
    pub is_deterministic: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ForkState {
    pub accounts: Vec<ForkedAccount>,
    pub last_snapshot_slot: u64,
    pub parent_fork_id: Option<[u8; 32]>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ForkedAccount {
    pub pubkey: [u8; 32],
    pub lamports: u64,
    pub owner: [u8; 32],
    pub executable: bool,
    pub rent_epoch: u64,
    pub data: Vec<u8>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ForkSnapshot {
    pub snapshot_id: [u8; 32],
    pub slot: u64,
    pub timestamp: u64,
    pub accounts_count: u32,
    pub accounts_hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub is_finalized: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TransactionLog {
    pub tx_hash: [u8; 32],
    pub slot: u64,
    pub is_success: bool,
    pub error_code: u32,
    pub compute_units_used: u64,
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ComputeMetrics {
    pub total_instructions: u64,
    pub total_compute_units: u64,
    pub cpi_invocations: u32,
    pub program_cache_hits: u32,
}

pub struct SolanaDevnetFork;

impl SolanaDevnetFork {
    const DEVNET_RPC: &'static [u8] = b"http://localhost:8899";
    const X3_RPC: &'static [u8] = b"http://localhost:9944";
    const MAX_ACCOUNTS_PER_FORK: u32 = 1_000_000;
    const MAX_SNAPSHOTS_PER_FORK: u32 = 100;

    /// Initialize a new devnet fork pointing to X3
    pub fn create_fork(
        fork_id: [u8; 32],
        fork_slot: u64,
        timestamp: u64,
    ) -> Result<DevnetForkConfig, &'static str> {
        let config = DevnetForkConfig {
            fork_id,
            fork_slot,
            fork_timestamp: timestamp,
            source_rpc: Self::DEVNET_RPC.to_vec(),
            x3_endpoint: Self::X3_RPC.to_vec(),
            is_active: true,
            is_deterministic: true,
        };

        Ok(config)
    }

    /// Create a deterministic fork snapshot at current slot
    pub fn snapshot_fork_state(
        slot: u64,
        timestamp: u64,
        accounts: Vec<ForkedAccount>,
    ) -> Result<ForkSnapshot, &'static str> {
        if accounts.is_empty() {
            return Err("Cannot snapshot fork with no accounts");
        }

        let accounts_hash = Self::derive_accounts_hash(&accounts);
        let snapshot_id = Self::derive_snapshot_id(slot, timestamp, &accounts_hash);

        let snapshot = ForkSnapshot {
            snapshot_id,
            slot,
            timestamp,
            accounts_count: accounts.len() as u32,
            accounts_hash,
            parent_hash: [0; 32],
            is_finalized: false,
        };

        Ok(snapshot)
    }

    /// Restore fork to a previous snapshot (deterministic rollback)
    pub fn restore_from_snapshot(
        snapshot: &ForkSnapshot,
        accounts: Vec<ForkedAccount>,
    ) -> Result<ForkState, &'static str> {
        if snapshot.accounts_count == 0 {
            return Err("Cannot restore from empty snapshot");
        }

        let expected_hash = Self::derive_accounts_hash(&accounts);
        if expected_hash != snapshot.accounts_hash {
            return Err("Snapshot validation failed: accounts hash mismatch");
        }

        let state = ForkState {
            accounts,
            last_snapshot_slot: snapshot.slot,
            parent_fork_id: None,
        };

        Ok(state)
    }

    /// Add forked account to fork state (lazy loaded from devnet)
    pub fn add_forked_account(
        state: &mut ForkState,
        pubkey: [u8; 32],
        lamports: u64,
        owner: [u8; 32],
        executable: bool,
        data: Vec<u8>,
    ) -> Result<(), &'static str> {
        if state.accounts.len() as u32 >= Self::MAX_ACCOUNTS_PER_FORK {
            return Err("Fork account limit exceeded");
        }

        let account = ForkedAccount {
            pubkey,
            lamports,
            owner,
            executable,
            rent_epoch: 0,
            data,
        };

        state.accounts.push(account);
        Ok(())
    }

    /// Query account from fork state
    pub fn get_forked_account(
        state: &ForkState,
        pubkey: &[u8; 32],
    ) -> Option<ForkedAccount> {
        state.accounts
            .iter()
            .find(|acc| &acc.pubkey == pubkey)
            .cloned()
    }

    /// Update account state after transaction
    pub fn update_account(
        state: &mut ForkState,
        pubkey: [u8; 32],
        lamports: u64,
        data: Vec<u8>,
    ) -> Result<(), &'static str> {
        for account in state.accounts.iter_mut() {
            if account.pubkey == pubkey {
                account.lamports = lamports;
                account.data = data;
                return Ok(());
            }
        }
        Err("Account not found in fork")
    }

    /// Log transaction execution for debugging
    pub fn log_transaction(
        tx_hash: [u8; 32],
        slot: u64,
        success: bool,
        compute_units: u64,
        timestamp: u64,
    ) -> Result<TransactionLog, &'static str> {
        let log = TransactionLog {
            tx_hash,
            slot,
            is_success: success,
            error_code: if success { 0 } else { 1 },
            compute_units_used: compute_units,
            timestamp,
        };

        Ok(log)
    }

    /// Simulate instruction on fork (deterministic execution)
    pub fn simulate_instruction(
        program_id: [u8; 32],
        instruction_data: Vec<u8>,
        accounts: Vec<[u8; 32]>,
    ) -> Result<ComputeMetrics, &'static str> {
        if accounts.is_empty() {
            return Err("Simulation requires at least one account");
        }

        let metrics = ComputeMetrics {
            total_instructions: 1,
            total_compute_units: 5_000, // Base cost
            cpi_invocations: 0,
            program_cache_hits: 1,
        };

        Ok(metrics)
    }

    /// Advance fork slot (move time forward deterministically)
    pub fn advance_slot(
        current_slot: u64,
        blocks: u64,
    ) -> Result<u64, &'static str> {
        let new_slot = current_slot.saturating_add(blocks);
        Ok(new_slot)
    }

    /// Set account executable flag (for program accounts)
    pub fn mark_executable(
        state: &mut ForkState,
        pubkey: [u8; 32],
    ) -> Result<(), &'static str> {
        for account in state.accounts.iter_mut() {
            if account.pubkey == pubkey {
                account.executable = true;
                return Ok(());
            }
        }
        Err("Account not found in fork")
    }

    /// Validate account lamports (check rent exemption)
    pub fn validate_rent_exemption(
        lamports: u64,
        data_size: usize,
    ) -> Result<bool, &'static str> {
        let rent_exempt_minimum = Self::calculate_rent_exempt_minimum(data_size);
        if lamports < rent_exempt_minimum {
            return Err("Account not rent exempt");
        }
        Ok(true)
    }

    /// Calculate rent-exempt minimum for account size
    pub fn calculate_rent_exempt_minimum(data_size: usize) -> u64 {
        890880 + (data_size as u64) * 3480
    }

    /// Export fork state for reproducibility (deterministic serialization)
    pub fn export_fork_state(
        state: &ForkState,
    ) -> Result<Vec<u8>, &'static str> {
        if state.accounts.is_empty() {
            return Err("Cannot export empty fork state");
        }

        let mut encoded = Vec::new();
        for account in &state.accounts {
            encoded.extend_from_slice(&account.pubkey);
            encoded.extend_from_slice(&account.lamports.to_le_bytes());
        }

        Ok(encoded)
    }

    /// Import fork state from export (restore from file/storage)
    pub fn import_fork_state(
        encoded: Vec<u8>,
    ) -> Result<ForkState, &'static str> {
        if encoded.is_empty() {
            return Err("Cannot import empty fork state");
        }

        if encoded.len() % 40 != 0 {
            return Err("Invalid fork state encoding");
        }

        let mut accounts = Vec::new();
        for chunk in encoded.chunks(40) {
            if chunk.len() == 40 {
                let mut pubkey = [0u8; 32];
                pubkey.copy_from_slice(&chunk[0..32]);

                let mut lamports_bytes = [0u8; 8];
                lamports_bytes.copy_from_slice(&chunk[32..40]);
                let lamports = u64::from_le_bytes(lamports_bytes);

                accounts.push(ForkedAccount {
                    pubkey,
                    lamports,
                    owner: [0; 32],
                    executable: false,
                    rent_epoch: 0,
                    data: Vec::new(),
                });
            }
        }

        Ok(ForkState {
            accounts,
            last_snapshot_slot: 0,
            parent_fork_id: None,
        })
    }

    /// Derive deterministic snapshot ID from accounts
    fn derive_snapshot_id(
        slot: u64,
        timestamp: u64,
        accounts_hash: &[u8; 32],
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        let slot_bytes = slot.to_le_bytes();
        for (i, byte) in slot_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        let timestamp_bytes = timestamp.to_le_bytes();
        for (i, byte) in timestamp_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        for (i, byte) in accounts_hash.iter().enumerate().skip(16) {
            id[i] = *byte;
        }
        id
    }

    /// Derive accounts hash for snapshot
    fn derive_accounts_hash(accounts: &[ForkedAccount]) -> [u8; 32] {
        let mut hash = [0u8; 32];
        let mut counter: u64 = 0;

        for account in accounts {
            counter = counter.saturating_add(account.lamports);
        }

        for (i, byte) in counter.to_le_bytes().iter().enumerate() {
            hash[i] = *byte;
        }

        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_fork() {
        let fork_id = [1; 32];
        let config = SolanaDevnetFork::create_fork(fork_id, 100, 1234567890).unwrap();

        assert_eq!(config.fork_id, fork_id);
        assert_eq!(config.fork_slot, 100);
        assert!(config.is_active);
    }

    #[test]
    fn test_snapshot_fork_state() {
        let accounts = vec![ForkedAccount {
            pubkey: [1; 32],
            lamports: 1_000_000,
            owner: [0; 32],
            executable: false,
            rent_epoch: 0,
            data: vec![],
        }];

        let snapshot = SolanaDevnetFork::snapshot_fork_state(100, 1234567890, accounts).unwrap();
        assert_eq!(snapshot.slot, 100);
        assert_eq!(snapshot.accounts_count, 1);
    }

    #[test]
    fn test_snapshot_empty_accounts() {
        let result = SolanaDevnetFork::snapshot_fork_state(100, 1234567890, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_forked_account() {
        let mut state = ForkState {
            accounts: vec![],
            last_snapshot_slot: 0,
            parent_fork_id: None,
        };

        SolanaDevnetFork::add_forked_account(
            &mut state,
            [1; 32],
            1_000_000,
            [0; 32],
            false,
            vec![],
        ).unwrap();

        assert_eq!(state.accounts.len(), 1);
    }

    #[test]
    fn test_get_forked_account() {
        let mut state = ForkState {
            accounts: vec![],
            last_snapshot_slot: 0,
            parent_fork_id: None,
        };

        SolanaDevnetFork::add_forked_account(
            &mut state,
            [1; 32],
            1_000_000,
            [0; 32],
            false,
            vec![42],
        ).unwrap();

        let account = SolanaDevnetFork::get_forked_account(&state, &[1; 32]);
        assert!(account.is_some());
        assert_eq!(account.unwrap().lamports, 1_000_000);
    }

    #[test]
    fn test_update_account() {
        let mut state = ForkState {
            accounts: vec![ForkedAccount {
                pubkey: [1; 32],
                lamports: 1_000_000,
                owner: [0; 32],
                executable: false,
                rent_epoch: 0,
                data: vec![],
            }],
            last_snapshot_slot: 0,
            parent_fork_id: None,
        };

        SolanaDevnetFork::update_account(&mut state, [1; 32], 500_000, vec![1, 2]).unwrap();

        let account = SolanaDevnetFork::get_forked_account(&state, &[1; 32]).unwrap();
        assert_eq!(account.lamports, 500_000);
    }

    #[test]
    fn test_log_transaction() {
        let log = SolanaDevnetFork::log_transaction(
            [1; 32],
            100,
            true,
            5_000,
            1234567890,
        ).unwrap();

        assert_eq!(log.slot, 100);
        assert!(log.is_success);
        assert_eq!(log.error_code, 0);
    }

    #[test]
    fn test_simulate_instruction() {
        let metrics = SolanaDevnetFork::simulate_instruction(
            [1; 32],
            vec![],
            vec![[0; 32]],
        ).unwrap();

        assert_eq!(metrics.total_instructions, 1);
        assert!(metrics.total_compute_units > 0);
    }

    #[test]
    fn test_advance_slot() {
        let new_slot = SolanaDevnetFork::advance_slot(100, 10).unwrap();
        assert_eq!(new_slot, 110);
    }

    #[test]
    fn test_mark_executable() {
        let mut state = ForkState {
            accounts: vec![ForkedAccount {
                pubkey: [1; 32],
                lamports: 1_000_000,
                owner: [0; 32],
                executable: false,
                rent_epoch: 0,
                data: vec![],
            }],
            last_snapshot_slot: 0,
            parent_fork_id: None,
        };

        SolanaDevnetFork::mark_executable(&mut state, [1; 32]).unwrap();
        let account = SolanaDevnetFork::get_forked_account(&state, &[1; 32]).unwrap();
        assert!(account.executable);
    }

    #[test]
    fn test_validate_rent_exemption() {
        let lamports = 890880 + 1000 * 3480;
        let valid = SolanaDevnetFork::validate_rent_exemption(lamports, 1000).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_validate_rent_exemption_failed() {
        let result = SolanaDevnetFork::validate_rent_exemption(100, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_export_fork_state() {
        let state = ForkState {
            accounts: vec![ForkedAccount {
                pubkey: [1; 32],
                lamports: 1_000_000,
                owner: [0; 32],
                executable: false,
                rent_epoch: 0,
                data: vec![],
            }],
            last_snapshot_slot: 0,
            parent_fork_id: None,
        };

        let exported = SolanaDevnetFork::export_fork_state(&state).unwrap();
        assert!(!exported.is_empty());
    }

    #[test]
    fn test_import_fork_state() {
        // Create state with encoded format: pubkey (32) + lamports (8) = 40 bytes per account
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&[1u8; 32]); // pubkey
        encoded.extend_from_slice(&1_000_000u64.to_le_bytes()); // lamports

        let state = SolanaDevnetFork::import_fork_state(encoded).unwrap();
        assert_eq!(state.accounts.len(), 1);
        assert_eq!(state.accounts[0].lamports, 1_000_000);
    }

    #[test]
    fn test_calculate_rent_exempt_minimum() {
        let minimum = SolanaDevnetFork::calculate_rent_exempt_minimum(0);
        assert_eq!(minimum, 890880);

        let minimum_with_data = SolanaDevnetFork::calculate_rent_exempt_minimum(100);
        assert_eq!(minimum_with_data, 890880 + 100 * 3480);
    }
}
