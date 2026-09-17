//! FuelVM Bridge Adapter
//!
//! Provides bridge functionality for Fuel — a UTXO-based blockchain using
//! the FuelVM with predicate/script-based smart contracts.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

pub struct FuelVmBridgeAdapter { chain_id: u64, rpc_url: String }

impl FuelVmBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self { Self { chain_id, rpc_url } }
    pub fn rpc_url(&self) -> &str { &self.rpc_url }
}

impl BridgeAdapter for FuelVmBridgeAdapter {
    fn chain_name(&self) -> &str { "fuel" }
    fn chain_id(&self) -> u64 { self.chain_id }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() { return Err(BridgeError::InvalidHeader("Fuel header is empty".to_string())); }
        let header_str = std::str::from_utf8(header).map_err(|e| BridgeError::InvalidHeader(format!("Fuel header is not valid UTF-8: {e}")))?;
        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| BridgeError::InvalidHeader(format!("Failed to parse Fuel header JSON: {e}")))?;
        let height = header_json.get("height").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::InvalidHeader("Fuel header missing height".to_string()))?;
        if height == 0 { return Err(BridgeError::InvalidHeader("Fuel height is zero".to_string())); }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "fuel_getBlock", serde_json::json!([block_number]))?;
        if result.is_null() { return Err(BridgeError::RpcError(format!("Block {} not found on Fuel", block_number))); }
        let proof_envelope = serde_json::json!({"proof_type":"fuel-block-proof-v1","chain_id":self.chain_id,"height":block_number,"hash":result.get("block_hash").and_then(|v| v.as_str())});
        serde_json::to_vec(&proof_envelope).map_err(|e| BridgeError::Serialization(format!("Failed to serialize Fuel proof: {e}")))
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "fuel_getLatestBlock", serde_json::json!([]))?;
        result.get("height").and_then(|v| v.as_u64()).ok_or_else(|| BridgeError::RpcError("fuel_getLatestBlock missing height".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_fuel_adapter_creation() { let a = FuelVmBridgeAdapter::new(1,"http://localhost:4000".into()); assert_eq!(a.chain_name(),"fuel"); }
    #[test] fn validate_header_rejects_empty() { assert!(FuelVmBridgeAdapter::new(1,"http://localhost:4000".into()).validate_header(&[]).is_err()); }
    #[test] fn validate_header_accepts_valid() { assert!(FuelVmBridgeAdapter::new(1,"http://localhost:4000".into()).validate_header(serde_json::json!({"height":123456,"hash":"0xabc..."}).to_string().as_bytes()).is_ok()); }
    #[test] fn validate_header_rejects_zero_height() { assert!(FuelVmBridgeAdapter::new(1,"http://localhost:4000".into()).validate_header(serde_json::json!({"height":0}).to_string().as_bytes()).is_err()); }
}