//! TON TVM Bridge Adapter
//!
//! Provides bridge functionality for The Open Network (TON) — a unique
//! blockchain using the Threaded Virtual Machine (TVM), a stack-based VM
//! with message-driven contract execution. TON uses shardchains and
//! a masterchain for consensus, fundamentally different from EVM.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// TON TVM Bridge Adapter
pub struct TonBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl TonBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Check TON node sync status.
    pub fn check_sync_status(&self) -> Result<bool, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "sync", serde_json::json!({}))?;
        Ok(result
            .get("sync_info")
            .and_then(|s| s.get("in_progress"))
            .and_then(|v| v.as_bool())
            .map(|b| !b)
            .unwrap_or(true))
    }
}

impl BridgeAdapter for TonBridgeAdapter {
    fn chain_name(&self) -> &str {
        "ton"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader("TON header is empty".to_string()));
        }

        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("TON header is not valid UTF-8: {e}"))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to parse TON header JSON: {e}"))
        })?;

        let seqno = header_json
            .get("seqno")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BridgeError::InvalidHeader("TON header missing seqno".to_string()))?;

        if seqno == 0 {
            return Err(BridgeError::InvalidHeader("TON seqno is zero".to_string()));
        }

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "getBlockHeader",
            serde_json::json!([{"workchain": -1, "shard": "-9223372036854775808", "seqno": block_number}]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!("Block {} not found on TON", block_number)));
        }

        let root_hash = result.get("root_hash").and_then(|v| v.as_str()).unwrap_or("");
        let prev_hash = result.get("prev_blocks").and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|b| b.get("root_hash"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let proof_envelope = serde_json::json!({
            "proof_type": "ton-block-proof-v1",
            "chain_id": self.chain_id,
            "seqno": block_number,
            "root_hash": root_hash,
            "prev_root_hash": prev_hash,
            "file_hash": result.get("file_hash").and_then(|v| v.as_str()),
            "min_ref_mc_seqno": result.get("min_ref_mc_seqno").and_then(|v| v.as_u64()),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope)
            .map_err(|e| BridgeError::Serialization(format!("Failed to serialize TON proof: {e}")))?;
        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "getMasterchainInfo", serde_json::json!({}))?;
        result
            .get("last")
            .and_then(|l| l.get("seqno"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BridgeError::RpcError("getMasterchainInfo missing seqno".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ton_adapter_creation() {
        let adapter = TonBridgeAdapter::new(1, "http://localhost:8081".to_string());
        assert_eq!(adapter.chain_name(), "ton");
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = TonBridgeAdapter::new(1, "http://localhost:8081".to_string());
        assert!(adapter.validate_header(&[]).is_err());
    }

    #[test]
    fn validate_header_accepts_valid() {
        let adapter = TonBridgeAdapter::new(1, "http://localhost:8081".to_string());
        let header = serde_json::json!({"seqno": 123456, "root_hash": "abc..."});
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn validate_header_rejects_zero_seqno() {
        let adapter = TonBridgeAdapter::new(1, "http://localhost:8081".to_string());
        let header = serde_json::json!({"seqno": 0});
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_err());
    }
}