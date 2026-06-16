//! Bitcoin Bridge Adapter
//!
//! Provides bridge functionality for Bitcoin-compatible chains.
//! Fail-closed pattern: when the `bitcoin-adapter` feature is off (default),
//! the adapter gate returns `BridgeError::BtcAdapterDisabled` before any
//! production logic runs. When enabled, proof generation and block number
//! retrieval use Bitcoin Core JSON-RPC. Header validation fails with an
//! explicit "not implemented" error — consumers wire up a real Bitcoin
//! light client in their runtime.

use crate::{make_json_rpc_call, BridgeAdapter, BridgeError};

/// Error code returned when the BTC adapter is disabled.
pub const BTC_ADAPTER_DISABLED_CODE: &str = "X3_BTC_ADAPTER_DISABLED";

/// Bitcoin Bridge Adapter
pub struct BitcoinBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl BitcoinBridgeAdapter {
    /// Create a new Bitcoin bridge adapter
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Feature gate. Returns `Ok(())` when `bitcoin-adapter` is enabled,
    /// otherwise `BridgeError::BtcAdapterDisabled`.
    fn check_gate(&self) -> Result<(), BridgeError> {
        #[cfg(feature = "bitcoin-adapter")]
        {
            Ok(())
        }
        #[cfg(not(feature = "bitcoin-adapter"))]
        {
            Err(BridgeError::BtcAdapterDisabled)
        }
    }
}

impl BridgeAdapter for BitcoinBridgeAdapter {
    fn chain_name(&self) -> &str {
        "bitcoin"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError> {
        self.check_gate()?;

        if header.is_empty() {
            return Err(BridgeError::InvalidHeader(
                "BTC finality proof is empty".to_string(),
            ));
        }

        // Bitcoin header is exactly 80 bytes.
        // Fields: version(4) | prev_block(32) | merkle_root(32) |
        //         timestamp(4) | bits(4) | nonce(4)
        if header.len() != 80 {
            return Err(BridgeError::InvalidHeader(format!(
                "BTC header must be 80 bytes, got {}",
                header.len()
            )));
        }

        // Decode the nBits compact target.
        let bits = u32::from_le_bytes([
            header[72], header[73], header[74], header[75],
        ]);
        let exponent = (bits >> 24) as usize;
        let mantissa = bits & 0x00FF_FFFF;
        if exponent == 0 || exponent > 34 {
            return Err(BridgeError::InvalidHeader(
                "BTC header: invalid nBits exponent".to_string(),
            ));
        }

        // Compute the target from nBits: target = mantissa * 256^(exponent - 3)
        let mut target = [0u8; 32];
        // Place the 3 mantissa bytes at byte position (exponent - 3)
        let shift = exponent.saturating_sub(3);
        if shift < 29 {
            target[shift] = ((mantissa >> 16) & 0xFF) as u8;
            target[shift + 1] = ((mantissa >> 8) & 0xFF) as u8;
            target[shift + 2] = (mantissa & 0xFF) as u8;
        }

        // Double-SHA256 of the 80-byte header = block hash
        let first_hash = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(header);
            h.finalize()
        };
        let block_hash = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&first_hash);
            h.finalize()
        };

        // PoW check: block_hash (as big-endian integer) must be <= target
        let hash_bytes: &[u8; 32] = block_hash.as_ref();
        for i in (0..32).rev() {
            let hb = hash_bytes[i];
            let tb = target[i];
            if hb < tb {
                return Ok(()); // hash < target → valid PoW
            }
            if hb > tb {
                return Err(BridgeError::InvalidHeader(format!(
                    "BTC header: proof-of-work invalid. hash {:?}, target {:?}",
                    hex::encode(hash_bytes),
                    hex::encode(&target)
                )));
            }
        }
        // hash exactly equals target → also valid (extremely unlikely but valid)
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        self.check_gate()?;

        // Retrieve the block header (verbosity 1) for the canonical
        // block hash and build a proof envelope suitable for downstream
        // verification.  PoW header-chain validation is performed by
        // a dedicated Bitcoin light client; this adapter ensures the
        // payload is a structured proof envelope instead of raw block
        // JSON.
        let block_hash_result = make_json_rpc_call(
            &self.rpc_url,
            "getblockhash",
            serde_json::json!([block_number]),
        )?;

        let block_hash = block_hash_result
            .as_str()
            .ok_or_else(|| {
                BridgeError::RpcError("getblockhash returned non-string".to_string())
            })?;

        if block_hash.is_empty() {
            return Err(BridgeError::RpcError(format!(
                "Block hash empty for block {}",
                block_number
            )));
        }

        let result = make_json_rpc_call(
            &self.rpc_url,
            "getblock",
            serde_json::json!([block_hash, 1]), // verbosity 1 = block header (hex-encoded)
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Block {} ({}) not found on Bitcoin chain",
                block_number, block_hash
            )));
        }

        // Build a verifiable proof envelope with block hash, header hex,
        // confirmations, and chainwork so downstream verifiers can
        // perform PoW header-chain validation.
        let proof_envelope = serde_json::json!({
            "proof_type": "bitcoin-block-proof-v1",
            "chain_id": self.chain_id,
            "block_hash": block_hash,
            "block_height": block_number,
            "header_hex": result.get("hex").and_then(|v| v.as_str()).unwrap_or(""),
            "confirmations": result.get("confirmations").and_then(|v| v.as_u64()).unwrap_or(0),
            "previous_blockhash": result.get("previousblockhash").and_then(|v| v.as_str()).unwrap_or(""),
            "merkle_root": result.get("merkleroot").and_then(|v| v.as_str()).unwrap_or(""),
            "chainwork": result.get("chainwork").and_then(|v| v.as_str()).unwrap_or(""),
            "difficulty": result.get("difficulty").and_then(|v| v.as_f64()).unwrap_or(0.0)
        });

        let proof_bytes = serde_json::to_vec(&proof_envelope).map_err(|e| {
            BridgeError::Serialization(format!("Failed to serialize Bitcoin proof: {e}"))
        })?;

        Ok(proof_bytes)
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        self.check_gate()?;

        let result = make_json_rpc_call(
            &self.rpc_url,
            "getblockcount",
            serde_json::json!([]),
        )?;

        result
            .as_u64()
            .ok_or_else(|| BridgeError::RpcError("getblockcount returned non-integer".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_adapter_creation() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        assert_eq!(adapter.chain_name(), "bitcoin");
        assert_eq!(adapter.chain_id(), 0);
    }

    #[test]
    fn validate_header_empty_proof_fails() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        let result = adapter.validate_header(&[]);
        assert!(result.is_err());

        let err = result.unwrap_err();
        // With default features (bitcoin-adapter off), should get BtcAdapterDisabled.
        // With bitcoin-adapter on, should get InvalidHeader for empty proof.
        match &err {
            BridgeError::BtcAdapterDisabled => {}
            BridgeError::InvalidHeader(msg) => {
                assert!(
                    msg.contains("empty"),
                    "Expected empty proof error, got: {}",
                    msg
                );
            }
            _ => panic!("Unexpected error variant: {:?}", err),
        }
    }

    #[test]
    fn validate_header_fails_closed() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        let result = adapter.validate_header(b"btc-header-v1:abcdef123456");
        assert!(result.is_err());
    }

    #[cfg(feature = "bitcoin-adapter")]
    #[test]
    fn feature_enabled_fails_closed_until_pow_validation_wired() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        let result = adapter.validate_header(b"btc-header-v1:abc");
        // With the feature enabled, check_gate passes, but real validation
        // is not implemented — it still fails.
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(&err, BridgeError::InvalidHeader(_)),
            "Expected InvalidHeader, got {:?}",
            err
        );
    }
}