//! Import Queue Wrapper Module
//!
//! Implements a wrapper around the transaction import queue with
//! parallel processing capabilities and GPU-accelerated signature verification.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
// Every one of the queue's mutexes is locked with `.lock().await` inside async
// functions that are `tokio::spawn`ed, so they must be `tokio::sync::Mutex`:
// the crate used `std::sync::Mutex` and awaited its guards, which does not
// compile (and would not be `Send` across an await point even if it did).
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::interval;
use uuid::Uuid;
// The crate referenced `TransactionMeta`, `GPUSignatureVerifier` and
// `VerifierConfig` without ever importing them, so it had never compiled.
use gpu_sig_verifier::GPUSignatureVerifier;
use parallel_proposer::TransactionMeta;

/// Import queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_queue_size: usize,
    pub parallel_workers: usize,
    pub batch_size: usize,
    pub verification_timeout: u64,
    pub cleanup_interval_seconds: u64,
    pub enable_priority: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            parallel_workers: 4,
            batch_size: 256,
            verification_timeout: 30,
            cleanup_interval_seconds: 60,
            enable_priority: true,
        }
    }
}

/// Import queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: String,
    pub transaction: TransactionMeta,
    pub priority: u8,
    pub submission_time: u64,
    pub verification_status: VerificationStatus,
    pub processing_stage: ProcessingStage,
}

/// Transaction verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verifying,
    Verified,
    Failed,
}

/// Processing stage
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStage {
    Queued,
    ContentionCheck,
    SignatureVerification,
    ReadyForInclusion,
    Included,
}

/// Import queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub total_entries: usize,
    pub pending_entries: usize,
    pub verifying_entries: usize,
    pub verified_entries: usize,
    pub failed_entries: usize,
    pub average_processing_time_ms: f64,
    pub throughput_tps: f64,
    pub current_queue_size: usize,
}

/// Import queue wrapper core
pub struct ImportQueueWrapper {
    config: QueueConfig,
    queue: Arc<Mutex<VecDeque<QueueEntry>>>,
    priority_queue: Arc<Mutex<HashMap<u8, VecDeque<QueueEntry>>>>,
    // No outer Mutex: GPUSignatureVerifier's methods all take `&self` and the
    // type is already internally synchronized (its own Arc<Mutex<..>>
    // fields), so wrapping it in another Mutex here only serialized workers
    // against each other for no correctness benefit — every worker held this
    // lock for the full `verify_signature(..).await`, so `parallel_workers`
    // could never actually verify in parallel.
    verification_service: Arc<GPUSignatureVerifier>,
    stats: Arc<Mutex<QueueStats>>,
    worker_handles: Vec<tokio::task::JoinHandle<()>>,
}

impl ImportQueueWrapper {
    /// Create a new import queue wrapper
    pub fn new(config: QueueConfig, verifier: GPUSignatureVerifier) -> Self {
        Self {
            config,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            priority_queue: Arc::new(Mutex::new(HashMap::new())),
            verification_service: Arc::new(verifier),
            stats: Arc::new(Mutex::new(QueueStats::new())),
            worker_handles: Vec::new(),
        }
    }

    /// Start the import queue processing
    pub async fn start(&mut self) -> Result<()> {
        info!(
            "Starting import queue with {} workers",
            self.config.parallel_workers
        );

        // Start worker pool
        for worker_id in 0..self.config.parallel_workers {
            let queue_clone = self.queue.clone();
            let priority_queue_clone = self.priority_queue.clone();
            let verifier_clone = self.verification_service.clone();
            let config_clone = self.config.clone();
            let stats_clone = self.stats.clone();

            let handle = tokio::spawn(async move {
                worker_main(
                    worker_id,
                    queue_clone,
                    priority_queue_clone,
                    verifier_clone,
                    config_clone,
                    stats_clone,
                )
                .await;
            });

            self.worker_handles.push(handle);
        }

        // Start cleanup task
        // `start_cleanup_task` is `async`, so it has to be awaited: pushing the
        // future itself into a `Vec<JoinHandle<()>>` does not typecheck (and the
        // task would never have started).
        let cleanup_handle = self.start_cleanup_task().await;
        self.worker_handles.push(cleanup_handle);

        Ok(())
    }

    /// Stop the import queue processing
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping import queue workers");

        // Cancel all worker tasks
        for handle in self.worker_handles.drain(..) {
            handle.abort();
        }

        Ok(())
    }

    /// Submit transaction to import queue
    pub async fn submit_transaction(&self, tx: TransactionMeta, priority: u8) -> Result<String> {
        let entry = QueueEntry {
            id: Uuid::new_v4().to_string(),
            transaction: tx,
            priority,
            submission_time: current_timestamp(),
            verification_status: VerificationStatus::Pending,
            processing_stage: ProcessingStage::Queued,
        };
        // The entry is moved into one of the queues below, so keep the id for
        // the return value (this used to be a borrow-after-move).
        let entry_id = entry.id.clone();

        // Add to appropriate queue
        if self.config.enable_priority {
            let mut priority_queue = self.priority_queue.lock().await;
            priority_queue
                .entry(priority)
                .or_insert_with(VecDeque::new)
                .push_back(entry);
        } else {
            let mut queue = self.queue.lock().await;
            queue.push_back(entry);
        }

        // Update stats
        self.update_stats().await;

        Ok(entry_id)
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> QueueStats {
        self.stats.lock().await.clone()
    }

    /// Get current queue size
    pub async fn get_queue_size(&self) -> usize {
        let queue = self.queue.lock().await;
        let priority_queue = self.priority_queue.lock().await;

        let mut size = queue.len();
        for (_, entries) in priority_queue.iter() {
            size += entries.len();
        }

        size
    }

    /// Start cleanup task
    async fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let queue_clone = self.queue.clone();
        let priority_queue_clone = self.priority_queue.clone();
        let stats_clone = self.stats.clone();
        let cleanup_interval = self.config.cleanup_interval_seconds;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(cleanup_interval));

            loop {
                interval.tick().await;
                cleanup_queues(
                    queue_clone.clone(),
                    priority_queue_clone.clone(),
                    stats_clone.clone(),
                )
                .await;
            }
        })
    }

    /// Update queue statistics
    async fn update_stats(&self) {
        // Acquire queue/priority_queue (inside get_queue_size) *before*
        // stats, matching cleanup_queues' order (queue -> priority_queue ->
        // stats). Locking stats first here would invert that order: with
        // cleanup_queues now actually running (see `start`), a
        // submit_transaction racing a cleanup pass could deadlock, one task
        // holding stats and waiting on queue while the other holds
        // queue/priority_queue and waits on stats.
        let queue_size = self.get_queue_size().await;
        let mut stats = self.stats.lock().await;
        stats.current_queue_size = queue_size;
    }
}

/// Worker main function
async fn worker_main(
    worker_id: usize,
    queue: Arc<Mutex<VecDeque<QueueEntry>>>,
    priority_queue: Arc<Mutex<HashMap<u8, VecDeque<QueueEntry>>>>,
    verifier: Arc<GPUSignatureVerifier>,
    config: QueueConfig,
    stats: Arc<Mutex<QueueStats>>,
) {
    info!("Worker {} started", worker_id);

    loop {
        // Get next transaction from queue
        let entry = match get_next_entry(
            queue.clone(),
            priority_queue.clone(),
            config.enable_priority,
        )
        .await
        {
            Some(entry) => entry,
            None => {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Process transaction
        let outcome = process_entry(entry, verifier.clone(), config.clone()).await;
        if let Err(e) = &outcome {
            warn!("Worker {} error processing entry: {}", worker_id, e);
        }

        // Update stats. `verified_entries`/`failed_entries` were declared but
        // never written, so a queue that rejected every transaction still
        // reported zero failures — the counters are the only way to observe
        // from outside whether anything got past verification.
        let mut stats_lock = stats.lock().await;
        stats_lock.total_entries += 1;
        match outcome {
            Ok(()) => stats_lock.verified_entries += 1,
            Err(_) => stats_lock.failed_entries += 1,
        }
    }
}

/// Get next entry from queue
async fn get_next_entry(
    queue: Arc<Mutex<VecDeque<QueueEntry>>>,
    priority_queue: Arc<Mutex<HashMap<u8, VecDeque<QueueEntry>>>>,
    enable_priority: bool,
) -> Option<QueueEntry> {
    if enable_priority {
        let mut priority_queue_lock = priority_queue.lock().await;
        // Get highest priority queue with entries
        let mut priorities: Vec<u8> = priority_queue_lock.keys().copied().collect();
        priorities.sort_unstable();
        priorities.reverse();

        for priority in priorities {
            if let Some(entry) = priority_queue_lock
                .get_mut(&priority)
                .and_then(|q| q.pop_front())
            {
                return Some(entry);
            }
        }
    } else {
        let mut queue_lock = queue.lock().await;
        return queue_lock.pop_front();
    }

    None
}

/// Process queue entry
async fn process_entry(
    mut entry: QueueEntry,
    verifier: Arc<GPUSignatureVerifier>,
    _config: QueueConfig,
) -> Result<()> {
    // Stage 1: Contention check
    entry.processing_stage = ProcessingStage::ContentionCheck;

    // Check for potential contention using the transaction features
    // High-value or high-gas transactions are flagged for contention analysis
    let has_high_value = entry.transaction.value > 1_000_000_000;
    let has_high_gas = entry.transaction.gas_price > 50_000_000;

    if has_high_value || has_high_gas {
        // Log potential contention for monitoring
        debug!(
            "Transaction {} flagged for contention check (value: {}, gas_price: {})",
            entry.id, entry.transaction.value, entry.transaction.gas_price
        );
        // In production, this would query the contention predictor
        // For now, we proceed but track the potential for parallel execution
    }

    // Stage 2: Signature verification
    entry.processing_stage = ProcessingStage::SignatureVerification;
    let verification_result = verifier
        .verify_signature(
            &entry.transaction.signature,
            entry.transaction_hash().as_bytes(),
        )
        .await?;

    if verification_result.verified {
        entry.verification_status = VerificationStatus::Verified;
        entry.processing_stage = ProcessingStage::ReadyForInclusion;
    } else {
        // `entry` is owned by this function and dropped on return — nothing
        // re-queues it, so this is a terminal failure, not a "return to
        // queue for retry" (the previous comment/state here claimed a retry
        // that never happened). Retrying a forged/invalid signature forever
        // would also be its own hazard, so terminal failure is the right
        // outcome, not just the honest one.
        entry.verification_status = VerificationStatus::Failed;
        return Err(anyhow!("Signature verification failed"));
    }

    // Stage 3: Ready for inclusion
    entry.processing_stage = ProcessingStage::ReadyForInclusion;

    Ok(())
}

/// Cleanup queues
async fn cleanup_queues(
    queue: Arc<Mutex<VecDeque<QueueEntry>>>,
    priority_queue: Arc<Mutex<HashMap<u8, VecDeque<QueueEntry>>>>,
    stats: Arc<Mutex<QueueStats>>,
) {
    // Remove old entries
    let mut queue_lock = queue.lock().await;
    while let Some(front) = queue_lock.front() {
        if is_entry_expired(front) {
            queue_lock.pop_front();
        } else {
            break;
        }
    }

    // Clean priority queues
    let mut priority_queue_lock = priority_queue.lock().await;
    for (_, entries) in priority_queue_lock.iter_mut() {
        while let Some(front) = entries.front() {
            if is_entry_expired(front) {
                entries.pop_front();
            } else {
                break;
            }
        }
    }

    // Update stats
    let mut stats_lock = stats.lock().await;
    stats_lock.current_queue_size = queue_lock.len();
    for (_, entries) in priority_queue_lock.iter() {
        stats_lock.current_queue_size += entries.len();
    }
}

/// Check if entry is expired
fn is_entry_expired(entry: &QueueEntry) -> bool {
    let current_time = current_timestamp();
    let age = current_time - entry.submission_time;
    age > 3600 // 1 hour
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl QueueEntry {
    fn transaction_hash(&self) -> String {
        // Generate transaction hash
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.transaction.tx_hash.as_bytes());
        format!("{}", hasher.finalize().to_hex())
    }
}

impl QueueStats {
    fn new() -> Self {
        Self {
            total_entries: 0,
            pending_entries: 0,
            verifying_entries: 0,
            verified_entries: 0,
            failed_entries: 0,
            average_processing_time_ms: 0.0,
            throughput_tps: 0.0,
            current_queue_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_sig_verifier::VerifierConfig;

    #[tokio::test]
    async fn test_import_queue_basic_flow() {
        let config = QueueConfig::default();
        let verifier = GPUSignatureVerifier::new(VerifierConfig::default());
        let mut queue = ImportQueueWrapper::new(config, verifier);

        // Start queue
        queue.start().await.unwrap();

        // Create test transaction
        let tx = TransactionMeta {
            tx_hash: "test_tx".to_string(),
            sender: "0x1234".to_string(),
            receiver: "0x5678".to_string(),
            value: 1_000_000_000,
            gas_limit: 21_000,
            gas_price: 20_000_000,
            nonce: 1,
            signature: "valid_sig".to_string(),
            contract_address: None,
            timestamp: 1234567890,
        };

        // Submit transaction
        let entry_id = queue.submit_transaction(tx, 1).await.unwrap();
        assert!(!entry_id.is_empty());

        // Get stats
        let stats = queue.get_stats().await;
        assert_eq!(stats.current_queue_size, 1);

        // Stop queue
        queue.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_priority_queue() {
        let config = QueueConfig {
            enable_priority: true,
            ..Default::default()
        };
        let verifier = GPUSignatureVerifier::new(VerifierConfig::default());
        let mut queue = ImportQueueWrapper::new(config, verifier);

        // Start queue
        queue.start().await.unwrap();

        // Create transactions with different priorities
        let tx1 = TransactionMeta {
            tx_hash: "tx1".to_string(),
            sender: "0x1".to_string(),
            receiver: "0x2".to_string(),
            value: 1_000_000_000,
            gas_limit: 21_000,
            gas_price: 20_000_000,
            nonce: 1,
            signature: "sig1".to_string(),
            contract_address: None,
            timestamp: 1234567890,
        };

        let tx2 = TransactionMeta {
            tx_hash: "tx2".to_string(),
            sender: "0x3".to_string(),
            receiver: "0x4".to_string(),
            value: 2_000_000_000,
            gas_limit: 21_000,
            gas_price: 30_000_000,
            nonce: 2,
            signature: "sig2".to_string(),
            contract_address: None,
            timestamp: 1234567891,
        };

        // Submit with different priorities
        queue.submit_transaction(tx1, 1).await.unwrap();
        queue.submit_transaction(tx2, 5).await.unwrap();

        // Get queue size
        let size = queue.get_queue_size().await;
        assert_eq!(size, 2);

        // Stop queue
        queue.stop().await.unwrap();
    }

    fn tx_with_signature(signature: String) -> TransactionMeta {
        TransactionMeta {
            tx_hash: "forged_tx".to_string(),
            sender: "0x1234".to_string(),
            receiver: "0x5678".to_string(),
            value: 1_000_000_000,
            gas_limit: 21_000,
            gas_price: 20_000_000,
            nonce: 7,
            signature,
            contract_address: None,
            timestamp: 1234567890,
        }
    }

    /// End-to-end proof that the forged-signature hole is closed.
    ///
    /// The verifier used to answer "verified" for any signature longer than 64
    /// characters, and `process_entry` marked such a transaction
    /// `ReadyForInclusion`. A 65-character string must now never be counted as
    /// verified by the queue.
    #[tokio::test]
    async fn forged_signature_is_never_verified() {
        let mut queue = ImportQueueWrapper::new(
            QueueConfig::default(),
            GPUSignatureVerifier::new(VerifierConfig::default()),
        );
        queue.start().await.unwrap();

        queue
            .submit_transaction(tx_with_signature("x".repeat(65)), 1)
            .await
            .unwrap();

        // Wait for a worker to actually record the outcome (workers poll
        // every 100 ms) instead of sleeping a fixed guess: on a slow or
        // loaded runner a fixed sleep can elapse before any worker has run,
        // which would make this assertion spuriously fail, not pass, since
        // it can only be wrong in the direction of "not processed yet."
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let stats = loop {
            let stats = queue.get_stats().await;
            if stats.failed_entries + stats.verified_entries >= 1 {
                break stats;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "no worker recorded an outcome within the deadline: {stats:?}"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        assert_eq!(
            stats.verified_entries, 0,
            "a 65-character string must never verify: {stats:?}"
        );
        assert!(
            stats.failed_entries >= 1,
            "the entry must be recorded as failed: {stats:?}"
        );

        queue.stop().await.unwrap();
    }
}
