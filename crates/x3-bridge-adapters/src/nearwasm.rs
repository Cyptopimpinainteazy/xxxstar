//! NEAR WASM Bridge Adapter
//!
//! Provides bridge functionality for NEAR Protocol — an account-based
//! blockchain using WASM smart contracts with sharded proof-of-stake consensus.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

pub struct NearWasmBridgeAdapter { chain_id: u64, rpc_url: String }

impl NearWasmBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self { Self { chain_id, rpc_url } }
    pub fn rpc_url(&self) -> &str { &self.rpc_url }
}

impl BridgeAdapter for NearWasmBridgeAdapter {
    fn chain_name(&self) -> &str { "near" }
    fn chain_id(&self) -> u64 { self.chain_id }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() { return Err(BridgeError::InvalidHeader("NEAR header is empty".to_string())); }
        let header_str = std::str::from_utf8(header).map_err(|e| BridgeError::InvalidHeader(format!("NEAR header is not valid UTF-8: {e}")))?;
        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| BridgeError::InvalidHeader(format!("Failed to parse NEAR header JSON: {e}")))?;
        let hash = header_json.get("hash").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::InvalidHeader("NEAR header missing hash".to_string()))?;
        if hash.is_empty() { return Err(BridgeError::InvalidHeader("NEAR block hash is empty".to_string())); }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "block", serde_json::json!([{"block_id": block_number}]))?;
        if result.is_null() { return Err(BridgeError::RpcError(format!("Block {} not found on NEAR", block_number))); }
        let proof_envelope = serde_json::json!({"proof_type":"near-block-proof-v1","chain_id":self.chain_id,"height":block_number,"hash":result.get("header").and_then(|h| h.get("hash")).and_then(|v| v.as_str())});
        serde_json::to_vec(&proof_envelope).map_err(|e| BridgeError::Serialization(format!("Failed to serialize NEAR proof: {e}")))
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "status", serde_json::json!([]))?;
        result.get("sync_info").and_then(|s| s.get("latest_block_height")).and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::RpcError("status missing latest_block_height".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_near_adapter_creation() { let a = NearWasmBridgeAdapter::new(1,"http://localhost:3030".into()); assert_eq!(a.chain_name(),"near"); }
    #[test] fn validate_header_rejects_empty() { assert!(NearWasmBridgeAdapter::new(1,"http://localhost:3030".into()).validate_header(&[]).is_err()); }
    #[test] fn validate_header_accepts_valid() { assert!(NearWasmBridgeAdapter::new(1,"http://localhost:3030".into()).validate_header(serde_json::json!({"hash":"0xabc...","height":123456}).to_string().as_bytes()).is_ok()); }
    #[test] fn validate_header_rejects_missing_hash() { assert!(NearWasmBridgeAdapter::new(1,"http://localhost:3030".into()).validate_header(serde_json::json!({"height":123}).to_string().as_bytes()).is_err()); }
}