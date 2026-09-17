//! Plutus / eUTXO Bridge Adapter (Cardano)
//!
//! Provides bridge functionality for Cardano — a UTXO-based blockchain using
//! the Plutus smart contract platform. Unlike EVM accounts, Cardano uses an
//! extended UTXO (eUTXO) model where scripts validate transactions via
//! datum/redeemer patterns.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// Plutus/eUTXO Bridge Adapter (Cardano)
pub struct PlutusBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl PlutusBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Check Cardano node sync status via Ogmios or Blockfrost-style endpoint.
    pub fn check_sync_status(&self) -> Result<bool, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "queryLedgerState", serde_json::json!({}))?;
        let sync_progress = result
            .get("syncProgress")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0");
        let progress: f64 = sync_progress.parse().unwrap_or(0.0);
        Ok(progress >= 100.0)
    }
}

impl BridgeAdapter for PlutusBridgeAdapter {
    fn chain_name(&self) -> &str {
        "cardano"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader("Cardano header is empty".to_string()));
        }

        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Cardano header is not valid UTF-8: {e}"))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to parse Cardano header JSON: {e}"))
        })?;

        let block_hash = header_json
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BridgeError::InvalidHeader("Cardano header missing hash".to_string()))?;

        if block_hash.is_empty() {
            return Err(BridgeError::InvalidHeader("Cardano block hash is empty".to_string()));
        }

        let _slot = header_json
            .get("slot")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BridgeError::InvalidHeader("Cardano header missing slot".to_string()))?;

        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "getBlockByNumber",
            serde_json::json!([block_number]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!("Block {} not found on Cardano", block_number)));
        }

        let block_hash = result.get("hash").and_then(|v| v.as_str()).unwrap_or("");
        let prev_hash = result.get("previousHash").and_then(|v| v.as_str()).unwrap_or("");
        let slot = result.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);

        let proof_envelope = serde_json::json!({
            "proof_type": "cardano-block-proof-v1",
            "chain_id": self.chain_id,
            "block_hash": block_hash,
            "block_number": block_number,
            "previous_hash": prev_hash,
            "slot": slot,
            "epoch": result.get("epoch").and_then(|v| v.as_u64()),
            "era": result.get("era").and_then(|v| v.as_str()),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope)
            .map_err(|e| BridgeError::Serialization(format!("Failed to serialize Cardano proof: {e}")))?;
        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(&self.rpc_url, "queryLedgerState", serde_json::json!({}))?;
        result
            .get("blockHeight")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| BridgeError::RpcError("queryLedgerState missing blockHeight".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardano_adapter_creation() {
        let adapter = PlutusBridgeAdapter::new(1, "http://localhost:1337".to_string());
        assert_eq!(adapter.chain_name(), "cardano");
    }

    #[test]
    fn validate_header_rejects_empty() {
        let adapter = PlutusBridgeAdapter::new(1, "http://localhost:1337".to_string());
        assert!(adapter.validate_header(&[]).is_err());
    }

    #[test]
    fn validate_header_accepts_valid_structure() {
        let adapter = PlutusBridgeAdapter::new(1, "http://localhost:1337".to_string());
        let header = serde_json::json!({
            "hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "slot": 12345678,
            "epoch": 425,
            "blockHeight": 9000000
        });
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_ok());
    }

    #[test]
    fn validate_header_rejects_missing_hash() {
        let adapter = PlutusBridgeAdapter::new(1, "http://localhost:1337".to_string());
        let header = serde_json::json!({"slot": 123});
        assert!(adapter.validate_header(header.to_string().as_bytes()).is_err());
    }
}