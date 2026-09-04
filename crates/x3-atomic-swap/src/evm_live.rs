//! # Live EVM HTLC executor (real on-chain path)
//!
//! Bridges the simulated `EvmHtlcContract`/`EvmAdapter` to a real EVM chain by
//! building, signing, and broadcasting HTLC transactions against the on-chain
//! `AtlasHTLC` contract, then reading back the produced transaction hash and
//! mined block from the JSON-RPC endpoint.
//!
//! Only compiled with the `std` feature (needs `k256`, `rlp`, `ureq`, `sha3`,
//! and `rand`). Without `std`, X3 is `no_std` and cannot sign/broadcast.
//!
//! Callers that only exercise simulation (unit tests, dry runs) never touch
//! this module; callers that construct a [`LiveEvmExecutor`] get genuine tx
//! hashes and receipt data — never a fabricated mock id.

use crate::adapter::{ClaimProof, LockProof, RefundProof, TxId, VmType};
use crate::error::SwapError;
use crate::ethereum_tx::Transaction;
use crate::rpc_client::RpcClient;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use hex::ToHex;
#[cfg(test)]
use sha3::{Digest, Keccak256};

/// 32-byte ABI word helpers.
fn word(v: &[u8; 32]) -> Vec<u8> {
    v.to_vec()
}

/// ABI-encode a 20-byte address into a 32-byte word (right-padded).
fn abi_address(a: &[u8; 20]) -> Vec<u8> {
    let mut out = vec![0u8; 12];
    out.extend_from_slice(a);
    out
}

/// ABI-encode a u256 value into a 32-byte word (value as u64 for our use).
fn abi_u256(v: u128) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    out[16..].copy_from_slice(&v.to_be_bytes());
    out
}

/// ABI-encode a uint256 from u64.
fn abi_u64(v: u64) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    out[24..].copy_from_slice(&v.to_be_bytes());
    out
}

/// The real `AtlasHTLC` selectors, taken from the compiled artifact
/// (`X3-contracts/evm/out/AtlasHTLC.sol/AtlasHTLC.json` methodIdentifiers), NOT
/// the stale selector comments in the .sol source.
mod selector {
    /// createHTLC(address,bytes32,uint256,address,uint256) => 0x502e9fd5
    pub const CREATE: [u8; 4] = [0x50, 0x2e, 0x9f, 0xd5];
    /// claimHTLC(bytes32,bytes32) => 0x9755dca0
    pub const CLAIM: [u8; 4] = [0x97, 0x55, 0xdc, 0xa0];
    /// refundHTLC(bytes32) => 0x43b920c5
    pub const REFUND: [u8; 4] = [0x43, 0xb9, 0x20, 0xc5];
}

/// Hex-encode bytes with a `0x` prefix.
fn to_0x_hex(bytes: &[u8]) -> String {
    format!("0x{}", bytes.encode_hex::<String>())
}

/// A live EVM HTLC executor bound to a specific chain endpoint and the
/// deployed `AtlasHTLC` contract.
pub struct LiveEvmExecutor {
    rpc: RpcClient,
    contract: [u8; 20],
    /// Private key hex (64 hex chars, no 0x prefix) used to sign broadcasts.
    signer_private_key: String,
    /// 20-byte address derived from the signing key.
    signer_addr: [u8; 20],
}

/// Parse a `0x…` or bare hex into 20 bytes.
fn parse_address_20(s: &str) -> Result<[u8; 20], SwapError> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes =
        hex::decode(s).map_err(|e| SwapError::Internal(format!("bad address hex: {}", e)))?;
    let mut out = [0u8; 20];
    if bytes.len() != 20 {
        return Err(SwapError::Internal("address must be 20 bytes".into()));
    }
    out.copy_from_slice(&bytes);
    Ok(out)
}

impl LiveEvmExecutor {
    /// Construct a live executor.
    ///
    /// - `rpc_url`: EVM JSON-RPC endpoint (e.g. `https://sepolia.base.org`).
    /// - `chain_id`: EIP-155 chain id of the target network.
    /// - `contract`: deployed `AtlasHTLC` address (20 bytes).
    /// - `signer_private_key`: hex private key (64 chars, may include `0x`).
    pub fn new(
        rpc_url: &str,
        chain_id: u64,
        contract: [u8; 20],
        signer_private_key: &str,
    ) -> Result<Self, SwapError> {
        let rpc = RpcClient::new(rpc_url.to_string(), chain_id);
        let signer_private_key = signer_private_key.trim_start_matches("0x").to_string();
        if signer_private_key.len() != 64 || !signer_private_key.is_ascii() {
            return Err(SwapError::Internal(
                "signer private key must be 64 hex characters".into(),
            ));
        }
        hex::decode(&signer_private_key)
            .map_err(|_| SwapError::Internal("signer private key is not valid hex".into()))?;
        let signer_addr =
            parse_address_20(&Transaction::address_from_private_key(&signer_private_key)?)?;
        Ok(Self {
            rpc,
            contract,
            signer_private_key,
            signer_addr,
        })
    }

    /// Address (0x hex) derived from the configured signing key.
    pub fn signer_address(&self) -> Result<String, SwapError> {
        Transaction::address_from_private_key(&self.signer_private_key)
    }

    /// Poll `eth_getTransactionReceipt` until a receipt appears or `timeout_ms`
    /// elapses.
    fn wait_for_receipt(&mut self, tx_hash: &str, timeout_ms: u64) -> Result<u64, SwapError> {
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(timeout_ms))
            .ok_or_else(|| SwapError::Internal("timer overflow".into()))?;
        loop {
            if self
                .rpc
                .get_transaction_receipt(tx_hash)
                .map_err(|e| SwapError::RpcError(e.to_string()))?
                .is_some()
            {
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err(SwapError::TxNotFound {
                    tx_hash: tx_hash.to_string(),
                });
            }
            std::thread::sleep(std::time::Duration::from_millis(1_500));
        }
        // Second read returns block number from receipt.
        let receipt = self
            .rpc
            .get_transaction_receipt(tx_hash)
            .map_err(|e| SwapError::RpcError(e.to_string()))?;
        let block = receipt
            .and_then(|r| r["blockNumber"].as_str().map(|s| s.to_string()))
            .and_then(|hex| u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok())
            .unwrap_or_default();
        Ok(block)
    }

    /// Broadcast a signed raw transaction and wait for confirmation.
    fn send_and_confirm(
        &mut self,
        tx: Transaction,
        timeout_ms: u64,
    ) -> Result<(String, u64), SwapError> {
        let signed = tx.sign(&self.signer_private_key)?;
        let tx_hash = self.rpc.send_raw_transaction(&signed)?;
        let block = self.wait_for_receipt(&tx_hash, timeout_ms)?;
        Ok((tx_hash, block))
    }

    /// Convenience: current gas price over 2 (never exceeds worst-case).
    fn gas_price_wei(&mut self) -> Result<u128, SwapError> {
        let gp = self
            .rpc
            .gas_price()
            .map_err(|e| SwapError::RpcError(e.to_string()))?;
        Ok(gp.saturating_div(2).max(1))
    }

    /// Nonce for the signer.
    fn next_nonce(&mut self) -> Result<u64, SwapError> {
        let addr = self.signer_address()?;
        self.rpc
            .get_transaction_count(&addr, "latest")
            .map_err(|e| SwapError::RpcError(e.to_string()))
    }

    /// Sign and broadcast a `createHTLC` transaction.
    ///
    /// Returns the real on-chain transaction hash.
    pub fn create_lock(
        &mut self,
        recipient: [u8; 20],
        hashlock: [u8; 32],
        timelock: u64,
        token: [u8; 20], // zero = native
        amount: u128,
        timeout_ms: u64,
    ) -> Result<String, SwapError> {
        // calldata: selector + recipient + hashLock + timeLock + token + amount
        let mut data = Vec::with_capacity(4 + 20 * 32);
        data.extend_from_slice(&selector::CREATE);
        data.extend_from_slice(&abi_address(&recipient));
        data.extend_from_slice(&word(&hashlock));
        data.extend_from_slice(&abi_u64(timelock));
        data.extend_from_slice(&abi_address(&token));
        data.extend_from_slice(&abi_u256(amount));

        let nonce = self.next_nonce()?;
        let gas_price = self.gas_price_wei()?;
        let chain_id = self
            .rpc
            .chain_id()
            .map_err(|e| SwapError::RpcError(e.to_string()))?;

        let gas_limit = self
            .rpc
            .estimate_gas(
                &self.signer_address()?,
                &to_0x_hex(&self.contract),
                &to_0x_hex(&data),
            )
            .map_err(|e| {
                if gas_price == 0 {
                    SwapError::RpcError("no gas price available".into())
                } else {
                    SwapError::RpcError(e.to_string())
                }
            })?
            .saturating_add(10_000);

        let tx = Transaction {
            nonce,
            gas_price,
            gas_limit,
            to: Some(to_0x_hex(&self.contract)),
            value: if token == [0u8; 20] { amount } else { 0 },
            data: to_0x_hex(&data),
            chain_id,
        };

        let (tx_hash, _block) = self.send_and_confirm(tx, timeout_ms)?;
        Ok(tx_hash)
    }

    /// Sign and broadcast a `claimHTLC(id, secret)` transaction.
    pub fn claim(
        &mut self,
        id: [u8; 32],
        secret: [u8; 32],
        timeout_ms: u64,
    ) -> Result<String, SwapError> {
        let mut data = Vec::with_capacity(4 + 64);
        data.extend_from_slice(&selector::CLAIM);
        data.extend_from_slice(&word(&id));
        data.extend_from_slice(&word(&secret));

        let nonce = self.next_nonce()?;
        let gas_price = self.gas_price_wei()?;
        let chain_id = self
            .rpc
            .chain_id()
            .map_err(|e| SwapError::RpcError(e.to_string()))?;
        let gas_limit = self
            .rpc
            .estimate_gas(
                &self.signer_address()?,
                &to_0x_hex(&self.contract),
                &to_0x_hex(&data),
            )
            .map_err(|e| SwapError::RpcError(e.to_string()))?
            .saturating_add(10_000);
        let tx = Transaction {
            nonce,
            gas_price,
            gas_limit,
            to: Some(to_0x_hex(&self.contract)),
            value: 0,
            data: to_0x_hex(&data),
            chain_id,
        };
        let (tx_hash, _block) = self.send_and_confirm(tx, timeout_ms)?;
        Ok(tx_hash)
    }

    /// Sign and broadcast a `refundHTLC(id)` transaction.
    pub fn refund(&mut self, id: [u8; 32], timeout_ms: u64) -> Result<String, SwapError> {
        let mut data = Vec::with_capacity(4 + 32);
        data.extend_from_slice(&selector::REFUND);
        data.extend_from_slice(&word(&id));

        let nonce = self.next_nonce()?;
        let gas_price = self.gas_price_wei()?;
        let chain_id = self
            .rpc
            .chain_id()
            .map_err(|e| SwapError::RpcError(e.to_string()))?;
        let gas_limit = self
            .rpc
            .estimate_gas(
                &self.signer_address()?,
                &to_0x_hex(&self.contract),
                &to_0x_hex(&data),
            )
            .map_err(|e| SwapError::RpcError(e.to_string()))?
            .saturating_add(10_000);
        let tx = Transaction {
            nonce,
            gas_price,
            gas_limit,
            to: Some(to_0x_hex(&self.contract)),
            value: 0,
            data: to_0x_hex(&data),
            chain_id,
        };
        let (tx_hash, _block) = self.send_and_confirm(tx, timeout_ms)?;
        Ok(tx_hash)
    }

    // ── Proof building (from a real mined tx) ─────────────────────────────

    /// Build a `LockProof` from an on-chain lock tx hash and inputs.
    ///
    /// This is the live counterpart to the simulated adapter's `lock` and uses
    /// the *real* mined transaction, not a fabricated id.
    pub fn lock_proof_from_tx(
        &self,
        chain_label: &str,
        tx_hash: TxId,
        block: u64,
        contract: [u8; 20],
        hashlock: [u8; 32],
        receiver: Vec<u8>,
        refund_address: Vec<u8>,
        amount: u128,
        timeout: u64,
    ) -> LockProof {
        LockProof {
            tx_id: tx_hash.clone(),
            chain_id: format!("{}-chain", chain_label),
            vm_type: VmType::Evm,
            block_number: block,
            block_hash: String::new(), // set by caller once receipt has a hash
            confirmations: 1,
            lock_address: format!("0x{}", contract.encode_hex::<String>()),
            locked_amount: amount,
            hashlock,
            receiver,
            refund_address,
            timeout,
            raw_proof: tx_hash.clone().into_bytes(),
        }
    }

    /// Broadcast a real `createHTLC` lock and return the genuine on-chain proof.
    ///
    /// This is the live counterpart callers (a configured relayer/controller)
    /// use instead of the simulated adapter path. Requires that the signer is
    /// funded/configured; never fabricates a tx hash.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_lock(
        &mut self,
        chain_label: &str,
        receiver: [u8; 20],
        hashlock: [u8; 32],
        timelock: u64,
        asset: [u8; 20],
        amount: u128,
        timeout_ms: u64,
    ) -> Result<LockProof, SwapError> {
        let tx_hash = self.create_lock(receiver, hashlock, timelock, asset, amount, timeout_ms)?;
        let block = self.wait_for_receipt(&tx_hash, timeout_ms)?;
        // AtlasHTLC has no refund param: the sender is the only party able to
        // refund after the timelock, so we surface the signer as refund address.
        Ok(self.lock_proof_from_tx(
            chain_label,
            tx_hash,
            block,
            self.contract,
            hashlock,
            receiver.to_vec(),
            self.signer_addr.to_vec(),
            amount,
            timelock,
        ))
    }

    /// Build a `ClaimProof` from a real `claimHTLC` tx hash.
    pub fn claim_proof_from_tx(
        &self,
        chain_label: &str,
        intent_id: u64,
        tx_hash: TxId,
        block: u64,
        preimage: [u8; 32],
    ) -> ClaimProof {
        ClaimProof {
            tx_id: tx_hash.clone(),
            intent_id,
            chain_id: format!("{}-chain", chain_label),
            vm_type: VmType::Evm,
            preimage,
            block_number: block,
            block_hash: String::new(),
            raw_proof: tx_hash.clone().into_bytes(),
        }
    }

    /// Broadcast a real `claimHTLC` claim and return the genuine on-chain proof.
    pub fn execute_claim(
        &mut self,
        chain_label: &str,
        id: [u8; 32],
        intent_id: u64,
        secret: [u8; 32],
        timeout_ms: u64,
    ) -> Result<ClaimProof, SwapError> {
        let tx_hash = self.claim(id, secret, timeout_ms)?;
        let block = self.wait_for_receipt(&tx_hash, timeout_ms)?;
        Ok(self.claim_proof_from_tx(chain_label, intent_id, tx_hash, block, secret))
    }

    /// Broadcast a real `refundHTLC` refund and return the genuine on-chain proof.
    pub fn execute_refund(
        &mut self,
        chain_label: &str,
        id: [u8; 32],
        intent_id: u64,
        timeout_ms: u64,
    ) -> Result<RefundProof, SwapError> {
        let tx_hash = self.refund(id, timeout_ms)?;
        let block = self.wait_for_receipt(&tx_hash, timeout_ms)?;
        Ok(RefundProof {
            tx_id: tx_hash.clone(),
            intent_id,
            chain_id: format!("{}-chain", chain_label),
            vm_type: VmType::Evm,
            block_number: block,
            block_hash: String::new(),
            raw_proof: tx_hash.clone().into_bytes(),
        })
    }
}

/// Deterministic keccak-256 value used to check selector correctness in tests.
#[cfg(test)]
pub fn checksum_keccak(bytes: &[u8]) -> [u8; 32] {
    let mut h = Keccak256::new();
    h.update(bytes);
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(n: u8) -> [u8; 20] {
        let mut a = [0u8; 20];
        a[0] = n;
        a[19] = n;
        a
    }

    #[test]
    fn selectors_match_keccak_of_canonical_signature() {
        // The real compiled selectors (from the foundry artifact) ARE keccak256
        // of the canonical signature. This proves the constants target the
        // correct on-chain functions. NOTE: the .sol source's own "selector:"
        // comments are stale and disagree with the artifact; the artifact wins.
        assert_eq!(
            &checksum_keccak(b"createHTLC(address,bytes32,uint256,address,uint256)")[..4],
            &selector::CREATE,
            "createHTLC must match compiled artifact 0x502e9fd5"
        );
        assert_eq!(
            &checksum_keccak(b"claimHTLC(bytes32,bytes32)")[..4],
            &selector::CLAIM
        );
        assert_eq!(
            &checksum_keccak(b"refundHTLC(bytes32)")[..4],
            &selector::REFUND
        );
        // Cross-reference against the compiled artifact exactly.
        assert_eq!(selector::CREATE, [0x50, 0x2e, 0x9f, 0xd5]);
        assert_eq!(selector::CLAIM, [0x97, 0x55, 0xdc, 0xa0]);
        assert_eq!(selector::REFUND, [0x43, 0xb9, 0x20, 0xc5]);
    }

    #[test]
    fn abi_address_pads_left() {
        let mut a = [0u8; 20];
        a[0] = 0xaa;
        let w = abi_address(&a);
        assert_eq!(w.len(), 32);
        assert_eq!(&w[..12], &[0u8; 12]);
        assert_eq!(&w[12..], &a);
    }

    #[test]
    fn abi_word_encoding() {
        let h = [7u8; 32];
        assert_eq!(word(&h).len(), 32);
        assert_eq!(abi_u256(12345)[16..], 12345u128.to_be_bytes());
        assert_eq!(abi_u64(999)[24..], 999u64.to_be_bytes());
    }

    #[test]
    fn calldata_layout_for_create_lock() {
        // Build the same payload create_lock would; assert lengths/components.
        let rec = addr(3);
        let hashlock = [9u8; 32];
        let mut data = Vec::new();
        data.extend_from_slice(&selector::CREATE);
        data.extend_from_slice(&abi_address(&rec));
        data.extend_from_slice(&word(&hashlock));
        data.extend_from_slice(&abi_u64(2_000));
        data.extend_from_slice(&abi_address(&[0u8; 20]));
        data.extend_from_slice(&abi_u256(1_000));
        // 4 + 5*32 = 164 bytes
        assert_eq!(data.len(), 4 + 5 * 32);
        assert_eq!(&data[0..4], &selector::CREATE);
        // token (4th word, offset 4+3*32) is zero for native
        assert_eq!(&data[4 + 3 * 32..4 + 4 * 32], &[0u8; 32]);
    }
}
