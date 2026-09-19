//! Multi-chain mempool scanner for ChronosFlash
//!
//! Scans 103+ chains simultaneously for pending transactions

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

use crate::config::{ChainConfig, ChronosConfig, MempoolConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::intent::{IntentDetector, SwapIntent};
use crate::types::{Address, ChainId, ChainStatus, Hash, MempoolStats, Timestamp};

/// Message for every mempool path that has no client behind it.
///
/// Each `connect` used to set `is_connected = true` and return `Ok(())` without
/// opening anything, so a trading loop could believe it was watching a live
/// mempool while reading nothing at all.
const NOT_IMPLEMENTED: &str =
    "mempool streaming is not implemented: no websocket/bloxroute/flashbots client is wired up";

/// Multi-chain mempool scanner
pub struct MempoolScanner {
    config: MempoolConfig,
    chains: HashMap<ChainId, ChainScanner>,
    intent_detector: IntentDetector,
    stats: Arc<RwLock<MempoolStats>>,
    intent_tx: mpsc::Sender<SwapIntent>,
}

impl MempoolScanner {
    /// Create new mempool scanner
    pub fn new(config: ChronosConfig, intent_tx: mpsc::Sender<SwapIntent>) -> Self {
        let mut chains = HashMap::new();

        for (chain_id, chain_config) in config.chains.iter() {
            if chain_config.enabled {
                chains.insert(*chain_id, ChainScanner::new(chain_config.clone()));
            }
        }

        Self {
            config: config.mempool,
            chains,
            intent_detector: IntentDetector::new(),
            stats: Arc::new(RwLock::new(MempoolStats::default())),
            intent_tx,
        }
    }

    /// Start scanning all chains
    pub async fn start(&mut self) -> ChronosResult<()> {
        let scan_interval = Duration::from_millis(self.config.scan_interval_ms);
        let mut ticker = interval(scan_interval);

        loop {
            ticker.tick().await;

            // Scan all chains in parallel
            let mut scan_futures = vec![];

            for (chain_id, scanner) in &mut self.chains {
                let chain_id = *chain_id;
                scan_futures.push(async move { scanner.scan().await.map(|txs| (chain_id, txs)) });
            }

            // Collect results
            let results = futures::future::join_all(scan_futures).await;

            let mut stats = self.stats.write().await;
            stats.chains_monitored = self.chains.len();

            for (chain_id, pending_txs) in results.into_iter().flatten() {
                stats.total_pending += pending_txs.len();

                // Process each pending transaction
                for tx in pending_txs {
                    if let Some(intent) =
                        self.intent_detector
                            .detect(chain_id, &tx.data, tx.sender, tx.gas_price)
                    {
                        stats.swap_intents_detected += 1;

                        // Send intent for processing
                        if self.intent_tx.send(intent).await.is_err() {
                            // Channel closed, stop scanning
                            return Err(ChronosError::MempoolScanFailed(
                                "Intent channel closed".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> MempoolStats {
        self.stats.read().await.clone()
    }

    /// Get chain status
    pub async fn get_chain_status(&self, chain_id: ChainId) -> Option<ChainStatus> {
        self.chains.get(&chain_id).map(|s| s.get_status())
    }

    /// Add a new chain to monitor
    pub fn add_chain(&mut self, config: ChainConfig) {
        self.chains
            .insert(config.chain_id, ChainScanner::new(config));
    }

    /// Remove a chain from monitoring
    pub fn remove_chain(&mut self, chain_id: ChainId) {
        self.chains.remove(&chain_id);
    }
}

/// Per-chain mempool scanner
struct ChainScanner {
    config: ChainConfig,
    pending_txs: HashMap<Hash, PendingTx>,
    last_scan: Timestamp,
    current_block: u64,
    is_connected: bool,
}

impl ChainScanner {
    fn new(config: ChainConfig) -> Self {
        Self {
            config,
            pending_txs: HashMap::new(),
            last_scan: 0,
            current_block: 0,
            is_connected: false,
        }
    }

    /// Scan mempool for new pending transactions
    async fn scan(&mut self) -> ChronosResult<Vec<PendingTx>> {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        self.last_scan = now;

        // Connect to chain RPC
        let pending = self.fetch_pending_transactions().await?;

        // Filter new transactions
        let mut new_txs = vec![];
        for tx in pending {
            if let std::collections::hash_map::Entry::Vacant(slot) = self.pending_txs.entry(tx.hash)
            {
                slot.insert(tx.clone());
                new_txs.push(tx);
            }
        }

        // Cleanup old pending txs (they've either been mined or dropped)
        self.cleanup_stale_txs();

        Ok(new_txs)
    }

    /// Fetch pending transactions from RPC
    async fn fetch_pending_transactions(&mut self) -> ChronosResult<Vec<PendingTx>> {
        // There is no mempool client here: fetching pending transactions needs
        // an `eth_subscribe` websocket, `eth_getBlockByNumber("pending", true)`
        // or a bloxroute/flashbots stream, none of which is wired up. This used
        // to set `is_connected = true` and return an empty vector, so a scanner
        // reported itself connected while reading nothing — and `ChainStatus`
        // published that as healthy.
        Err(ChronosError::MempoolScanFailed(NOT_IMPLEMENTED.to_string()))
    }

    /// Remove stale transactions
    fn cleanup_stale_txs(&mut self) {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let max_age = 60_000; // 60 seconds

        self.pending_txs
            .retain(|_, tx| now - tx.detected_at < max_age);
    }

    /// Get chain status
    fn get_status(&self) -> ChainStatus {
        ChainStatus {
            chain_id: self.config.chain_id,
            name: self.config.name.clone(),
            is_connected: self.is_connected,
            current_block: self.current_block,
            pending_txs: self.pending_txs.len(),
            avg_block_time_ms: self.config.block_time_ms,
            last_updated: self.last_scan,
        }
    }
}

/// Pending transaction from mempool
#[derive(Debug, Clone)]
pub struct PendingTx {
    pub hash: Hash,
    pub sender: Address,
    pub to: Option<Address>,
    pub data: Vec<u8>,
    pub value: u128,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub nonce: u64,
    pub detected_at: Timestamp,
}

/// Mempool stream connection for real-time updates
pub struct MempoolStream {
    chain_id: ChainId,
    endpoint: String,
    is_connected: bool,
}

impl MempoolStream {
    pub fn new(chain_id: ChainId, endpoint: String) -> Self {
        Self {
            chain_id,
            endpoint,
            is_connected: false,
        }
    }

    /// Connect to mempool WebSocket stream
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // No websocket client is wired up. Setting `is_connected = true` and
        // returning `Ok(())` made a stub indistinguishable from a live stream,
        // and `subscribe` then handed back a receiver that never receives.
        let _ = &self.endpoint;
        Err(ChronosError::MempoolScanFailed(NOT_IMPLEMENTED.to_string()))
    }

    /// Subscribe to pending transactions
    pub async fn subscribe(&mut self) -> ChronosResult<mpsc::Receiver<PendingTx>> {
        if !self.is_connected {
            return Err(ChronosError::MempoolScanFailed(
                "cannot subscribe to a mempool stream that is not connected".to_string(),
            ));
        }

        let (tx, rx) = mpsc::channel(10000);

        // Spawn background task to receive transactions
        let endpoint = self.endpoint.clone();
        let chain_id = self.chain_id;

        tokio::spawn(async move {
            // WebSocket connection and subscription logic
            // eth_subscribe("newPendingTransactions")
            loop {
                // Receive and forward pending txs
                tokio::time::sleep(Duration::from_millis(10)).await;

                // In production: receive from WebSocket and send to channel
            }
        });

        Ok(rx)
    }
}

/// Bloxroute mempool stream (premium mempool data)
pub struct BloxrouteMempoolStream {
    auth_token: String,
    is_connected: bool,
}

impl BloxrouteMempoolStream {
    pub fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            is_connected: false,
        }
    }

    /// Connect to Bloxroute mempool stream
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // Bloxroute would give faster mempool data, but no client exists.
        let _ = &self.auth_token;
        Err(ChronosError::MempoolScanFailed(NOT_IMPLEMENTED.to_string()))
    }
}

/// Flashbots mempool stream (private transactions)
pub struct FlashbotsMempoolStream {
    relay_url: String,
    is_connected: bool,
}

impl FlashbotsMempoolStream {
    pub fn new(relay_url: String) -> Self {
        Self {
            relay_url,
            is_connected: false,
        }
    }

    /// Connect to Flashbots relay for private transaction hints
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // Flashbots would give private hints, but no relay client exists.
        let _ = &self.relay_url;
        Err(ChronosError::MempoolScanFailed(NOT_IMPLEMENTED.to_string()))
    }
}

/// Multi-stream aggregator
pub struct MempoolAggregator {
    streams: Vec<Box<dyn MempoolProvider + Send + Sync>>,
    dedupe_window: HashMap<Hash, Timestamp>,
}

impl MempoolAggregator {
    pub fn new() -> Self {
        Self {
            streams: vec![],
            dedupe_window: HashMap::new(),
        }
    }

    /// Add a mempool provider
    pub fn add_provider(&mut self, provider: Box<dyn MempoolProvider + Send + Sync>) {
        self.streams.push(provider);
    }

    /// Aggregate pending transactions from all providers
    pub async fn aggregate(&mut self) -> ChronosResult<Vec<PendingTx>> {
        let mut all_txs = vec![];
        let now = chrono::Utc::now().timestamp_millis() as u64;

        for stream in &mut self.streams {
            if let Ok(txs) = stream.get_pending().await {
                for tx in txs {
                    // Deduplicate
                    if let std::collections::hash_map::Entry::Vacant(slot) =
                        self.dedupe_window.entry(tx.hash)
                    {
                        slot.insert(now);
                        all_txs.push(tx);
                    }
                }
            }
        }

        // Cleanup old dedupe entries
        self.dedupe_window.retain(|_, ts| now - *ts < 60_000);

        Ok(all_txs)
    }
}

impl Default for MempoolAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for mempool data providers
#[async_trait::async_trait]
pub trait MempoolProvider {
    async fn get_pending(&mut self) -> ChronosResult<Vec<PendingTx>>;
    fn chain_id(&self) -> ChainId;
    fn is_connected(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `connect` used to answer `Ok(())` without opening anything. A
    /// trading loop that trusted it would be acting on a mempool it never read.
    #[tokio::test]
    async fn mempool_connects_refuse_instead_of_claiming_success() {
        let mut stream = MempoolStream::new(1, "wss://example.invalid".to_string());
        let err = stream.connect().await.expect_err("connect must refuse");
        assert!(err.to_string().contains("not implemented"), "{err}");
        assert!(
            !stream.is_connected,
            "a refused connection must not report itself connected"
        );

        let mut bloxroute = BloxrouteMempoolStream::new("token".to_string());
        assert!(bloxroute.connect().await.is_err());
        assert!(!bloxroute.is_connected);

        let mut flashbots = FlashbotsMempoolStream::new("https://relay.invalid".to_string());
        assert!(flashbots.connect().await.is_err());
        assert!(!flashbots.is_connected);
    }

    /// Subscribing to a stream that was never connected is refused rather than
    /// handing back a receiver that never receives.
    #[tokio::test]
    async fn subscribe_requires_a_connection() {
        let mut stream = MempoolStream::new(1, "wss://example.invalid".to_string());
        let err = stream
            .subscribe()
            .await
            .expect_err("subscribe must refuse without a connection");
        assert!(err.to_string().contains("not connected"), "{err}");
    }
}
