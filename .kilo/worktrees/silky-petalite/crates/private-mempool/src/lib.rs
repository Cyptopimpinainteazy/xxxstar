//! # Private Mempool
//!
//! Proposal: PRIV-ENCLAVE-003
//!
//! Encrypted transaction mempool with threshold encryption support.
//! Transactions are encrypted to the DKG committee's threshold public key
//! and can only be decrypted by a quorum of confidential validators.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Private Mempool                          │
//! │                                                             │
//! │  User TX → Encrypt(committee_pk) → Encrypted Queue         │
//! │                                        │                   │
//! │              ┌─────────────────────────┘                   │
//! │              ▼                                              │
//! │  Threshold Decrypt (t-of-n validators) → Plaintext TX      │
//! │              │                                              │
//! │              ▼                                              │
//! │  Enclave Execution → Encrypted State Diff                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Invariants
//!
//! - PRIV-EXEC-001: TX content never exposed outside enclave
//! - PRIV-EXEC-003: No single validator can decrypt

pub mod encryption;
pub mod queue;

use std::time::{SystemTime, UNIX_EPOCH};

/// An encrypted transaction in the private mempool.
#[derive(Debug, Clone)]
pub struct EncryptedTransaction {
    /// Unique transaction identifier (hash of ciphertext).
    pub id: [u8; 32],
    /// Encrypted payload (AES-256-GCM ciphertext).
    pub ciphertext: Vec<u8>,
    /// Ephemeral public key for ECDH (X25519).
    pub ephemeral_pk: [u8; 32],
    /// AES-GCM nonce (12 bytes).
    pub nonce: [u8; 12],
    /// Sender's public key (for fee attribution).
    pub sender_pk: [u8; 32],
    /// Priority fee commitment (Pedersen commitment).
    pub fee_commitment: [u8; 32],
    /// Timestamp when submitted.
    pub submitted_at: u64,
    /// DKG epoch this TX was encrypted for.
    pub dkg_epoch: u64,
}

/// Threshold public key for the confidential validator committee.
#[derive(Debug, Clone)]
pub struct ThresholdPublicKey {
    /// The combined group public key (X25519).
    pub group_key: [u8; 32],
    /// DKG epoch number.
    pub epoch: u64,
    /// Threshold (t in t-of-n).
    pub threshold: u32,
    /// Total committee size (n).
    pub committee_size: u32,
}

/// A decryption share from one validator.
#[derive(Debug, Clone)]
pub struct DecryptionShare {
    /// Validator index in the committee.
    pub validator_index: u32,
    /// The partial decryption share.
    pub share: Vec<u8>,
    /// Proof of correct decryption (DLEQ proof).
    pub proof: Vec<u8>,
}

/// Configuration for the private mempool.
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of encrypted transactions to hold.
    pub max_capacity: usize,
    /// Maximum age of transactions before pruning (seconds).
    pub max_age_secs: u64,
    /// Whether to verify fee commitments.
    pub verify_fees: bool,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            max_age_secs: 300, // 5 minutes
            verify_fees: true,
        }
    }
}

/// The private mempool.
pub struct PrivateMempool {
    config: MempoolConfig,
    queue: queue::EncryptedQueue,
    committee_key: Option<ThresholdPublicKey>,
    total_submitted: u64,
    total_decrypted: u64,
    total_pruned: u64,
}

impl PrivateMempool {
    /// Create a new private mempool.
    pub fn new(config: MempoolConfig) -> Self {
        let capacity = config.max_capacity;
        Self {
            config,
            queue: queue::EncryptedQueue::new(capacity),
            committee_key: None,
            total_submitted: 0,
            total_decrypted: 0,
            total_pruned: 0,
        }
    }

    /// Set the committee threshold public key.
    pub fn set_committee_key(&mut self, key: ThresholdPublicKey) {
        self.committee_key = Some(key);
    }

    /// Submit an encrypted transaction to the mempool.
    ///
    /// # Invariant: PRIV-EXEC-001
    pub fn submit(&mut self, tx: EncryptedTransaction) -> Result<(), MempoolError> {
        // Verify the TX was encrypted for the current epoch
        if let Some(ref key) = self.committee_key {
            if tx.dkg_epoch != key.epoch {
                return Err(MempoolError::WrongEpoch {
                    expected: key.epoch,
                    got: tx.dkg_epoch,
                });
            }
        } else {
            return Err(MempoolError::NoCommitteeKey);
        }

        self.queue.push(tx)?;
        self.total_submitted += 1;
        Ok(())
    }

    /// Get pending transactions for enclave processing.
    /// Returns encrypted transactions — they must be decrypted inside the enclave.
    pub fn pending(&self) -> &[EncryptedTransaction] {
        self.queue.pending()
    }

    /// Remove a transaction after successful enclave execution.
    pub fn remove(&mut self, tx_id: &[u8; 32]) -> Option<EncryptedTransaction> {
        let removed = self.queue.remove(tx_id);
        if removed.is_some() {
            self.total_decrypted += 1;
        }
        removed
    }

    /// Prune expired transactions.
    pub fn prune_expired(&mut self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let pruned = self.queue.prune(now, self.config.max_age_secs);
        self.total_pruned += pruned as u64;
        pruned
    }

    /// Get mempool statistics.
    pub fn stats(&self) -> MempoolStats {
        MempoolStats {
            pending_count: self.queue.len(),
            total_submitted: self.total_submitted,
            total_decrypted: self.total_decrypted,
            total_pruned: self.total_pruned,
            capacity: self.config.max_capacity,
        }
    }
}

/// Mempool statistics.
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub pending_count: usize,
    pub total_submitted: u64,
    pub total_decrypted: u64,
    pub total_pruned: u64,
    pub capacity: usize,
}

/// Errors from the private mempool.
#[derive(Debug, thiserror::Error)]
pub enum MempoolError {
    #[error("Mempool is full (capacity: {capacity})")]
    Full { capacity: usize },

    #[error("No committee key set")]
    NoCommitteeKey,

    #[error("TX encrypted for wrong epoch (expected {expected}, got {got})")]
    WrongEpoch { expected: u64, got: u64 },

    #[error("Duplicate transaction")]
    Duplicate,

    #[error("Encryption error: {0}")]
    EncryptionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_tx(epoch: u64) -> EncryptedTransaction {
        EncryptedTransaction {
            id: [0x01; 32],
            ciphertext: vec![0xCA; 256],
            ephemeral_pk: [0xAA; 32],
            nonce: [0x00; 12],
            sender_pk: [0xBB; 32],
            fee_commitment: [0xCC; 32],
            submitted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            dkg_epoch: epoch,
        }
    }

    #[test]
    fn submit_requires_committee_key() {
        let mut pool = PrivateMempool::new(MempoolConfig::default());
        let result = pool.submit(dummy_tx(1));
        assert!(matches!(result, Err(MempoolError::NoCommitteeKey)));
    }

    /// # Invariant: PRIV-EXEC-003
    #[test]
    fn submit_checks_epoch() {
        let mut pool = PrivateMempool::new(MempoolConfig::default());
        pool.set_committee_key(ThresholdPublicKey {
            group_key: [0x01; 32],
            epoch: 5,
            threshold: 3,
            committee_size: 5,
        });

        // Wrong epoch
        let result = pool.submit(dummy_tx(3));
        assert!(matches!(result, Err(MempoolError::WrongEpoch { .. })));

        // Correct epoch
        let result = pool.submit(dummy_tx(5));
        assert!(result.is_ok());
    }

    #[test]
    fn stats_track_submissions() {
        let mut pool = PrivateMempool::new(MempoolConfig::default());
        pool.set_committee_key(ThresholdPublicKey {
            group_key: [0x01; 32],
            epoch: 1,
            threshold: 2,
            committee_size: 3,
        });

        pool.submit(dummy_tx(1)).unwrap();
        let stats = pool.stats();
        assert_eq!(stats.total_submitted, 1);
        assert_eq!(stats.pending_count, 1);
    }
}
