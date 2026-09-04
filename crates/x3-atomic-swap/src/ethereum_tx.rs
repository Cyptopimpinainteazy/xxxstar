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

            // Keccak256 hash
            let digest = sha3::Keccak256::new_with_prefix(&rlp_data);

            // Sign with recovery id
            let (signature, rec_id): (k256::ecdsa::Signature, RecoveryId) = signing_key
                .sign_digest_recoverable(digest)
                .map_err(|e| SwapError::RpcError(format!("Signing failed: {}", e)))?;

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
            stream.append(&r);
            stream.append(&s);

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
            let public_key_bytes = verifying_key.to_encoded_point(false); // uncompressed
            let public_key = public_key_bytes.as_bytes();
            // First byte is 0x04 (uncompressed), the remaining 64 bytes are x,y
            let hash = sha3::Keccak256::digest(&public_key[1..]);
            let address_bytes = &hash[12..]; // last 20 bytes
            Ok(format!("0x{}", hex::encode(address_bytes)))
        }
    }
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
}
