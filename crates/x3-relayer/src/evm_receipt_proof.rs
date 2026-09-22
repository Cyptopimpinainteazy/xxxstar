//! Build the receipt-inclusion proof a settlement engine accepts.
//!
//! The settlement engine's EVM path walks a receipt to a receipts root it has
//! attested, so a producer has to hand it three things that agree with each
//! other: the receipt in its **consensus** encoding, the index the block listed
//! it at, and the trie path between them. This module fetches them from a chain
//! and checks its own work before returning:
//!
//! 1. every receipt of the block is encoded the way the consensus spec says
//!    (EIP-2718 type byte included for typed receipts, which is what the trie
//!    leaf commits to);
//! 2. the trie built from those encodings has the `receiptsRoot` the *block
//!    header* carries — a mismatch is a hard error, because every proof built
//!    from that trie would be rejected and the reason would look like a
//!    verification bug rather than an encoding bug;
//! 3. the proof just built verifies against that root with the same function the
//!    verifier uses, so a producer and a verifier cannot drift into two
//!    conventions.
//!
//! What this does *not* do: decide whether the header is the chain's. That is the
//! verifier's anchor, and it has to come from whoever holds the chain's attested
//! view.
//!
//! The tests here spawn `anvil` (foundry) because a receipt proof is about a real
//! block — the same requirement `crates/external-chains/tests/send_message_broadcasts.rs`
//! and the `EVM contract lifecycle` gate already carry.

use serde_json::{json, Value};
use x3_verification_router::evm_receipt::{
    receipt_trie_key, receipts_trie_proof, receipts_trie_root, verify_merkle_patricia_proof,
};

/// A receipt's inclusion in a block's receipts trie, as the settlement engine
/// needs to see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptInclusion {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub state_root: [u8; 32],
    /// The block header's `receiptsRoot` — the root the trie below was checked
    /// against, not a root this module chose.
    pub receipts_root: [u8; 32],
    pub receipt_index: u32,
    /// The receipt in its consensus encoding.
    pub receipt_rlp: Vec<u8>,
    /// The inclusion path: an RLP list of trie node byte strings.
    pub trie_proof: Vec<u8>,
    /// Blocks between this one and the head when the proof was built.
    pub confirmations: u64,
}

impl ReceiptInclusion {
    /// The settlement engine's proof for this inclusion.
    ///
    /// The mapping is not free: the engine checks each of these before it looks
    /// at anything else, and a proof that misses one is refused for a reason that
    /// reads like a verification failure.
    ///
    /// - **`tx_hash` is the *receipt's* hash**, `keccak256(receipt_data)`, not the
    ///   transaction hash. The engine's EVM path requires
    ///   `keccak256(receipt_data) == proof.tx_hash` and its comment says the two
    ///   "are the same in Ethereum" — they are not (a receipt and its
    ///   transaction are different objects), so the field carries the receipt
    ///   hash here. The transaction hash is what the *caller* used to find the
    ///   receipt; it is not part of the proof.
    /// - `merkle_proof` carries the two roots the header is matched against, in
    ///   the order the engine reads them: `state_root` first, then the root the
    ///   receipt is walked to (`receipts_root`).
    /// - `chain_height` and `receipt_index` are `Some`: the engine refuses a
    ///   proof that does not state them, because both used to be derivable from
    ///   data the prover chose.
    /// - the two byte vectors are bounded by the pallet, and a receipt wider than
    ///   `MAX_RECEIPT_DATA_SIZE` is an error here rather than a truncated proof.
    pub fn settlement_proof(
        &self,
    ) -> Result<pallet_x3_settlement_engine::SettlementProof, ProducerError> {
        use frame_support::{traits::ConstU32, BoundedVec};
        use pallet_x3_settlement_engine::SettlementProof;
        use sha3::{Digest, Keccak256};

        let receipt_hash: [u8; 32] = Keccak256::digest(&self.receipt_rlp).into();
        let receipt_data =
            BoundedVec::<u8, ConstU32<MAX_RECEIPT_DATA_SIZE>>::try_from(self.receipt_rlp.clone())
                .map_err(|_| ProducerError::ProofTooLarge {
                what: "receipt",
                size: self.receipt_rlp.len(),
                limit: MAX_RECEIPT_DATA_SIZE as usize,
            })?;
        let trie_proof =
            BoundedVec::<u8, ConstU32<MAX_TRIE_PROOF_SIZE>>::try_from(self.trie_proof.clone())
                .map_err(|_| ProducerError::ProofTooLarge {
                    what: "inclusion path",
                    size: self.trie_proof.len(),
                    limit: MAX_TRIE_PROOF_SIZE as usize,
                })?;
        let merkle_proof = BoundedVec::<_, ConstU32<MAX_MERKLE_PROOF_DEPTH>>::try_from(vec![
            sp_core::H256(self.state_root),
            sp_core::H256(self.receipts_root),
        ])
        .map_err(|_| ProducerError::Malformed("two roots always fit".to_string()))?;
        let confirmations = u32::try_from(self.confirmations).map_err(|_| {
            ProducerError::Malformed(format!(
                "confirmations {} do not fit in u32",
                self.confirmations
            ))
        })?;

        Ok(SettlementProof {
            proof_type: pallet_x3_settlement_engine::ProofType::MerkleTrie,
            tx_hash: sp_core::H256(receipt_hash),
            block_hash: sp_core::H256(self.block_hash),
            chain_height: Some(self.block_number),
            confirmations,
            merkle_proof,
            receipt_data,
            receipt_index: Some(self.receipt_index),
            trie_proof: Some(trie_proof),
        })
    }
}

use pallet_x3_settlement_engine::{
    MAX_MERKLE_PROOF_DEPTH, MAX_RECEIPT_DATA_SIZE, MAX_TRIE_PROOF_SIZE,
};

#[derive(Debug, thiserror::Error)]
pub enum ProducerError {
    #[error("rpc {method} failed: {message}")]
    Rpc { method: String, message: String },
    #[error("rpc transport: {0}")]
    Transport(String),
    #[error("{0}")]
    Malformed(String),
    #[error("no transaction receipt for {0}")]
    NoReceipt(String),
    #[error(
        "the trie built from this block's receipts does not have the header's receiptsRoot \
         (header {header:?}, built {built:?}): the block's receipts were not encoded the way \
         the consensus spec says"
    )]
    RootMismatch { header: [u8; 32], built: [u8; 32] },
    #[error("the proof this module built does not verify against the header's root")]
    SelfCheckFailed,
    #[error("the {what} is {size} bytes, past the settlement engine's {limit}")]
    ProofTooLarge {
        what: &'static str,
        size: usize,
        limit: usize,
    },
}

async fn rpc(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: Value,
) -> Result<Value, ProducerError> {
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| ProducerError::Transport(e.to_string()))?;
    let body: Value = response
        .json()
        .await
        .map_err(|e| ProducerError::Transport(e.to_string()))?;
    if let Some(error) = body.get("error").filter(|e| !e.is_null()) {
        return Err(ProducerError::Rpc {
            method: method.to_string(),
            message: error.to_string(),
        });
    }
    body.get("result")
        .cloned()
        .ok_or_else(|| ProducerError::Rpc {
            method: method.to_string(),
            message: "response carried neither result nor error".to_string(),
        })
}

fn hex_bytes(value: &Value, what: &str) -> Result<Vec<u8>, ProducerError> {
    let raw = value
        .as_str()
        .ok_or_else(|| ProducerError::Malformed(format!("{what} is not a hex string")))?;
    hex::decode(raw.strip_prefix("0x").unwrap_or(raw))
        .map_err(|e| ProducerError::Malformed(format!("{what} is not hex: {e}")))
}

fn hex_u64(value: &Value, what: &str) -> Result<u64, ProducerError> {
    let raw = value
        .as_str()
        .ok_or_else(|| ProducerError::Malformed(format!("{what} is not a hex quantity")))?;
    u64::from_str_radix(raw.strip_prefix("0x").unwrap_or(raw), 16)
        .map_err(|e| ProducerError::Malformed(format!("{what} is not hex: {e}")))
}

/// A hex *quantity* (`0x1`, `0x5208`) as the bytes a consensus encoding carries.
///
/// Quantities are not byte strings: they are numbers in hex, so an odd number of
/// digits is normal and `0x0` is `0x0` rather than an empty string. Decoding them
/// as bytes reads `0x1` as an error and `0x01` as two bytes instead of one.
fn hex_quantity_bytes(value: &Value, what: &str) -> Result<Vec<u8>, ProducerError> {
    let raw = value
        .as_str()
        .ok_or_else(|| ProducerError::Malformed(format!("{what} is not a hex quantity")))?;
    let digits = raw.strip_prefix("0x").unwrap_or(raw);
    if digits.is_empty() {
        return Err(ProducerError::Malformed(format!("{what} is empty")));
    }
    let padded = if digits.len() % 2 == 1 {
        format!("0{digits}")
    } else {
        digits.to_string()
    };
    hex::decode(&padded).map_err(|e| ProducerError::Malformed(format!("{what} is not hex: {e}")))
}

fn hex32(value: &Value, what: &str) -> Result<[u8; 32], ProducerError> {
    let bytes = hex_bytes(value, what)?;
    bytes
        .try_into()
        .map_err(|_| ProducerError::Malformed(format!("{what} is not 32 bytes")))
}

/// RLP-encode a quantity the way the consensus spec does: big-endian with
/// leading zeros removed, and zero as the empty string.
fn rlp_quantity(bytes: &[u8]) -> Vec<u8> {
    let first = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    let significant = &bytes[first..];
    let mut stream = rlp::RlpStream::new();
    stream.append(&significant.to_vec());
    stream.out().to_vec()
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut stream = rlp::RlpStream::new_list(items.len());
    for item in items {
        stream.append_raw(item, 1);
    }
    stream.out().to_vec()
}

fn rlp_value(bytes: &[u8]) -> Vec<u8> {
    let mut stream = rlp::RlpStream::new();
    stream.append(&bytes.to_vec());
    stream.out().to_vec()
}

/// The consensus encoding of one `eth_getTransactionReceipt` result.
///
/// `[status, cumulativeGasUsed, logsBloom, logs]` for a legacy receipt, prefixed
/// with its type byte for an EIP-2718 one. The type byte is part of the leaf the
/// trie commits to, so dropping it produces a root that matches nothing.
pub fn consensus_receipt_rlp(receipt: &Value) -> Result<Vec<u8>, ProducerError> {
    let status = receipt
        .get("status")
        .filter(|v| !v.is_null())
        .ok_or_else(|| ProducerError::Malformed("receipt has no status".to_string()))?;
    let logs_bloom = hex_bytes(
        receipt
            .get("logsBloom")
            .ok_or_else(|| ProducerError::Malformed("receipt has no logsBloom".to_string()))?,
        "logsBloom",
    )?;
    if logs_bloom.len() != 256 {
        return Err(ProducerError::Malformed(format!(
            "logsBloom is {} bytes, not 256",
            logs_bloom.len()
        )));
    }

    let mut logs = Vec::new();
    let log_list = receipt
        .get("logs")
        .and_then(Value::as_array)
        .ok_or_else(|| ProducerError::Malformed("receipt has no logs array".to_string()))?;
    for log in log_list {
        let address = hex_bytes(
            log.get("address")
                .ok_or_else(|| ProducerError::Malformed("log has no address".to_string()))?,
            "log address",
        )?;
        if address.len() != 20 {
            return Err(ProducerError::Malformed(format!(
                "log address is {} bytes, not 20",
                address.len()
            )));
        }
        let mut topics = Vec::new();
        for topic in log
            .get("topics")
            .and_then(Value::as_array)
            .ok_or_else(|| ProducerError::Malformed("log has no topics".to_string()))?
        {
            let bytes = hex_bytes(topic, "log topic")?;
            if bytes.len() != 32 {
                return Err(ProducerError::Malformed(format!(
                    "log topic is {} bytes, not 32",
                    bytes.len()
                )));
            }
            topics.push(rlp_value(&bytes));
        }
        let data = hex_bytes(
            log.get("data")
                .ok_or_else(|| ProducerError::Malformed("log has no data".to_string()))?,
            "log data",
        )?;
        logs.push(rlp_list(&[
            rlp_value(&address),
            rlp_list(&topics),
            rlp_value(&data),
        ]));
    }

    let cumulative = hex_quantity_bytes(
        receipt.get("cumulativeGasUsed").ok_or_else(|| {
            ProducerError::Malformed("receipt has no cumulativeGasUsed".to_string())
        })?,
        "cumulativeGasUsed",
    )?;
    let payload = rlp_list(&[
        rlp_quantity(&hex_quantity_bytes(status, "status")?),
        rlp_quantity(&cumulative),
        rlp_value(&logs_bloom),
        rlp_list(&logs),
    ]);

    let type_byte = receipt
        .get("type")
        .filter(|v| !v.is_null())
        .map(|v| hex_u64(v, "type"))
        .transpose()?
        .unwrap_or(0);
    if type_byte == 0 {
        Ok(payload)
    } else if type_byte <= u8::MAX as u64 {
        let mut typed = vec![type_byte as u8];
        typed.extend_from_slice(&payload);
        Ok(typed)
    } else {
        Err(ProducerError::Malformed(format!(
            "receipt type {type_byte} is not one byte"
        )))
    }
}

/// Build the inclusion proof for `tx_hash` against the block that mined it.
pub async fn prove_evm_receipt(
    rpc_url: &str,
    tx_hash: &str,
) -> Result<ReceiptInclusion, ProducerError> {
    let client = reqwest::Client::new();
    let receipt = rpc(
        &client,
        rpc_url,
        "eth_getTransactionReceipt",
        json!([tx_hash]),
    )
    .await?;
    if receipt.is_null() {
        return Err(ProducerError::NoReceipt(tx_hash.to_string()));
    }

    let block_number = hex_u64(
        receipt
            .get("blockNumber")
            .ok_or_else(|| ProducerError::Malformed("receipt has no blockNumber".to_string()))?,
        "blockNumber",
    )?;
    let block_hash = hex32(
        receipt
            .get("blockHash")
            .ok_or_else(|| ProducerError::Malformed("receipt has no blockHash".to_string()))?,
        "blockHash",
    )?;
    let receipt_index = hex_u64(
        receipt.get("transactionIndex").ok_or_else(|| {
            ProducerError::Malformed("receipt has no transactionIndex".to_string())
        })?,
        "transactionIndex",
    )?;

    // Every receipt in the block, in block order: the trie is over all of them.
    let block_tag = format!("{block_number:#x}");
    let all = match rpc(&client, rpc_url, "eth_getBlockReceipts", json!([block_tag])).await {
        Ok(receipts) if receipts.is_array() => receipts.as_array().cloned().unwrap_or_default(),
        _ => {
            // Older nodes have no `eth_getBlockReceipts`: walk the block's
            // transactions instead.
            let block = rpc(
                &client,
                rpc_url,
                "eth_getBlockByNumber",
                json!([block_tag, true]),
            )
            .await?;
            let mut receipts = Vec::new();
            for transaction in block
                .get("transactions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                let hash = transaction
                    .get("hash")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ProducerError::Malformed("a transaction has no hash".to_string())
                    })?
                    .to_string();
                receipts
                    .push(rpc(&client, rpc_url, "eth_getTransactionReceipt", json!([hash])).await?);
            }
            receipts
        }
    };
    if all.is_empty() {
        return Err(ProducerError::Malformed(format!(
            "block {block_number} reported no receipts"
        )));
    }

    let mut receipts = Vec::with_capacity(all.len());
    for entry in &all {
        receipts.push(consensus_receipt_rlp(entry)?);
    }
    let index = usize::try_from(receipt_index).map_err(|_| {
        ProducerError::Malformed(format!("receipt index {receipt_index} is out of range"))
    })?;
    let receipt_rlp = receipts.get(index).cloned().ok_or_else(|| {
        ProducerError::Malformed(format!(
            "the block listed {} receipts, so index {index} does not exist",
            receipts.len()
        ))
    })?;

    // The root the header carries, and the root these encodings produce. If they
    // differ, this module's encoding is wrong and every proof it built would be
    // refused by the verifier for a reason that looks like a verification bug.
    let header = rpc(
        &client,
        rpc_url,
        "eth_getBlockByNumber",
        json!([block_tag, false]),
    )
    .await?;
    let header_receipts_root = hex32(
        header
            .get("receiptsRoot")
            .ok_or_else(|| ProducerError::Malformed("header has no receiptsRoot".to_string()))?,
        "receiptsRoot",
    )?;
    let state_root = hex32(
        header
            .get("stateRoot")
            .ok_or_else(|| ProducerError::Malformed("header has no stateRoot".to_string()))?,
        "stateRoot",
    )?;
    let built_root = receipts_trie_root(&receipts)
        .map_err(|e| ProducerError::Malformed(format!("could not build the receipts trie: {e}")))?;
    if built_root != header_receipts_root {
        return Err(ProducerError::RootMismatch {
            header: header_receipts_root,
            built: built_root,
        });
    }

    let trie_proof = receipts_trie_proof(&receipts, index).map_err(|e| {
        ProducerError::Malformed(format!("could not build the inclusion path: {e}"))
    })?;
    // The producer and the verifier are two callers of one convention; check that
    // here rather than discovering it at settlement.
    verify_merkle_patricia_proof(
        &header_receipts_root,
        &receipt_trie_key(receipt_index),
        Some(&receipt_rlp),
        &trie_proof,
    )
    .map_err(|_| ProducerError::SelfCheckFailed)?;

    let head = rpc(&client, rpc_url, "eth_blockNumber", json!([])).await?;
    let head = hex_u64(&head, "blockNumber")?;

    Ok(ReceiptInclusion {
        block_number,
        block_hash,
        state_root,
        receipts_root: header_receipts_root,
        receipt_index: receipt_index as u32,
        receipt_rlp,
        trie_proof,
        confirmations: head.saturating_sub(block_number),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    /// Anvil's first two default accounts. `eth_sendTransaction` from the first
    /// keeps this test independent of any signer in this crate.
    const FROM: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    const TO: &str = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";

    struct AnvilGuard(Child);

    impl Drop for AnvilGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    /// Each test gets its own port: the suite runs tests in parallel, and two
    /// anvils cannot share one.
    fn endpoint(port: u16) -> (AnvilGuard, String) {
        let child = Command::new("anvil")
            .args([
                "--port",
                &port.to_string(),
                "--chain-id",
                "31337",
                "--silent",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect(
                "anvil (foundry) is required: this test proves a receipt in a real block. \
                 The `EVM contract lifecycle` gate requires the same tool.",
            );
        (AnvilGuard(child), format!("http://127.0.0.1:{port}"))
    }

    async fn wait_for_rpc(url: &str) {
        let client = reqwest::Client::new();
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            if rpc(&client, url, "eth_chainId", json!([])).await.is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(250));
        }
        panic!("anvil did not answer on {url} within 30s");
    }

    async fn send_and_mine(url: &str, typed: bool) -> String {
        let client = reqwest::Client::new();
        let mut transaction = json!({ "from": FROM, "to": TO, "value": "0x1" });
        // Stated rather than left to the node's default: anvil defaults to
        // EIP-1559, so "no type" is not a legacy transaction.
        transaction["type"] = if typed { json!("0x2") } else { json!("0x0") };
        let hash = rpc(&client, url, "eth_sendTransaction", json!([transaction]))
            .await
            .expect("anvil mines the transaction");
        let hash = hash.as_str().expect("a transaction hash").to_string();

        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let receipt = rpc(
                &client,
                url,
                "eth_getTransactionReceipt",
                json!([hash.clone()]),
            )
            .await
            .expect("receipt query");
            if !receipt.is_null() {
                return hash;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("the transaction was never mined");
    }

    #[tokio::test]
    async fn a_receipt_proves_inclusion_in_its_blocks_trie() {
        let (_anvil, url) = endpoint(18960);
        wait_for_rpc(&url).await;
        // A second transaction in the same window makes the block ordering
        // meaningful rather than accidental.
        send_and_mine(&url, false).await;
        let hash = send_and_mine(&url, false).await;

        let inclusion = prove_evm_receipt(&url, &hash)
            .await
            .expect("the producer builds a proof");
        assert_ne!(inclusion.receipts_root, [0u8; 32], "the header's root");
        assert_ne!(inclusion.trie_proof, Vec::<u8>::new(), "the path");
        assert_eq!(inclusion.confirmations, 0, "the transaction is in the head");
        // The producer already checks both of these internally; asserting them
        // here keeps the test honest about what it is relying on.
        assert_eq!(
            verify_merkle_patricia_proof(
                &inclusion.receipts_root,
                &receipt_trie_key(inclusion.receipt_index as u64),
                Some(&inclusion.receipt_rlp),
                &inclusion.trie_proof,
            ),
            Ok(())
        );

        // And it adapts into the settlement engine's proof, on real bytes.
        let proof = inclusion
            .settlement_proof()
            .expect("a real inclusion adapts");
        assert_eq!(
            proof.tx_hash.0,
            <sha3::Keccak256 as sha3::Digest>::digest(&inclusion.receipt_rlp).as_slice(),
            "the engine's tx_hash check is over the receipt"
        );
        assert_eq!(proof.chain_height, Some(inclusion.block_number));
        assert_eq!(proof.receipt_index, Some(inclusion.receipt_index));
        assert_eq!(
            proof.merkle_proof.as_slice(),
            [
                sp_core::H256(inclusion.state_root),
                sp_core::H256(inclusion.receipts_root)
            ]
            .as_slice(),
            "state root first, then the root the receipt is walked to"
        );
    }

    fn receipt_json(kind: Option<&str>) -> Value {
        let mut receipt = json!({
            "status": "0x1",
            "cumulativeGasUsed": "0x5208",
            "logsBloom": format!("0x{}", "00".repeat(256)),
            "logs": [],
        });
        if let Some(kind) = kind {
            receipt["type"] = json!(kind);
        }
        receipt
    }

    #[test]
    fn a_legacy_receipt_is_encoded_as_a_bare_rlp_list() {
        // Anvil reports every receipt as typed, so the legacy shape is checked
        // here rather than on a chain: no type byte, a four-element list, the
        // status and gas-used as quantities, the bloom whole and the logs empty.
        let encoded = consensus_receipt_rlp(&receipt_json(None)).expect("legacy receipt");
        assert!(
            encoded[0] >= 0xc0,
            "a legacy receipt is a bare RLP list, got first byte {:#x}",
            encoded[0]
        );

        let list = rlp::Rlp::new(&encoded);
        assert_eq!(list.item_count(), Ok(4));
        assert_eq!(list.at(0).and_then(|v| v.as_val::<u8>()), Ok(1), "status");
        assert_eq!(
            list.at(1).and_then(|v| v.as_val::<u64>()),
            Ok(0x5208),
            "cumulativeGasUsed is a quantity, not four bytes of hex text"
        );
        assert_eq!(
            list.at(2)
                .and_then(|v| v.as_val::<Vec<u8>>())
                .map(|b| b.len()),
            Ok(256),
            "the bloom is carried whole"
        );
        assert_eq!(list.at(3).and_then(|v| v.item_count()), Ok(0), "no logs");
    }

    #[test]
    fn a_typed_receipt_is_prefixed_with_its_type_byte() {
        for (kind, byte) in [("0x1", 0x01u8), ("0x2", 0x02), ("0x3", 0x03)] {
            let encoded = consensus_receipt_rlp(&receipt_json(Some(kind))).expect(kind);
            assert_eq!(encoded[0], byte, "{kind} keeps its type byte");
            // And what follows is the same four-element receipt the legacy form
            // carries — the prefix is the only difference.
            let list = rlp::Rlp::new(&encoded[1..]);
            assert_eq!(list.item_count(), Ok(4));
            assert_eq!(list.at(0).and_then(|v| v.as_val::<u8>()), Ok(1));
        }
    }

    fn inclusion(receipt_rlp: Vec<u8>, trie_proof_len: usize) -> ReceiptInclusion {
        ReceiptInclusion {
            block_number: 100,
            block_hash: [3u8; 32],
            state_root: [2u8; 32],
            receipts_root: [4u8; 32],
            receipt_index: 1,
            receipt_rlp,
            trie_proof: vec![0xAB; trie_proof_len],
            confirmations: 12,
        }
    }

    #[test]
    fn the_adapters_proof_satisfies_every_check_the_engine_makes() {
        // The settlement engine's EVM path checks these before the walk, and each
        // one has a reason to fail loudly rather than look like a bad proof:
        let inclusion = inclusion(vec![0xc3, 0x01, 0x02, 0xc0], 64);
        let proof = inclusion.settlement_proof().expect("adapts");

        // `proof_type` must be one the EVM arm accepts.
        assert!(matches!(
            proof.proof_type,
            pallet_x3_settlement_engine::ProofType::MerkleTrie
        ));
        // `receipt_data` must be non-empty and structurally a receipt: a list
        // prefix, or an EIP-2718 type byte followed by one.
        assert!(!proof.receipt_data.is_empty());
        assert!(proof.receipt_data[0] >= 0xc0);
        // `tx_hash` must be the receipt's own hash — the field name is the
        // engine's, the value is keccak over the receipt bytes.
        assert_eq!(
            proof.tx_hash.0,
            <sha3::Keccak256 as sha3::Digest>::digest(&inclusion.receipt_rlp).as_slice()
        );
        // Confirmations must be at least one, and the height and index must be
        // stated: the engine refuses a proof that leaves either as `None`.
        assert!(proof.confirmations >= 1);
        assert!(proof.chain_height.is_some());
        assert!(proof.receipt_index.is_some());
        // Both roots, in the order the engine reads them.
        assert_eq!(proof.merkle_proof.len(), 2);
        assert_eq!(proof.merkle_proof[0], sp_core::H256([2u8; 32]));
        assert_eq!(proof.merkle_proof[1], sp_core::H256([4u8; 32]));
        // And the inclusion path travels with it.
        assert_eq!(proof.trie_proof.as_ref().map(|path| path.len()), Some(64));
    }

    #[test]
    fn a_receipt_wider_than_the_engine_accepts_is_an_error_not_a_truncation() {
        let too_wide = vec![0xc0; pallet_x3_settlement_engine::MAX_RECEIPT_DATA_SIZE as usize + 1];
        let error = inclusion(too_wide, 64)
            .settlement_proof()
            .expect_err("past the bound");
        assert!(matches!(
            error,
            ProducerError::ProofTooLarge {
                what: "receipt",
                ..
            }
        ));

        let error = inclusion(
            vec![0xc3, 0x01, 0x02, 0xc0],
            pallet_x3_settlement_engine::MAX_TRIE_PROOF_SIZE as usize + 1,
        )
        .settlement_proof()
        .expect_err("past the bound");
        assert!(matches!(
            error,
            ProducerError::ProofTooLarge {
                what: "inclusion path",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn a_typed_receipt_is_encoded_with_its_type_byte() {
        // EIP-1559: the trie leaf is `0x02 || rlp(payload)`. Dropping the type
        // byte produces a root that matches nothing, so this is the case that
        // catches an encoder that only knows the legacy shape.
        let (_anvil, url) = endpoint(18961);
        wait_for_rpc(&url).await;
        let hash = send_and_mine(&url, true).await;

        let inclusion = prove_evm_receipt(&url, &hash)
            .await
            .expect("the producer builds a proof");
        assert_eq!(
            inclusion.receipt_rlp[0], 0x02,
            "a type-2 receipt keeps its type byte in the trie leaf"
        );
    }

    #[tokio::test]
    async fn a_transaction_the_chain_does_not_know_has_no_proof() {
        let (_anvil, url) = endpoint(18962);
        wait_for_rpc(&url).await;
        let error = prove_evm_receipt(&url, &format!("0x{}", "11".repeat(32)))
            .await
            .expect_err("nothing to prove");
        assert!(matches!(error, ProducerError::NoReceipt(_)), "{error:?}");
    }
}
