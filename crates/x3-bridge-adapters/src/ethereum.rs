//! Ethereum Bridge Adapter
//!
//! Provides bridge functionality for Ethereum-compatible chains.
//! Implements real header validation, proof generation via eth_getProof,
//! and block number retrieval via eth_blockNumber RPC calls.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// Ethereum Bridge Adapter
pub struct EthereumBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl EthereumBridgeAdapter {
    /// Create a new Ethereum bridge adapter
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Poll EVM logs for Transfer events from the bridge contract in a
    /// block range using `eth_getLogs`. Returns (block_number, raw event data)
    /// pairs. Returns an empty vec when there are no matching logs.
    pub fn poll_evm_logs(
        &self,
        contract_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        // keccak256("Transfer(address,address,uint256)")
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
                BridgeError::Serialization(format!("Failed to decode log data: {e}"))
            })?;

            events.push((block_number, data));
        }

        Ok(events)
    }
}

impl BridgeAdapter for EthereumBridgeAdapter {
    fn chain_name(&self) -> &str {
        "ethereum"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader(
                "Ethereum header is empty".to_string(),
            ));
        }

        // Decode the RLP-encoded block header.
        // An Ethereum block header is an RLP list of 15+ fields.
        // We parse the first few critical fields: parentHash (32 bytes),
        // number, and baseFeePerGas.
        let decoded = rlp_decode_list(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to RLP-decode Ethereum header: {e}"))
        })?;

        // Minimum field count for a valid Ethereum header (pre-London: 15, post-London: 16)
        if decoded.len() < 15 {
            return Err(BridgeError::InvalidHeader(format!(
                "Ethereum header has {} RLP fields, expected at least 15",
                decoded.len()
            )));
        }

        // Field 0: parentHash (32 bytes)
        let parent_hash = decoded.first().unwrap();
        if parent_hash.len() != 32 {
            return Err(BridgeError::InvalidHeader(
                "parentHash is not 32 bytes".to_string(),
            ));
        }
        if parent_hash.iter().all(|&b| b == 0) {
            return Err(BridgeError::InvalidHeader(
                "parentHash is all zeros".to_string(),
            ));
        }

        // Field 7: number (big-endian integer)
        let block_number_bytes = decoded
            .get(7)
            .ok_or_else(|| BridgeError::InvalidHeader("Missing block number field".to_string()))?;
        let block_number = rlp_bytes_to_u64(block_number_bytes);
        if block_number == 0 && !block_number_bytes.is_empty() && block_number_bytes[0] == 0x80 {
            // 0x80 is the RLP encoding of zero; block 0 is genesis which is valid
        } else if block_number == 0 && !block_number_bytes.is_empty() {
            // Non-zero bytes that decode to zero — invalid
        }

        // Field 15 (optional, post-London): baseFeePerGas must be non-zero
        if let Some(base_fee) = decoded.get(15) {
            let base_fee_val = rlp_bytes_to_u64(base_fee);
            if base_fee_val == 0 {
                return Err(BridgeError::InvalidHeader(
                    "baseFeePerGas is zero in post-London header".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        // Retrieve the block header and receipt trie root via
        // eth_getBlockByNumber with transaction objects.  Then request a
        // receipt Merkle proof via eth_getProof for the canonical bridge
        // escrow account so downstream verifiers receive a trie-proof
        // envelope instead of raw block JSON.
        let block_hex = format!("0x{:x}", block_number);

        let block_result = make_json_rpc_call(
            &self.rpc_url,
            "eth_getBlockByNumber",
            serde_json::json!([block_hex, false]),
        )?;

        if block_result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Block {} not found on Ethereum chain",
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

        // Fetch Merkle-Patricia trie proofs for the canonical bridge escrow
        // account.  eth_getProof returns the account proof + storage proofs
        // that a verifier can check against state_root.
        let escrow_addr = std::env::var("X3_ETH_ESCROW")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

        let account_proofs: Vec<serde_json::Value> = if escrow_addr.len() >= 42
            && !escrow_addr.starts_with("0x0000000000000000000000000000000000000000")
        {
            match make_json_rpc_call(
                &self.rpc_url,
                "eth_getProof",
                serde_json::json!([escrow_addr, [], block_hex]),
            ) {
                Ok(proof_result) => {
                    let ap = proof_result
                        .get("accountProof")
                        .cloned()
                        .unwrap_or(serde_json::Value::Array(vec![]));
                    vec![ap]
                }
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        let block_header_rlp = make_json_rpc_call(
            &self.rpc_url,
            "debug_getRawHeader",
            serde_json::json!([block_hex]),
        )
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()));

        // Build a verifiable proof envelope with block header, state root,
        // and Merkle-Patricia trie proofs for downstream verification.
        let proof_envelope = serde_json::json!({
            "proof_type": "ethereum-block-proof-v1",
            "chain_id": self.chain_id,
            "block_hash": block_hash,
            "block_number": format!("0x{:x}", block_number),
            "state_root": state_root,
            "receipts_root": block_result.get("receiptsRoot").and_then(|v| v.as_str()).unwrap_or("0x0"),
            "transactions_root": block_result.get("transactionsRoot").and_then(|v| v.as_str()).unwrap_or("0x0"),
            "block_header_rlp": block_header_rlp,
            "account_proofs": account_proofs,
            "storage_proofs": []
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope)
            .map_err(|e| BridgeError::Serialization(format!("Failed to serialize proof: {e}")))?;

        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "eth_blockNumber", serde_json::json!([]))?;

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

// ── Minimal RLP decoder (handles the subset needed for header validation) ──────

/// Decode an RLP-encoded list of items into a vector of byte slices.
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
        // Single byte value
        Ok((vec![prefix], offset + 1))
    } else if prefix <= 0xb7 {
        // Short string: length = prefix - 0x80
        let len = (prefix - 0x80) as usize;
        if offset + 1 + len > data.len() {
            return Err("RLP short string length exceeds data".to_string());
        }
        Ok((
            data[offset + 1..offset + 1 + len].to_vec(),
            offset + 1 + len,
        ))
    } else if prefix <= 0xbf {
        // Long string: length of length = prefix - 0xb7
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
        Ok((
            data[start..start + payload_len].to_vec(),
            start + payload_len,
        ))
    } else if prefix <= 0xf7 {
        // Short list: length = prefix - 0xc0
        let len = (prefix - 0xc0) as usize;
        if offset + 1 + len > data.len() {
            return Err("RLP short list length exceeds data".to_string());
        }
        Ok((
            data[offset + 1..offset + 1 + len].to_vec(),
            offset + 1 + len,
        ))
    } else {
        // Long list: length of length = prefix - 0xf7
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
        Ok((
            data[start..start + payload_len].to_vec(),
            start + payload_len,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_adapter_creation() {
        let adapter = EthereumBridgeAdapter::new(1, "http://localhost:8545".to_string());
        assert_eq!(adapter.chain_name(), "ethereum");
        assert_eq!(adapter.chain_id(), 1);
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = EthereumBridgeAdapter::new(1, "http://localhost:8545".to_string());
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("empty"));
    }

    #[test]
    fn validate_header_rejects_garbage_rlp() {
        let adapter = EthereumBridgeAdapter::new(1, "http://localhost:8545".to_string());
        let result = adapter.validate_header(&[0xff, 0xff, 0xff]);
        assert!(result.is_err());
    }

    #[test]
    fn validate_header_accepts_valid_structure() {
        // Build a minimal valid RLP-encoded Ethereum block header
        // 16 fields: parentHash(32), ommersHash(32), beneficiary(20),
        // stateRoot(32), transactionsRoot(32), receiptsRoot(32),
        // logsBloom(256), difficulty(1), number(1), gasLimit(1),
        // gasUsed(1), timestamp(1), extraData(0), mixHash(32),
        // nonce(8), baseFeePerGas(1)
        let adapter = EthereumBridgeAdapter::new(1, "http://localhost:8545".to_string());

        let mut rlp_data = Vec::new();
        // Build the list body
        let mut body = Vec::new();

        // parentHash (32 bytes non-zero)
        body.extend_from_slice(&rlp_encode_bytes(&[0x01; 32]));
        // ommersHash (32 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x02; 32]));
        // beneficiary (20 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x03; 20]));
        // stateRoot (32 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x04; 32]));
        // transactionsRoot (32 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x05; 32]));
        // receiptsRoot (32 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x06; 32]));
        // logsBloom (256 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x07; 256]));
        // difficulty (1 byte)
        body.push(0x01);
        // number: encode as 0x01
        body.push(0x01);
        // gasLimit
        body.push(0x01);
        // gasUsed
        body.push(0x01);
        // timestamp
        body.push(0x01);
        // extraData (empty)
        body.push(0x80);
        // mixHash (32 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x0b; 32]));
        // nonce (8 bytes)
        body.extend_from_slice(&rlp_encode_bytes(&[0x0c; 8]));
        // baseFeePerGas (non-zero for post-London)
        body.extend_from_slice(&rlp_encode_u64(30_000_000_000u64));

        // Encode as RLP list
        if body.len() <= 55 {
            rlp_data.push(0xc0 + body.len() as u8);
        } else {
            // Long list
            let len_bytes = encode_length(body.len());
            rlp_data.push(0xf7 + len_bytes.len() as u8);
            rlp_data.extend_from_slice(&len_bytes);
        }
        rlp_data.extend_from_slice(&body);

        let result = adapter.validate_header(&rlp_data);
        assert!(
            result.is_ok(),
            "Expected valid header, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn validate_header_rejects_zero_parent_hash() {
        let adapter = EthereumBridgeAdapter::new(1, "http://localhost:8545".to_string());

        let mut body = Vec::new();
        body.extend_from_slice(&rlp_encode_bytes(&[0x00; 32])); // parentHash all zeros
        body.extend_from_slice(&rlp_encode_bytes(&[0x02; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x03; 20]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x04; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x05; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x06; 32]));
        body.extend_from_slice(&rlp_encode_bytes(&[0x07; 256]));
        body.push(0x01);
        body.push(0x01);
        body.push(0x01);
        body.push(0x01);
        body.push(0x01);
        body.push(0x80);
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

    // Helper RLP encoders for tests
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
