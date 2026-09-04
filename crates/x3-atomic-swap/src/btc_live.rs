//! # Live Bitcoin broadcaster (real on-chain path)
//!
//! The mock `BtcTransactionBuilder`/`BtcHtlcAdapter` in [`crate::bitcoin_htlc`]
//! serialize transactions with placeholder conventions (hex-string bytes,
//! single-byte counts) that no real Bitcoin node accepts. This module provides
//! the *real* counterpart:
//!
//! - A correct Bitcoin transaction serializer (compact-size varints,
//!   little-endian amounts/locktime, reversed little-endian 32-byte prevouts,
//!   genuine double-SHA256 txid) so transactions are broadcastable.
//! - [`BtcRpcBroadcaster`], a minimal Bitcoin Core `sendrawtransaction`
//!   JSON-RPC client using the `std` HTTP transport.
//!
//! Only compiled with the `std` feature. `no_std` callers cannot broadcast.
//!
//! # Bitcoin Core auth
//! Bitcoin Core JSON-RPC requires HTTP Basic auth (usually a cookie or
//! `rpcuser`/`rpcpassword`). Provide `user:pass` in `rpc_url_userpass` or an
//! empty string when the endpoint is open (e.g. some regtest setups).

use crate::error::SwapError;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

/// A serialized Bitcoin transaction in wire format.
pub struct BtcRawTx {
    /// Official transaction id: double-SHA256 of the serialized tx, big-endian
    /// hex (the quantity shown by block explorers and used in `prev_txid`).
    pub txid: String,
    /// Raw serialized bytes (hex-encoded) suitable for `sendrawtransaction`.
    pub raw_hex: String,
}

/// Append a Bitcoin compact-size (varint) prefix.
fn varint(n: usize) -> Vec<u8> {
    if n < 0xfd {
        vec![n as u8]
    } else if n <= 0xffff {
        let mut v = vec![0xfd];
        v.extend_from_slice(&(n as u16).to_le_bytes());
        v
    } else if n <= 0xffff_ffff {
        let mut v = vec![0xfe];
        v.extend_from_slice(&(n as u32).to_le_bytes());
        v
    } else {
        let mut v = vec![0xff];
        v.extend_from_slice(&(n as u64).to_le_bytes());
        v
    }
}

/// Bitcoin transaction input (prevout).
pub struct BtcIn {
    /// Prevout transaction id, as 32 raw bytes in *internal* (non-reversed)
    /// order — feed it the hex from an explorer and it will be byte-reversed
    /// on the wire per Bitcoin's little-endian convention.
    pub prev_txid: [u8; 32],
    pub prev_vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
}

/// Bitcoin transaction output.
pub struct BtcOut {
    pub value_sats: u64,
    pub script_pubkey: Vec<u8>,
}

/// A well-formed legacy (non-segwit) Bitcoin transaction.
pub struct BtcTx {
    pub version: i32,
    pub inputs: Vec<BtcIn>,
    pub outputs: Vec<BtcOut>,
    pub locktime: u32,
}

impl BtcTx {
    /// Serialize to wire bytes and compute the genuine txid.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&varint(self.inputs.len()));
        for i in &self.inputs {
            // prevout txid is serialized byte-reversed (little-endian).
            for b in i.prev_txid.iter().rev() {
                out.push(*b);
            }
            out.extend_from_slice(&i.prev_vout.to_le_bytes());
            out.extend_from_slice(&varint(i.script_sig.len()));
            out.extend_from_slice(&i.script_sig);
            out.extend_from_slice(&i.sequence.to_le_bytes());
        }
        out.extend_from_slice(&varint(self.outputs.len()));
        for o in &self.outputs {
            out.extend_from_slice(&o.value_sats.to_le_bytes());
            out.extend_from_slice(&varint(o.script_pubkey.len()));
            out.extend_from_slice(&o.script_pubkey);
        }
        out.extend_from_slice(&self.locktime.to_le_bytes());
        out
    }

    /// Compute the raw tx wire bytes and its official (reversed, hex) txid.
    pub fn build(self) -> BtcRawTx {
        let bytes = self.serialize();
        let first = Sha256::digest(&bytes);
        let second = Sha256::digest(first);
        // txid = byte-reversal of the second hash, rendered hex big-endian.
        let mut rev = [0u8; 32];
        for (i, b) in second.iter().enumerate() {
            rev[31 - i] = *b;
        }
        BtcRawTx {
            txid: hex::encode(rev),
            raw_hex: hex::encode(bytes),
        }
    }
}

/// Broadcasts raw transactions to a Bitcoin Core JSON-RPC endpoint.
pub struct BtcRpcBroadcaster {
    pub rpc_url: String,
    /// `user:pass` for HTTP Basic auth (empty string = no auth).
    pub userpass: String,
}

impl BtcRpcBroadcaster {
    pub fn new(rpc_url: String, userpass: String) -> Self {
        Self { rpc_url, userpass }
    }

    /// `sendrawtransaction` — submits a raw serialized tx to the network.
    /// Returns the node-confirmed txid on success; the node's JSON-RPC error
    /// otherwise.
    pub fn send_raw_tx(&self, raw_hex: &str) -> Result<String, SwapError> {
        #[cfg(feature = "std")]
        {
            let body = format!(
                r#"{{"jsonrpc":"1.0","id":"x3-live","method":"sendrawtransaction","params":["{}"]}}"#,
                raw_hex
            );
            let mut req = ureq::post(&self.rpc_url)
                .set("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(30));
            if !self.userpass.is_empty() {
                req = req.set(
                    "Authorization",
                    &format!("Basic {}", base64_std(&self.userpass),),
                );
            }
            let response = req
                .send_string(&body)
                .map_err(|e| SwapError::RpcError(format!("sendrawtransaction failed: {}", e)))?;
            let text = response
                .into_string()
                .map_err(|e| SwapError::RpcError(format!("read response: {}", e)))?;
            // Honor JSON-RPC "error" tombstone (Bitcoin Core returns HTTP 200
            // with an error object for many failures).
            if text.contains("\"error\":{") || text.contains("\"code\":") {
                return Err(SwapError::RpcError(format!("bitcoind rpc error: {}", text)));
            }
            let parsed: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| SwapError::RpcError(format!("bad rpc json: {}", e)))?;
            parsed["result"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| SwapError::RpcError(format!("no result in rpc: {}", text)))
        }
        #[cfg(not(feature = "std"))]
        {
            let _ = raw_hex;
            Err(SwapError::RpcError(
                "Bitcoin broadcast requires the 'std' feature".into(),
            ))
        }
    }
}

/// Minimal std-only base64 for Basic auth header (avoids a new dependency).
#[cfg(feature = "std")]
fn base64_std(s: &str) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let a = bytes[i];
        let b = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let c = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        out.push(ALPHA[(a >> 2) as usize] as char);
        out.push(ALPHA[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(ALPHA[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(ALPHA[(c & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_in(n: u8) -> BtcIn {
        BtcIn {
            prev_txid: [n; 32],
            prev_vout: 0,
            script_sig: vec![],
            sequence: 0xffff_ffff,
        }
    }

    #[test]
    fn txid_is_reversed_double_sha256() {
        // Empty-input edge verifies the double-hash pipeline end-to-end.
        let tx = BtcTx {
            version: 1,
            inputs: vec![dummy_in(1)],
            outputs: vec![BtcOut {
                value_sats: 1000,
                script_pubkey: vec![0x51],
            }],
            locktime: 0,
        };
        let built = tx.build();
        // Recompute expected manually: SHA256(SHA256(serialized)) reversed.
        // Serialized: version(01000000) cnt(01) prev(txid rev of 0x01.., 32) vout(00000000)
        //   scriptlen(00) seq(ffffffff) out_cnt(01) val(1000 => e803000000000000) slen(01) 51 locktime(00000000)
        let bytes = {
            let mut v = Vec::new();
            v.extend_from_slice(&1i32.to_le_bytes());
            v.push(1);
            v.extend(std::iter::repeat_n(1, 32));
            v.extend_from_slice(&0u32.to_le_bytes());
            v.push(0);
            v.extend_from_slice(&0xffff_ffffu32.to_le_bytes());
            v.push(1);
            v.extend_from_slice(&1000u64.to_le_bytes());
            v.push(1);
            v.push(0x51);
            v.extend_from_slice(&0u32.to_le_bytes());
            v
        };
        let h1 = Sha256::digest(&bytes);
        let h2 = Sha256::digest(h1);
        let mut expect = [0u8; 32];
        for (i, b) in h2.iter().enumerate() {
            expect[31 - i] = *b;
        }
        assert_eq!(built.txid, hex::encode(expect));
        assert_eq!(built.raw_hex.len(), bytes.len() * 2);
    }

    #[test]
    fn varint_encoding_thresholds() {
        assert_eq!(varint(0), vec![0]);
        assert_eq!(varint(252), vec![0xfc]);
        // 253..65535 => 0xfd + u16 le
        assert_eq!(varint(253), vec![0xfd, 0xfd, 0x00]);
        assert_eq!(varint(65535), vec![0xfd, 0xff, 0xff]);
        // 65536.. => 0xfe + u32 le
        assert_eq!(varint(65536), vec![0xfe, 0x00, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn prevout_txid_is_byte_reversed_on_wire() {
        let tx = BtcTx {
            version: 1,
            inputs: vec![BtcIn {
                prev_txid: {
                    // 0x01..0x20
                    let mut p = [0u8; 32];
                    for (i, b) in p.iter_mut().enumerate() {
                        *b = (i + 1) as u8;
                    }
                    p
                },
                prev_vout: 0,
                script_sig: vec![],
                sequence: 0xffff_ffff,
            }],
            outputs: vec![BtcOut {
                value_sats: 0,
                script_pubkey: vec![],
            }],
            locktime: 0,
        };
        let bytes = tx.serialize();
        // After version(4) + cnt(1) the prevout begins; first wire byte should
        // be the *last* source byte (0x20).
        assert_eq!(bytes[5], 0x20, "prevout must be reversed: LSB first");
        assert_eq!(bytes[5 + 31], 0x01);
    }

    #[test]
    fn base64_basic_auth_known_vector() {
        assert_eq!(base64_std("user:pass"), "dXNlcjpwYXNz");
    }
}
