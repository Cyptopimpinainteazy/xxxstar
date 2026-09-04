//! Mempool Module - Transaction memory pool

use crate::config::GulfstreamConfig;
use crate::error::GulfstreamResult;
use crate::metrics::GulfstreamMetrics;
use crate::transaction::{Transaction, TransactionMeta, TransactionStatus};
use lru::LruCache;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Transaction entry in mempool
struct MempoolEntry {
    transaction: Transaction,
    status: TransactionStatus,
    added_at: Instant,
}

/// Transaction mempool
pub struct TransactionMempool {
    config: GulfstreamConfig,
    metrics: Arc<GulfstreamMetrics>,
    /// Main transaction storage
    transactions: RwLock<HashMap<String, MempoolEntry>>,
    /// Priority queues (by priority level)
    priority_queues: RwLock<Vec<VecDeque<String>>>,
    /// Dedup cache
    dedup_cache: RwLock<LruCache<String, ()>>,
    /// Current slot
    current_slot: RwLock<u64>,
    /// Cleanup task handle
    cleanup_handle: RwLock<Option<mpsc::Sender<()>>>,
}

impl TransactionMempool {
    /// Create new mempool
    pub fn new(config: GulfstreamConfig) -> Self {
        let dedup_cache = LruCache::new(config.dedup_cache_size);
        let priority_queues = (0..config.priority_levels)
            .map(|_| VecDeque::new())
            .collect();

        Self {
            config,
            metrics: Arc::new(GulfstreamMetrics::new()),
            transactions: RwLock::new(HashMap::new()),
            priority_queues: RwLock::new(priority_queues),
            dedup_cache: RwLock::new(dedup_cache),
            current_slot: RwLock::new(0),
            cleanup_handle: RwLock::new(None),
        }
    }

    /// Start cleanup task
    pub async fn start_cleanup_task(&self) {
        let (tx, mut rx) = mpsc::channel(1);
        *self.cleanup_handle.write() = Some(tx);

        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_millis(config.stale_check_interval_ms)
            );

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Cleanup is handled by the mempool owner
                    }
                    _ = rx.recv() => {
                        info!("Mempool cleanup task stopped");
                        break;
                    }
                }
            }
        });
    }

    /// Add transaction to mempool
    pub async fn add_transaction(&self, mut transaction: Transaction) -> GulfstreamResult<String> {
        let tx_hash = transaction.hash().to_string();

        // Check dedup cache
        {
            let mut dedup = self.dedup_cache.write();
            if dedup.contains(&tx_hash) {
                return Err(GulfstreamError::MempoolError("Duplicate transaction".into()));
            }
            dedup.put(tx_hash.clone(), ());
        }

        // Check capacity
        {
            let tx_count = self.transactions.read().len();
            if tx_count >= self.config.max_mempool_size {
                // Try to remove oldest
                self.remove_oldest()?;
            }
        }

        // Set current slot
        let slot = *self.current_slot.read();
        transaction.set_slot(slot);

        // Add to storage
        let entry = MempoolEntry {
            transaction,
            status: TransactionStatus::Pending,
            added_at: Instant::now(),
        };

        self.transactions.write().insert(tx_hash.clone(), entry);

        // Add to priority queue
        let priority = entry.transaction.meta().priority as usize;
        if priority < self.config.priority_levels {
            self.priority_queues.write()[priority].push_back(tx_hash.clone());
        }

        debug!("Transaction added to mempool: {}", tx_hash);
        Ok(tx_hash)
    }

    /// Get transaction status
    pub fn get_status(&self, tx_hash: &str) -> Option<TransactionStatus> {
        self.transactions.read()
            .get(tx_hash)
            .map(|e| e.status)
    }

    /// Get transaction
    pub fn get_transaction(&self, tx_hash: &str) -> Option<Transaction> {
        self.transactions.read()
            .get(tx_hash)
            .map(|e| e.transaction.clone())
    }

    /// Remove oldest transaction
    fn remove_oldest(&self) -> GulfstreamResult<()> {
        let mut transactions = self.transactions.write();
        
        // Find oldest
        let oldest = transactions.iter()
            .min_by_key(|(_, e)| e.added_at)
            .map(|(k, _)| k.clone());

        if let Some(hash) = oldest {
            transactions.remove(&hash);
            self.metrics.record_transaction_expired();
        }

        Ok(())
    }

    /// Remove expired transactions
    pub fn remove_expired(&self, current_slot: u64) -> Vec<String> {
        let mut removed = Vec::new();
        let max_age = self.config.max_transaction_age_slots;

        let mut transactions = self.transactions.write();
        
        transactions.retain(|hash, entry| {
            let age = current_slot.saturating_sub(entry.transaction.meta().created_slot);
            if age > max_age {
                removed.push(hash.clone());
                self.metrics.record_transaction_expired();
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get next transactions to forward
    pub fn get_transactions_to_forward(&self, count: usize) -> Vec<Transaction> {
        let mut result = Vec::with_capacity(count);
        let mut transactions = self.transactions.write();
        let mut queues = self.priority_queues.write();

        // Get from highest priority queue first
        for queue in queues.iter_mut().rev() {
            while let Some(tx_hash) = queue.pop_front() {
                if let Some(entry) = transactions.get_mut(&tx_hash) {
                    if entry.status == TransactionStatus::Pending {
                        entry.status = TransactionStatus::Forwarded;
                        result.push(entry.transaction.clone());
                        
                        if result.len() >= count {
                            return result;
                        }
                    }
                }
            }
        }

        result
    }

    /// Update current slot
    pub fn set_current_slot(&self, slot: u64) {
        *self.current_slot.write() = slot;
        
        // Remove expired transactions
        self.remove_expired(slot);
    }

    /// Get mempool size
    pub fn size(&self) -> usize {
        self.transactions.read().len()
    }

    /// Clear mempool
    pub fn clear(&self) {
        self.transactions.write().clear();
        for queue in self.priority_queues.write().iter_mut() {
            queue.clear();
        }
        self.dedup_cache.write().clear();
    }
}