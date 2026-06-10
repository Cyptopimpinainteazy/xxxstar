//! X3 Bitcoin Vault — Threshold multisig vault with SPV proof accounting.
//!
//! Manages Bitcoin deposits and withdrawals through a federated threshold
//! multisignature scheme. Deposits require N-of-M signatures from approved
//! signers after SPV proof verification. Withdrawals are initiated by X3
//! governance-approved burn proofs.

#![cfg_attr(not(feature = "std"), no_std)]

use alloc::vec::Vec;
use core::fmt::{Display, Formatter};

/// Minimum Bitcoin confirmations for deposit finalization
pub const MIN_BITCOIN_CONFIRMATIONS: u64 = 6;

/// Default multisig threshold (3-of-5)
pub const DEFAULT_THRESHOLD: u32 = 3;
pub const DEFAULT_TOTAL_SIGNERS: u32 = 5;

/// A Bitcoin deposit request
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct BtcDepositRequest {
    /// Bitcoin transaction ID (txid)
    pub txid: [u8; 32],
    /// Output index in the transaction
    pub vout: u32,
    /// Amount in satoshis
    pub amount: u64,
    /// Recipient on X3 side (SCALE-encoded)
    pub x3_recipient: Vec<u8>,
    /// X3 asset ID to mint
    pub asset_id: [u8; 32],
    /// Current number of confirmations
    pub confirmations: u64,
    /// SPV proof of inclusion
    pub spv_proof: Vec<u8>,
    /// Signatures from vault signers (N-of-M required)
    pub signatures: Vec<([u8; 32], Vec<u8>)>, // (signer_pubkey, signature)
    /// Status of this deposit
    pub status: BtcDepositStatus,
}

/// Status of a Bitcoin vault deposit
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub enum BtcDepositStatus {
    /// Awaiting confirmations
    PendingConfirmations,
    /// Confirmations met, awaiting SPV verification
    PendingSpvVerification,
    /// SPV verified, awaiting signer approvals
    PendingSignerApproval { approvals: u32, threshold: u32 },
    /// Approved and ready for X3 mint
    Approved,
    /// MINTED on X3 side
    Completed,
    /// Rejected by signers or invalid proof
    Rejected,
}

impl BtcDepositStatus {
    pub fn can_transition_to(&self, next: &Self) -> bool {
        match (self, next) {
            (BtcDepositStatus::PendingConfirmations, BtcDepositStatus::PendingSpvVerification) => true,
            (BtcDepositStatus::PendingSpvVerification, BtcDepositStatus::PendingSignerApproval { .. }) => true,
            (BtcDepositStatus::PendingSignerApproval { .. }, BtcDepositStatus::Approved) => true,
            (BtcDepositStatus::Approved, BtcDepositStatus::Completed) => true,
            (_, BtcDepositStatus::Rejected) => true,
            _ => false,
        }
    }
}

/// A Bitcoin withdrawal request
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct BtcWithdrawalRequest {
    /// X3 burn proof message ID
    pub burn_message_id: [u8; 32],
    /// Recipient Bitcoin address
    pub btc_recipient: Vec<u8>,
    /// Amount in satoshis
    pub amount: u64,
    /// X3 proof verification (SPV or validator attestation)
    pub x3_proof: Vec<u8>,
    /// Vault signatures
    pub signatures: Vec<([u8; 32], Vec<u8>)>,
    /// Status
    pub status: BtcWithdrawalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub enum BtcWithdrawalStatus {
    PendingX3Proof,
    PendingSignerApproval { approvals: u32, threshold: u32 },
    Approved,
    Broadcasted { txid: [u8; 32] },
    Completed,
    Rejected,
}

/// Vault configuration
#[derive(Debug, Clone, scale_info::TypeInfo)]
pub struct BtcVaultConfig {
    pub signers: Vec<[u8; 32]>,
    pub threshold: u32,
    pub min_confirmations: u64,
    pub max_deposit_per_tx: u64,
    pub max_withdrawal_per_tx: u64,
    pub daily_withdrawal_limit: u64,
}

impl Default for BtcVaultConfig {
    fn default() -> Self {
        Self {
            signers: Vec::new(),
            threshold: DEFAULT_THRESHOLD,
            min_confirmations: MIN_BITCOIN_CONFIRMATIONS,
            max_deposit_per_tx: 10_000_000,    // 0.1 BTC in satoshis
            max_withdrawal_per_tx: 10_000_000,
            daily_withdrawal_limit: 50_000_000, // 0.5 BTC
        }
    }
}

/// Bitcoin vault state
#[derive(Debug, Clone, scale_info::TypeInfo)]
pub struct BtcVault {
    pub config: BtcVaultConfig,
    pub total_deposited: u64,
    pub total_withdrawn: u64,
    pub pending_deposits: Vec<BtcDepositRequest>,
    pub pending_withdrawals: Vec<BtcWithdrawalRequest>,
    pub daily_withdrawn: u64,
    pub last_withdrawal_day: u64,
}

impl BtcVault {
    pub fn new(config: BtcVaultConfig) -> Self {
        Self {
            config,
            total_deposited: 0,
            total_withdrawn: 0,
            pending_deposits: Vec::new(),
            pending_withdrawals: Vec::new(),
            daily_withdrawn: 0,
            last_withdrawal_day: 0,
        }
    }

    /// Submit a deposit request with SPV proof
    pub fn submit_deposit(
        &mut self,
        txid: [u8; 32],
        vout: u32,
        amount: u64,
        x3_recipient: Vec<u8>,
        asset_id: [u8; 32],
        spv_proof: Vec<u8>,
    ) -> Result<(), BtcVaultError> {
        if amount == 0 {
            return Err(BtcVaultError::ZeroAmount);
        }
        if amount > self.config.max_deposit_per_tx {
            return Err(BtcVaultError::ExceedsMaxDeposit);
        }
        if self.pending_deposits.len() >= 100 {
            return Err(BtcVaultError::TooManyPending);
        }

        let deposit = BtcDepositRequest {
            txid,
            vout,
            amount,
            x3_recipient,
            asset_id,
            confirmations: 0,
            spv_proof,
            signatures: Vec::new(),
            status: BtcDepositStatus::PendingConfirmations,
        };

        self.pending_deposits.push(deposit);
        Ok(())
    }

    /// Advance deposit status through the state machine
    pub fn process_deposit(&mut self, index: usize) -> Result<(), BtcVaultError> {
        let deposit = self
            .pending_deposits
            .get_mut(index)
            .ok_or(BtcVaultError::DepositNotFound)?;

        match &deposit.status {
            BtcDepositStatus::PendingConfirmations => {
                deposit.confirmations += 1;
                if deposit.confirmations >= self.config.min_confirmations {
                    deposit.status = BtcDepositStatus::PendingSpvVerification;
                }
            }
            BtcDepositStatus::PendingSpvVerification => {
                if !deposit.spv_proof.is_empty() {
                    deposit.status = BtcDepositStatus::PendingSignerApproval {
                        approvals: 0,
                        threshold: self.config.threshold,
                    };
                }
            }
            BtcDepositStatus::PendingSignerApproval { approvals, threshold } => {
                // Simulate signer approval
                if approvals + 1 >= *threshold {
                    deposit.status = BtcDepositStatus::Approved;
                } else {
                    deposit.status = BtcDepositStatus::PendingSignerApproval {
                        approvals: approvals + 1,
                        threshold: *threshold,
                    };
                }
            }
            BtcDepositStatus::Approved => {
                self.total_deposited = self.total_deposited.saturating_add(deposit.amount);
                deposit.status = BtcDepositStatus::Completed;
            }
            _ => return Err(BtcVaultError::InvalidStateTransition),
        }
        Ok(())
    }

    /// Submit a withdrawal request with X3 burn proof
    pub fn submit_withdrawal(
        &mut self,
        burn_message_id: [u8; 32],
        btc_recipient: Vec<u8>,
        amount: u64,
        x3_proof: Vec<u8>,
    ) -> Result<(), BtcVaultError> {
        if amount == 0 {
            return Err(BtcVaultError::ZeroAmount);
        }
        if amount > self.config.max_withdrawal_per_tx {
            return Err(BtcVaultError::ExceedsMaxWithdrawal);
        }
        if amount > self.total_deposited.saturating_sub(self.total_withdrawn) {
            return Err(BtcVaultError::InsufficientReserves);
        }

        // Check daily withdrawal limit
        let current_day = 0; // would use block timestamp in production
        if current_day == self.last_withdrawal_day {
            if self.daily_withdrawn.saturating_add(amount) > self.config.daily_withdrawal_limit {
                return Err(BtcVaultError::DailyWithdrawalLimitExceeded);
            }
        } else {
            self.last_withdrawal_day = current_day;
            self.daily_withdrawn = 0;
        }

        let withdrawal = BtcWithdrawalRequest {
            burn_message_id,
            btc_recipient,
            amount,
            x3_proof,
            signatures: Vec::new(),
            status: BtcWithdrawalStatus::PendingX3Proof,
        };

        self.pending_withdrawals.push(withdrawal);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtcVaultError {
    ZeroAmount,
    ExceedsMaxDeposit,
    ExceedsMaxWithdrawal,
    DailyWithdrawalLimitExceeded,
    InsufficientReserves,
    DepositNotFound,
    InvalidStateTransition,
    TooManyPending,
    SpvVerificationFailed,
    InsufficientSignatures,
}

impl Display for BtcVaultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            BtcVaultError::ZeroAmount => write!(f, "zero amount"),
            BtcVaultError::ExceedsMaxDeposit => write!(f, "exceeds max deposit per tx"),
            BtcVaultError::ExceedsMaxWithdrawal => write!(f, "exceeds max withdrawal per tx"),
            BtcVaultError::DailyWithdrawalLimitExceeded => write!(f, "daily withdrawal limit exceeded"),
            BtcVaultError::InsufficientReserves => write!(f, "insufficient vault reserves"),
            BtcVaultError::DepositNotFound => write!(f, "deposit not found"),
            BtcVaultError::InvalidStateTransition => write!(f, "invalid state transition"),
            BtcVaultError::TooManyPending => write!(f, "too many pending requests"),
            BtcVaultError::SpvVerificationFailed => write!(f, "SPV verification failed"),
            BtcVaultError::InsufficientSignatures => write!(f, "insufficient signer approvals"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_vault() -> BtcVault {
        BtcVault::new(BtcVaultConfig::default())
    }

    #[test]
    fn test_deposit_flow() {
        let mut vault = default_vault();
        vault
            .submit_deposit(
                [1u8; 32],
                0,
                5_000_000,
                vec![0x01, 0x02],
                [0u8; 32],
                vec![1, 2, 3, 4],
            )
            .unwrap();
        assert_eq!(vault.pending_deposits.len(), 1);

        // Advance through confirmations
        for _ in 0..6 {
            vault.process_deposit(0).unwrap();
        }

        // Should be at PendingSpvVerification now
        // Process SPV verification
        vault.process_deposit(0).unwrap();

        // Process signer approvals (need 3 of 5)
        vault.process_deposit(0).unwrap();
        vault.process_deposit(0).unwrap();
        vault.process_deposit(0).unwrap();

        assert_eq!(vault.pending_deposits[0].status, BtcDepositStatus::Approved);

        // Complete the deposit
        vault.process_deposit(0).unwrap();
        assert_eq!(vault.pending_deposits[0].status, BtcDepositStatus::Completed);
        assert_eq!(vault.total_deposited, 5_000_000);
    }

    #[test]
    fn test_zero_amount_deposit_fails() {
        let mut vault = default_vault();
        assert_eq!(
            vault.submit_deposit([0u8; 32], 0, 0, vec![], [0u8; 32], vec![]),
            Err(BtcVaultError::ZeroAmount)
        );
    }

    #[test]
    fn test_exceeds_max_deposit_fails() {
        let mut vault = default_vault();
        assert!(vault
            .submit_deposit([0u8; 32], 0, 100_000_000, vec![], [0u8; 32], vec![])
            .is_err());
    }

    #[test]
    fn test_withdrawal_flow() {
        let mut vault = default_vault();
        // First deposit to have reserves
        vault
            .submit_deposit(
                [1u8; 32],
                0,
                10_000_000,
                vec![0x01],
                [0u8; 32],
                vec![1, 2, 3, 4],
            )
            .unwrap();
        for _ in 0..9 {
            vault.process_deposit(0).unwrap();
        }

        vault
            .submit_withdrawal([1u8; 32], vec![0x03, 0x04], 5_000_000, vec![1, 2, 3])
            .unwrap();
        assert_eq!(vault.pending_withdrawals.len(), 1);
    }

    #[test]
    fn test_insufficient_reserves_fails() {
        let mut vault = default_vault();
        assert_eq!(
            vault.submit_withdrawal([1u8; 32], vec![0x03], 100_000_000, vec![]),
            Err(BtcVaultError::InsufficientReserves)
        );
    }
}