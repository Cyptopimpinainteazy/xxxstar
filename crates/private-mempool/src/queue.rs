//! Encrypted transaction queue.

use crate::{EncryptedTransaction, MempoolError};
use std::collections::HashMap;

/// Queue of encrypted transactions, indexed by ID.
pub struct EncryptedQueue {
    max_capacity: usize,
    /// Ordered list of transactions (insertion order).
    transactions: Vec<EncryptedTransaction>,
    /// Index: tx_id → position in `transactions`.
    index: HashMap<[u8; 32], usize>,
}

impl EncryptedQueue {
    /// Create a new encrypted queue.
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            transactions: Vec::with_capacity(max_capacity.min(1024)),
            index: HashMap::new(),
        }
    }

    /// Push an encrypted transaction.
    pub fn push(&mut self, tx: EncryptedTransaction) -> Result<(), MempoolError> {
        if self.transactions.len() >= self.max_capacity {
            return Err(MempoolError::Full {
                capacity: self.max_capacity,
            });
        }

        if self.index.contains_key(&tx.id) {
            return Err(MempoolError::Duplicate);
        }

        let idx = self.transactions.len();
        self.index.insert(tx.id, idx);
        self.transactions.push(tx);
        Ok(())
    }

    /// Get all pending transactions.
    pub fn pending(&self) -> &[EncryptedTransaction] {
        &self.transactions
    }

    /// Remove a transaction by ID.
    pub fn remove(&mut self, tx_id: &[u8; 32]) -> Option<EncryptedTransaction> {
        if let Some(&idx) = self.index.get(tx_id) {
            self.index.remove(tx_id);

            // Swap-remove for O(1)
            let tx = self.transactions.swap_remove(idx);

            // Update index for swapped element
            if idx < self.transactions.len() {
                let swapped_id = self.transactions[idx].id;
                self.index.insert(swapped_id, idx);
            }

            Some(tx)
        } else {
            None
        }
    }

    /// Prune transactions older than `max_age` seconds.
    pub fn prune(&mut self, now_secs: u64, max_age_secs: u64) -> usize {
        let cutoff = now_secs.saturating_sub(max_age_secs);
        let before = self.transactions.len();

        // Collect IDs to remove
        let expired_ids: Vec<[u8; 32]> = self
            .transactions
            .iter()
            .filter(|tx| tx.submitted_at < cutoff)
            .map(|tx| tx.id)
            .collect();

        for id in &expired_ids {
            self.remove(id);
        }

        before - self.transactions.len()
    }

    /// Number of pending transactions.
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tx(id_byte: u8, submitted_at: u64) -> EncryptedTransaction {
        EncryptedTransaction {
            id: [id_byte; 32],
            ciphertext: vec![0xCA; 64],
            ephemeral_pk: [0xAA; 32],
            nonce: [0x00; 12],
            sender_pk: [0xBB; 32],
            fee_commitment: [0xCC; 32],
            submitted_at,
            dkg_epoch: 1,
        }
    }

    #[test]
    fn push_and_remove() {
        let mut q = EncryptedQueue::new(100);
        let tx = make_tx(0x01, 1000);
        q.push(tx).unwrap();
        assert_eq!(q.len(), 1);

        let removed = q.remove(&[0x01; 32]).unwrap();
        assert_eq!(removed.id, [0x01; 32]);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn rejects_duplicate() {
        let mut q = EncryptedQueue::new(100);
        q.push(make_tx(0x01, 1000)).unwrap();
        let result = q.push(make_tx(0x01, 1001));
        assert!(matches!(result, Err(MempoolError::Duplicate)));
    }

    #[test]
    fn capacity_enforced() {
        let mut q = EncryptedQueue::new(2);
        q.push(make_tx(0x01, 1000)).unwrap();
        q.push(make_tx(0x02, 1000)).unwrap();
        let result = q.push(make_tx(0x03, 1000));
        assert!(matches!(result, Err(MempoolError::Full { .. })));
    }

    #[test]
    fn prune_expired() {
        let mut q = EncryptedQueue::new(100);
        q.push(make_tx(0x01, 100)).unwrap(); // old
        q.push(make_tx(0x02, 500)).unwrap(); // newer
        q.push(make_tx(0x03, 900)).unwrap(); // newest

        let pruned = q.prune(1000, 300); // cutoff = 700
        assert_eq!(pruned, 2); // 0x01 and 0x02 pruned
        assert_eq!(q.len(), 1);
    }
}
