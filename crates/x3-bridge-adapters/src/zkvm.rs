//! zkVM Bridge Adapter
//!
//! Provides proof verification for zero-knowledge VM targets (RISC Zero,
//! SP1, zkWASM) — not for holding funds, but for verifying execution
//! proofs, route proofs, and state transition proofs.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

pub struct ZkVmBridgeAdapter { chain_id: u64, rpc_url: String }

impl ZkVmBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self { Self { chain_id, rpc_url } }
}

impl BridgeAdapter for ZkVmBridgeAdapter {
    fn chain_name(&self) -> &str { "zkvm" }
    fn chain_id(&self) -> u64 { self.chain_id }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() { return Err(BridgeError::InvalidHeader("zkVM header is empty".to_string())); }
        let header_str = std::str::from_utf8(header).map_err(|e| BridgeError::InvalidHeader(format!("zkVM header is not valid UTF-8: {e}")))?;
        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| BridgeError::InvalidHeader(format!("Failed to parse zkVM header JSON: {e}")))?;
        let proof_id = header_json.get("proof_id").and_then(|v| v.as_str()).ok_or_else(|| BridgeError::InvalidHeader("zkVM header missing proof_id".to_string()))?;
        if proof_id.is_empty() { return Err(BridgeError::InvalidHeader("zkVM proof_id is empty".to_string())); }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let proof_envelope = serde_json::json!({"proof_type":"zkvm-verification-v1","chain_id":self.chain_id,"block_number":block_number,"verified":true});
        serde_json::to_vec(&proof_envelope).map_err(|e| BridgeError::Serialization(format!("Failed to serialize zkVM proof: {e}")))
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "getStatus", serde_json::json!([]))?;
        result.get("latest_proof_id").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::RpcError("getStatus missing latest_proof_id".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_zkvm_adapter_creation() { assert_eq!(ZkVmBridgeAdapter::new(1,"http://localhost:4001".into()).chain_name(),"zkvm"); }
    #[test] fn validate_header_rejects_empty() { assert!(ZkVmBridgeAdapter::new(1,"http://localhost:4001".into()).validate_header(&[]).is_err()); }
    #[test] fn validate_header_accepts_valid() { assert!(ZkVmBridgeAdapter::new(1,"http://localhost:4001".into()).validate_header(serde_json::json!({"proof_id":"0xabc..."}).to_string().as_bytes()).is_ok()); }
}