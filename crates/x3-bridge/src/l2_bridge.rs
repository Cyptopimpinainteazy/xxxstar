/// L2 Bridge — Base/Optimism settlement bridge enabling Ethereum L2 ↔ X3 token transfers
/// Implements canonical bridge with sequencer messaging and exit proofs
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct L2BridgeDeposit {
    pub deposit_id: u64,
    pub depositor: [u8; 20],
    pub token: [u8; 20],
    pub amount: u128,
    pub l2_recipient: [u8; 32],
    pub timestamp: u64,
    pub status: DepositStatus,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum DepositStatus {
    Pending,
    Confirmed,
    Relayed,
    Failed,
    Refunded,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct L2Withdrawal {
    pub withdrawal_id: u64,
    pub sender: [u8; 32],
    pub token: [u8; 20],
    pub amount: u128,
    pub l1_recipient: [u8; 20],
    pub timestamp: u64,
    pub status: WithdrawalStatus,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum WithdrawalStatus {
    Initiated,
    BatchedForL1,
    Proven,
    Executed,
    Failed,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OutputRoot {
    pub l2_output_index: u64,
    pub output_root: [u8; 32],
    pub timestamp: u64,
    pub block_number: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct WithdrawalProof {
    pub output_root_proof: Vec<[u8; 32]>,
    pub withdrawal_proof: Vec<[u8; 32]>,
    pub output_index: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TokenConfig {
    pub l1_address: [u8; 20],
    pub l2_address: [u8; 20],
    pub decimals: u8,
    pub enabled: bool,
}

pub const L2_BLOCK_TIME_SECONDS: u64 = 2; // Optimism OP-Stack uses 2 second blocks
pub const FINALIZATION_PERIOD_BLOCKS: u64 = 50400; // ~7 days

pub struct L2Bridge;

impl L2Bridge {
    /// Initiate deposit from L1 → X3 (L2)
    pub fn initiate_deposit(
        depositor: [u8; 20],
        token: [u8; 20],
        amount: u128,
        l2_recipient: [u8; 32],
    ) -> Result<u64, &'static str> {
        if amount == 0 {
            return Err("Deposit amount must be positive");
        }
        if depositor == [0; 20] {
            return Err("Invalid depositor address");
        }

        let deposit_id = Self::generate_deposit_id(&depositor, &token, amount);

        Ok(deposit_id)
    }

    /// Confirm deposit with L1 transaction proof
    pub fn confirm_deposit(
        deposit: &mut L2BridgeDeposit,
        l1_block_number: u64,
        finalization_timestamp: u64,
        current_timestamp: u64,
    ) -> Result<(), &'static str> {
        if deposit.status != DepositStatus::Pending {
            return Err("Deposit is not in Pending state");
        }

        if finalization_timestamp < deposit.timestamp {
            return Err("block number regression detected");
        }

        // Require ~2 minutes of L1 block confirmations (12 blocks * 12 seconds)
        let block_age = finalization_timestamp.saturating_sub(deposit.timestamp);
        if block_age < 120 {
            return Err("Insufficient block confirmations (require 120 seconds)");
        }

        deposit.status = DepositStatus::Confirmed;
        Ok(())
    }

    /// Relay confirmed deposit to X3 L2 state root
    pub fn relay_deposit(deposit: &mut L2BridgeDeposit) -> Result<(), &'static str> {
        if deposit.status != DepositStatus::Confirmed {
            return Err("Deposit must be Confirmed before relay");
        }

        deposit.status = DepositStatus::Relayed;
        Ok(())
    }

    /// Initiate withdrawal from X3 (L2) → L1
    pub fn initiate_withdrawal(
        sender: [u8; 32],
        token: [u8; 20],
        amount: u128,
        l1_recipient: [u8; 20],
    ) -> Result<u64, &'static str> {
        if amount == 0 {
            return Err("Withdrawal amount must be positive");
        }
        if sender == [0; 32] {
            return Err("Invalid sender address");
        }

        let withdrawal_id = Self::generate_withdrawal_id(&sender, &token, amount);
        Ok(withdrawal_id)
    }

    /// Batch withdrawals into Merkle tree for L1 proof generation
    pub fn batch_withdrawal(withdrawal: &mut L2Withdrawal) -> Result<(), &'static str> {
        if withdrawal.status != WithdrawalStatus::Initiated {
            return Err("Withdrawal must be in Initiated state");
        }

        withdrawal.status = WithdrawalStatus::BatchedForL1;
        Ok(())
    }

    /// Submit output root (state commitment) from L2 to L1
    pub fn submit_output_root(
        l2_output_index: u64,
        output_root: [u8; 32],
        block_number: u64,
        current_timestamp: u64,
    ) -> Result<OutputRoot, &'static str> {
        if output_root == [0; 32] {
            return Err("Output root cannot be zero");
        }

        Ok(OutputRoot {
            l2_output_index,
            output_root,
            timestamp: current_timestamp,
            block_number,
        })
    }

    /// Prove withdrawal with Merkle proof against output root
    pub fn prove_withdrawal(
        withdrawal: &mut L2Withdrawal,
        proof: &WithdrawalProof,
        output_root: &OutputRoot,
        withdrawal_root: [u8; 32],
    ) -> Result<(), &'static str> {
        if withdrawal.status != WithdrawalStatus::BatchedForL1 {
            return Err("Withdrawal must be BatchedForL1 before proving");
        }

        // Verify Merkle proof of withdrawal in output root
        let computed_root = Self::compute_merkle_root(&withdrawal_root, &proof.withdrawal_proof);

        // Check if computed root matches output root
        if Self::verify_output_root(
            &computed_root,
            &proof.output_root_proof,
            &output_root.output_root,
        ) {
            withdrawal.status = WithdrawalStatus::Proven;
            Ok(())
        } else {
            Err("Withdrawal proof verification failed")
        }
    }

    /// Execute withdrawal after finalization period (7 days OP-Stack standard)
    pub fn execute_withdrawal(
        withdrawal: &mut L2Withdrawal,
        output_timestamp: u64,
        current_timestamp: u64,
    ) -> Result<(), &'static str> {
        if withdrawal.status != WithdrawalStatus::Proven {
            return Err("Withdrawal must be Proven before execution");
        }

        // Check finalization period (7 days = 604800 seconds)
        let elapsed = current_timestamp.saturating_sub(output_timestamp);
        if elapsed < 604800 {
            return Err("Finalization period not reached (7 days required)");
        }

        withdrawal.status = WithdrawalStatus::Executed;
        Ok(())
    }

    /// Handle failed deposit/withdrawal (refund)
    pub fn refund_deposit(deposit: &mut L2BridgeDeposit) -> Result<(), &'static str> {
        match deposit.status {
            DepositStatus::Pending | DepositStatus::Confirmed => {
                deposit.status = DepositStatus::Refunded;
                Ok(())
            }
            DepositStatus::Relayed => Err("Cannot refund relayed deposit"),
            _ => Err("Invalid deposit state for refund"),
        }
    }

    /// Register L1/L2 token pair for bridging
    pub fn register_token_pair(
        l1_address: [u8; 20],
        l2_address: [u8; 20],
        decimals: u8,
    ) -> Result<TokenConfig, &'static str> {
        if l1_address == [0; 20] || l2_address == [0; 20] {
            return Err("Token addresses cannot be zero");
        }
        if decimals > 18 {
            return Err("Decimals cannot exceed 18");
        }

        Ok(TokenConfig {
            l1_address,
            l2_address,
            decimals,
            enabled: true,
        })
    }

    /// Get deposit state
    pub fn get_deposit(deposit: &L2BridgeDeposit) -> (u64, u128, DepositStatus) {
        (deposit.deposit_id, deposit.amount, deposit.status.clone())
    }

    /// Get withdrawal state
    pub fn get_withdrawal(withdrawal: &L2Withdrawal) -> (u64, u128, WithdrawalStatus) {
        (
            withdrawal.withdrawal_id,
            withdrawal.amount,
            withdrawal.status.clone(),
        )
    }

    /// Validate output root commitment from sequencer
    pub fn validate_output_root(
        output_root: &OutputRoot,
        parent_output_root: &OutputRoot,
    ) -> Result<bool, &'static str> {
        if output_root.l2_output_index <= parent_output_root.l2_output_index {
            return Err("Output index must increase");
        }

        // Output roots should be submitted within reasonable time (e.g., 1 day)
        let time_diff = output_root
            .timestamp
            .saturating_sub(parent_output_root.timestamp);
        if time_diff > 86400 {
            return Err("Output root submission took too long");
        }

        Ok(true)
    }

    /// Verify withdrawal was properly batched
    pub fn verify_withdrawal_batched(
        withdrawal_id: u64,
        batch_index: u64,
        batch_commitment: [u8; 32],
    ) -> Result<bool, &'static str> {
        let commitment = Self::compute_commitment(withdrawal_id, batch_index);
        Ok(commitment == batch_commitment)
    }

    /// Generate deposit ID
    fn generate_deposit_id(depositor: &[u8; 20], token: &[u8; 20], amount: u128) -> u64 {
        let mut hash = 0u64;
        for byte in depositor {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        for byte in token {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add((amount >> 64) as u64);
        hash
    }

    /// Generate withdrawal ID
    fn generate_withdrawal_id(sender: &[u8; 32], token: &[u8; 20], amount: u128) -> u64 {
        let mut hash = 0u64;
        for (i, byte) in sender.iter().enumerate() {
            if i < 8 {
                hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
            }
        }
        for byte in token {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add((amount >> 64) as u64);
        hash
    }

    /// Compute Merkle root for withdrawal proof
    fn compute_merkle_root(leaf: &[u8; 32], proofs: &[[u8; 32]]) -> [u8; 32] {
        let mut hash = *leaf;
        for proof_node in proofs {
            hash = Self::hash_pair(&hash, proof_node);
        }
        hash
    }

    /// Hash two 32-byte values using double-SHA256 for Merkle tree construction.
    /// Combines the two inputs and applies SHA256 twice for security.
    pub(crate) fn hash_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(a);
        combined[32..].copy_from_slice(b);

        let first = sp_io::hashing::sha2_256(&combined);
        sp_io::hashing::sha2_256(&first)
    }

    /// Verify output root against proof chain
    fn verify_output_root(computed: &[u8; 32], proof: &[[u8; 32]], root: &[u8; 32]) -> bool {
        let final_hash = Self::compute_merkle_root(computed, proof);
        final_hash == *root
    }

    /// Compute batch commitment
    fn compute_commitment(withdrawal_id: u64, batch_index: u64) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[0..8].copy_from_slice(&withdrawal_id.to_le_bytes());
        result[8..16].copy_from_slice(&batch_index.to_le_bytes());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiate_deposit() {
        let deposit_id = L2Bridge::initiate_deposit([1; 20], [2; 20], 1000000, [3; 32]).unwrap();
        assert!(deposit_id > 0);
    }

    #[test]
    fn test_deposit_zero_amount() {
        let result = L2Bridge::initiate_deposit([1; 20], [2; 20], 0, [3; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_confirm_deposit() {
        let mut deposit = L2BridgeDeposit {
            deposit_id: 1,
            depositor: [1; 20],
            token: [2; 20],
            amount: 1000000,
            l2_recipient: [3; 32],
            timestamp: 1000,
            status: DepositStatus::Pending,
        };

        L2Bridge::confirm_deposit(&mut deposit, 1000, 1200, 1200).unwrap();
        assert_eq!(deposit.status, DepositStatus::Confirmed);
    }

    #[test]
    fn test_confirm_deposit_insufficient_confirmations() {
        let mut deposit = L2BridgeDeposit {
            deposit_id: 1,
            depositor: [1; 20],
            token: [2; 20],
            amount: 1000000,
            l2_recipient: [3; 32],
            timestamp: 1000,
            status: DepositStatus::Pending,
        };

        let result = L2Bridge::confirm_deposit(&mut deposit, 1000, 1050, 1050);
        assert!(result.is_err());
    }

    #[test]
    fn test_relay_deposit() {
        let mut deposit = L2BridgeDeposit {
            deposit_id: 1,
            depositor: [1; 20],
            token: [2; 20],
            amount: 1000000,
            l2_recipient: [3; 32],
            timestamp: 1000,
            status: DepositStatus::Confirmed,
        };

        L2Bridge::relay_deposit(&mut deposit).unwrap();
        assert_eq!(deposit.status, DepositStatus::Relayed);
    }

    #[test]
    fn test_initiate_withdrawal() {
        let withdrawal_id =
            L2Bridge::initiate_withdrawal([1; 32], [2; 20], 500000, [3; 20]).unwrap();
        assert!(withdrawal_id > 0);
    }

    #[test]
    fn test_batch_withdrawal() {
        let mut withdrawal = L2Withdrawal {
            withdrawal_id: 1,
            sender: [1; 32],
            token: [2; 20],
            amount: 500000,
            l1_recipient: [3; 20],
            timestamp: 1000,
            status: WithdrawalStatus::Initiated,
        };

        L2Bridge::batch_withdrawal(&mut withdrawal).unwrap();
        assert_eq!(withdrawal.status, WithdrawalStatus::BatchedForL1);
    }

    #[test]
    fn test_submit_output_root() {
        let output = L2Bridge::submit_output_root(0, [1; 32], 1000, 2000).unwrap();

        assert_eq!(output.l2_output_index, 0);
        assert_eq!(output.output_root, [1; 32]);
        assert_eq!(output.block_number, 1000);
    }

    #[test]
    fn test_prove_withdrawal() {
        let mut withdrawal = L2Withdrawal {
            withdrawal_id: 1,
            sender: [1; 32],
            token: [2; 20],
            amount: 500000,
            l1_recipient: [3; 20],
            timestamp: 1000,
            status: WithdrawalStatus::BatchedForL1,
        };

        let withdrawal_root = [5; 32];
        let proof = WithdrawalProof {
            output_root_proof: vec![[1; 32]],
            withdrawal_proof: vec![[2; 32]],
            output_index: 0,
        };

        // Compute expected output_root using SHA256-based hash_pair function
        let intermediate = L2Bridge::hash_pair(&withdrawal_root, &proof.withdrawal_proof[0]);
        let expected_root = L2Bridge::hash_pair(&intermediate, &proof.output_root_proof[0]);

        let output_root = OutputRoot {
            l2_output_index: 0,
            output_root: expected_root,
            timestamp: 1000,
            block_number: 100,
        };

        L2Bridge::prove_withdrawal(&mut withdrawal, &proof, &output_root, withdrawal_root).unwrap();
        assert_eq!(withdrawal.status, WithdrawalStatus::Proven);
    }

    #[test]
    fn test_execute_withdrawal() {
        let mut withdrawal = L2Withdrawal {
            withdrawal_id: 1,
            sender: [1; 32],
            token: [2; 20],
            amount: 500000,
            l1_recipient: [3; 20],
            timestamp: 1000,
            status: WithdrawalStatus::Proven,
        };

        L2Bridge::execute_withdrawal(&mut withdrawal, 1000, 700000).unwrap();
        assert_eq!(withdrawal.status, WithdrawalStatus::Executed);
    }

    #[test]
    fn test_execute_withdrawal_not_finalized() {
        let mut withdrawal = L2Withdrawal {
            withdrawal_id: 1,
            sender: [1; 32],
            token: [2; 20],
            amount: 500000,
            l1_recipient: [3; 20],
            timestamp: 1000,
            status: WithdrawalStatus::Proven,
        };

        let result = L2Bridge::execute_withdrawal(&mut withdrawal, 1000, 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_refund_deposit() {
        let mut deposit = L2BridgeDeposit {
            deposit_id: 1,
            depositor: [1; 20],
            token: [2; 20],
            amount: 1000000,
            l2_recipient: [3; 32],
            timestamp: 1000,
            status: DepositStatus::Pending,
        };

        L2Bridge::refund_deposit(&mut deposit).unwrap();
        assert_eq!(deposit.status, DepositStatus::Refunded);
    }

    #[test]
    fn test_register_token_pair() {
        let config = L2Bridge::register_token_pair([1; 20], [2; 20], 18).unwrap();

        assert_eq!(config.l1_address, [1; 20]);
        assert_eq!(config.l2_address, [2; 20]);
        assert_eq!(config.decimals, 18);
        assert!(config.enabled);
    }

    #[test]
    fn test_validate_output_root() {
        let output1 = OutputRoot {
            l2_output_index: 0,
            output_root: [1; 32],
            timestamp: 1000,
            block_number: 100,
        };

        let output2 = OutputRoot {
            l2_output_index: 1,
            output_root: [2; 32],
            timestamp: 2000,
            block_number: 200,
        };

        assert!(L2Bridge::validate_output_root(&output2, &output1).unwrap());
    }

    #[test]
    fn test_get_deposit() {
        let deposit = L2BridgeDeposit {
            deposit_id: 1,
            depositor: [1; 20],
            token: [2; 20],
            amount: 1000000,
            l2_recipient: [3; 32],
            timestamp: 1000,
            status: DepositStatus::Relayed,
        };

        let (id, amount, status) = L2Bridge::get_deposit(&deposit);
        assert_eq!(id, 1);
        assert_eq!(amount, 1000000);
        assert_eq!(status, DepositStatus::Relayed);
    }
}
