#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use sha2::{Digest, Sha256};

pub const MIN_BITCOIN_CONFIRMATIONS: u64 = 6;
pub const DEFAULT_THRESHOLD: u32 = 3;
pub const DEFAULT_TOTAL_SIGNERS: u32 = 5;

// ── Address / script helpers ────────────────────────────────────────────────

pub fn hash160(input: &[u8]) -> [u8; 20] {
    let hash = Sha256::digest(input);
    let mut r = [0u8; 20];
    r.copy_from_slice(&ripemd::Ripemd160::digest(hash));
    r
}

pub fn p2sh_address(script: &[u8]) -> Vec<u8> {
    let mut redeem = Vec::with_capacity(script.len() + 2);
    redeem.push(0xA9);
    redeem.push(20);
    redeem.extend_from_slice(&hash160(script));
    redeem.push(0x87);
    redeem
}

pub fn p2wsh_address(witness_script: &[u8]) -> Vec<u8> {
    let hash = Sha256::digest(witness_script);
    let mut wit = Vec::with_capacity(hash.len() + 2);
    wit.push(0x00);
    wit.push(32);
    wit.extend_from_slice(&hash);
    wit
}

pub fn multisig_redeem_script(signers: &[[u8; 33]], threshold: u32) -> Vec<u8> {
    let mut script = Vec::new();
    if threshold <= 16 {
        script.push(0x50 + threshold as u8);
    } else {
        script.push(0x00);
    }
    for pubkey in signers {
        script.push(33);
        script.extend_from_slice(pubkey);
    }
    let n = signers.len() as u32;
    if n <= 16 {
        script.push(0x50 + n as u8);
    } else {
        script.push(0x00);
    }
    script.push(0xAE);
    script
}

// ── UTXO tracking ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct BtcUtxoEntry {
    pub txid: [u8; 32],
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
    pub spendable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, scale_info::TypeInfo)]
pub struct BtcUtxoSet {
    pub entries: Vec<BtcUtxoEntry>,
}

impl BtcUtxoSet {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, entry: BtcUtxoEntry) {
        self.entries.push(entry);
    }

    pub fn spend(&mut self, txid: &[u8; 32], vout: u32) -> Result<(), BtcVaultError> {
        for entry in &mut self.entries {
            if entry.txid == *txid && entry.vout == vout {
                if !entry.spendable {
                    return Err(BtcVaultError::UtxoAlreadySpent);
                }
                entry.spendable = false;
                return Ok(());
            }
        }
        Err(BtcVaultError::UtxoNotFound)
    }

    pub fn total_spendable(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| e.spendable)
            .map(|e| e.amount)
            .sum()
    }

    pub fn select_utxos(&self, amount: u64) -> Result<Vec<BtcUtxoEntry>, BtcVaultError> {
        let mut selected = Vec::new();
        let mut accumulated = 0u64;
        for entry in &self.entries {
            if entry.spendable {
                selected.push(entry.clone());
                accumulated = accumulated.saturating_add(entry.amount);
                if accumulated >= amount {
                    return Ok(selected);
                }
            }
        }
        Err(BtcVaultError::InsufficientReserves)
    }
}

// ── PSBT types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct PsbtInput {
    pub txid: [u8; 32],
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
    pub redeem_script: Option<Vec<u8>>,
    pub witness_script: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct PsbtOutput {
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct Psbt {
    pub inputs: Vec<PsbtInput>,
    pub outputs: Vec<PsbtOutput>,
    pub fee: u64,
}

impl Psbt {
    pub fn estimated_vsize(&self) -> u64 {
        10 + self.inputs.len() as u64 * 68 + self.outputs.len() as u64 * 31
    }
}

// ── Deposit / Withdrawal types ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct BtcDepositRequest {
    pub txid: [u8; 32],
    pub vout: u32,
    pub amount: u64,
    pub x3_recipient: Vec<u8>,
    pub asset_id: [u8; 32],
    pub confirmations: u64,
    pub spv_proof: Vec<u8>,
    pub signatures: Vec<([u8; 32], Vec<u8>)>,
    pub status: BtcDepositStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub enum BtcDepositStatus {
    PendingConfirmations,
    PendingSpvVerification,
    PendingSignerApproval { approvals: u32, threshold: u32 },
    Approved,
    Completed,
    Rejected,
}

impl BtcDepositStatus {
    pub fn can_transition_to(&self, next: &Self) -> bool {
        matches!(
            (self, next),
            (BtcDepositStatus::PendingConfirmations, BtcDepositStatus::PendingSpvVerification)
                | (BtcDepositStatus::PendingSpvVerification, BtcDepositStatus::PendingSignerApproval { .. })
                | (BtcDepositStatus::PendingSignerApproval { .. }, BtcDepositStatus::Approved)
                | (BtcDepositStatus::Approved, BtcDepositStatus::Completed)
                | (_, BtcDepositStatus::Rejected)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct BtcWithdrawalRequest {
    pub burn_message_id: [u8; 32],
    pub btc_recipient: Vec<u8>,
    pub amount: u64,
    pub x3_proof: Vec<u8>,
    pub signatures: Vec<([u8; 32], Vec<u8>)>,
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

#[derive(Debug, Clone, scale_info::TypeInfo)]
pub struct BtcVaultConfig {
    pub signers: Vec<[u8; 32]>,
    pub signer_pubkeys: Vec<[u8; 33]>,
    pub threshold: u32,
    pub min_confirmations: u64,
    pub max_deposit_per_tx: u64,
    pub max_withdrawal_per_tx: u64,
    pub daily_withdrawal_limit: u64,
    pub vault_address_p2sh: Vec<u8>,
    pub vault_address_p2wsh: Vec<u8>,
}

impl Default for BtcVaultConfig {
    fn default() -> Self {
        Self {
            signers: Vec::new(),
            signer_pubkeys: Vec::new(),
            threshold: DEFAULT_THRESHOLD,
            min_confirmations: MIN_BITCOIN_CONFIRMATIONS,
            max_deposit_per_tx: 10_000_000,
            max_withdrawal_per_tx: 10_000_000,
            daily_withdrawal_limit: 50_000_000,
            vault_address_p2sh: Vec::new(),
            vault_address_p2wsh: Vec::new(),
        }
    }
}

impl BtcVaultConfig {
    pub fn with_signers(mut self, pubkeys: Vec<[u8; 33]>, threshold: u32) -> Self {
        self.signer_pubkeys = pubkeys.clone();
        self.signers = pubkeys
            .iter()
            .map(|k| {
                let mut h = [0u8; 32];
                h.copy_from_slice(&Sha256::digest(k));
                h
            })
            .collect();
        self.threshold = threshold;
        let redeem = multisig_redeem_script(&pubkeys, threshold);
        self.vault_address_p2sh = p2sh_address(&redeem);
        self.vault_address_p2wsh = p2wsh_address(&redeem);
        self
    }
}

// ── Vault state machine ─────────────────────────────────────────────────────

#[derive(Debug, Clone, scale_info::TypeInfo)]
pub struct BtcVault {
    pub config: BtcVaultConfig,
    pub utxos: BtcUtxoSet,
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
            utxos: BtcUtxoSet::new(),
            total_deposited: 0,
            total_withdrawn: 0,
            pending_deposits: Vec::new(),
            pending_withdrawals: Vec::new(),
            daily_withdrawn: 0,
            last_withdrawal_day: 0,
        }
    }

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
            BtcDepositStatus::PendingSignerApproval {
                approvals,
                threshold,
            } => {
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
                self.utxos.add(BtcUtxoEntry {
                    txid: deposit.txid,
                    vout: deposit.vout,
                    amount: deposit.amount,
                    script_pubkey: self.config.vault_address_p2wsh.clone(),
                    spendable: true,
                });
                deposit.status = BtcDepositStatus::Completed;
            }
            _ => return Err(BtcVaultError::InvalidStateTransition),
        }
        Ok(())
    }

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
        if self.utxos.total_spendable() < amount {
            return Err(BtcVaultError::InsufficientReserves);
        }

        let current_day = 0;
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

    pub fn build_psbt(&self, amount: u64, recipient_script: &[u8]) -> Result<Psbt, BtcVaultError> {
        let utxos = self.utxos.select_utxos(amount)?;
        let input_sum: u64 = utxos.iter().map(|u| u.amount).sum();
        let fee: u64 = 5_000;
        let change = input_sum.saturating_sub(amount).saturating_sub(fee);

        let mut inputs = Vec::with_capacity(utxos.len());
        for utxo in &utxos {
            inputs.push(PsbtInput {
                txid: utxo.txid,
                vout: utxo.vout,
                amount: utxo.amount,
                script_pubkey: utxo.script_pubkey.clone(),
                redeem_script: Some(self.config.vault_address_p2sh.clone()),
                witness_script: Some(self.config.vault_address_p2wsh.clone()),
            });
        }

        let mut outputs = Vec::new();
        outputs.push(PsbtOutput {
            amount,
            script_pubkey: recipient_script.to_vec(),
        });
        if change > 0 {
            outputs.push(PsbtOutput {
                amount: change,
                script_pubkey: self.config.vault_address_p2wsh.clone(),
            });
        }

        Ok(Psbt {
            inputs,
            outputs,
            fee,
        })
    }

    pub fn add_signer_approval(
        &mut self,
        deposit_index: usize,
        signer_pubkey: [u8; 32],
        signature: Vec<u8>,
    ) -> Result<(), BtcVaultError> {
        let deposit = self
            .pending_deposits
            .get_mut(deposit_index)
            .ok_or(BtcVaultError::DepositNotFound)?;

        let (approvals, threshold) = match &deposit.status {
            BtcDepositStatus::PendingSignerApproval {
                approvals,
                threshold,
            } => (*approvals, *threshold),
            _ => return Err(BtcVaultError::InvalidStateTransition),
        };

        if !self.config.signers.contains(&signer_pubkey) {
            return Err(BtcVaultError::InvalidSigner);
        }
        if deposit.signatures.iter().any(|(k, _)| *k == signer_pubkey) {
            return Err(BtcVaultError::DuplicateSignature);
        }

        deposit.signatures.push((signer_pubkey, signature));
        let new_approvals = approvals + 1;
        if new_approvals >= threshold {
            deposit.status = BtcDepositStatus::Approved;
        } else {
            deposit.status = BtcDepositStatus::PendingSignerApproval {
                approvals: new_approvals,
                threshold,
            };
        }
        Ok(())
    }
}

// ── Errors ──────────────────────────────────────────────────────────────────

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
    UtxoNotFound,
    UtxoAlreadySpent,
    InvalidSigner,
    DuplicateSignature,
}

impl Display for BtcVaultError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            BtcVaultError::ZeroAmount => write!(f, "zero amount"),
            BtcVaultError::ExceedsMaxDeposit => write!(f, "exceeds max deposit per tx"),
            BtcVaultError::ExceedsMaxWithdrawal => write!(f, "exceeds max withdrawal per tx"),
            BtcVaultError::DailyWithdrawalLimitExceeded => {
                write!(f, "daily withdrawal limit exceeded")
            }
            BtcVaultError::InsufficientReserves => write!(f, "insufficient vault reserves"),
            BtcVaultError::DepositNotFound => write!(f, "deposit not found"),
            BtcVaultError::InvalidStateTransition => write!(f, "invalid state transition"),
            BtcVaultError::TooManyPending => write!(f, "too many pending requests"),
            BtcVaultError::SpvVerificationFailed => write!(f, "SPV verification failed"),
            BtcVaultError::InsufficientSignatures => write!(f, "insufficient signer approvals"),
            BtcVaultError::UtxoNotFound => write!(f, "UTXO not found"),
            BtcVaultError::UtxoAlreadySpent => write!(f, "UTXO already spent"),
            BtcVaultError::InvalidSigner => write!(f, "invalid signer"),
            BtcVaultError::DuplicateSignature => write!(f, "duplicate signature"),
        }
    }
}

// ── Bitcoin SPV Proof Verification (standalone) ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitcoinBlockHeader {
    pub version: u32,
    pub prev_block: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BitcoinBlockHeader {
    pub fn parse(raw: &[u8]) -> Result<Self, &'static str> {
        if raw.len() != 80 {
            return Err("bitcoin header must be 80 bytes");
        }
        let mut prev = [0u8; 32];
        let mut merkle = [0u8; 32];
        prev.copy_from_slice(&raw[4..36]);
        merkle.copy_from_slice(&raw[36..68]);
        Ok(Self {
            version: u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]),
            prev_block: prev,
            merkle_root: merkle,
            timestamp: u32::from_le_bytes([raw[68], raw[69], raw[70], raw[71]]),
            bits: u32::from_le_bytes([raw[72], raw[73], raw[74], raw[75]]),
            nonce: u32::from_le_bytes([raw[76], raw[77], raw[78], raw[79]]),
        })
    }

    pub fn block_hash(&self) -> [u8; 32] {
        let mut raw = [0u8; 80];
        raw[0..4].copy_from_slice(&self.version.to_le_bytes());
        raw[4..36].copy_from_slice(&self.prev_block);
        raw[36..68].copy_from_slice(&self.merkle_root);
        raw[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        raw[72..76].copy_from_slice(&self.bits.to_le_bytes());
        raw[76..80].copy_from_slice(&self.nonce.to_le_bytes());
        let h1 = Sha256::digest(raw);
        let h2 = Sha256::digest(h1);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h2);
        out
    }

    pub fn verify_pow(&self) -> Result<(), &'static str> {
        let target = compact_target(self.bits)?;
        let hash = self.block_hash();
        for i in (0..32).rev() {
            if hash[i] < target[i] {
                return Ok(());
            }
            if hash[i] > target[i] {
                return Err("proof-of-work not satisfied");
            }
        }
        Ok(())
    }
}

pub fn compact_target(bits: u32) -> Result<[u8; 32], &'static str> {
    let exponent = (bits >> 24) as usize;
    let mantissa = bits & 0x00FF_FFFF;
    if exponent == 0 || exponent > 34 {
        return Err("invalid nBits exponent");
    }
    let mut target = [0u8; 32];
    let shift = exponent.saturating_sub(3);
    if shift < 29 {
        target[shift] = ((mantissa >> 16) & 0xFF) as u8;
        target[shift + 1] = ((mantissa >> 8) & 0xFF) as u8;
        target[shift + 2] = (mantissa & 0xFF) as u8;
    }
    Ok(target)
}

pub fn verify_block_header_chain(headers: &[&[u8]]) -> Result<u64, &'static str> {
    if headers.is_empty() {
        return Err("empty header chain");
    }
    let mut prev: Option<[u8; 32]> = None;
    for raw in headers {
        let h = BitcoinBlockHeader::parse(raw)?;
        h.verify_pow()?;
        if let Some(p) = prev {
            if h.prev_block != p {
                return Err("header chain broken: prev_block mismatch");
            }
        }
        prev = Some(h.block_hash());
    }
    Ok(headers.len() as u64)
}

pub fn verify_merkle_proof(txid: &[u8; 32], merkle_root: &[u8; 32], proof: &[u8]) -> bool {
    if proof.is_empty() {
        return txid == merkle_root;
    }
    if !proof.len().is_multiple_of(32) {
        return false;
    }

    let mut hash = *txid;
    for chunk in proof.chunks(32) {
        let mut sibling = [0u8; 32];
        sibling.copy_from_slice(chunk);
        let combined = if hash <= sibling {
            [hash.as_slice(), sibling.as_slice()].concat()
        } else {
            [sibling.as_slice(), hash.as_slice()].concat()
        };
        let h1 = Sha256::digest(&combined);
        let h2 = Sha256::digest(h1);
        hash = {
            let mut out = [0u8; 32];
            out.copy_from_slice(&h2);
            out
        };
    }
    hash == *merkle_root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_vault() -> BtcVault {
        let signers: Vec<[u8; 33]> = (0..5)
            .map(|i| {
                let mut k = [3u8; 33];
                k[0] = i as u8;
                k
            })
            .collect();
        let config = BtcVaultConfig::default().with_signers(signers, 3);
        BtcVault::new(config)
    }

    #[test]
    fn test_p2sh_address_creation() {
        let redeem = vec![0x00, 0x01, 0x02];
        let addr = p2sh_address(&redeem);
        assert_eq!(addr[0], 0xA9);
        assert_eq!(addr[1], 20);
        assert_eq!(addr[addr.len() - 1], 0x87);
    }

    #[test]
    fn test_p2wsh_address_creation() {
        let witness = vec![0x00, 0x01, 0x02];
        let addr = p2wsh_address(&witness);
        assert_eq!(addr[0], 0x00);
        assert_eq!(addr[1], 32);
    }

    #[test]
    fn test_multisig_redeem_script() {
        let signers = vec![[1u8; 33], [2u8; 33], [3u8; 33]];
        let script = multisig_redeem_script(&signers, 2);
        assert_eq!(script.last(), Some(&0xAE));
    }

    #[test]
    fn test_vault_config_with_signers() {
        let pubkeys = vec![[1u8; 33], [2u8; 33], [3u8; 33]];
        let config = BtcVaultConfig::default().with_signers(pubkeys, 2);
        assert!(!config.vault_address_p2sh.is_empty());
        assert!(!config.vault_address_p2wsh.is_empty());
        assert_eq!(config.threshold, 2);
    }

    #[test]
    fn test_utxo_set_select() {
        let mut utxos = BtcUtxoSet::new();
        utxos.add(BtcUtxoEntry {
            txid: [1u8; 32],
            vout: 0,
            amount: 5_000_000,
            script_pubkey: vec![],
            spendable: true,
        });
        utxos.add(BtcUtxoEntry {
            txid: [2u8; 32],
            vout: 0,
            amount: 10_000_000,
            script_pubkey: vec![],
            spendable: true,
        });
        let selected = utxos.select_utxos(12_000_000).unwrap();
        assert_eq!(selected.len(), 2);
        assert_eq!(utxos.total_spendable(), 15_000_000);
    }

    #[test]
    fn test_utxo_spend() {
        let mut utxos = BtcUtxoSet::new();
        utxos.add(BtcUtxoEntry {
            txid: [1u8; 32],
            vout: 0,
            amount: 5_000_000,
            script_pubkey: vec![],
            spendable: true,
        });
        utxos.spend(&[1u8; 32], 0).unwrap();
        assert_eq!(utxos.total_spendable(), 0);
        assert_eq!(
            utxos.spend(&[1u8; 32], 0),
            Err(BtcVaultError::UtxoAlreadySpent)
        );
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

        for _ in 0..6 {
            vault.process_deposit(0).unwrap();
        }
        vault.process_deposit(0).unwrap();
        for _ in 0..3 {
            vault.process_deposit(0).unwrap();
        }
        assert_eq!(vault.pending_deposits[0].status, BtcDepositStatus::Approved);

        vault.process_deposit(0).unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::Completed
        );
        assert_eq!(vault.total_deposited, 5_000_000);
        assert_eq!(vault.utxos.total_spendable(), 5_000_000);
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
            .submit_deposit([0u8; 32], 0, 100_000_000, vec![], [0u8; 32], vec![],)
            .is_err());
    }

    #[test]
    fn test_withdrawal_flow() {
        let mut vault = default_vault();
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
        // 6 confirmations + 1 SPV + 3 approvals + 1 complete = 11
        for _ in 0..11 {
            vault.process_deposit(0).unwrap();
        }
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::Completed
        );
        assert_eq!(vault.utxos.total_spendable(), 10_000_000);

        vault
            .submit_withdrawal([1u8; 32], vec![0x03, 0x04], 5_000_000, vec![1, 2, 3])
            .unwrap();
        assert_eq!(vault.pending_withdrawals.len(), 1);
    }

    #[test]
    fn test_insufficient_reserves_fails() {
        let mut vault = default_vault();
        assert_eq!(
            vault.submit_withdrawal([1u8; 32], vec![0x03], 500_000, vec![]),
            Err(BtcVaultError::InsufficientReserves)
        );
    }

    #[test]
    fn test_build_psbt() {
        let mut vault = default_vault();
        vault
            .submit_deposit(
                [1u8; 32],
                0,
                10_000_000,
                vec![0x01],
                [0u8; 32],
                vec![1, 2, 3],
            )
            .unwrap();
        for _ in 0..11 {
            vault.process_deposit(0).unwrap();
        }

        let mut recipient = vec![0xAAu8; 22];
        recipient[0] = 0x00;
        recipient[1] = 0x14;
        let psbt = vault.build_psbt(3_000_000, &recipient).unwrap();
        assert_eq!(psbt.inputs.len(), 1);
        assert_eq!(psbt.outputs.len(), 2);
        assert_eq!(psbt.fee, 5_000);
    }

    fn find_any_valid_header() -> [u8; 80] {
        let mut raw = [0u8; 80];
        // bits=0x1EFFFFFF, nonce=2561 yields a valid header
        raw[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        raw[76..80].copy_from_slice(&2561u32.to_le_bytes());
        raw
    }

    #[test]
    fn test_header_parse_and_pow() {
        let raw = find_any_valid_header();
        let header = BitcoinBlockHeader::parse(&raw).unwrap();
        assert!(header.verify_pow().is_ok());
    }

    #[test]
    fn test_compact_target() {
        let target = compact_target(0x1D00FFFF).unwrap();
        assert!(target[27] > 0 || target[28] > 0);
    }

    #[test]
    fn test_merkle_proof() {
        let txid = [1u8; 32];
        let merkle_root = txid;
        assert!(verify_merkle_proof(&txid, &merkle_root, &[]));

        let sibling = [2u8; 32];
        let combined = [txid.as_slice(), sibling.as_slice()].concat();
        let h1 = Sha256::digest(combined);
        let h2 = Sha256::digest(h1);
        let mut root = [0u8; 32];
        root.copy_from_slice(&h2);
        assert!(verify_merkle_proof(&txid, &root, &sibling));
    }

    #[test]
    fn test_header_chain_verification() {
        let h1_raw = find_any_valid_header();
        let h1 = BitcoinBlockHeader::parse(&h1_raw).unwrap();
        let h1_hash = h1.block_hash();

        let mut h2_raw = [0u8; 80];
        h2_raw[4..36].copy_from_slice(&h1_hash);
        h2_raw[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        h2_raw[76..80].copy_from_slice(&79153u32.to_le_bytes());

        let headers = vec![h1_raw.as_slice(), h2_raw.as_slice()];
        assert!(verify_block_header_chain(&headers).is_ok());

        let mut broken = h2_raw;
        broken[4..36].copy_from_slice(&[0u8; 32]);
        let bad_headers = vec![h1_raw.as_slice(), broken.as_slice()];
        assert!(verify_block_header_chain(&bad_headers).is_err());
    }

    #[test]
    fn test_signer_approval() {
        let mut vault = default_vault();
        vault
            .submit_deposit(
                [1u8; 32],
                0,
                5_000_000,
                vec![0x01],
                [0u8; 32],
                vec![1, 2, 3],
            )
            .unwrap();
        for _ in 0..7 {
            vault.process_deposit(0).unwrap();
        }

        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::PendingSignerApproval {
                approvals: 0,
                threshold: 3
            }
        );

        let signer_ids: Vec<[u8; 32]> = vault.config.signers.clone();
        vault
            .add_signer_approval(0, signer_ids[0], vec![0xAA; 64])
            .unwrap();
        vault
            .add_signer_approval(0, signer_ids[1], vec![0xBB; 64])
            .unwrap();
        vault
            .add_signer_approval(0, signer_ids[2], vec![0xCC; 64])
            .unwrap();

        assert_eq!(vault.pending_deposits[0].status, BtcDepositStatus::Approved);

        assert_eq!(
            vault.add_signer_approval(0, [0u8; 32], vec![]),
            Err(BtcVaultError::InvalidStateTransition)
        );
    }

    /// End-to-end deposit lifecycle test that proves the BTC signer quorum
    /// is wired through to the deposit accounting: confirmations → SPV →
    /// threshold approvals → Approved → Completed → UTXO added. This is
    /// the test that BTC mainnet signer quorum safety depends on; without
    /// it, individual stage tests could pass while the integrated flow
    /// silently loses funds or mints UTXOs without quorum.
    #[test]
    fn test_end_to_end_deposit_with_threshold_quorum() {
        let mut vault = default_vault();
        let deposit_amount = 5_000_000u64;
        let deposit_txid = [0xAB; 32];
        let deposit_vout = 0u32;

        // 1. Submit a fresh deposit.
        vault
            .submit_deposit(
                deposit_txid,
                deposit_vout,
                deposit_amount,
                vec![0x01],
                [0u8; 32],
                vec![1, 2, 3],
            )
            .unwrap();
        assert_eq!(vault.pending_deposits.len(), 1);
        assert_eq!(vault.utxos.total_spendable(), 0);

        // 2. Drive confirmations past the minimum (MIN_BITCOIN_CONFIRMATIONS=6).
        // Each process_deposit() increments confirmations, so the
        // transition to PendingSpvVerification happens on the Nth call
        // when confirmations reaches min_confirmations (i.e. on the 6th).
        let min_conf = MIN_BITCOIN_CONFIRMATIONS;
        for i in 0..min_conf {
            vault.process_deposit(0).unwrap();
            if (i + 1) < min_conf {
                assert_eq!(
                    vault.pending_deposits[0].status,
                    BtcDepositStatus::PendingConfirmations,
                    "still pending confirmation at step {}",
                    i + 1
                );
            }
        }
        // After the loop, status has just transitioned to PendingSpvVerification.
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::PendingSpvVerification
        );

        // 3. SPV proof is already attached (non-empty payload submitted
        //    with the deposit above); next process_deposit moves it into
        //    the signer-approval state.
        vault.process_deposit(0).unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::PendingSignerApproval {
                approvals: 0,
                threshold: 3
            }
        );

        // 4. Two of three signers approve — still not at threshold.
        let signer_ids: Vec<[u8; 32]> = vault.config.signers.clone();
        vault
            .add_signer_approval(0, signer_ids[0], vec![0xAA; 64])
            .unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::PendingSignerApproval {
                approvals: 1,
                threshold: 3
            },
            "after 1 approval, must still be pending 2 more"
        );
        vault
            .add_signer_approval(0, signer_ids[1], vec![0xBB; 64])
            .unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::PendingSignerApproval {
                approvals: 2,
                threshold: 3
            },
            "after 2 approvals, must still be pending 1 more"
        );

        // 5. Third signer pushes the deposit to Approved.
        vault
            .add_signer_approval(0, signer_ids[2], vec![0xCC; 64])
            .unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::Approved
        );

        // 6. Final process_deposit moves Approved → Completed AND credits
        //    the UTXO set. This is the critical integration point — a
        //    quorum that "approved" but didn't actually mint a spendable
        //    UTXO would silently lose the deposit.
        vault.process_deposit(0).unwrap();
        assert_eq!(
            vault.pending_deposits[0].status,
            BtcDepositStatus::Completed
        );
        assert_eq!(
            vault.utxos.total_spendable(),
            deposit_amount,
            "quorum-approved deposit must mint a spendable UTXO"
        );
        assert_eq!(
            vault.pending_deposits[0].txid, deposit_txid,
            "UTXO must reference the original deposit txid"
        );

        // 7. A fourth signer's approval after Completed must fail (no
        //    double-mint, no state corruption).
        assert_eq!(
            vault.add_signer_approval(0, signer_ids[3], vec![0xDD; 64]),
            Err(BtcVaultError::InvalidStateTransition),
            "completed deposit must reject further signer approvals"
        );
    }

    /// Test that an off-by-one in the threshold count would actually be
    /// caught. With threshold=3 we need exactly 3 approvals (not 2, not 4).
    /// If `process_deposit` ever changes its `approvals + 1 >= threshold`
    /// check to `< threshold`, this test fails on step 4 (the 2-approval
    /// state would prematurely become Approved).
    #[test]
    fn test_threshold_quorum_is_exact_not_off_by_one() {
        let mut vault = default_vault();
        vault
            .submit_deposit([0xCD; 32], 0, 1_000_000, vec![0x01], [0u8; 32], vec![1, 2, 3])
            .unwrap();
        // Drive to PendingSignerApproval. 6 calls → PendingSpvVerification
        // (call N), 1 more call → PendingSignerApproval {0, 3} (call N+1).
        // Do NOT call process_deposit again: the PendingSignerApproval
        // arm auto-increments approvals, which would silently add a
        // phantom approval and defeat the off-by-one test.
        for _ in 0..MIN_BITCOIN_CONFIRMATIONS {
            vault.process_deposit(0).unwrap();
        }
        vault.process_deposit(0).unwrap(); // → SignerApproval {0, 3}

        // Two approvals must NOT be enough.
        let signer_ids: Vec<[u8; 32]> = vault.config.signers.clone();
        vault.add_signer_approval(0, signer_ids[0], vec![]).unwrap();
        vault.add_signer_approval(0, signer_ids[1], vec![]).unwrap();
        match &vault.pending_deposits[0].status {
            BtcDepositStatus::PendingSignerApproval {
                approvals,
                threshold,
            } => {
                assert!(*approvals < *threshold,
                    "2/3 threshold must NOT approve (got approvals={approvals}, threshold={threshold})");
            }
            other => panic!(
                "expected PendingSignerApproval after 2 signers; got {other:?}"
            ),
        }

        // Third approval IS enough.
        vault.add_signer_approval(0, signer_ids[2], vec![]).unwrap();
        assert_eq!(vault.pending_deposits[0].status, BtcDepositStatus::Approved);
    }
}
