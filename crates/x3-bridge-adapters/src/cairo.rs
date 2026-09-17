//! CairoVM Bridge Adapter (Starknet)
//!
//! Provides bridge functionality for Starknet — a ZK-rollup on Ethereum using
//! the CairoVM. Cairo uses a felt-based arithmetic model with native account
//! abstraction, fundamentally different from EVM.
//!
//! Supported chains: Starknet mainnet (chain_id 23448594291968334),
//! Starknet Sepolia testnet.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// CairoVM/Starknet Bridge Adapter
pub struct CairoBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl CairoBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Poll Starknet events using `starknet_getEvents`.
    pub fn poll_starknet_events(
        &self,
        contract_address: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        let params = serde_json::json!([{
            "from_block": {"block_number": from_block},
            "to_block": {"block_number": to_block},
            "address": contract_address,
            "keys": [],
            "chunk_size": 100
        }]);

        let result = make_json_rpc_call(&self.rpc_url, "starknet_getEvents", params)?;

        let events = result
            .get("events")
            .and_then(|v| v.as_array())
            .ok_or_else(|| BridgeError::RpcError("starknet_getEvents returned non-array".to_string()))?;

        let mut results = Vec::with_capacity(events.len());
        for event in events {
            let block = event
                .get("block_number")
                .and_then(|b| b.as_u64())
                .unwrap_or(0);
            let event_bytes = serde_json::to_vec(event).unwrap_or_default();
            results.push((block, event_bytes));
        }
        Ok(results)
    }

    /// Check Starknet node sync status.
    pub fn check_sync_status(&self) -> Result<bool, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "starknet_syncing", serde_json::json!([]))?;
        Ok(result.as_bool().map(|b| !b).unwrap_or(true))
    }
}

impl BridgeAdapter for CairoBridgeAdapter {
    fn chain_name(&self) -> &str {
        "starknet"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader("Starknet header is empty".to_string()));
        }

        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Starknet header is not valid UTF-8: {e}"))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to parse Starknet header JSON: {e}"))
        })?;

        let block_hash = header_json
            .get("block_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BridgeError::InvalidHeader("Starknet header missing block_hash".to_string()))?;

        if block_hash.is_empty() || block_hash.len() < 4 {
            return Err(BridgeError::InvalidHeader("Starknet block_hash is empty".to_string()));
        }

        let _block_number = header_json
            .get("block_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BridgeError::InvalidHeader("Starknet header missing block_number".to_string()))?;

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "starknet_getBlockWithTxs",
            serde_json::json!([{"block_number": block_number}]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!("Block {} not found on Starknet", block_number)));
        }

        let block_hash = result.get("block_hash").and_then(|v| v.as_str()).unwrap_or("");
        let parent_hash = result.get("parent_hash").and_then(|v| v.as_str()).unwrap_or("");
        let new_root = result.get("new_root").and_then(|v| v.as_str()).unwrap_or("");
        let l1_gas_price = result.get("l1_gas_price").and_then(|v| v.as_str()).unwrap_or("0");

        let proof_envelope = serde_json::json!({
            "proof_type": "starknet-block-proof-v1",
            "chain_id": self.chain_id,
            "block_hash": block_hash,
            "block_number": block_number,
            "parent_hash": parent_hash,
            "new_root": new_root,
            "l1_gas_price": l1_gas_price,
            "timestamp": result.get("timestamp").and_then(|v| v.as_u64()),
            "sequencer_address": result.get("sequencer_address").and_then(|v| v.as_str()),
            "starknet_version": result.get("starknet_version").and_then(|v| v.as_str()),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope)
            .map_err(|e| BridgeError::Serialization(format!("Failed to serialize Starknet proof: {e}")))?;
        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "starknet_blockNumber", serde_json::json!([]))?;
        result
            .as_u64()
            .ok_or_else(|| BridgeError::RpcError("starknet_blockNumber returned non-integer".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starknet_adapter_creation() {
        let adapter = CairoBridgeAdapter::new(23448594291968334, "http://localhost:9545".to_string());
        assert_eq!(adapter.chain_name(), "starknet");
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = CairoBridgeAdapter::new(1, "http://localhost:9545".to_string());
        assert!(adapter.validate_header(&[]).is_err());
    }

    #[test]
    fn validate_header_rejects_non_utf8() {
        let adapter = CairoBridgeAdapter::new(1, "http://localhost:9545".to_string());
        assert!(adapter.validate_header(&[0xff, 0xfe, 0x00]).is_err());
    }

    #[test]
    fn validate_header_accepts_valid_structure() {
        let adapter = CairoBridgeAdapter::new(1, "http://localhost:9545".to_string());
        let header = serde_json::json!({
            "block_hash": "0x05a4fe...",
            "block_number": 123456,
            "parent_hash": "0xabc...",
            "new_root": "0xdef...",
            "timestamp": 1700000000,
            "starknet_version": "0.13.1"
        });
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn validate_header_rejects_missing_block_hash() {
        let adapter = CairoBridgeAdapter::new(1, "http://localhost:9545".to_string());
        let header = serde_json::json!({"block_number": 123});
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_err());
    }
}