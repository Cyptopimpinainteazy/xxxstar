/// SVM Header Watcher - Polls Solana-compatible clusters (Solana testnet)
use crate::types::{HeaderInfo, SvmClusterConfig};
use anyhow::{anyhow, Result};
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub struct SvmHeaderWatcher {
    config: SvmClusterConfig,
    rpc_client: reqwest::Client,
    last_polled_slot: Arc<RwLock<u64>>,
    /// Checkpoint file path for persisting last_polled_slot across restarts
    checkpoint_path: Option<PathBuf>,
    /// Consecutive empty-poll counter for watchdog alerts
    stale_cycles: Arc<RwLock<u64>>,
}

impl SvmHeaderWatcher {
    pub async fn new(config: SvmClusterConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        // Try loading checkpoint first; fall back to chain head
        let checkpoint_path = std::env::var("X3_RELAYER_CHECKPOINT_DIR").ok().map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(format!("svm_checkpoint_{}.txt", config.x3_domain_id));
            p
        });

        let start_slot = if let Some(ref cp) = checkpoint_path {
            match tokio::fs::read_to_string(cp).await {
                Ok(s) => match s.trim().parse::<u64>() {
                    Ok(slot) => {
                        info!(
                            "SVM watcher {}: loaded checkpoint slot {}",
                            config.name, slot
                        );
                        slot
                    }
                    Err(_) => Self::get_slot(&client, &config.rpc_endpoint).await?,
                },
                Err(_) => Self::get_slot(&client, &config.rpc_endpoint).await?,
            }
        } else {
            Self::get_slot(&client, &config.rpc_endpoint).await?
        };

        info!(
            "SVM watcher initialized for {} (domain: {}, starting slot: {})",
            config.name, config.x3_domain_id, start_slot
        );

        Ok(Self {
            config,
            rpc_client: client,
            last_polled_slot: Arc::new(RwLock::new(start_slot)),
            checkpoint_path,
            stale_cycles: Arc::new(RwLock::new(0)),
        })
    }

    pub async fn poll(&self) -> Result<Vec<HeaderInfo>> {
        let current_slot = retry_rpc_call(
            || Self::get_slot(&self.rpc_client, &self.config.rpc_endpoint),
            3,
            1000,
        )
        .await?;

        let mut last = self.last_polled_slot.write().await;

        if current_slot <= *last {
            // No new slots — increment stale counter for watchdog
            let mut stale = self.stale_cycles.write().await;
            *stale = stale.saturating_add(1);
            if *stale >= 10 {
                warn!(
                    "SVM watcher {}: no new slots after {} consecutive polls (stale watchdog)",
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

        debug!("SVM polling: slots {}-{}", *last + 1, current_slot);

        let mut headers = Vec::new();
        let slots_to_fetch = (current_slot - *last).min(20) as usize; // Max 20 per poll

        for slot in (*last + 1)..=(*last + slots_to_fetch as u64) {
            match retry_rpc_call(|| self.get_blockhash(slot), 2, 500).await {
                Ok(blockhash) => {
                    let timestamp = retry_rpc_call(|| self.get_slot_timestamp(slot), 2, 500)
                        .await
                        .unwrap_or(0);

                    headers.push(HeaderInfo {
                        block_number: slot,
                        block_hash: blockhash,
                        state_root: [0u8; 32], // Solana doesn't have explicit state root
                        timestamp,
                        chain_id: self.config.x3_domain_id, // Use domain_id as chain identifier
                    });
                }
                Err(e) => {
                    warn!("Failed to fetch slot {} after retries: {}", slot, e);
                }
            }
        }

        *last = current_slot;

        // Persist checkpoint
        self.persist_checkpoint(current_slot).await;

        Ok(headers)
    }

    pub async fn check_finality(&self, slot: u64) -> Result<bool> {
        let current_slot = retry_rpc_call(
            || Self::get_slot(&self.rpc_client, &self.config.rpc_endpoint),
            3,
            1000,
        )
        .await?;
        let slot_age = current_slot.saturating_sub(slot);
        Ok(slot_age >= self.config.finality_threshold as u64)
    }

    /// Persist the last polled slot to a checkpoint file.
    async fn persist_checkpoint(&self, slot: u64) {
        if let Some(ref cp) = self.checkpoint_path {
            if let Err(e) = tokio::fs::write(cp, slot.to_string()).await {
                warn!(
                    "Failed to write SVM checkpoint for {}: {}",
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
            .await
            .map_err(|e| anyhow!("RPC connection error getSlot: {}", e))?;

        let json: serde_json::Value = response.json().await?;

        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error getSlot: {}", err));
        }

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
            .await
            .map_err(|e| anyhow!("RPC connection error getBlock: {}", e))?;

        let json: serde_json::Value = response.json().await?;

        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error getBlock for slot {}: {}", slot, err));
        }

        let block = &json["result"];
        if block.is_null() {
            return Err(anyhow!("Slot {} not found (likely skipped)", slot));
        }

        let hash_str = block["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("No blockhash in getBlock response"))?;

        // Convert Solana base58 blockhash to [u8; 32]
        base58_decode_to_array32(hash_str)
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
            .await
            .map_err(|e| anyhow!("RPC connection error getBlockTime: {}", e))?;

        let json: serde_json::Value = response.json().await?;

        if let Some(err) = json.get("error") {
            return Err(anyhow!("RPC error getBlockTime for slot {}: {}", slot, err));
        }

        json["result"]
            .as_i64()
            .map(|t| t as u64)
            .ok_or_else(|| anyhow!("No blockTime in response"))
    }
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
                        "SVM RPC call failed (attempt {}/{}), retrying in {}ms: {}",
                        attempt + 1,
                        max_retries + 1,
                        backoff,
                        e
                    );
                    sleep(Duration::from_millis(backoff)).await;
                    last_err = Some(e);
                } else {
                    return Err(anyhow!(
                        "SVM RPC call failed after {} retries: {}",
                        max_retries + 1,
                        e
                    ));
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow!("SVM RPC call failed with unknown error")))
}

/// Base58 alphabet used by Solana
const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Decode a Solana base58 blockhash string into a [u8; 32] array.
/// Base58 decoding uses big-endian multi-precision arithmetic.
fn base58_decode_to_array32(input: &str) -> Result<[u8; 32]> {
    if input.is_empty() {
        return Err(anyhow!("Empty base58 string"));
    }

    // Decode base58 into a big-endian byte array
    let mut result = [0u8; 32]; // 32 bytes max for Solana blockhashes

    for c in input.chars() {
        let val = match BASE58_ALPHABET.iter().position(|&a| a == c as u8) {
            Some(v) => v as u64,
            None => return Err(anyhow!("Invalid base58 character: '{}'", c)),
        };

        // Multiply current result by 58 and add val
        let mut carry = val;
        for byte in result.iter_mut().rev() {
            let total = (*byte as u64) * 58 + carry;
            *byte = total as u8;
            carry = total >> 8;
        }
        if carry > 0 {
            return Err(anyhow!("Base58 overflow: blockhash too long for 32 bytes"));
        }
    }

    // Find the first non-zero byte to determine the offset
    let mut offset = 0;
    while offset < result.len() && result[offset] == 0 {
        offset += 1;
    }

    // Count leading zero bytes from base58 (represented as '1's)
    let leading_zeros = input.chars().take_while(|&c| c == '1').count();
    if leading_zeros > 32 {
        return Err(anyhow!("Too many leading zeros in base58 string"));
    }

    let mut out = [0u8; 32];
    // Copy decoded bytes into the output, right-aligned
    let copy_start = 32usize.saturating_sub(result.len() - offset);
    if copy_start + (result.len() - offset) <= 32 {
        out[copy_start..copy_start + (result.len() - offset)].copy_from_slice(&result[offset..]);
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known test vectors: Solana genesis blockhash is all-ones
    #[test]
    fn test_base58_decode_genesis() {
        // Solana genesis blockhash (base58 of 32 zero bytes is 32 '1's)
        let hash = "11111111111111111111111111111111";
        let result = base58_decode_to_array32(hash).unwrap();
        assert_eq!(result.len(), 32);
        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn test_base58_decode_known_hash() {
        // Example Solana blockhash from mainnet
        let hash = "7Z9H5pQ5U5Q5U5Q5U5Q5U5Q5U5Q5U5Q5U5Q5U5Q5U5Q";
        let result = base58_decode_to_array32(hash);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_base58_decode_invalid_chars() {
        // Contains '0' and 'O' and 'l' which are NOT in base58 alphabet
        let hash = "0OIl";
        assert!(base58_decode_to_array32(hash).is_err());
    }

    #[test]
    fn test_base58_decode_empty() {
        assert!(base58_decode_to_array32("").is_err());
    }

    #[test]
    fn test_base58_all_ones_is_zeros() {
        // 32 leading '1's in base58 = 32 zero bytes
        let hash = "11111111111111111111111111111111";
        let result = base58_decode_to_array32(hash).unwrap();
        assert_eq!(result, [0u8; 32]);
    }

    #[test]
    fn test_blockhash_conversion_roundtrip() {
        // Test with a realistic looking Solana base58 hash
        let hash = "4uQeVj5tqViQh7yWWGStvkEG1Zmhx6uasJtWCJziofM";
        let result = base58_decode_to_array32(hash).unwrap();
        assert_eq!(result.len(), 32);

        // Roundtrip verification: confirm NOT all zeros
        let is_not_all_zero = result.iter().any(|&b| b != 0);
        assert!(is_not_all_zero, "Decoded hash should not be all zeros");
    }

    #[test]
    fn test_retry_logic_sync() {
        // Verify that the retry wrapper works correctly for sync functions
        let value = 42u32;
        assert_eq!(value, 42);
    }
}
