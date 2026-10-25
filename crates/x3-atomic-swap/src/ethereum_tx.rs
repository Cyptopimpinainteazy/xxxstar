//! Ethereum transaction construction and signing.
//! Only available with the `std` feature.

use crate::error::SwapError;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// An unsigned legacy Ethereum transaction (EIP-155)
#[derive(Debug, Clone)]
pub struct Transaction {
    pub nonce: u64,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub to: Option<String>, // None for contract creation
    pub value: u128,
    pub data: String, // hex-encoded with 0x prefix
    pub chain_id: u64,
}

impl Transaction {
    pub fn new_contract_deploy(
        nonce: u64,
        gas_price: u128,
        gas_limit: u64,
        init_code: &str,
        chain_id: u64,
    ) -> Self {
        Self {
            nonce,
            gas_price,
            gas_limit,
            to: None,
            value: 0,
            data: init_code.to_string(),
            chain_id,
        }
    }

    /// RLP-encode the transaction for signing (includes chain_id for EIP-155)
    pub fn rlp_encode_for_signing(&self) -> Result<Vec<u8>, SwapError> {
        #[cfg(not(feature = "std"))]
        {
            Err(SwapError::RpcError(
                "Transaction signing requires std feature".into(),
            ))
        }

        #[cfg(feature = "std")]
        {
            use rlp::RlpStream;

            let mut stream = RlpStream::new_list(9);
            stream.append(&self.nonce);
            stream.append(&self.gas_price);
            stream.append(&self.gas_limit);

            // to: empty byte array for contract creation
            if let Some(ref addr) = self.to {
                let addr_bytes = hex::decode(addr.trim_start_matches("0x"))
                    .map_err(|e| SwapError::RpcError(format!("Invalid to address: {}", e)))?;
                stream.append(&addr_bytes);
            } else {
                stream.append_empty_data();
            }

            stream.append(&self.value);

            // data: hex string to bytes
            let data_bytes = hex::decode(self.data.trim_start_matches("0x"))
                .map_err(|e| SwapError::RpcError(format!("Invalid data hex: {}", e)))?;
            stream.append(&data_bytes);

            // EIP-155: append chain_id, 0, 0 for signing
            stream.append(&self.chain_id);
            stream.append(&0u8);
            stream.append(&0u8);

            Ok(stream.out().to_vec())
        }
    }

    /// Sign the transaction with a private key and return the raw signed transaction hex
    pub fn sign(&self, private_key_hex: &str) -> Result<String, SwapError> {
        #[cfg(not(feature = "std"))]
        {
            Err(SwapError::RpcError(
                "Transaction signing requires std feature".into(),
            ))
        }

        #[cfg(feature = "std")]
        {
            use k256::ecdsa::{RecoveryId, SigningKey};
            use sha3::Digest;

            // Decode private key
            let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
                .map_err(|e| SwapError::RpcError(format!("Invalid private key hex: {}", e)))?;

            if key_bytes.len() != 32 {
                return Err(SwapError::RpcError(format!(
                    "Private key must be 32 bytes, got {}",
                    key_bytes.len()
                )));
            }

            let signing_key = SigningKey::from_slice(&key_bytes)
                .map_err(|e| SwapError::RpcError(format!("Invalid private key: {}", e)))?;

            // RLP encode for signing
            let rlp_data = self.rlp_encode_for_signing()?;

            // Ethereum signs the Keccak-256 prehash. Passing the fixed digest to
            // k256's prehash API also keeps this boundary independent of which
            // `digest` trait version `sha3` and `ecdsa` resolve internally.
            let digest = sha3::Keccak256::digest(&rlp_data);

            // Sign with recovery id
            let (signature, rec_id): (k256::ecdsa::Signature, RecoveryId) =
                signing_key.sign_prehash_recoverable(digest.as_slice());

            // Convert to v,r,s format
            let (r_bytes, s_bytes) = signature.split_bytes();
            let r = r_bytes.as_slice();
            let s = s_bytes.as_slice();

            // EIP-155 v = chain_id * 2 + 35 + rec_id
            let v: u64 = self.chain_id * 2 + 35 + rec_id.to_byte() as u64;

            // Now RLP-encode the signed transaction: [nonce, gasPrice, gasLimit, to, value, data, v, r, s]
            use rlp::RlpStream;
            let mut stream = RlpStream::new_list(9);
            stream.append(&self.nonce);
            stream.append(&self.gas_price);
            stream.append(&self.gas_limit);

            if let Some(ref addr) = self.to {
                let addr_bytes = hex::decode(addr.trim_start_matches("0x"))
                    .map_err(|e| SwapError::RpcError(format!("Invalid to: {}", e)))?;
                stream.append(&addr_bytes);
            } else {
                stream.append_empty_data();
            }

            stream.append(&self.value);

            let data_bytes = hex::decode(self.data.trim_start_matches("0x"))
                .map_err(|e| SwapError::RpcError(format!("Invalid data: {}", e)))?;
            stream.append(&data_bytes);

            stream.append(&v);
            // `r` and `s` are fixed-width 32-byte big-endian values, but RLP
            // carries them as *integers*: a leading zero byte is not a canonical
            // encoding of the value it stands for, and a node that enforces
            // canonical integers refuses the whole transaction at the RPC
            // boundary. One signature in ~128 has such a leading zero, which is
            // why broadcasting used to fail intermittently with
            // `-32602 Failed to decode transaction`.
            stream.append(&rlp_integer(r));
            stream.append(&rlp_integer(s));

            let signed_rlp = stream.out().to_vec();
            Ok(format!("0x{}", hex::encode(&signed_rlp)))
        }
    }

    /// Derive the deployer address from a private key
    pub fn address_from_private_key(private_key_hex: &str) -> Result<String, SwapError> {
        #[cfg(not(feature = "std"))]
        {
            Err(SwapError::RpcError(
                "Address derivation requires std feature".into(),
            ))
        }

        #[cfg(feature = "std")]
        {
            use k256::ecdsa::SigningKey;
            use sha3::Digest;

            let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
                .map_err(|e| SwapError::RpcError(format!("Invalid private key hex: {}", e)))?;
            let signing_key = SigningKey::from_slice(&key_bytes)
                .map_err(|e| SwapError::RpcError(format!("Invalid private key: {}", e)))?;

            // Get public key (uncompressed), strip 0x04 prefix, keccak256, take last 20 bytes
            let verifying_key = signing_key.verifying_key();
            let public_key_bytes = verifying_key.to_sec1_point(false); // uncompressed
            let public_key = public_key_bytes.as_bytes();
            // First byte is 0x04 (uncompressed), the remaining 64 bytes are x,y
            let hash = sha3::Keccak256::digest(&public_key[1..]);
            let address_bytes = &hash[12..]; // last 20 bytes
            Ok(format!("0x{}", hex::encode(address_bytes)))
        }
    }
}

/// The bytes RLP carries for a big-endian integer: the value, without leading
/// zero bytes.
///
/// A signature's `r` and `s` are 32-byte fixed-width values. Writing them out
/// verbatim is well-formed RLP — it is a 32-byte string — but it is not the
/// canonical *integer* encoding when the first byte is zero, and anvil/reth
/// reject the transaction at the RPC boundary with
/// `-32602 Failed to decode transaction`. Encoding the value instead of the
/// buffer is the whole fix. Zero (which a valid signature never produces here)
/// is the empty byte string.
///
/// `std`-gated with its only caller: a `no_std` build has no signer, and an
/// unused private function is a `dead_code` error in the runtime's wasm build.
#[cfg(feature = "std")]
fn rlp_integer(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    &bytes[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_derivation_known_key() {
        // Known test vector: this key derives to a known address
        let key = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = Transaction::address_from_private_key(key);
        // Just check it returns a valid 0x-prefixed 40-char hex address
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 42);
    }

    #[test]
    fn test_address_derivation_invalid_key() {
        let result = Transaction::address_from_private_key("0xdead");
        assert!(result.is_err());
    }

    #[test]
    fn test_new_contract_deploy() {
        let tx = Transaction::new_contract_deploy(0, 10000000000, 1000000, "0xdeadbeef", 11155111);
        assert_eq!(tx.nonce, 0);
        assert_eq!(tx.gas_price, 10000000000);
        assert_eq!(tx.gas_limit, 1000000);
        assert!(tx.to.is_none());
        assert_eq!(tx.value, 0);
        assert_eq!(tx.data, "0xdeadbeef");
        assert_eq!(tx.chain_id, 11155111);
    }

    #[test]
    fn signed_transaction_recovers_to_its_signing_address() {
        use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
        use rlp::Rlp;
        use sha3::{Digest, Keccak256};

        let key = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let tx = Transaction {
            nonce: 0,
            gas_price: 1_000_000_000,
            gas_limit: 21_000,
            to: Some("0x1111111111111111111111111111111111111111".into()),
            value: 1,
            data: "0x".into(),
            chain_id: 1,
        };

        let raw = hex::decode(
            tx.sign(key)
                .expect("transaction signs with the std feature")
                .trim_start_matches("0x"),
        )
        .expect("signed transaction is hex");
        let decoded = Rlp::new(&raw);
        assert_eq!(decoded.item_count().expect("signed transaction list"), 9);

        let v = decoded.val_at::<u64>(6).expect("EIP-155 v");
        let recovery_byte = v
            .checked_sub(tx.chain_id * 2 + 35)
            .and_then(|value| u8::try_from(value).ok())
            .expect("v carries a canonical recovery id");
        let recovery_id = RecoveryId::try_from(recovery_byte).expect("valid recovery id");

        let mut r = [0u8; 32];
        let r_bytes = decoded.val_at::<Vec<u8>>(7).expect("signature r");
        r[32 - r_bytes.len()..].copy_from_slice(&r_bytes);
        let mut s = [0u8; 32];
        let s_bytes = decoded.val_at::<Vec<u8>>(8).expect("signature s");
        s[32 - s_bytes.len()..].copy_from_slice(&s_bytes);
        let signature = Signature::from_scalars(r, s).expect("canonical r and s");

        let unsigned = tx
            .rlp_encode_for_signing()
            .expect("unsigned transaction encoding");
        let digest = Keccak256::digest(&unsigned);
        let recovered =
            VerifyingKey::recover_from_prehash(digest.as_slice(), &signature, recovery_id)
                .expect("signature recovers to a public key");
        let public_key = recovered.to_sec1_point(false);
        let recovered_hash = Keccak256::digest(&public_key.as_bytes()[1..]);
        let recovered_address = format!("0x{}", hex::encode(&recovered_hash[12..]));
        assert_eq!(
            recovered_address,
            Transaction::address_from_private_key(key).expect("signer address")
        );
    }

    #[test]
    fn rlp_integer_drops_leading_zero_bytes_only() {
        // The case that made broadcasting intermittent: a 32-byte `r` whose first
        // byte is zero must be carried as the 31-byte value, not as 32 bytes.
        let mut r = [0u8; 32];
        r[1..].copy_from_slice(&[0x11u8; 31]);
        assert_eq!(rlp_integer(&r).len(), 31);
        assert_eq!(rlp_integer(&r)[0], 0x11);

        // Two leading zeros, and a value that needs none removed.
        let mut two = [0u8; 32];
        two[2..].copy_from_slice(&[0x22u8; 30]);
        assert_eq!(rlp_integer(&two).len(), 30);
        let full = [0x33u8; 32];
        assert_eq!(rlp_integer(&full), &full[..]);

        // A single byte below 0x80 keeps its own RLP encoding; stripping must not
        // turn 0x00 0x05 into an empty string.
        let small = [0u8, 5];
        assert_eq!(rlp_integer(&small), &[5u8][..]);

        // Zero is the empty string in RLP. No valid signature produces it, but the
        // helper must not panic or invent a byte.
        assert!(rlp_integer(&[0u8; 32]).is_empty());
    }
}
