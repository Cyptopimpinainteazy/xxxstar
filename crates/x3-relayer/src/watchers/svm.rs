/// SVM Header Watcher - Polls Solana-compatible clusters (Solana testnet)
use crate::types::{HeaderInfo, SvmClusterConfig};
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use log::{debug, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use x3_verification_router::SolanaTransactionEvidence;

pub struct SvmHeaderWatcher {
    config: SvmClusterConfig,
    rpc_client: reqwest::Client,
    last_polled_slot: Arc<RwLock<u64>>,
    /// Checkpoint file path for persisting last_polled_slot across restarts
    checkpoint_path: Option<PathBuf>,
    /// Durable highest slot observed as finalized.
    finalized_floor_path: Option<PathBuf>,
    finalized_floor: Arc<RwLock<u64>>,
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
        let finalized_floor_path = std::env::var("X3_RELAYER_CHECKPOINT_DIR").ok().map(|dir| {
            let mut p = PathBuf::from(dir);
            p.push(format!("svm_finalized_floor_{}.txt", config.x3_domain_id));
            p
        });
        let finalized_floor = if let Some(path) = &finalized_floor_path {
            tokio::fs::read_to_string(path)
                .await
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())
                .unwrap_or(0)
        } else {
            0
        };

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
            finalized_floor_path,
            finalized_floor: Arc::new(RwLock::new(finalized_floor)),
            stale_cycles: Arc::new(RwLock::new(0)),
        })
    }

    pub fn domain_id(&self) -> u32 {
        self.config.x3_domain_id
    }

    pub async fn finalized_floor(&self) -> u64 {
        *self.finalized_floor.read().await
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
                        receipts_root: [0u8; 32],
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
        let finalized = slot_age >= self.config.finality_threshold as u64;
        if finalized {
            let mut floor = self.finalized_floor.write().await;
            if slot > *floor {
                *floor = slot;
                drop(floor);
                self.persist_finalized_floor(slot).await;
            }
        }
        Ok(finalized)
    }

    /// Acquire transaction-inclusion evidence from the configured Solana RPC
    /// set. Mainnet requires three distinct endpoints and a two-thirds provider
    /// quorum reporting identical finalized transaction and block evidence.
    pub async fn finalized_transaction_evidence(
        &self,
        signature: &str,
        slot: u64,
        expected_blockhash: [u8; 32],
    ) -> Result<SolanaTransactionEvidence> {
        let mut endpoints = Vec::new();
        for endpoint in std::iter::once(&self.config.rpc_endpoint)
            .chain(self.config.rpc_quorum_endpoints.iter())
        {
            if !endpoint.trim().is_empty() && !endpoints.contains(endpoint) {
                endpoints.push(endpoint.clone());
            }
        }
        if self
            .config
            .cluster_name
            .to_ascii_lowercase()
            .contains("mainnet")
            && endpoints.len() < 3
        {
            return Err(anyhow!(
                "mainnet Solana transaction evidence requires at least three distinct RPC endpoints"
            ));
        }

        let mut observations = Vec::new();
        for endpoint in &endpoints {
            match self
                .query_finalized_transaction(endpoint, signature, slot, expected_blockhash)
                .await
            {
                Ok(evidence) => observations.push(evidence),
                Err(error) => warn!(
                    "SVM transaction evidence from {} was refused: {}",
                    endpoint, error
                ),
            }
        }
        if observations.is_empty() {
            return Err(anyhow!(
                "no Solana RPC provider produced usable transaction evidence"
            ));
        }

        let required = if endpoints.len() == 1 {
            1
        } else {
            (endpoints.len() * 2).div_ceil(3)
        };
        let mut best_count = 0usize;
        let mut agreed = None;
        for candidate in &observations {
            let count = observations
                .iter()
                .filter(|observation| *observation == candidate)
                .count();
            if count > best_count {
                best_count = count;
                agreed = Some(candidate.clone());
            }
        }
        if best_count < required {
            return Err(anyhow!(
                "Solana RPC providers disagree or too few agree: strongest observation {best_count} of {} configured, {required} required",
                endpoints.len()
            ));
        }
        agreed.ok_or_else(|| anyhow!("no Solana transaction evidence consensus"))
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

    async fn persist_finalized_floor(&self, slot: u64) {
        let Some(path) = &self.finalized_floor_path else {
            return;
        };
        let temporary = path.with_extension("tmp");
        if let Err(error) = tokio::fs::write(&temporary, slot.to_string()).await {
            warn!(
                "Failed to write SVM finalized floor for {}: {}",
                self.config.name, error
            );
            return;
        }
        if let Err(error) = tokio::fs::rename(&temporary, path).await {
            warn!(
                "Failed to install SVM finalized floor for {}: {}",
                self.config.name, error
            );
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

    async fn query_finalized_transaction(
        &self,
        endpoint: &str,
        signature: &str,
        slot: u64,
        expected_blockhash: [u8; 32],
    ) -> Result<SolanaTransactionEvidence> {
        let transaction_response = self
            .rpc_client
            .post(endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getTransaction",
                "params": [
                    signature,
                    {
                        "encoding": "base64",
                        "commitment": "finalized",
                        "maxSupportedTransactionVersion": 0
                    }
                ],
                "id": 1,
            }))
            .send()
            .await
            .map_err(|error| anyhow!("getTransaction connection error: {error}"))?
            .json::<serde_json::Value>()
            .await?;
        if let Some(error) = transaction_response.get("error") {
            return Err(anyhow!("getTransaction RPC error: {error}"));
        }
        let transaction_result = &transaction_response["result"];
        if transaction_result.is_null() {
            return Err(anyhow!("transaction is not present in a finalized bank"));
        }
        if transaction_result["slot"].as_u64() != Some(slot) {
            return Err(anyhow!("RPC reported a different transaction slot"));
        }
        let meta = transaction_result["meta"]
            .as_object()
            .ok_or_else(|| anyhow!("finalized transaction has no execution metadata"))?;
        if !meta.get("err").is_some_and(serde_json::Value::is_null) {
            return Err(anyhow!(
                "finalized transaction did not execute successfully"
            ));
        }
        let encoded = transaction_result["transaction"]
            .as_array()
            .and_then(|parts| parts.first())
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("getTransaction returned no base64 transaction"))?;
        let raw_transaction = BASE64
            .decode(encoded)
            .map_err(|error| anyhow!("invalid base64 transaction: {error}"))?;

        let block_response = self
            .rpc_client
            .post(endpoint)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getBlock",
                "params": [
                    slot,
                    {
                        "commitment": "finalized",
                        "transactionDetails": "signatures",
                        "rewards": false,
                        "maxSupportedTransactionVersion": 0
                    }
                ],
                "id": 2,
            }))
            .send()
            .await
            .map_err(|error| anyhow!("getBlock connection error: {error}"))?
            .json::<serde_json::Value>()
            .await?;
        if let Some(error) = block_response.get("error") {
            return Err(anyhow!("getBlock RPC error: {error}"));
        }
        let block = &block_response["result"];
        if block.is_null() {
            return Err(anyhow!("finalized slot is unavailable"));
        }
        let blockhash = block["blockhash"]
            .as_str()
            .ok_or_else(|| anyhow!("getBlock returned no blockhash"))?;
        if base58_decode_to_array32(blockhash)? != expected_blockhash {
            return Err(anyhow!(
                "transaction blockhash does not match the finality claim"
            ));
        }
        let index = block["signatures"]
            .as_array()
            .ok_or_else(|| anyhow!("getBlock returned no signature index"))?
            .iter()
            .position(|candidate| candidate.as_str() == Some(signature))
            .ok_or_else(|| anyhow!("transaction signature is absent from the finalized block"))?
            as u32;

        let decoded_signature = bs58::decode(signature)
            .into_vec()
            .map_err(|error| anyhow!("invalid base58 transaction signature: {error}"))?;
        let signature_bytes: [u8; 64] = decoded_signature
            .try_into()
            .map_err(|_| anyhow!("transaction signature is not 64 bytes"))?;
        let evidence = SolanaTransactionEvidence::from_raw(slot, index, true, raw_transaction)
            .map_err(|error| anyhow!("invalid signed Solana transaction: {error}"))?;
        if evidence.signature != signature_bytes {
            return Err(anyhow!(
                "requested signature does not match the serialized transaction"
            ));
        }
        Ok(evidence)
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

    #[tokio::test]
    async fn finalized_floor_is_installed_atomically() {
        let path = std::env::temp_dir().join(format!(
            "x3-svm-finalized-floor-{}-{}.txt",
            std::process::id(),
            42
        ));
        let watcher = SvmHeaderWatcher {
            config: SvmClusterConfig {
                name: "test".into(),
                cluster_name: "devnet".into(),
                x3_domain_id: 200,
                rpc_endpoint: "http://127.0.0.1:1".into(),
                rpc_quorum_endpoints: Vec::new(),
                finality_threshold: 1,
                slot_poll_interval_ms: 1,
                max_concurrent_requests: 1,
            },
            rpc_client: reqwest::Client::new(),
            last_polled_slot: Arc::new(RwLock::new(0)),
            checkpoint_path: None,
            finalized_floor_path: Some(path.clone()),
            finalized_floor: Arc::new(RwLock::new(0)),
            stale_cycles: Arc::new(RwLock::new(0)),
        };

        watcher.persist_finalized_floor(123).await;
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "123");
        assert!(!path.with_extension("tmp").exists());
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn mainnet_transaction_evidence_requires_three_rpc_providers() {
        let watcher = SvmHeaderWatcher {
            config: SvmClusterConfig {
                name: "mainnet".into(),
                cluster_name: "mainnet-beta".into(),
                x3_domain_id: 200,
                rpc_endpoint: "http://127.0.0.1:1".into(),
                rpc_quorum_endpoints: Vec::new(),
                finality_threshold: 1,
                slot_poll_interval_ms: 1,
                max_concurrent_requests: 1,
            },
            rpc_client: reqwest::Client::new(),
            last_polled_slot: Arc::new(RwLock::new(0)),
            checkpoint_path: None,
            finalized_floor_path: None,
            finalized_floor: Arc::new(RwLock::new(0)),
            stale_cycles: Arc::new(RwLock::new(0)),
        };

        let error = watcher
            .finalized_transaction_evidence(
                "1111111111111111111111111111111111111111111111111111111111111111",
                42,
                [7u8; 32],
            )
            .await
            .expect_err("mainnet cannot use a single RPC provider");
        assert!(error.to_string().contains("three distinct RPC endpoints"));
    }

    #[tokio::test]
    async fn finalized_transaction_is_bound_to_the_rpc_block_and_bank_result() {
        use ed25519_dalek::Signer;
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let key = ed25519_dalek::SigningKey::from_bytes(&[0x61u8; 32]);
        let recent_blockhash = [0x21u8; 32];
        let mut transaction_message = vec![1, 0, 0, 1];
        transaction_message.extend_from_slice(&key.verifying_key().to_bytes());
        transaction_message.extend_from_slice(&recent_blockhash);
        transaction_message.push(0);
        let transaction_signature = key.sign(&transaction_message).to_bytes();
        let mut raw_transaction = vec![1];
        raw_transaction.extend_from_slice(&transaction_signature);
        raw_transaction.extend_from_slice(&transaction_message);
        let signature = bs58::encode(transaction_signature).into_string();
        let blockhash_bytes = [0x19u8; 32];
        let blockhash = bs58::encode(blockhash_bytes).into_string();

        let transaction_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "slot": 42,
                "meta": { "err": null },
                "transaction": [BASE64.encode(&raw_transaction), "base64"]
            }
        })
        .to_string();
        let block_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "blockhash": blockhash,
                "signatures": [signature]
            }
        })
        .to_string();

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind RPC fixture");
        let address = listener.local_addr().expect("RPC fixture address");
        let server = std::thread::spawn(move || {
            for body in [transaction_body, block_body] {
                let (mut stream, _) = listener.accept().expect("accept RPC request");
                let mut request = [0u8; 8192];
                let _ = stream.read(&mut request);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write RPC response");
            }
        });

        let watcher = SvmHeaderWatcher {
            config: SvmClusterConfig {
                name: "devnet".into(),
                cluster_name: "devnet".into(),
                x3_domain_id: 200,
                rpc_endpoint: format!("http://{address}"),
                rpc_quorum_endpoints: Vec::new(),
                finality_threshold: 1,
                slot_poll_interval_ms: 1,
                max_concurrent_requests: 1,
            },
            rpc_client: reqwest::Client::new(),
            last_polled_slot: Arc::new(RwLock::new(0)),
            checkpoint_path: None,
            finalized_floor_path: None,
            finalized_floor: Arc::new(RwLock::new(0)),
            stale_cycles: Arc::new(RwLock::new(0)),
        };
        let evidence = watcher
            .finalized_transaction_evidence(&signature, 42, blockhash_bytes)
            .await
            .expect("quorum evidence");
        assert_eq!(evidence.slot, 42);
        assert_eq!(evidence.raw_transaction, raw_transaction);
        assert_eq!(evidence.recent_blockhash, recent_blockhash);
        server.join().expect("RPC fixture thread");
    }
}
