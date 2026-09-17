//! Polkadot PVM / ink! Bridge Adapter
//!
//! Provides bridge functionality for Polkadot Hub smart contracts — covering
//! both the native Polkadot Virtual Machine (PVM) and ink! WASM contracts
//! deployed on Substrate-based chains.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

pub struct PolkadotPvmBridgeAdapter { chain_id: u64, rpc_url: String }

impl PolkadotPvmBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self { Self { chain_id, rpc_url } }
}

impl BridgeAdapter for PolkadotPvmBridgeAdapter {
    fn chain_name(&self) -> &str { "polkadot-pvm" }
    fn chain_id(&self) -> u64 { self.chain_id }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() { return Err(BridgeError::InvalidHeader("PVM header is empty".to_string())); }
        let header_str = std::str::from_utf8(header).map_err(|e| BridgeError::InvalidHeader(format!("PVM header is not valid UTF-8: {e}")))?;
        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| BridgeError::InvalidHeader(format!("Failed to parse PVM header JSON: {e}")))?;
        let hash = header_json.get("parentHash").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::InvalidHeader("PVM header missing parentHash".to_string()))?;
        if hash.is_empty() { return Err(BridgeError::InvalidHeader("PVM parentHash is empty".to_string())); }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "chain_getBlockHash", serde_json::json!([block_number]))?;
        if result.is_null() { return Err(BridgeError::RpcError(format!("Block {} not found on PVM", block_number))); }
        let proof_envelope = serde_json::json!({"proof_type":"polkadot-pvm-proof-v1","chain_id":self.chain_id,"block_number":block_number,"hash":result.as_str()});
        serde_json::to_vec(&proof_envelope).map_err(|e| BridgeError::Serialization(format!("Failed to serialize PVM proof: {e}")))
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "chain_getHeader", serde_json::json!([]))?;
        result.get("number").and_then(|v| v.as_str()).and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"),16).ok()).ok_or_else(|| BridgeError::RpcError("chain_getHeader missing number".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_pvm_adapter_creation() { assert_eq!(PolkadotPvmBridgeAdapter::new(1,"http://localhost:9933".into()).chain_name(),"polkadot-pvm"); }
    #[test] fn validate_header_rejects_empty() { assert!(PolkadotPvmBridgeAdapter::new(1,"http://localhost:9933".into()).validate_header(&[]).is_err()); }
    #[test] fn validate_header_accepts_valid() { assert!(PolkadotPvmBridgeAdapter::new(1,"http://localhost:9933".into()).validate_header(serde_json::json!({"parentHash":"0xabc...","number":"0x100"}).to_string().as_bytes()).is_ok()); }
}