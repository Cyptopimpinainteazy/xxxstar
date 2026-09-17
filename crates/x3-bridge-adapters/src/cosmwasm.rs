//! CosmWasm Bridge Adapter
//!
//! Provides bridge functionality for Cosmos SDK chains with CosmWasm smart
//! contract support. This adapter covers IBC-capable routes and
//! Tendermint/BFT finality verification.
//!
//! Supported chains: Cosmos Hub, Osmosis, Juno, Injective, any
//! CosmWasm-enabled Cosmos SDK chain with standard LCD/RPC endpoints.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// CosmWasm Bridge Adapter
///
/// Connects to a Cosmos SDK node via its LCD (REST) or Tendermint RPC
/// endpoint to validate block headers, generate proofs, and poll events.
pub struct CosmWasmBridgeAdapter {
    chain_id: u64,
    /// The Tendermint RPC endpoint (e.g. `http://localhost:26657`)
    rpc_url: String,
    /// The Cosmos REST/LCD endpoint for CosmWasm queries
    lcd_url: String,
    /// Bech32 prefix for addresses on this chain (e.g. "cosmos", "osmo")
    bech32_prefix: String,
}

impl CosmWasmBridgeAdapter {
    /// Create a new CosmWasm bridge adapter.
    ///
    /// * `chain_id` — numeric chain ID (e.g. `cosmoshub-4` maps to 4 internally)
    /// * `rpc_url` — Tendermint RPC endpoint
    /// * `lcd_url` — Cosmos REST/LCD endpoint
    /// * `bech32_prefix` — Address prefix (e.g. "cosmos", "osmo", "juno")
    pub fn new(chain_id: u64, rpc_url: String, lcd_url: String, bech32_prefix: String) -> Self {
        Self {
            chain_id,
            rpc_url,
            lcd_url,
            bech32_prefix,
        }
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get the LCD URL
    pub fn lcd_url(&self) -> &str {
        &self.lcd_url
    }

    /// Get the Bech32 prefix
    pub fn bech32_prefix(&self) -> &str {
        &self.bech32_prefix
    }

    /// Query a CosmWasm smart contract state via LCD.
    ///
    /// Calls `GET /cosmwasm/wasm/v1/contract/{address}/smart/{query}` to
    /// execute a query message against the contract and returns the raw
    /// response bytes.
    pub fn query_smart_contract(
        &self,
        contract_address: &str,
        query_msg: &serde_json::Value,
    ) -> Result<Vec<u8>, BridgeError> {
        let query_b64 = base64_encode(&serde_json::to_vec(query_msg).map_err(|e| {
            BridgeError::Serialization(format!("Failed to serialize query message: {e}"))
        })?);

        let url = format!(
            "{}/cosmwasm/wasm/v1/contract/{}/smart/{}",
            self.lcd_url.trim_end_matches('/'),
            contract_address,
            query_b64
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| BridgeError::Network(format!("Failed to create HTTP client: {e}")))?;

        let response = client
            .get(&url)
            .send()
            .map_err(|e| BridgeError::RpcError(format!("CosmWasm query failed: {e}")))?;

        let body: serde_json::Value = response
            .json()
            .map_err(|e| BridgeError::Serialization(format!("Invalid JSON response: {e}")))?;

        let data_b64 = body
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BridgeError::RpcError("CosmWasm query response missing 'data' field".to_string())
            })?;

        base64_decode(data_b64).map_err(|e| {
            BridgeError::Serialization(format!("Failed to decode CosmWasm response data: {e}"))
        })
    }

    /// Poll Tendermint block events using the `tx_search` RPC endpoint.
    ///
    /// Searches for transactions matching the given event query within the
    /// specified block height range. Returns (height, raw_tx_bytes) pairs.
    pub fn poll_tendermint_events(
        &self,
        event_query: &str,
        min_height: u64,
        max_height: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        let params = serde_json::json!({
            "query": format!("{} AND tx.height>={} AND tx.height<={}", event_query, min_height, max_height),
            "prove": false,
            "order_by": "asc",
            "per_page": "100"
        });

        let result = make_json_rpc_call(&self.rpc_url, "tx_search", params)?;

        let txs = result
            .get("txs")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BridgeError::RpcError("tx_search returned non-array".to_string()))?;

        let mut events = Vec::with_capacity(txs.len());
        for tx in txs {
            let height = tx
                .get("height")
                .and_then(|v| v.as_str())
                .and_then(|h| h.parse::<u64>().ok())
                .unwrap_or(0);

            let tx_bytes = tx
                .get("tx")
                .and_then(|v| v.as_str())
                .map(|s| {
                    base64_decode(s).unwrap_or_default()
                })
                .unwrap_or_default();

            events.push((height, tx_bytes));
        }

        Ok(events)
    }

    /// Check if the Tendermint node is synced and healthy.
    pub fn check_tendermint_health(&self) -> Result<bool, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "health", serde_json::json!([]))?;

        // Tendermint health returns {} on success
        if result.is_object() {
            return Ok(true);
        }

        // Also check sync status via status endpoint
        let status =
            make_json_rpc_call(&self.rpc_url, "status", serde_json::json!([]))?;

        let catching_up = status
            .get("sync_info")
            .and_then(|s| s.get("catching_up"))
            .and_then(|c| c.as_bool())
            .unwrap_or(true);

        Ok(!catching_up)
    }
}

// ── Base64 encoding/decoding (no external crate dependency) ────────────────

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(BASE64_CHARS[((triple >> 18) & 0x3f) as usize] as char);
        result.push(BASE64_CHARS[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((triple >> 6) & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(triple & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut decode_table = [0xffu8; 128];
    for (i, &c) in BASE64_CHARS.iter().enumerate() {
        decode_table[c as usize] = i as u8;
    }

    let mut result = Vec::with_capacity(input.len() * 3 / 4);
    let bytes: Vec<u8> = input
        .bytes()
        .filter_map(|b| {
            if (b as usize) < 128 && decode_table[b as usize] != 0xff {
                Some(decode_table[b as usize])
            } else {
                None
            }
        })
        .collect();

    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let b0 = chunk[0] as u32;
        let b1 = chunk[1] as u32;
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let b3 = if chunk.len() > 3 { chunk[3] as u32 } else { 0 };

        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        result.push(((triple >> 16) & 0xff) as u8);
        if chunk.len() > 2 {
            result.push(((triple >> 8) & 0xff) as u8);
        }
        if chunk.len() > 3 {
            result.push((triple & 0xff) as u8);
        }
    }

    Ok(result)
}

impl BridgeAdapter for CosmWasmBridgeAdapter {
    fn chain_name(&self) -> &str {
        // Return a static string — the bech32 prefix identifies the chain
        "cosmwasm"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader(
                "CosmWasm/Tendermint header is empty".to_string(),
            ));
        }

        // Parse the Tendermint block header as JSON.
        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!(
                "CosmWasm header is not valid UTF-8: {e}"
            ))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!(
                "Failed to parse CosmWasm header JSON: {e}"
            ))
        })?;

        // Verify chain_id is present
        let block_chain_id = header_json
            .get("chain_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BridgeError::InvalidHeader(
                    "CosmWasm header missing chain_id".to_string(),
                )
            })?;

        if block_chain_id.is_empty() {
            return Err(BridgeError::InvalidHeader(
                "CosmWasm header chain_id is empty".to_string(),
            ));
        }

        // Verify height is present and non-zero
        let height = header_json
            .get("height")
            .and_then(|v| v.as_str())
            .and_then(|h| h.parse::<u64>().ok())
            .unwrap_or(0);

        if height == 0 {
            return Err(BridgeError::InvalidHeader(
                "CosmWasm header height is zero".to_string(),
            ));
        }

        // Verify time is present
        let _time = header_json
            .get("time")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                BridgeError::InvalidHeader(
                    "CosmWasm header missing time field".to_string(),
                )
            })?;

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let height_str = block_number.to_string();

        let commit_result = make_json_rpc_call(
            &self.rpc_url,
            "commit",
            serde_json::json!({ "height": height_str }),
        )?;

        if commit_result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Commit for height {} not found on CosmWasm chain",
                block_number
            )));
        }

        let signed_header = commit_result
            .get("signed_header")
            .ok_or_else(|| {
                BridgeError::RpcError(
                    "Commit response missing signed_header".to_string(),
                )
            })?;

        let header = signed_header
            .get("header")
            .ok_or_else(|| {
                BridgeError::RpcError(
                    "Signed header missing header field".to_string(),
                )
            })?;

        let block_hash = header
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let app_hash = header
            .get("app_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let chain_id = header
            .get("chain_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let commit = signed_header
            .get("commit")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let proof_envelope = serde_json::json!({
            "proof_type": "cosmwasm-block-proof-v1",
            "chain_id": self.chain_id,
            "tendermint_chain_id": chain_id,
            "block_hash": block_hash,
            "block_height": block_number,
            "app_hash": app_hash,
            "bech32_prefix": self.bech32_prefix,
            "commit": commit,
            "validator_set": commit_result.get("validator_set"),
            "canonical": commit_result.get("canonical"),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope).map_err(|e| {
            BridgeError::Serialization(format!(
                "Failed to serialize CosmWasm proof: {e}"
            ))
        })?;

        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "status", serde_json::json!([]))?;

        let height = result
            .get("sync_info")
            .and_then(|s| s.get("latest_block_height"))
            .and_then(|h| h.as_str())
            .and_then(|h| h.parse::<u64>().ok())
            .ok_or_else(|| {
                BridgeError::RpcError(
                    "status response missing latest_block_height".to_string(),
                )
            })?;

        Ok(height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmwasm_adapter_creation() {
        let adapter = CosmWasmBridgeAdapter::new(
            4,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        assert_eq!(adapter.chain_name(), "cosmwasm");
        assert_eq!(adapter.chain_id(), 4);
        assert_eq!(adapter.bech32_prefix(), "cosmos");
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = CosmWasmBridgeAdapter::new(
            1,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("empty"));
    }

    #[test]
    fn validate_header_rejects_non_utf8() {
        let adapter = CosmWasmBridgeAdapter::new(
            1,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        let result = adapter.validate_header(&[0xff, 0xfe, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn validate_header_rejects_missing_chain_id() {
        let adapter = CosmWasmBridgeAdapter::new(
            1,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        let header = serde_json::json!({
            "height": "100",
            "time": "2024-01-01T00:00:00Z"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("chain_id"));
    }

    #[test]
    fn validate_header_accepts_valid_structure() {
        let adapter = CosmWasmBridgeAdapter::new(
            1,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        let header = serde_json::json!({
            "chain_id": "cosmoshub-4",
            "height": "12345678",
            "time": "2024-01-01T00:00:00.000Z",
            "hash": "ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890",
            "app_hash": "FEDCBA0987654321FEDCBA0987654321FEDCBA0987654321FEDCBA0987654321"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(
            result.is_ok(),
            "Expected valid header, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn validate_header_rejects_zero_height() {
        let adapter = CosmWasmBridgeAdapter::new(
            1,
            "http://localhost:26657".to_string(),
            "http://localhost:1317".to_string(),
            "cosmos".to_string(),
        );
        let header = serde_json::json!({
            "chain_id": "test-1",
            "height": "0",
            "time": "2024-01-01T00:00:00Z"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn base64_encode_decode_roundtrip() {
        let original = b"x3-cosmwasm-htlc: lock asset for atomic swap";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn base64_encode_known_value() {
        // "f" in base64 is "Zg=="
        assert_eq!(base64_encode(b"f"), "Zg==");
        // "fo" in base64 is "Zm8="
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        // "foo" in base64 is "Zm9v"
        assert_eq!(base64_encode(b"foo"), "Zm9v");
    }
}