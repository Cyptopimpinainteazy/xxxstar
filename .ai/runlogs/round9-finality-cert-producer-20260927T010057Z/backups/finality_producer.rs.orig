//! # Building a [`FinalityCertificate`] from real EVM chain data
//!
//! The certificate type is a checked *shape*: it cannot invent depth for an anchor, but on its own
//! it does not prove that its `block_hash` is the block at `block_height` on `chain`. Someone has
//! to read the chain and bind the two together. This module is that reader for EVM chains.
//!
//! The reader does three things the certificate cannot do for itself:
//!
//! 1. **Binds the receipt to the chain.** `eth_chainId` must equal the chain id the caller expects.
//!    A receipt is evidence only about the chain that produced it, so a foreign chain's receipt is
//!    refused *for the chain* (`FinalityChainIdMismatch`), before any depth is considered.
//! 2. **Binds the hash to the height.** The receipt names a `blockNumber` and a `blockHash`; the
//!    node's block at that height must carry the same hash, or the receipt does not describe this
//!    chain at that height. A mismatch is refused (`FinalityBlockHashMismatch`), never repaired.
//! 3. **Takes the tip from the chain, not the caller.** `observed_at` is `eth_blockNumber` at read
//!    time, so the derived depth cannot be chosen by whoever is being paid out.
//!
//! Only compiled with the `std` feature, because it performs HTTP through [`RpcClient`].

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use std::path::PathBuf;

use serde_json::Value;

use crate::error::SwapError;
use crate::finality::{FinalityCertificate, FinalityTipRecord, FinalityTipStore};
use crate::intent::ChainKind;
use crate::rpc_client::RpcClient;

/// The EVM JSON-RPC reads a certificate producer needs.
///
/// Keeping this behind a trait lets the binding logic be exercised against RPC-shaped data in
/// tests while the live path uses the real [`RpcClient`]; the checks themselves are the same code.
pub trait EvmChainReader {
    /// `eth_chainId` — the chain id the node answers for.
    fn chain_id(&mut self) -> Result<u64, SwapError>;

    /// `eth_blockNumber` — the node's tip at the moment it is asked.
    fn tip_height(&mut self) -> Result<u64, SwapError>;

    /// `eth_getTransactionReceipt` — `None` when the transaction is not mined here.
    fn transaction_receipt(&mut self, tx_hash: &str) -> Result<Option<Value>, SwapError>;

    /// `eth_getBlockByNumber` — the block the node reports at `height`.
    fn block_by_number(&mut self, height: u64) -> Result<Value, SwapError>;
}

impl EvmChainReader for RpcClient {
    fn chain_id(&mut self) -> Result<u64, SwapError> {
        RpcClient::chain_id(self)
    }

    fn tip_height(&mut self) -> Result<u64, SwapError> {
        self.get_block_number()
    }

    fn transaction_receipt(&mut self, tx_hash: &str) -> Result<Option<Value>, SwapError> {
        self.get_transaction_receipt(tx_hash)
    }

    fn block_by_number(&mut self, height: u64) -> Result<Value, SwapError> {
        self.get_block_by_number(height, false)
    }
}

/// Builds [`FinalityCertificate`]s from the data a live EVM node reports.
pub struct EvmFinalityProducer<R: EvmChainReader> {
    reader: R,
    chain: ChainKind,
    expected_chain_id: u64,
}

impl<R: EvmChainReader> EvmFinalityProducer<R> {
    /// Bind a producer to a reader, the chain its certificates name, and the chain id it requires.
    pub fn new(reader: R, chain: ChainKind, expected_chain_id: u64) -> Self {
        Self {
            reader,
            chain,
            expected_chain_id,
        }
    }

    /// The chain this producer builds certificates for.
    pub fn chain(&self) -> ChainKind {
        self.chain
    }

    /// Read the chain and build a certificate for `tx_hash`, or refuse.
    ///
    /// Every value in the certificate comes from the node, not from the caller: the height and
    /// hash from the receipt (checked against the block at that height), and `observed_at` from
    /// the node's tip. The only caller-chosen inputs are *which* transaction to look at and which
    /// chain the reader must be.
    pub fn observe(&mut self, tx_hash: &str) -> Result<FinalityCertificate, SwapError> {
        // (1) Chain identity first. A receipt from another chain must be refused for the chain, so
        // it cannot be graded against this chain's depth rule and accepted as shallow-but-valid.
        let found_chain_id = self.reader.chain_id()?;
        if found_chain_id != self.expected_chain_id {
            return Err(SwapError::FinalityChainIdMismatch {
                expected: self.expected_chain_id,
                found: found_chain_id,
            });
        }

        // (2) The receipt is the chain's own statement of where the transaction landed.
        let receipt =
            self.reader
                .transaction_receipt(tx_hash)?
                .ok_or_else(|| SwapError::TxNotFound {
                    tx_hash: tx_hash.to_string(),
                })?;
        let block_height = hex_field_u64(&receipt, "blockNumber")?;
        let receipt_block_hash = hex_field_bytes32(&receipt, "blockHash")?;

        // (3) Bind the hash to the height: the block the node reports at that height must be the
        // receipt's block. A hash the chain does not carry at that height is refused, not repaired.
        let block = self.reader.block_by_number(block_height)?;
        let block_number = hex_field_u64(&block, "number")?;
        if block_number != block_height {
            return Err(SwapError::FinalityBlockNumberMismatch {
                receipt_number: block_height,
                block_number,
            });
        }
        let chain_block_hash = hex_field_bytes32(&block, "hash")?;
        if chain_block_hash != receipt_block_hash {
            return Err(SwapError::FinalityBlockHashMismatch {
                block_height,
                receipt_hash: to_0x_hex(&receipt_block_hash),
                block_hash: to_0x_hex(&chain_block_hash),
            });
        }

        // (4) The tip is the chain's, read now, so the depth is derived and not chosen.
        let observed_at = self.reader.tip_height()?;

        let tx_id = parse_hex32("transaction hash", tx_hash)?;
        FinalityCertificate::observe(
            self.chain,
            block_height,
            chain_block_hash,
            tx_id,
            observed_at,
        )
    }
}

/// A [`FinalityTipStore`] that persists the oracle's tips to a JSON file.
///
/// This is the process-side implementation for hosts that have a filesystem; the trait itself stays
/// in the library so a different host can store the same tips wherever it already keeps state.
#[derive(Debug, Clone)]
pub struct FileFinalityTipStore {
    path: PathBuf,
}

impl FileFinalityTipStore {
    /// Persist tips at `path`.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The file-backed store's path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn store_error(&self, detail: impl core::fmt::Display) -> SwapError {
        SwapError::FinalityTipStore(format!("{}: {}", self.path.display(), detail))
    }
}

impl FinalityTipStore for FileFinalityTipStore {
    fn load_tips(&self) -> Result<BTreeMap<ChainKind, FinalityTipRecord>, SwapError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => {
                let entries: alloc::vec::Vec<(ChainKind, FinalityTipRecord)> =
                    serde_json::from_str(&text).map_err(|e| self.store_error(e))?;
                Ok(entries.into_iter().collect())
            }
            // No file yet means nothing has been remembered; that is not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(self.store_error(e)),
        }
    }

    fn store_tips(&self, tips: &BTreeMap<ChainKind, FinalityTipRecord>) -> Result<(), SwapError> {
        let entries: alloc::vec::Vec<(&ChainKind, &FinalityTipRecord)> = tips.iter().collect();
        let text = serde_json::to_string(&entries).map_err(|e| self.store_error(e))?;

        // Write to a sibling temp file and rename over the target, so a crash mid-write cannot
        // leave a truncated ledger that reads as "nothing remembered" on the next start.
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| self.store_error(e))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| self.store_error(e))?;
        Ok(())
    }
}

fn hex_field_u64(value: &Value, field: &str) -> Result<u64, SwapError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| SwapError::RpcError(format!("chain data is missing {}", field)))?;
    u64::from_str_radix(text.trim_start_matches("0x"), 16).map_err(|_| {
        SwapError::RpcError(format!("chain data field {} is not a block number", field))
    })
}

fn hex_field_bytes32(value: &Value, field: &str) -> Result<[u8; 32], SwapError> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| SwapError::RpcError(format!("chain data is missing {}", field)))?;
    parse_hex32(field, text)
}

fn parse_hex32(name: &str, text: &str) -> Result<[u8; 32], SwapError> {
    let stripped = text.strip_prefix("0x").unwrap_or(text);
    let bytes = hex::decode(stripped)
        .map_err(|_| SwapError::RpcError(format!("{} is not valid hex", name)))?;
    if bytes.len() != 32 {
        return Err(SwapError::RpcError(format!(
            "{} must be 32 bytes, got {}",
            name,
            bytes.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn to_0x_hex(bytes: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TX: &str = "0x2afc93e1da9618ed343bae749559e36a9b6facc30ea6056984684239246aa301";
    const HASH_A: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";
    const HASH_B: &str = "0x2222222222222222222222222222222222222222222222222222222222222222";

    /// A reader that answers with fixed RPC-shaped data, so the binding checks can be driven with
    /// the same JSON an anvil/reth node produces.
    struct FakeChain {
        chain_id: u64,
        tip: u64,
        receipt: Option<Value>,
        block: Value,
    }

    impl FakeChain {
        fn new(chain_id: u64, tip: u64, receipt: Option<Value>, block: Value) -> Self {
            Self {
                chain_id,
                tip,
                receipt,
                block,
            }
        }
    }

    impl EvmChainReader for FakeChain {
        fn chain_id(&mut self) -> Result<u64, SwapError> {
            Ok(self.chain_id)
        }
        fn tip_height(&mut self) -> Result<u64, SwapError> {
            Ok(self.tip)
        }
        fn transaction_receipt(&mut self, _tx_hash: &str) -> Result<Option<Value>, SwapError> {
            Ok(self.receipt.clone())
        }
        fn block_by_number(&mut self, _height: u64) -> Result<Value, SwapError> {
            Ok(self.block.clone())
        }
    }

    fn receipt(number: u64, hash: &str) -> Value {
        json!({
            "transactionHash": TX,
            "blockNumber": format!("0x{:x}", number),
            "blockHash": hash,
            "status": "0x1",
        })
    }

    fn block(number: u64, hash: &str) -> Value {
        json!({
            "number": format!("0x{:x}", number),
            "hash": hash,
        })
    }

    fn producer(chain: FakeChain, expected_chain_id: u64) -> EvmFinalityProducer<FakeChain> {
        EvmFinalityProducer::new(chain, ChainKind::Ethereum, expected_chain_id)
    }

    /// Golden path: the anchor, hash and tip all come from the node, and depth is derived.
    #[test]
    fn test_a_receipt_bound_to_its_block_builds_a_certificate() {
        let chain = FakeChain::new(31337, 21, Some(receipt(10, HASH_A)), block(10, HASH_A));
        let mut producer = producer(chain, 31337);

        let cert = producer.observe(TX).expect("bound receipt builds");
        assert_eq!(cert.chain(), ChainKind::Ethereum);
        assert_eq!(cert.block_height(), 10);
        assert_eq!(cert.block_hash(), &parse_hex32("h", HASH_A).unwrap());
        assert_eq!(cert.observed_at(), 21);
        assert_eq!(cert.confirmations(), 12, "derived from the chain's tip");
    }

    /// A receipt whose hash is not the block at its own height is refused, not repaired.
    #[test]
    fn test_a_receipt_hash_that_is_not_the_block_at_that_height_is_refused() {
        let chain = FakeChain::new(31337, 21, Some(receipt(10, HASH_A)), block(10, HASH_B));
        let mut producer = producer(chain, 31337);

        match producer.observe(TX) {
            Err(SwapError::FinalityBlockHashMismatch {
                block_height,
                receipt_hash,
                block_hash,
            }) => {
                assert_eq!(block_height, 10);
                assert_eq!(receipt_hash, HASH_A);
                assert_eq!(block_hash, HASH_B);
            }
            other => panic!("expected a block-hash mismatch, got {:?}", other),
        }
    }

    /// A receipt from a different chain is refused for the chain, before depth is considered.
    #[test]
    fn test_a_receipt_from_another_chain_is_refused_for_the_chain() {
        // Everything else is a perfectly valid, deep anchor — only the chain id is wrong.
        let chain = FakeChain::new(5, 21, Some(receipt(10, HASH_A)), block(10, HASH_A));
        let mut producer = producer(chain, 31337);

        match producer.observe(TX) {
            Err(SwapError::FinalityChainIdMismatch { expected, found }) => {
                assert_eq!(expected, 31337);
                assert_eq!(found, 5);
            }
            other => panic!("expected a chain-id mismatch, got {:?}", other),
        }
    }

    /// A node that answers a height with a different block number is refused.
    #[test]
    fn test_a_block_reported_at_the_wrong_height_is_refused() {
        let chain = FakeChain::new(31337, 21, Some(receipt(10, HASH_A)), block(11, HASH_A));
        let mut producer = producer(chain, 31337);

        assert_eq!(
            producer.observe(TX),
            Err(SwapError::FinalityBlockNumberMismatch {
                receipt_number: 10,
                block_number: 11,
            })
        );
    }

    /// An unmined transaction cannot become a certificate.
    #[test]
    fn test_an_unmined_transaction_is_refused() {
        let chain = FakeChain::new(31337, 21, None, block(10, HASH_A));
        let mut producer = producer(chain, 31337);

        assert_eq!(
            producer.observe(TX),
            Err(SwapError::TxNotFound {
                tx_hash: TX.to_string(),
            })
        );
    }

    /// A chain that has not yet reached the anchor's height cannot observe it.
    #[test]
    fn test_a_tip_below_the_anchor_is_refused() {
        let chain = FakeChain::new(31337, 9, Some(receipt(10, HASH_A)), block(10, HASH_A));
        let mut producer = producer(chain, 31337);

        assert_eq!(
            producer.observe(TX),
            Err(SwapError::CertificateBlockAfterObservation {
                block_height: 10,
                observed_at: 9,
            })
        );
    }
}
