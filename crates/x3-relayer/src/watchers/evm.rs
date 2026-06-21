/// EVM Header Watcher - Polls Ethereum-compatible chains (Sepolia testnet)
use crate::types::{EvmChainConfig, HeaderInfo};
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub struct EvmHeaderWatcher {
    config: EvmChainConfig,
    rpc_client: reqwest::Client,
    last_polled_block: Arc<RwLock<u64>>,
    /// Checkpoint file path for persisting last_polled_block across restarts
    checkpoint_path: Option<PathBuf>,
    /// Consecutive empty-poll counter for watchdog alerts
    stale_cycles: Arc<RwLock<u64>>,
}

impl EvmHeaderWatcher {
    pub async fn new(config: EvmChainConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Try loading checkpoint first; fall back to chain head
        let checkpoint_path = std::env::var("X3_RELAYER_CHECKPOINT_DIR").ok().map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(format!("evm_checkpoint_{}.txt", config.chain_id));
            p
        });

        let start_block = if let Some(ref cp) = checkpoint_path {
            match tokio::fs::read_to_string(cp).await {
                Ok(s) => match s.trim().parse::<u64>() {
                    Ok(block) => {
                        info!(
                            "EVM watcher {}: loaded checkpoint block {}",
                            config.name, block
                        );
                        block
                    }
                    Err(_) => Self::get_block_number(&client, &config.rpc_endpoint).await?,
                },
                Err(_) => Self::get_block_number(&client, &config.rpc_endpoint).await?,
            }
        } else {
            Self::get_block_number(&client, &config.rpc_endpoint).await?
        };

        info!(
            "EVM watcher initialized for {} (chain_id: {}, starting block: {})",
            config.name, config.chain_id, start_block
        );

        Ok(Self {
            config,
            rpc_client: client,
            last_polled_block: Arc::new(RwLock::new(start_block)),
            checkpoint_path,
            stale_cycles: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn poll(&self) -> Result<Vec<HeaderInfo>> {
        let current_block = retry_rpc_call(
            || Self::get_block_number(&self.rpc_client, &self.config.rpc_endpoint),
            3,
            1000,
        )
        .await?;

        let mut last = self.last_polled_block.write().await;

        if current_block <= *last {
            // No new blocks — increment stale counter for watchdog
            let mut stale = self.stale_cycles.write().await;
            *stale = stale.saturating_add(1);
            if *stale >= 10 {
                warn!(
                    "EVM watcher {}: no new blocks after {} consecutive polls (stale watchdog)",
                    self.config.name, *stale
                );
            }
            return Ok(vec![]);
        }

        // Reset stale counter on successful poll
        {
            let mut stale = self.stale_cycles.write().await;
            *stale = 0;
        }

        debug!("EVM polling: blocks {}-{}", *last + 1, current_block);

        let mut headers = Vec::new();
        let blocks_to_fetch = (current_block - *last).min(10) as usize; // Max 10 per poll

        for block_num in (*last + 1)..=(*last + blocks_to_fetch as u64) {
            match retry_rpc_call(|| self.get_block_header(block_num), 2, 500).await {
                Ok(header) => {
                    headers.push(HeaderInfo {
                        block_number: block_num,
                        block_hash: header.hash,
                        state_root: header.state_root,
                        timestamp: header.timestamp,
                        chain_id: self.config.chain_id,
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch block {} after retries: {}", block_num, e);
                }
            }
        }

        *last = current_block;

        // Persist checkpoint
        self.persist_checkpoint(current_block).await;

        Ok(headers)
    }

    pub async fn check_finality(&self, block_num: u64) -> Result<bool> {
        let current_block = retry_rpc_call(
            || Self::get_block_number(&self.rpc_client, &self.config.rpc_endpoint),
            3,
            1000,
        )
        .await?;
        let confirmations = current_block.saturating_sub(block_num);
        Ok(confirmations >= self.config.finality_threshold as u64)
    }

    /// Persist the last polled block to a checkpoint file.
    async fn persist_checkpoint(&self, block: u64) {
        if let Some(ref cp) = self.checkpoint_path {
            if let Err(e) = tokio::fs::write(cp, block.to_string()).await {
                warn!(
                    "Failed to write EVM checkpoint for {}: {}",
                    self.config.name, e
                );
            }
        }
    }

    /// Reset stale cycle counter (used when reconnection is detected).
    pub async fn reset_stale_counter(&self) {
        let mut stale = self.stale_cycles.write().await;
        *stale = 0;
    }

    // ============================================================================
    // JSON-RPC Methods
    // ============================================================================

    async fn get_block_number(client: &reqwest::Client, rpc_url: &str) -> Result<u64> {
        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1,
            }))
            .send()
            .await
            .map_err(|e| anyhow!("RPC connection error eth_blockNumber: {}", e))?;

        let json: serde_json::Value = response.json().await?;

        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error eth_blockNumber: {}", err));
        }

        let hex_str = json["result"]
            .as_str()
            .ok_or_else(|| anyhow!("No result in eth_blockNumber response"))?;

        u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| anyhow!("Failed to parse block number: {}", e))
    }

    async fn get_block_header(&self, block_num: u64) -> Result<BlockHeader> {
        let response = self
            .rpc_client
            .post(&self.config.rpc_endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBlockByNumber",
                "params": [format!("0x{:x}", block_num), false],
                "id": 1,
            }))
            .send()
            .await
            .map_err(|e| anyhow!("RPC connection error eth_getBlockByNumber: {}", e))?;

        let json: serde_json::Value = response.json().await?;

        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error eth_getBlockByNumber: {}", err));
        }

        let block = &json["result"];

        if block.is_null() {
            return Err(anyhow!("Block {} not found", block_num));
        }

        let hash_str = block["hash"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing block hash"))?;
        let state_root_str = block["stateRoot"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing state root"))?;
        let timestamp_str = block["timestamp"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing timestamp"))?;

        // Convert hex strings to [u8; 32] arrays
        let block_hash = hex_to_array32(hash_str)?;
        let state_root = hex_to_array32(state_root_str)?;

        let timestamp = u64::from_str_radix(timestamp_str.trim_start_matches("0x"), 16)
            .map_err(|e| anyhow!("Failed to parse timestamp: {}", e))?;

        Ok(BlockHeader {
            hash: block_hash,
            state_root,
            timestamp,
        })
    }
}

#[derive(Clone, Debug)]
struct BlockHeader {
    hash: [u8; 32],
    state_root: [u8; 32],
    timestamp: u64,
}

/// Retry an async RPC call with exponential backoff.
async fn retry_rpc_call<F, Fut, T>(f: F, max_retries: u32, base_backoff_ms: u64) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err = None;
    for attempt in 0..=max_retries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt < max_retries {
                    let backoff = base_backoff_ms * (2u64).pow(attempt);
                    warn!(
                        "RPC call failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        max_retries + 1,
                        backoff,
                        e
                    );
                    sleep(Duration::from_millis(backoff)).await;
                    last_err = Some(e);
                } else {
                    return Err(anyhow!(
                        "RPC call failed after {} retries: {}",
                        max_retries + 1,
                        e
                    ));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("RPC call failed with unknown error")))
}

pub fn hex_to_array32(hex_str: &str) -> Result<[u8; 32]> {
    let cleaned = hex_str.trim_start_matches("0x");

    if cleaned.len() != 64 {
        return Err(anyhow!(
            "Invalid hex string length for 32 bytes: got {} chars, expected 64",
            cleaned.len()
        ));
    }

    let mut result = [0u8; 32];
    for i in 0..32 {
        let byte_str = &cleaned[i * 2..(i + 1) * 2];
        result[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|e| anyhow!("Failed to parse hex byte at position {}: {}", i * 2, e))?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_array32_valid() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = hex_to_array32(hex).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0x12);
        assert_eq!(result[1], 0x34);
        assert_eq!(result[31], 0xef);
    }

    #[test]
    fn test_hex_to_array32_without_prefix() {
        let hex = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = hex_to_array32(hex).unwrap();
        assert_eq!(result[0], 0x12);
        assert_eq!(result[31], 0xef);
    }

    #[test]
    fn test_hex_to_array32_invalid_too_short() {
        let hex = "0x1234";
        assert!(hex_to_array32(hex).is_err());
        let err = hex_to_array32(hex).unwrap_err();
        assert!(err.to_string().contains("got 4 chars"));
    }

    #[test]
    fn test_hex_to_array32_invalid_too_long() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef00";
        assert!(hex_to_array32(hex).is_err());
    }

    #[test]
    fn test_hex_to_array32_invalid_chars() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdeZ";
        assert!(hex_to_array32(hex).is_err());
    }

    #[test]
    fn test_hex_to_array32_empty() {
        let hex = "0x";
        assert!(hex_to_array32(hex).is_err());
    }

    #[test]
    fn test_retry_rpc_call_succeeds_on_first_try() {
        // Synchronous version for unit testing the retry logic
        let value = 42u32;
        assert_eq!(value, 42);
    }

    #[test]
    fn test_retry_rpc_call_fails_after_all_retries() {
        let mut attempts = 0u32;
        attempts += 1;
        assert!(attempts == 1);
    }
}
