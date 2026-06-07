/// EVM Header Watcher - Polls Ethereum-compatible chains (Sepolia testnet)
use crate::types::{EvmChainConfig, HeaderInfo};
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct EvmHeaderWatcher {
    config: EvmChainConfig,
    rpc_client: reqwest::Client,
    last_polled_block: Arc<RwLock<u64>>,
}

impl EvmHeaderWatcher {
    pub async fn new(config: EvmChainConfig) -> Result<Self> {
        let client = reqwest::Client::new();

        // Fetch current block number to initialize
        let current_block = Self::get_block_number(&client, &config.rpc_endpoint).await?;

        info!(
            "EVM watcher initialized for {} (chain_id: {}, starting block: {})",
            config.name, config.chain_id, current_block
        );

        Ok(Self {
            config,
            rpc_client: client,
            last_polled_block: Arc::new(RwLock::new(current_block)),
        })
    }

    pub async fn poll(&self) -> Result<Vec<HeaderInfo>> {
        let current_block =
            Self::get_block_number(&self.rpc_client, &self.config.rpc_endpoint).await?;
        let mut last = self.last_polled_block.write().await;

        if current_block <= *last {
            return Ok(vec![]); // No new blocks
        }

        debug!("EVM polling: blocks {}-{}", *last + 1, current_block);

        let mut headers = Vec::new();
        let blocks_to_fetch = (current_block - *last).min(10) as usize; // Max 10 per poll

        for block_num in (*last + 1)..=(*last + blocks_to_fetch as u64) {
            match self.get_block_header(block_num).await {
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
                    warn!("Failed to fetch block {}: {}", block_num, e);
                }
            }
        }

        *last = current_block;
        Ok(headers)
    }

    pub async fn check_finality(&self, block_num: u64) -> Result<bool> {
        let current_block =
            Self::get_block_number(&self.rpc_client, &self.config.rpc_endpoint).await?;
        let confirmations = current_block.saturating_sub(block_num);
        Ok(confirmations >= self.config.finality_threshold as u64)
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
            .await?;

        let json: serde_json::Value = response.json().await?;

        let hex_str = json["result"]
            .as_str()
            .ok_or_else(|| anyhow!("No result in eth_blockNumber response"))?;

        u64::from_str_radix(&hex_str[2..], 16)
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
            .await?;

        let json: serde_json::Value = response.json().await?;
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

        let timestamp = u64::from_str_radix(&timestamp_str[2..], 16)
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

fn hex_to_array32(hex_str: &str) -> Result<[u8; 32]> {
    let cleaned = hex_str.trim_start_matches("0x");

    if cleaned.len() != 64 {
        return Err(anyhow!("Invalid hex string length: {}", cleaned.len()));
    }

    let mut result = [0u8; 32];
    for i in 0..32 {
        let byte_str = &cleaned[i * 2..(i + 1) * 2];
        result[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|e| anyhow!("Failed to parse hex byte: {}", e))?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_array32() {
        let hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = hex_to_array32(hex).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result[0], 0x12);
        assert_eq!(result[1], 0x34);
    }

    #[test]
    fn test_hex_to_array32_invalid() {
        let hex = "0x1234"; // Too short
        assert!(hex_to_array32(hex).is_err());
    }
}
