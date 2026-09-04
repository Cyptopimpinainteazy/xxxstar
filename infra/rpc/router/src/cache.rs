//! Response cache with chain-aware TTLs.
//!
//! Caches read-only RPC responses keyed by (method, params_hash).
//! State-changing methods NEVER go through the cache.
//! Supports in-memory cache (always available) and optional Redis backend.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// An entry in the cache.
#[derive(Debug, Clone)]
struct CacheEntry {
    response: String,
    inserted_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.ttl
    }
}

/// In-memory RPC response cache.
pub struct ResponseCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
}

impl ResponseCache {
    /// Maximum entries before GC is triggered.
    const MAX_ENTRIES: usize = 50_000;

    /// Create a new in-memory cache.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries: Self::MAX_ENTRIES,
        }
    }

    /// Try to get a cached response. Returns None if not cached or expired.
    pub async fn get(&self, method: &str, params_hash: &str) -> Option<String> {
        let key = cache_key(method, params_hash);
        let entries = self.entries.read().await;

        if let Some(entry) = entries.get(&key) {
            if !entry.is_expired() {
                return Some(entry.response.clone());
            }
        }

        None
    }

    /// Store a response in the cache with a method-aware TTL.
    pub async fn set(&self, method: &str, params_hash: &str, response: &str) {
        let ttl = default_ttl_for_method(method);
        if ttl.as_millis() == 0 {
            return; // don't cache methods with zero TTL
        }

        let key = cache_key(method, params_hash);
        let mut entries = self.entries.write().await;

        // GC if too many entries
        if entries.len() >= self.max_entries {
            // Remove expired entries
            entries.retain(|_, v| !v.is_expired());

            // If still too many, clear oldest 20%
            if entries.len() >= self.max_entries {
                let mut sorted: Vec<_> = entries.drain().collect();
                sorted.sort_by_key(|(_, v)| v.inserted_at);
                let keep = self.max_entries * 4 / 5;
                for (k, v) in sorted.into_iter().rev().take(keep) {
                    entries.insert(k, v);
                }
            }
        }

        entries.insert(
            key,
            CacheEntry {
                response: response.to_string(),
                inserted_at: Instant::now(),
                ttl,
            },
        );
    }

    /// Invalidate all entries for a specific method.
    pub async fn invalidate_method(&self, method: &str) {
        let prefix = format!("{}:", method);
        let mut entries = self.entries.write().await;
        entries.retain(|key, _| !key.starts_with(&prefix));
    }
}

/// Build a cache key from method and params hash.
fn cache_key(method: &str, params_hash: &str) -> String {
    format!("{}:{}", method, params_hash)
}

/// Default TTL for a method. Returns Duration::ZERO for never-cache methods.
fn default_ttl_for_method(method: &str) -> Duration {
    match method {
        // ── Never cache ────────────────────────────────────────────
        "eth_sendRawTransaction"
        | "eth_sendTransaction"
        | "sendTransaction"
        | "sendrawtransaction"
        | "x3_submitExtrinsic"
        | "eth_estimateGas"
        | "eth_call"                      // can change every block
        | "eth_getBalance"                // freshness matters
        | "getLatestBlockhash"            // NEVER cache — used for tx signing
        | "getSlot"                       // tight freshness
        | "getHealth"                     // ephemeral
        | "x3_getProof"                   // proofs expire
        | "x3_getFinalizedState"          // state changes
        | "x3_getAtomicRoute"             // dynamic routing
        => Duration::ZERO,

        // ── Very short cache (500ms) ───────────────────────────────
        "eth_blockNumber" => Duration::from_millis(500),
        "eth_getBlockByNumber" => Duration::from_millis(1000),

        // ── Short cache (2–5s) ─────────────────────────────────────
        "eth_getTransactionReceipt" => Duration::from_secs(2),
        "getBlock" => Duration::from_secs(2),
        "getAccountInfo" => Duration::from_secs(3),
        "getBalance" => Duration::from_secs(3),

        // ── Medium cache (30s) ─────────────────────────────────────
        "eth_getLogs" => Duration::from_secs(30),
        "x3_getBlock" => Duration::from_secs(10),

        // ── Long cache (1 hour) ────────────────────────────────────
        "eth_chainId" => Duration::from_secs(3600),
        "net_version" => Duration::from_secs(3600),
        "getGenesisHash" => Duration::from_secs(3600),
        "getblockhash" => Duration::from_secs(3600),

        // ── Default (5s) ───────────────────────────────────────────
        _ => Duration::from_secs(5),
    }
}

/// Hash function for request parameters.
pub fn hash_params(params: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_string(params).unwrap_or_default().as_bytes());
    format!("{:x}", hasher.finalize())
}

// ── Cached routing helper ──────────────────────────────────────────────────

/// Try to serve from cache, or execute the request and cache the result.
pub async fn cached_or_execute<F, Fut>(
    cache: &ResponseCache,
    method: &str,
    params_json: &str,
    fetch: F,
) -> anyhow::Result<String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<String>>,
{
    // Compute params hash
    let params: serde_json::Value = serde_json::from_str(params_json).unwrap_or(serde_json::Value::Null);
    let params_hash = hash_params(&params);

    // Check cache
    if let Some(cached) = cache.get(method, &params_hash).await {
        return Ok(cached);
    }

    // Execute
    let result = fetch().await?;

    // Cache successful result
    cache.set(method, &params_hash, &result).await;

    Ok(result)
}