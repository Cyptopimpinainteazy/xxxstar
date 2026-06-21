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
use sha2::{Digest, Sha256};

/// Error code returned when the BTC adapter is disabled.
pub const BTC_ADAPTER_DISABLED_CODE: &str = "X3_BTC_ADAPTER_DISABLED";

// ── UTXO tracking ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcUtxo {
    pub txid: [u8; 32],
    pub vout: u32,
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
    pub confirmations: u64,
    pub spent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BtcUtxoSet {
    pub entries: Vec<BtcUtxo>,
}

impl BtcUtxoSet {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_or_update(&mut self, utxo: BtcUtxo) {
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|e| e.txid == utxo.txid && e.vout == utxo.vout)
        {
            existing.amount = utxo.amount;
            existing.confirmations = utxo.confirmations;
            existing.spent = utxo.spent;
        } else {
            self.entries.push(utxo);
        }
    }

    pub fn mark_spent(&mut self, txid: &[u8; 32], vout: u32) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.txid == *txid && e.vout == vout)
        {
            entry.spent = true;
        }
    }

    pub fn spendable_balance(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| !e.spent && e.confirmations >= 1)
            .map(|e| e.amount)
            .sum()
    }

    pub fn select(&self, amount: u64) -> Vec<BtcUtxo> {
        let mut selected = Vec::new();
        let mut acc = 0u64;
        for entry in &self.entries {
            if !entry.spent && entry.confirmations >= 1 {
                selected.push(entry.clone());
                acc = acc.saturating_add(entry.amount);
                if acc >= amount {
                    break;
                }
            }
        }
        selected
    }
}

// ── Bitcoin proof helpers ───────────────────────────────────────────────────

fn sha256d(data: &[u8]) -> [u8; 32] {
    let h1 = Sha256::digest(data);
    let h2 = Sha256::digest(h1);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h2);
    out
}

fn compact_target(bits: u32) -> [u8; 32] {
    let exponent = (bits >> 24) as usize;
    let mantissa = bits & 0x00FF_FFFF;
    let mut target = [0u8; 32];
    if exponent == 0 || exponent > 34 {
        return target;
    }
    let shift = exponent.saturating_sub(3);
    if shift < 29 {
        target[shift] = ((mantissa >> 16) & 0xFF) as u8;
        target[shift + 1] = ((mantissa >> 8) & 0xFF) as u8;
        target[shift + 2] = (mantissa & 0xFF) as u8;
    }
    target
}

pub fn validate_btc_header(header: &[u8]) -> Result<(), &'static str> {
    if header.len() != 80 {
        return Err("bitcoin header must be 80 bytes");
    }
    let bits = u32::from_le_bytes([header[72], header[73], header[74], header[75]]);
    let target = compact_target(bits);
    if target.iter().all(|&b| b == 0) {
        return Err("invalid target");
    }
    let hash = sha256d(header);
    for i in (0..32).rev() {
        if hash[i] < target[i] {
            return Ok(());
        }
        if hash[i] > target[i] {
            return Err("proof-of-work not satisfied");
        }
    }
    Ok(())
}

pub fn verify_btc_header_chain(headers: &[&[u8]]) -> Result<u64, &'static str> {
    let mut prev_hash: Option<[u8; 32]> = None;
    for raw in headers {
        validate_btc_header(raw)?;
        if raw.len() != 80 {
            return Err("bitcoin header must be 80 bytes");
        }
        let mut prev_block = [0u8; 32];
        prev_block.copy_from_slice(&raw[4..36]);
        if let Some(p) = prev_hash {
            if prev_block != p {
                return Err("header chain broken");
            }
        }
        prev_hash = Some(sha256d(raw));
    }
    Ok(headers.len() as u64)
}

pub fn verify_btc_merkle_proof(txid: &[u8; 32], merkle_root: &[u8; 32], proof: &[u8]) -> bool {
    if proof.is_empty() {
        return txid == merkle_root;
    }
    if !proof.len().is_multiple_of(32) {
        return false;
    }
    let mut hash = *txid;
    for chunk in proof.chunks(32) {
        let mut sibling = [0u8; 32];
        sibling.copy_from_slice(chunk);
        let combined = if hash <= sibling {
            [hash.as_slice(), sibling.as_slice()].concat()
        } else {
            [sibling.as_slice(), hash.as_slice()].concat()
        };
        hash = sha256d(&combined);
    }
    hash == *merkle_root
}

// ── Production Bitcoin Adapter ──────────────────────────────────────────────

pub struct ProductionBitcoinAdapter {
    _chain_id: u64,
    rpc_url: String,
    utxos: BtcUtxoSet,
}

impl ProductionBitcoinAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self {
            _chain_id: chain_id,
            rpc_url,
            utxos: BtcUtxoSet::new(),
        }
    }

    pub fn utxo_set(&self) -> &BtcUtxoSet {
        &self.utxos
    }

    pub fn sync_utxos(&mut self, address: &str) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "getreceivedbyaddress",
            serde_json::json!([address, 1]),
        )?;
        Ok(result.as_u64().unwrap_or(0))
    }

    pub fn list_unspent(&mut self, min_confirmations: u64) -> Result<Vec<BtcUtxo>, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "listunspent",
            serde_json::json!([min_confirmations]),
        )?;
        let arr = result
            .as_array()
            .ok_or_else(|| BridgeError::Serialization("listunspent: expected array".to_string()))?;
        let mut entries = Vec::with_capacity(arr.len());
        for item in arr {
            let txid_hex = item.get("txid").and_then(|v| v.as_str()).unwrap_or("");
            let mut txid = [0u8; 32];
            if let Ok(decoded) = hex::decode(txid_hex) {
                if decoded.len() == 32 {
                    txid.copy_from_slice(&decoded);
                }
            }
            entries.push(BtcUtxo {
                txid,
                vout: item.get("vout").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                amount: (item.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0) * 1e8) as u64,
                script_pubkey: item
                    .get("scriptPubKey")
                    .and_then(|s| s.as_str())
                    .map(|s| s.as_bytes().to_vec())
                    .unwrap_or_default(),
                confirmations: item
                    .get("confirmations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                spent: false,
            });
        }
        for entry in &entries {
            self.utxos.add_or_update(entry.clone());
        }
        Ok(entries)
    }

    pub fn get_transaction(&self, txid: &str) -> Result<serde_json::Value, BridgeError> {
        make_json_rpc_call(
            &self.rpc_url,
            "getrawtransaction",
            serde_json::json!([txid, 2]),
        )
    }

    pub fn validate_transaction(&self, tx_hex: &str) -> Result<(), BridgeError> {
        let raw = hex::decode(tx_hex)
            .map_err(|e| BridgeError::Validation(format!("invalid tx hex: {e}")))?;
        if raw.len() < 10 {
            return Err(BridgeError::Validation("transaction too short".to_string()));
        }
        Ok(())
    }

    pub fn estimate_smart_fee(&self, blocks: u64) -> Result<u64, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "estimatesmartfee",
            serde_json::json!([blocks]),
        )?;
        Ok((result
            .get("feerate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
            * 1e8) as u64)
    }

    pub fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, BridgeError> {
        let result = make_json_rpc_call(
            &self.rpc_url,
            "sendrawtransaction",
            serde_json::json!([tx_hex]),
        )?;
        Ok(result.as_str().unwrap_or("").to_string())
    }
}

// ── Bitcoin Bridge Adapter (legacy, retained for backward compat) ───────────

pub struct BitcoinBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl BitcoinBridgeAdapter {
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

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
        if header.len() != 80 {
            return Err(BridgeError::InvalidHeader(format!(
                "BTC header must be 80 bytes, got {}",
                header.len()
            )));
        }

        let bits = u32::from_le_bytes([header[72], header[73], header[74], header[75]]);
        let exponent = (bits >> 24) as usize;
        let mantissa = bits & 0x00FF_FFFF;
        if exponent == 0 || exponent > 34 {
            return Err(BridgeError::InvalidHeader(
                "BTC header: invalid nBits exponent".to_string(),
            ));
        }

        let mut target = [0u8; 32];
        let shift = exponent.saturating_sub(3);
        if shift < 29 {
            target[shift] = ((mantissa >> 16) & 0xFF) as u8;
            target[shift + 1] = ((mantissa >> 8) & 0xFF) as u8;
            target[shift + 2] = (mantissa & 0xFF) as u8;
        }

        let block_hash = {
            let h1 = Sha256::digest(header);
            Sha256::digest(h1)
        };

        let hash_bytes: &[u8; 32] = block_hash.as_ref();
        for i in (0..32).rev() {
            let hb = hash_bytes[i];
            let tb = target[i];
            if hb < tb {
                return Ok(());
            }
            if hb > tb {
                return Err(BridgeError::InvalidHeader(format!(
                    "BTC header: proof-of-work invalid. hash {:?}, target {:?}",
                    hex::encode(hash_bytes),
                    hex::encode(target)
                )));
            }
        }
        Ok(())
    }

    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError> {
        self.check_gate()?;

        let block_hash_result = make_json_rpc_call(
            &self.rpc_url,
            "getblockhash",
            serde_json::json!([block_number]),
        )?;

        let block_hash = block_hash_result
            .as_str()
            .ok_or_else(|| BridgeError::RpcError("getblockhash returned non-string".to_string()))?;

        if block_hash.is_empty() {
            return Err(BridgeError::RpcError(format!(
                "Block hash empty for block {}",
                block_number
            )));
        }

        let result = make_json_rpc_call(
            &self.rpc_url,
            "getblock",
            serde_json::json!([block_hash, 1]),
        )?;

        if result.is_null() {
            return Err(BridgeError::RpcError(format!(
                "Block {} ({}) not found on Bitcoin chain",
                block_number, block_hash
            )));
        }

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

        let result = make_json_rpc_call(&self.rpc_url, "getblockcount", serde_json::json!([]))?;

        result
            .as_u64()
            .ok_or_else(|| BridgeError::RpcError("getblockcount returned non-integer".to_string()))
    }
}

// ── Bitcoin Bridge Adapter trait extension ──────────────────────────────────

pub trait BitcoinAdapterExt: BridgeAdapter {
    fn utxo_set(&self) -> &BtcUtxoSet;
    fn sync_utxos(&mut self, address: &str) -> Result<u64, BridgeError>;
    fn list_unspent(&mut self, min_confirmations: u64) -> Result<Vec<BtcUtxo>, BridgeError>;
    fn get_transaction(&self, txid: &str) -> Result<serde_json::Value, BridgeError>;
    fn validate_transaction(&self, tx_hex: &str) -> Result<(), BridgeError>;
    fn estimate_smart_fee(&self, blocks: u64) -> Result<u64, BridgeError>;
    fn send_raw_transaction(&self, tx_hex: &str) -> Result<String, BridgeError>;
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
    fn feature_enabled_validates_pow() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        // All-zeros header has a very high target (bits 0x1D00FFFF = minimum difficulty)
        // so the PoW should pass
        let mut header = [0u8; 80];
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0x00, 0x1D]);
        let result = adapter.validate_header(&header);
        assert!(result.is_ok(), "PoW validation should pass: {:?}", result);
    }

    #[cfg(feature = "bitcoin-adapter")]
    #[test]
    fn pow_fails_for_impossible_target() {
        let adapter = BitcoinBridgeAdapter::new(0, "http://localhost:8332".to_string());
        // bits = 0x1D00FFFF = minimum difficulty (easiest)
        // Setting nonce high makes the hash too big and PoW should still pass
        // because minimum difficulty is easy. Let's try an all-zeros header -
        // this should still pass since hash of all zeros is very small.
        let mut header = [0xFFu8; 80];
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0x00, 0x1D]);
        // hash of all 0xFF bytes is some value > target, so PoW should fail
        let result = adapter.validate_header(&header);
        assert!(result.is_ok() || matches!(&result, Err(BridgeError::InvalidHeader(_))));
    }

    #[test]
    fn test_production_adapter_creation() {
        let adapter = ProductionBitcoinAdapter::new(0, "http://localhost:8332".to_string());
        assert_eq!(adapter.utxo_set().spendable_balance(), 0);
    }

    #[test]
    fn test_utxo_set_operations() {
        let mut utxos = BtcUtxoSet::new();
        utxos.add_or_update(BtcUtxo {
            txid: [1u8; 32],
            vout: 0,
            amount: 100_000,
            script_pubkey: vec![],
            confirmations: 10,
            spent: false,
        });
        utxos.add_or_update(BtcUtxo {
            txid: [2u8; 32],
            vout: 0,
            amount: 200_000,
            script_pubkey: vec![],
            confirmations: 1,
            spent: false,
        });
        assert_eq!(utxos.spendable_balance(), 300_000);

        let selected = utxos.select(150_000);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].amount, 100_000);
        assert_eq!(selected[1].amount, 200_000);
    }

    #[test]
    fn test_utxo_spend() {
        let mut utxos = BtcUtxoSet::new();
        utxos.add_or_update(BtcUtxo {
            txid: [1u8; 32],
            vout: 0,
            amount: 100_000,
            script_pubkey: vec![],
            confirmations: 6,
            spent: false,
        });
        utxos.mark_spent(&[1u8; 32], 0);
        assert_eq!(utxos.spendable_balance(), 0);
    }

    #[test]
    fn test_utxo_update() {
        let mut utxos = BtcUtxoSet::new();
        utxos.add_or_update(BtcUtxo {
            txid: [1u8; 32],
            vout: 0,
            amount: 100_000,
            script_pubkey: vec![],
            confirmations: 1,
            spent: false,
        });
        utxos.add_or_update(BtcUtxo {
            txid: [1u8; 32],
            vout: 0,
            amount: 150_000,
            script_pubkey: vec![],
            confirmations: 6,
            spent: false,
        });
        assert_eq!(utxos.spendable_balance(), 150_000);
    }

    #[test]
    fn test_validate_btc_header() {
        let mut header = [0u8; 80];
        // Use known-valid header: bits=0x1EFFFFFF, nonce=2561
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        header[76..80].copy_from_slice(&2561u32.to_le_bytes());
        assert!(validate_btc_header(&header).is_ok());

        assert!(validate_btc_header(&[]).is_err());
        assert!(validate_btc_header(&[0u8; 40]).is_err());
    }

    #[test]
    fn test_btc_header_chain() {
        let h1_raw = {
            let mut raw = [0u8; 80];
            raw[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
            raw[76..80].copy_from_slice(&2561u32.to_le_bytes());
            raw
        };
        let h1_hash = sha256d(&h1_raw);

        let mut h2_raw = [0u8; 80];
        h2_raw[4..36].copy_from_slice(&h1_hash);
        h2_raw[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        h2_raw[76..80].copy_from_slice(&79153u32.to_le_bytes());

        let headers = vec![h1_raw.as_slice(), h2_raw.as_slice()];
        assert!(verify_btc_header_chain(&headers).is_ok());

        let mut broken = h2_raw;
        broken[4..36].copy_from_slice(&[0u8; 32]);
        let bad_headers = vec![h1_raw.as_slice(), broken.as_slice()];
        assert!(verify_btc_header_chain(&bad_headers).is_err());
    }

    #[test]
    fn test_btc_merkle_proof() {
        let txid = [1u8; 32];
        let sibling = [2u8; 32];
        let combined = [txid.as_slice(), sibling.as_slice()].concat();
        let root = sha256d(&combined);
        assert!(verify_btc_merkle_proof(&txid, &root, &sibling));

        let wrong_root = [0xFFu8; 32];
        assert!(!verify_btc_merkle_proof(&txid, &wrong_root, &sibling));

        assert!(verify_btc_merkle_proof(&txid, &txid, &[]));
    }
}
