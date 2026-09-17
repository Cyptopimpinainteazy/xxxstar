//! Soroban WASM Bridge Adapter (Stellar)
//!
//! Provides bridge functionality for Stellar Soroban — Stellar's smart
//! contract platform using Rust-compiled WASM contracts with the Stellar
//! Consensus Protocol (SCP) for finality.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

pub struct SorobanBridgeAdapter { chain_id: u64, rpc_url: String }

impl SorobanBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self { Self { chain_id, rpc_url } }
    pub fn rpc_url(&self) -> &str { &self.rpc_url }
}

impl BridgeAdapter for SorobanBridgeAdapter {
    fn chain_name(&self) -> &str { "stellar" }
    fn chain_id(&self) -> u64 { self.chain_id }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() { return Err(BridgeError::InvalidHeader("Stellar header is empty".to_string())); }
        let header_str = std::str::from_utf8(header).map_err(|e| BridgeError::InvalidHeader(format!("Stellar header is not valid UTF-8: {e}")))?;
        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| BridgeError::InvalidHeader(format!("Failed to parse Stellar header JSON: {e}")))?;
        let ledger = header_json.get("sequence").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::InvalidHeader("Stellar header missing sequence".to_string()))?;
        if ledger == 0 { return Err(BridgeError::InvalidHeader("Stellar sequence is zero".to_string())); }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let params = serde_json::json!([{"startLedger": block_number}]);
        let result = make_json_rpc_call(&self.rpc_url, "getLedgers", params)?;
        let ledgers = result.get("ledgers").and_then(|v| v.as_array()).ok_or_else(|| BridgeError::RpcError("getLedgers returned non-array".to_string()))?;
        let ledger = ledgers.first().ok_or_else(|| BridgeError::RpcError(format!("Ledger {} not found on Stellar", block_number)))?;
        let proof_envelope = serde_json::json!({"proof_type":"stellar-ledger-proof-v1","chain_id":self.chain_id,"sequence":block_number,"hash":ledger.get("hash").and_then(|v| v.as_str())});
        serde_json::to_vec(&proof_envelope).map_err(|e| BridgeError::Serialization(format!("Failed to serialize Stellar proof: {e}")))
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "getLatestLedger", serde_json::json!([]))?;
        result.get("sequence").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::RpcError("getLatestLedger missing sequence".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_soroban_adapter_creation() { let a = SorobanBridgeAdapter::new(1,"http://localhost:8000".into()); assert_eq!(a.chain_name(),"stellar"); }
    #[test] fn validate_header_rejects_empty() { assert!(SorobanBridgeAdapter::new(1,"http://localhost:8000".into()).validate_header(&[]).is_err()); }
    #[test] fn validate_header_accepts_valid() { assert!(SorobanBridgeAdapter::new(1,"http://localhost:8000".into()).validate_header(serde_json::json!({"sequence":123456,"hash":"0xabc..."}).to_string().as_bytes()).is_ok()); }
    #[test] fn validate_header_rejects_zero_sequence() { assert!(SorobanBridgeAdapter::new(1,"http://localhost:8000".into()).validate_header(serde_json::json!({"sequence":0}).to_string().as_bytes()).is_err()); }
}