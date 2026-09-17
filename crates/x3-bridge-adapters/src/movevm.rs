//! MoveVM Bridge Adapter (Sui + Aptos)
//!
//! Provides bridge functionality for Move-based blockchains — Sui and Aptos.
//! Move uses a resource/object-based asset model fundamentally different from
//! EVM accounts, making this adapter critical for X3's cross-VM reach.
//!
//! Supported chains:
//!   * Sui mainnet (chain_id 21), Sui testnet
//!   * Aptos mainnet (chain_id 1), Aptos testnet (chain_id 2)

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// MoveVM variant — Sui or Aptos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveVmVariant {
    Sui,
    Aptos,
}

/// MoveVM Bridge Adapter
///
/// Connects to a Sui or Aptos full node via its JSON-RPC endpoint to validate
/// block headers, generate proofs, and poll Move events.
pub struct MoveVmBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
    variant: MoveVmVariant,
}

impl MoveVmBridgeAdapter {
    /// Create a new MoveVM bridge adapter.
    ///
    /// * `chain_id` — Sui mainnet = 21, Aptos mainnet = 1
    /// * `rpc_url` — Full node RPC endpoint
    /// * `variant` — Which MoveVM flavor this adapter targets
    pub fn new(chain_id: u64, rpc_url: String, variant: MoveVmVariant) -> Self {
        Self {
            chain_id,
            rpc_url,
            variant,
        }
    }

    /// Get the RPC URL.
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get the MoveVM variant.
    pub fn variant(&self) -> MoveVmVariant {
        self.variant
    }

    /// Poll Move events for a given object or account.
    ///
    /// For Sui, uses `sui_getEvents` with a filter on the object ID.
    /// For Aptos, uses `get_events_by_event_handle` with the account address
    /// and event handle struct.
    pub fn poll_move_events(
        &self,
        object_id: &str,
        from_block: u64,
        to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        match self.variant {
            MoveVmVariant::Sui => self.poll_sui_events(object_id, from_block, to_block),
            MoveVmVariant::Aptos => self.poll_aptos_events(object_id, from_block, to_block),
        }
    }

    fn poll_sui_events(
        &self,
        object_id: &str,
        from_block: u64,
        _to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        let params = serde_json::json!([{
            "EventFilter": {
                "MoveModule": {
                    "package": "0x3",
                    "module": "sui_system"
                }
            }
        }, null, 50, false]); // cursor, limit, descending

        let result = make_json_rpc_call(&self.rpc_url, "sui_getEvents", params)?;

        let events = result
            .as_array()
            .ok_or_else(|| BridgeError::RpcError("sui_getEvents returned non-array".to_string()))?;

        let mut results = Vec::with_capacity(events.len());
        for event in events {
            let checkpoint = event
                .get("checkpoint")
                .and_then(|c| c.as_u64())
                .unwrap_or(0);

            let event_bytes = serde_json::to_vec(event).unwrap_or_default();
            results.push((checkpoint, event_bytes));
        }

        // Filter to matching object
        let filtered: Vec<_> = results
            .into_iter()
            .filter(|(_, data)| {
                if let Ok(s) = String::from_utf8(data.clone()) {
                    s.contains(object_id)
                } else {
                    false
                }
            })
            .collect();

        Ok(filtered)
    }

    fn poll_aptos_events(
        &self,
        account_address: &str,
        from_block: u64,
        _to_block: u64,
    ) -> Result<Vec<(u64, Vec<u8>)>, BridgeError> {
        let params = serde_json::json!([account_address, "0x1::account::AccountEvents", from_block, 50]);

        let result =
            make_json_rpc_call(&self.rpc_url, "get_events_by_event_handle", params)?;

        let events = result
            .as_array()
            .ok_or_else(|| {
                BridgeError::RpcError("get_events_by_event_handle returned non-array".to_string())
            })?;

        let mut results = Vec::with_capacity(events.len());
        for event in events {
            let version = event
                .get("version")
                .and_then(|v| v.as_str())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);

            let event_bytes = serde_json::to_vec(event).unwrap_or_default();
            results.push((version, event_bytes));
        }

        Ok(results)
    }

    /// Check if the Sui/Aptos node is healthy and synced.
    pub fn check_node_health(&self) -> Result<bool, BridgeError> {
        match self.variant {
            MoveVmVariant::Sui => {
                let result =
                    make_json_rpc_call(&self.rpc_url, "sui_getLatestCheckpointSequenceNumber", serde_json::json!([]))?;
                Ok(result.as_u64().unwrap_or(0) > 0)
            }
            MoveVmVariant::Aptos => {
                let result =
                    make_json_rpc_call(&self.rpc_url, "get_ledger_info", serde_json::json!([]))?;
                let version = result
                    .get("ledger_version")
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                Ok(version > 0)
            }
        }
    }
}

impl BridgeAdapter for MoveVmBridgeAdapter {
    fn chain_name(&self) -> &str {
        match self.variant {
            MoveVmVariant::Sui => "sui",
            MoveVmVariant::Aptos => "aptos",
        }
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        if header.is_empty() {
            return Err(BridgeError::InvalidHeader(format!(
                "{} header is empty",
                self.chain_name()
            )));
        }

        match self.variant {
            MoveVmVariant::Sui => self.validate_sui_header(header),
            MoveVmVariant::Aptos => self.validate_aptos_header(header),
        }
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        match self.variant {
            MoveVmVariant::Sui => self.generate_sui_proof(block_number),
            MoveVmVariant::Aptos => self.generate_aptos_proof(block_number),
        }
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        match self.variant {
            MoveVmVariant::Sui => {
                let result = make_json_rpc_call(
                    &self.rpc_url,
                    "sui_getLatestCheckpointSequenceNumber",
                    serde_json::json!([]),
                )?;
                result
                    .as_u64()
                    .ok_or_else(|| {
                        BridgeError::RpcError(
                            "sui_getLatestCheckpointSequenceNumber returned non-integer"
                                .to_string(),
                        )
                    })
            }
            MoveVmVariant::Aptos => {
                let result =
                    make_json_rpc_call(&self.rpc_url, "get_ledger_info", serde_json::json!([]))?;
                let version = result
                    .get("ledger_version")
                    .and_then(|v| v.as_str())
                    .and_then(|v| v.parse::<u64>().ok())
                    .ok_or_else(|| {
                        BridgeError::RpcError(
                            "get_ledger_info missing ledger_version".to_string(),
                        )
                    })?;
                Ok(version)
            }
        }
    }
}

// ── Sui-specific header validation + proof generation ─────────────────────

impl MoveVmBridgeAdapter {
    fn validate_sui_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Sui header is not valid UTF-8: {e}"))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to parse Sui header JSON: {e}"))
        })?;

        // Sui checkpoint summary includes: epoch, sequence_number, digest, etc.
        let digest = header_json
            .get("digest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BridgeError::InvalidHeader("Sui header missing digest".to_string()))?;

        if digest.is_empty() {
            return Err(BridgeError::InvalidHeader(
                "Sui header digest is empty".to_string(),
            ));
        }

        let _epoch = header_json
            .get("epoch")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| BridgeError::InvalidHeader("Sui header missing epoch".to_string()))?;

        Ok(())
    }

    fn validate_aptos_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        let header_str = std::str::from_utf8(header).map_err(|e| {
            BridgeError::InvalidHeader(format!("Aptos header is not valid UTF-8: {e}"))
        })?;

        let header_json: serde_json::Value = serde_json::from_str(header_str).map_err(|e| {
            BridgeError::InvalidHeader(format!("Failed to parse Aptos header JSON: {e}"))
        })?;

        // Aptos ledger info includes: ledger_version, timestamp, etc.
        let _version = header_json
            .get("ledger_version")
            .and_then(|v| v.as_str())
            .and_then(|v| v.parse::<u64>().ok())
            .ok_or_else(|| {
                BridgeError::InvalidHeader("Aptos header missing ledger_version".to_string())
            })?;

        Ok(())
    }

    fn generate_sui_proof(&self, checkpoint: u64) -> Result<Vec<u8>, BridgeError> {
        let checkpoint_str = checkpoint.to_string();

        let result = make_json_rpc_call(
            &self.rpc_url,
            "sui_getCheckpoint",
            serde_json::json!([checkpoint_str]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Checkpoint {} not found on Sui chain",
                checkpoint
            )));
        }

        let digest = result
            .get("digest")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let epoch = result
            .get("epoch")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let network_total_transactions = result
            .get("network_total_transactions")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let proof_envelope = serde_json::json!({
            "proof_type": "sui-checkpoint-proof-v1",
            "chain_id": self.chain_id,
            "move_variant": "sui",
            "checkpoint_sequence": checkpoint,
            "digest": digest,
            "epoch": epoch,
            "network_total_transactions": network_total_transactions,
            "validator_signatures": result.get("validator_signatures"),
            "previous_digest": result.get("previous_digest"),
            "end_of_epoch_data": result.get("end_of_epoch_data"),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope).map_err(|e| {
            BridgeError::Serialization(format!("Failed to serialize Sui proof: {e}"))
        })?;

        Ok(proof_bytes)
    }

    fn generate_aptos_proof(&self, version: u64) -> Result<Vec<u8>, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "get_block_by_version",
            serde_json::json!([version, false]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Block at version {} not found on Aptos chain",
                version
            )));
        }

        let block_hash = result
            .get("block_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let block_height = result
            .get("block_height")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let timestamp = result
            .get("block_timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        let proof_envelope = serde_json::json!({
            "proof_type": "aptos-block-proof-v1",
            "chain_id": self.chain_id,
            "move_variant": "aptos",
            "version": version,
            "block_hash": block_hash,
            "block_height": block_height,
            "block_timestamp": timestamp,
            "first_version": result.get("first_version").and_then(|v| v.as_str()),
            "last_version": result.get("last_version").and_then(|v| v.as_str()),
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope).map_err(|e| {
            BridgeError::Serialization(format!("Failed to serialize Aptos proof: {e}"))
        })?;

        Ok(proof_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Sui tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_sui_adapter_creation() {
        let adapter =
            MoveVmBridgeAdapter::new(21, "http://localhost:9000".to_string(), MoveVmVariant::Sui);
        assert_eq!(adapter.chain_name(), "sui");
        assert_eq!(adapter.chain_id(), 21);
        assert_eq!(adapter.variant(), MoveVmVariant::Sui);
    }

    #[test]
    fn sui_validate_header_rejects_empty() {
        let adapter =
            MoveVmBridgeAdapter::new(21, "http://localhost:9000".to_string(), MoveVmVariant::Sui);
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("empty"));
    }

    #[test]
    fn sui_validate_header_accepts_valid_checkpoint() {
        let adapter =
            MoveVmBridgeAdapter::new(21, "http://localhost:9000".to_string(), MoveVmVariant::Sui);
        let header = serde_json::json!({
            "epoch": "42",
            "sequence_number": "1000000",
            "digest": "5grEX6u33tCVYrFwNzq1GpRgTEFZFnYKKQ7PfqVCc3MN",
            "previous_digest": "HA4pTYKvGX2CLqrMmFCYeQJgdgEmaq9dHfrkPXeMBzAj",
            "network_total_transactions": "50000000",
            "timestamp_ms": "1700000000000"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(result.is_ok(), "Expected valid header, got: {:?}", result.err());
    }

    #[test]
    fn sui_validate_header_rejects_missing_digest() {
        let adapter =
            MoveVmBridgeAdapter::new(21, "http://localhost:9000".to_string(), MoveVmVariant::Sui);
        let header = serde_json::json!({
            "epoch": "42",
            "sequence_number": "1000000"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("digest"));
    }

    // ── Aptos tests ───────────────────────────────────────────────────────

    #[test]
    fn test_aptos_adapter_creation() {
        let adapter = MoveVmBridgeAdapter::new(
            1,
            "http://localhost:8080".to_string(),
            MoveVmVariant::Aptos,
        );
        assert_eq!(adapter.chain_name(), "aptos");
        assert_eq!(adapter.chain_id(), 1);
        assert_eq!(adapter.variant(), MoveVmVariant::Aptos);
    }

    #[test]
    fn aptos_validate_header_rejects_empty() {
        let adapter = MoveVmBridgeAdapter::new(
            1,
            "http://localhost:8080".to_string(),
            MoveVmVariant::Aptos,
        );
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("empty"));
    }

    #[test]
    fn aptos_validate_header_accepts_valid_ledger_info() {
        let adapter = MoveVmBridgeAdapter::new(
            1,
            "http://localhost:8080".to_string(),
            MoveVmVariant::Aptos,
        );
        let header = serde_json::json!({
            "ledger_version": "123456789",
            "ledger_timestamp": "1700000000000000",
            "chain_id": 1,
            "epoch": "50",
            "block_height": "100000"
        });
        let result = adapter.validate_header(header.to_string().as_bytes());
        assert!(result.is_ok(), "Expected valid header, got: {:?}", result.err());
    }
}