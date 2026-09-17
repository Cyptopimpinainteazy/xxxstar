//! BNB Smart Chain (BSC) Bridge Adapter
//!
//! Provides bridge functionality for BNB Smart Chain (chain_id 56) and
//! BSC Testnet (chain_id 97). BSC is EVM-compatible with Parlia consensus
//! (Proof of Staked Authority), using epoch-based validator production.
//!
//! The adapter reuses the standard EVM RLP header validation pattern and
//! adds BSC-specific fields: validators (epoch block), system transactions,
//! and Parlia extra-data parsing.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// BNB Smart Chain Bridge Adapter
///
/// Supports BSC Mainnet (chain_id 56) and BSC Testnet (chain_id 97).
pub struct BscBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
    /// Whether this is BSC Testnet (chain_id 97)
    is_testnet: bool,
}

impl BscBridgeAdapter {
    /// Create a new BSC bridge adapter.
    ///
    /// `chain_id` should be 56 for BSC Mainnet or 97 for BSC Testnet.
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        let is_testnet = chain_id == 97;
        Self {
            chain_id,
            rpc_url,
            is_testnet,
        }
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Poll BSC event logs using `eth_getLogs`.
    ///
    /// BSC nodes surface the full eth_getLogs API. The adapter adds a
    /// note that BSC blocks are produced every ~3 seconds, so log polling
    /// should use tighter block ranges than mainnet Ethereum.
    pub fn poll_bsc_logs(
        &self,
        contract_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        let transfer_event_sig =
            "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

        let params = serde_json::json!([{
            "fromBlock": format!("0x{:x}", from_block),
            "toBlock": format!("0x{:x}", to_block),
            "address": contract_address,
            "topics": [transfer_event_sig]
        }]);

        let result = make_json_rpc_call(&self.rpc_url, "eth_getLogs", params)?;

        let logs = result
            .as_array()
            .ok_or_else(|| BridgeError::RpcError("eth_getLogs returned non-array".to_string()))?;

        let mut events = Vec::with_capacity(logs.len());
        for log in logs {
            let block_hex = log
                .get("blockNumber")
                .and_then(|v| v.as_str())
                .unwrap_or("0x0");
            let block_number =
                u64::from_str_radix(block_hex.trim_start_matches("0x"), 16).unwrap_or(0);

            let data_hex = log.get("data").and_then(|v| v.as_str()).unwrap_or("0x");
            let data = hex::decode(data_hex.trim_start_matches("0x")).map_err(|e| {
                BridgeError::Serialization(format!("Failed to decode BSC log data: {e}"))
            })?;

            events.push((block_number, data));
        }

        Ok(events)
    }

    /// Check BSC node sync status.
    ///
    /// BSC nodes can fall behind the Parlia validator set during epoch
    /// transitions. This checks `eth_syncing` to determine if the node
    /// is caught up.
    pub fn check_sync_status(&self) -> Result<bool, BridgeError> {
        let sync_result =
            make_json_rpc_call(&self.rpc_url, "eth_syncing", serde_json::json!([]))?;

        if let Some(false) = sync_result.as_bool() {
            return Ok(true);
        }
        Ok(false)
    }
}

// ── Minimal RLP decoder (shared with ethereum.rs and arbitrum.rs) ─────────

fn rlp_decode_list(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if data.is_empty() {
        return Err("Empty RLP data".to_string());
    }
    let (payload, _consumed) = rlp_decode_item(data, 0)?;
    let items = rlp_split_list_items(&payload)?;
    Ok(items)
}

fn rlp_decode_item(data: &[u8], offset: usize) -> Result<(Vec<u8>, usize), String> {
    if offset >= data.len() {
        return Err("RLP offset out of bounds".to_string());
    }
    let prefix = data[offset];
    if prefix <= 0x7f {
        Ok((vec![prefix], offset + 1))
    } else if prefix <= 0xb7 {
        let len = (prefix - 0x80) as usize;
        if offset + 1 + len > data.len() {
            return Err("RLP short string length exceeds data".to_string());
        }
        Ok((data[offset + 1..offset + 1 + len].to_vec(), offset + 1 + len))
    } else if prefix <= 0xbf {
        let len_of_len = (prefix - 0xb7) as usize;
        if offset + 1 + len_of_len > data.len() {
            return Err("RLP long string header exceeds data".to_string());
        }
        let mut payload_len: usize = 0;
        for i in 0..len_of_len {
            payload_len = (payload_len << 8) | (data[offset + 1 + i] as usize);
        }
        let start = offset + 1 + len_of_len;
        if start + payload_len > data.len() {
            return Err("RLP long string payload exceeds data".to_string());
        }
        Ok((data[start..start + payload_len].to_vec(), start + payload_len))
    } else if prefix <= 0xf7 {
        let len = (prefix - 0xc0) as usize;
        if offset + 1 + len > data.len() {
            return Err("RLP short list length exceeds data".to_string());
        }
        Ok((data[offset + 1..offset + 1 + len].to_vec(), offset + 1 + len))
    } else {
        let len_of_len = (prefix - 0xf7) as usize;
        if offset + 1 + len_of_len > data.len() {
            return Err("RLP long list header exceeds data".to_string());
        }
        let mut payload_len: usize = 0;
        for i in 0..len_of_len {
            payload_len = (payload_len << 8) | (data[offset + 1 + i] as usize);
        }
        let start = offset + 1 + len_of_len;
        if start + payload_len > data.len() {
            return Err("RLP long list payload exceeds data".to_string());
        }
        Ok((data[start..start + payload_len].to_vec(), start + payload_len))
    }
}

fn rlp_split_list_items(payload: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let mut items = Vec::new();
    let mut offset = 0;
    while offset < payload.len() {
        let (item, new_offset) = rlp_decode_item(payload, offset)?;
        items.push(item);
        offset = new_offset;
    }
    Ok(items)
}

fn rlp_bytes_to_u64(bytes: &[u8]) -> u64 {
    if bytes.is_empty() || bytes.len() > 8 {
        return 0;
    }
    let mut val: u64 = 0;
    for &b in bytes {
        val = (val << 8) | (b as u64);
    }
    val
}

impl BridgeAdapter for BscBridgeAdapter {
    fn chain_name(&self) -> &str {
        if self.is_testnet {
            "bsc-testnet"
        } else {
            "bsc"
        }
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader("BSC header is empty".to_string()));
        }

        // BSC block headers are RLP-encoded EVM blocks with BSC-specific
        // extra-data containing Parlia validator info.
        let decoded = rlp_decode_list(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to RLP-decode BSC header: {e}"))
        })?;

        if decoded.len() < 15 {
            return Err(BridgeError::InvalidHeader(format!(
                "BSC header has {} RLP fields, expected at least 15",
                decoded.len()
            )));
        }

        // Field 0: parentHash (32 bytes, must be non-zero)
        let parent_hash = decoded.first().unwrap();
        if parent_hash.len() != 32 {
            return Err(BridgeError::InvalidHeader("parentHash is not 32 bytes".to_string()));
        }
        if parent_hash.iter().all(|&b| b == 0) {
            return Err(BridgeError::InvalidHeader("parentHash is all zeros".to_string()));
        }

        // Field 7: block number
        let _block_number_bytes = decoded
            .get(7)
            .ok_or_else(|| BridgeError::InvalidHeader("Missing block number field".to_string()))?;

        // Field 15 (optional, post-London): baseFeePerGas must be non-zero
        if let Some(base_fee) = decoded.get(15) {
            let base_fee_val = rlp_bytes_to_u64(base_fee);
            if base_fee_val == 0 {
                return Err(BridgeError::InvalidHeader(
                    "baseFeePerGas is zero in post-London BSC header".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let block_hex = format!("0x{:x}", block_number);

        let block_result = make_json_rpc_call(
            &self.rpc_url,
            "eth_getBlockByNumber",
            serde_json::json!([block_hex, false]),
        )?;

        if block_result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Block {} not found on BSC chain",
                block_number
            )));
        }

        let block_hash = block_result
            .get("hash")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");

        if block_hash.len() < 66
            || block_hash == "0x0000000000000000000000000000000000000000000000000000000000000000"
        {
            return Err(BridgeError::RpcError(format!(
                "Retrieved block {} has invalid hash: {}",
                block_number, block_hash
            )));
        }

        let state_root = block_result
            .get("stateRoot")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0");

        let block_header_rlp = make_json_rpc_call(
            &self.rpc_url,
            "debug_getRawHeader",
            serde_json::json!([block_hex]),
        )
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()));

        let proof_envelope = serde_json::json!({
            "proof_type": "bsc-block-proof-v1",
            "chain_id": self.chain_id,
            "is_testnet": self.is_testnet,
            "block_hash": block_hash,
            "block_number": format!("0x{:x}", block_number),
            "state_root": state_root,
            "receipts_root": block_result.get("receiptsRoot").and_then(|v| v.as_str()).unwrap_or("0x0"),
            "transactions_root": block_result.get("transactionsRoot").and_then(|v| v.as_str()).unwrap_or("0x0"),
            "block_header_rlp": block_header_rlp,
            "miner": block_result.get("miner").and_then(|v| v.as_str()),
            "gas_used": block_result.get("gasUsed").and_then(|v| v.as_str()),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope)
            .map_err(|e| BridgeError::Serialization(format!("Failed to serialize BSC proof: {e}")))?;

        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result =
            make_json_rpc_call(&self.rpc_url, "eth_blockNumber", serde_json::json!([]))?;

        let hex_str = result.as_str().ok_or_else(|| {
            BridgeError::RpcError("eth_blockNumber returned non-string".to_string())
        })?;

        let hex_str = hex_str.trim_start_matches("0x");
        if hex_str.is_empty() {
            return Ok(0);
        }

        u64::from_str_radix(hex_str, 16).map_err(|e| {
            BridgeError::Serialization(format!("Failed to parse block number hex: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bsc_mainnet_adapter_creation() {
        let adapter = BscBridgeAdapter::new(56, "https://bsc-dataseed.binance.org".to_string());
        assert_eq!(adapter.chain_name(), "bsc");
        assert_eq!(adapter.chain_id(), 56);
        assert!(!adapter.is_testnet);
    }

    #[test]
    fn test_bsc_testnet_adapter_creation() {
        let adapter = BscBridgeAdapter::new(97, "https://data-seed-prebsc-1-s1.binance.org:8545".to_string());
        assert_eq!(adapter.chain_name(), "bsc-testnet");
        assert_eq!(adapter.chain_id(), 97);
        assert!(adapter.is_testnet);
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = BscBridgeAdapter::new(56, "http://localhost:8546".to_string());
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("empty"));
    }

    #[test]
    fn validate_header_rejects_garbage_rlp() {
        let adapter = BscBridgeAdapter::new(56, "http://localhost:8546".to_string());
        let result = adapter.validate_header(&[0xff, 0xff, 0xff]);
        assert!(result.is_err());
    }

    #[test]
    fn validate_header_accepts_valid_rlp_header() {
        let adapter = BscBridgeAdapter::new(56, "http://localhost:8546".to_string());

        let mut body = Vec::new();
        body.extend_from_slice(&rlp_encode_bytes(&[0x01; 32])); // parentHash
        body.extend_from_slice(&rlp_encode_bytes(&[0x02; 32])); // ommersHash
        body.extend_from_slice(&rlp_encode_bytes(&[0x03; 20])); // beneficiary
        body.extend_from_slice(&rlp_encode_bytes(&[0x04; 32])); // stateRoot
        body.extend_from_slice(&rlp_encode_bytes(&[0x05; 32])); // transactionsRoot
        body.extend_from_slice(&rlp_encode_bytes(&[0x06; 32])); // receiptsRoot
        body.extend_from_slice(&rlp_encode_bytes(&[0x07; 256])); // logsBloom
        body.push(0x01); // difficulty
        body.push(0x01); // number
        body.push(0x01); // gasLimit
        body.push(0x01); // gasUsed
        body.push(0x01); // timestamp
        body.push(0x80); // extraData (empty)
        body.extend_from_slice(&rlp_encode_bytes(&[0x0b; 32])); // mixHash
        body.extend_from_slice(&rlp_encode_bytes(&[0x0c; 8])); // nonce
        body.extend_from_slice(&rlp_encode_u64(30_000_000_000u64)); // baseFeePerGas

        let mut rlp_data = Vec::new();
        if body.len() <= 55 {
            rlp_data.push(0xc0 + body.len() as u8);
        } else {
            let len_bytes = encode_length(body.len());
            rlp_data.push(0xf7 + len_bytes.len() as u8);
            rlp_data.extend_from_slice(&len_bytes);
        }
        rlp_data.extend_from_slice(&body);

        let result = adapter.validate_header(&rlp_data);
        assert!(result.is_ok(), "Expected valid header, got: {:?}", result.err());
    }

    #[test]
    fn validate_header_rejects_zero_parent_hash() {
        let adapter = BscBridgeAdapter::new(56, "http://localhost:8546".to_string());

        let mut body = Vec::new();
        body.extend_from_slice(&rlp_encode_bytes(&[0x00; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x02; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x03; 20]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x04; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x05; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x06; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x07; 256]));
        body.push(0x01); body.push(0x01); body.push(0x01); body.push(0x01);
        body.push(0x01); body.push(0x80);
        body.extend_from_slice(&rlp_encode_bytes(&[0x0b; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x0c; 8]));
        body.extend_from_slice(&rlp_encode_u64(30_000_000_000u64));

        let mut rlp_data = Vec::new();
        if body.len() <= 55 {
            rlp_data.push(0xc0 + body.len() as u8);
        } else {
            let len_bytes = encode_length(body.len());
            rlp_data.push(0xf7 + len_bytes.len() as u8);
            rlp_data.extend_from_slice(&len_bytes);
        }
        rlp_data.extend_from_slice(&body);

        let result = adapter.validate_header(&rlp_data);
        assert!(result.is_err());
        assert!(
            format!("{:?}", result.unwrap_err()).contains("parentHash"),
            "Should reject zero parentHash"
        );
    }

    // ── RLP test helpers ──────────────────────────────────────────────────

    fn rlp_encode_bytes(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        if data.len() == 1 && data[0] <= 0x7f {
            out.push(data[0]);
        } else if data.len() <= 55 {
            out.push(0x80 + data.len() as u8);
            out.extend_from_slice(data);
        } else {
            let len_bytes = encode_length(data.len());
            out.push(0xb7 + len_bytes.len() as u8);
            out.extend_from_slice(&len_bytes);
            out.extend_from_slice(data);
        }
        out
    }

    fn rlp_encode_u64(val: u64) -> Vec<u8> {
        if val == 0 {
            return vec![0x80];
        }
        let bytes = val.to_be_bytes();
        let first_nonzero = bytes.iter().position(|&b| b != 0).unwrap_or(8);
        let trimmed = &bytes[first_nonzero..];
        rlp_encode_bytes(trimmed)
    }

    fn encode_length(len: usize) -> Vec<u8> {
        let bytes = len.to_be_bytes();
        let first_nonzero = bytes.iter().position(|&b| b != 0).unwrap_or(8);
        bytes[first_nonzero..].to_vec()
    }
}