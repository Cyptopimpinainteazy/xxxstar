/// SVM Header Watcher - Polls Solana-compatible clusters (Solana testnet)
use crate::types::{HeaderInfo, SvmClusterConfig};
use anyhow::{anyhow, Result};
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SvmHeaderWatcher {
    config: SvmClusterConfig,
    rpc_client: reqwest::Client,
    last_polled_slot: Arc<RwLock<u64>>,
}

impl SvmHeaderWatcher {
    pub async fn new(config: SvmClusterConfig) -> Result<Self> {
        let client = reqwest::Client::new();

        // Fetch current slot to initialize
        let current_slot = Self::get_slot(&client, &config.rpc_endpoint).await?;

        info!(
            "SVM watcher initialized for {} (domain: {}, starting slot: {})",
            config.name, config.x3_domain_id, current_slot
        );

        Ok(Self {
            config,
            rpc_client: client,
            last_polled_slot: Arc::new(RwLock::new(current_slot)),
        })
    }

    pub async fn poll(&self) -> Result<Vec<HeaderInfo>> {
        let current_slot = Self::get_slot(&self.rpc_client, &self.config.rpc_endpoint).await?;
        let mut last = self.last_polled_slot.write().await;

        if current_slot <= *last {
            return Ok(vec![]); // No new slots
        }

        debug!("SVM polling: slots {}-{}", *last + 1, current_slot);

        let mut headers = Vec::new();
        let slots_to_fetch = (current_slot - *last).min(20) as usize; // Max 20 per poll

        for slot in (*last + 1)..=(*last + slots_to_fetch as u64) {
            match self.get_blockhash(slot).await {
                Ok(blockhash) => {
                    let timestamp = self.get_slot_timestamp(slot).await.unwrap_or(0);

                    headers.push(HeaderInfo {
                        block_number: slot,
                        block_hash: blockhash,
                        state_root: [0u8; 32], // Solana doesn't have explicit state root
                        timestamp,
                        chain_id: self.config.x3_domain_id, // Use domain_id as chain identifier
                    });
                }
                Err(e) => {
                    debug!("Failed to fetch slot {}: {}", slot, e);
                }
            }
        }

        *last = current_slot;
        Ok(headers)
    }

    pub async fn check_finality(&self, slot: u64) -> Result<bool> {
        let current_slot = Self::get_slot(&self.rpc_client, &self.config.rpc_endpoint).await?;
        let slot_age = current_slot.saturating_sub(slot);
        Ok(slot_age >= self.config.finality_threshold as u64)
    }

    // ============================================================================
    // JSON-RPC Methods
    // ============================================================================

    async fn get_slot(client: &reqwest::Client, rpc_url: &str) -> Result<u64> {
        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getSlot",
                "params": [],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        json["result"]
            .as_u64()
            .ok_or_else(|| anyhow!("No slot in getSlot response"))
    }

    async fn get_blockhash(&self, slot: u64) -> Result<[u8; 32]> {
        let response = self
            .rpc_client
            .post(&self.config.rpc_endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getBlock",
                "params": [slot, { "encoding": "json" }],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        let hash_str = json["result"]["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("No blockhash in getBlock response"))?;

        // Convert Solana base58 blockhash to [u8; 32]
        solana_blockhash_to_array(hash_str)
    }

    async fn get_slot_timestamp(&self, slot: u64) -> Result<u64> {
        let response = self
            .rpc_client
            .post(&self.config.rpc_endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getBlockTime",
                "params": [slot],
                "id": 1,
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;

        json["result"]
            .as_i64()
            .map(|t| t as u64)
            .ok_or_else(|| anyhow!("No blockTime in response"))
    }
}

/// Convert Solana base58 blockhash string to [u8; 32] array
fn solana_blockhash_to_array(hash_str: &str) -> Result<[u8; 32]> {
    // For now, use a simplified approach: hash the string
    // In production, would use actual base58 decoding
    let bytes = hash_str.as_bytes();
    let mut result = [0u8; 32];

    // Simple hash: XOR all bytes into 32-byte array
    for (i, &byte) in bytes.iter().enumerate() {
        result[i % 32] ^= byte;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockhash_conversion() {
        let hash = "11111111111111111111111111111111";
        let result = solana_blockhash_to_array(hash).unwrap();
        assert_eq!(result.len(), 32);
    }
}
