//! Cryptographic signing module for X3 Chain
//!
//! Provides unified signing and verification for:
//! - **ed25519**: SVM/Cosmos signature scheme
//! - **secp256k1**: EVM signature scheme
//! - **sr25519**: Substrate/X3 signature scheme
//!
//! # Usage
//!
//! ```rust
//! use x3_common::signing::{
//!     Signer, Ed25519Signer, Secp256k1Signer, Sr25519Signer,
//!     Signature, PublicKey
//! };
//!
//! // Generate a new ed25519 keypair
//! let signer = Ed25519Signer::generate();
//! let public_key = signer.public_key();
//! let signature = signer.sign(b"hello world");
//!
//! // Verify the signature
//! assert!(signature.verify(b"hello world", &public_key));
//! ```

use sp_core::crypto::Ss58Codec;
use sp_core::{ed25519, sr25519, Pair};
use sp_io::hashing::{blake2_256, keccak_256};
use sp_runtime::traits::Verify;

/// Re-export of the canonical [`KeyType`] (defined at crate root for no-std use).
pub use crate::KeyType;

/// Public key wrapper for different signature schemes
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublicKey {
    /// ed25519 public key (32 bytes)
    Ed25519(ed25519::Public),
    /// secp256k1 public key (65 bytes uncompressed)
    Secp256k1([u8; 65]),
    /// sr25519 public key (32 bytes)
    Sr25519(sr25519::Public),
}

impl PublicKey {
    /// Get the key type
    pub fn key_type(&self) -> KeyType {
        match self {
            PublicKey::Ed25519(_) => KeyType::Ed25519,
            PublicKey::Secp256k1(_) => KeyType::Secp256k1,
            PublicKey::Sr25519(_) => KeyType::Sr25519,
        }
    }

    /// Get the raw bytes of the public key
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PublicKey::Ed25519(pk) => pk.as_ref(),
            PublicKey::Secp256k1(pk) => pk.as_ref(),
            PublicKey::Sr25519(pk) => pk.as_ref(),
        }
    }

    /// Convert to EVM address (keccak256 hash of uncompressed pubkey)
    pub fn to_evm_address(&self) -> [u8; 20] {
        match self {
            PublicKey::Secp256k1(pk) => {
                // For secp256k1, use the last 20 bytes of keccak256(uncompressed_pubkey)
                let hash = keccak_256(pk);
                let mut addr = [0u8; 20];
                addr.copy_from_slice(&hash[12..]);
                addr
            }
            _ => {
                // For other schemes, use blake2_256 and take last 20 bytes
                let hash = blake2_256(self.as_bytes());
                let mut addr = [0u8; 20];
                addr.copy_from_slice(&hash[12..]);
                addr
            }
        }
    }
}

/// Signature wrapper for different signature schemes
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signature {
    /// ed25519 signature (64 bytes)
    Ed25519(ed25519::Signature),
    /// secp256k1 signature (65 bytes with recovery id)
    Secp256k1([u8; 65]),
    /// sr25519 signature (64 bytes)
    Sr25519(sr25519::Signature),
}

impl Signature {
    /// Get the raw bytes of the signature
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Signature::Ed25519(sig) => sig.as_ref(),
            Signature::Secp256k1(sig) => sig.as_ref(),
            Signature::Sr25519(sig) => sig.as_ref(),
        }
    }

    /// Verify this signature against a message and public key
    pub fn verify(&self, message: &[u8], public_key: &PublicKey) -> bool {
        match (self, public_key) {
            (Signature::Ed25519(sig), PublicKey::Ed25519(pk)) => {
                ed25519::Signature::verify(sig, message, pk)
            }
            (Signature::Secp256k1(sig), PublicKey::Secp256k1(pk)) => {
                // EVM uses secp256k1_ecdsa_recover with 65-byte signature (r, s, v)
                match sp_io::crypto::secp256k1_ecdsa_recover(sig, &blake2_256(message)) {
                    Ok(recovered) => recovered.as_ref() == &pk[1..],
                    Err(_) => false,
                }
            }
            (Signature::Sr25519(sig), PublicKey::Sr25519(pk)) => {
                sr25519::Signature::verify(sig, message, pk)
            }
            _ => false, // Mismatched key types
        }
    }

    /// Verify this signature against a pre-hashed message
    pub fn verify_hash(&self, message_hash: &[u8; 32], public_key: &PublicKey) -> bool {
        match (self, public_key) {
            (Signature::Ed25519(sig), PublicKey::Ed25519(pk)) => {
                ed25519::Signature::verify(sig, &message_hash[..], pk)
            }
            (Signature::Secp256k1(sig), PublicKey::Secp256k1(pk)) => {
                match sp_io::crypto::secp256k1_ecdsa_recover(sig, message_hash) {
                    Ok(recovered) => recovered.as_ref() == &pk[1..],
                    Err(_) => false,
                }
            }
            (Signature::Sr25519(sig), PublicKey::Sr25519(pk)) => {
                sr25519::Signature::verify(sig, &message_hash[..], pk)
            }
            _ => false,
        }
    }
}

/// Trait for cryptographic signing
pub trait Signer: Send + Sync {
    /// Get the key type for this signer
    fn key_type(&self) -> KeyType;

    /// Sign a message
    fn sign(&self, message: &[u8]) -> Signature;

    /// Sign a pre-hashed message (32 bytes)
    fn sign_hash(&self, message_hash: &[u8; 32]) -> Signature;

    /// Get the public key
    fn public_key(&self) -> PublicKey;

    /// Get the account ID (SS58 encoded for Substrate, hex for EVM/SVM)
    fn account_id(&self) -> String {
        match self.public_key() {
            PublicKey::Ed25519(pk) => pk.to_ss58check(),
            PublicKey::Secp256k1(pk) => {
                let hash = keccak_256(&pk);
                format!("0x{}", hex::encode(&hash[12..]))
            }
            PublicKey::Sr25519(pk) => pk.to_ss58check(),
        }
    }
}

/// ed25519 signer for SVM/Cosmos
#[derive(Clone)]
pub struct Ed25519Signer {
    pair: ed25519::Pair,
    public_key: ed25519::Public,
}

impl Ed25519Signer {
    /// Generate a new ed25519 keypair
    pub fn generate() -> Self {
        let (pair, _, _) = ed25519::Pair::generate_with_phrase(None);
        let public_key = pair.public();
        Self { pair, public_key }
    }

    /// Create a signer from a seed phrase
    pub fn from_phrase(phrase: &str, password: Option<&str>) -> Result<Self, &'static str> {
        let (pair, _) =
            ed25519::Pair::from_phrase(phrase, password).map_err(|_| "Invalid seed phrase")?;
        let public_key = pair.public();
        Ok(Self { pair, public_key })
    }

    /// Create a signer from a raw seed (32 bytes)
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let pair = ed25519::Pair::from_seed(seed);
        let public_key = pair.public();
        Self { pair, public_key }
    }

    /// Create a signer from a secret key (32 bytes)
    pub fn from_secret_key(secret: &[u8; 32]) -> Self {
        let pair = ed25519::Pair::from_seed(secret);
        let public_key = pair.public();
        Self { pair, public_key }
    }
}

impl Signer for Ed25519Signer {
    fn key_type(&self) -> KeyType {
        KeyType::Ed25519
    }

    fn sign(&self, message: &[u8]) -> Signature {
        let signature = self.pair.sign(message);
        Signature::Ed25519(signature)
    }

    fn sign_hash(&self, message_hash: &[u8; 32]) -> Signature {
        let signature = self.pair.sign(message_hash);
        Signature::Ed25519(signature)
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Ed25519(self.public_key)
    }
}

/// secp256k1 signer for EVM
#[derive(Clone)]
pub struct Secp256k1Signer {
    secret_key: [u8; 32],
    public_key: [u8; 65],
}

impl Secp256k1Signer {
    /// Generate a new secp256k1 keypair
    pub fn generate() -> Self {
        use secp256k1::{rand::thread_rng, Secp256k1};

        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut thread_rng());
        let secret_key = secret_key.secret_bytes();
        let public_key = public_key.serialize_uncompressed();

        let mut pk = [0u8; 65];
        pk.copy_from_slice(&public_key);

        Self {
            secret_key,
            public_key: pk,
        }
    }

    /// Create a signer from a secret key (32 bytes)
    pub fn from_secret_key(secret: &[u8; 32]) -> Result<Self, &'static str> {
        use secp256k1::{Secp256k1, SecretKey};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(secret).map_err(|_| "Invalid secret key")?;
        let public_key = secret_key.public_key(&secp).serialize_uncompressed();

        let mut pk = [0u8; 65];
        pk.copy_from_slice(&public_key);

        let mut sk = [0u8; 32];
        sk.copy_from_slice(secret);

        Ok(Self {
            secret_key: sk,
            public_key: pk,
        })
    }

    /// Create a signer from a hex-encoded secret key
    pub fn from_hex_secret(hex: &str) -> Result<Self, &'static str> {
        let secret = hex::decode(hex).map_err(|_| "Invalid hex")?;
        if secret.len() != 32 {
            return Err("Secret key must be 32 bytes");
        }
        let mut secret_array = [0u8; 32];
        secret_array.copy_from_slice(&secret);
        Self::from_secret_key(&secret_array)
    }
}

impl Signer for Secp256k1Signer {
    fn key_type(&self) -> KeyType {
        KeyType::Secp256k1
    }

    fn sign(&self, message: &[u8]) -> Signature {
        use secp256k1::{Message, Secp256k1};

        let secp = Secp256k1::new();
        let message_hash = blake2_256(message);
        let message = Message::from_digest_slice(&message_hash).unwrap();
        let secret_key = secp256k1::SecretKey::from_slice(&self.secret_key).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret_key);
        let mut sig_bytes = [0u8; 65];
        let compact = signature.serialize_compact();
        sig_bytes[..32].copy_from_slice(&compact[..32]);
        sig_bytes[32..64].copy_from_slice(&compact[32..]);
        sig_bytes[64] = 0; // recovery id (not used in this context)

        Signature::Secp256k1(sig_bytes)
    }

    fn sign_hash(&self, message_hash: &[u8; 32]) -> Signature {
        use secp256k1::{Message, Secp256k1};

        let secp = Secp256k1::new();
        let message = Message::from_digest_slice(message_hash).unwrap();
        let secret_key = secp256k1::SecretKey::from_slice(&self.secret_key).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret_key);
        let mut sig_bytes = [0u8; 65];
        let compact = signature.serialize_compact();
        sig_bytes[..32].copy_from_slice(&compact[..32]);
        sig_bytes[32..64].copy_from_slice(&compact[32..]);
        sig_bytes[64] = 0; // recovery id

        Signature::Secp256k1(sig_bytes)
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Secp256k1(self.public_key)
    }
}

/// sr25519 signer for Substrate/X3
#[derive(Clone)]
pub struct Sr25519Signer {
    pair: sr25519::Pair,
    public_key: sr25519::Public,
}

impl Sr25519Signer {
    /// Generate a new sr25519 keypair
    pub fn generate() -> Self {
        let (pair, _, _) = sr25519::Pair::generate_with_phrase(None);
        let public_key = pair.public();
        Self { pair, public_key }
    }

    /// Create a signer from a seed phrase
    pub fn from_phrase(phrase: &str, password: Option<&str>) -> Result<Self, &'static str> {
        let (pair, _) =
            sr25519::Pair::from_phrase(phrase, password).map_err(|_| "Invalid seed phrase")?;
        let public_key = pair.public();
        Ok(Self { pair, public_key })
    }

    /// Create a signer from a raw seed (32 bytes)
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let pair = sr25519::Pair::from_seed(seed);
        let public_key = pair.public();
        Self { pair, public_key }
    }

    /// Create a signer from a secret key (32 bytes)
    pub fn from_secret_key(secret: &[u8; 32]) -> Self {
        let pair = sr25519::Pair::from_seed(secret);
        let public_key = pair.public();
        Self { pair, public_key }
    }
}

impl Signer for Sr25519Signer {
    fn key_type(&self) -> KeyType {
        KeyType::Sr25519
    }

    fn sign(&self, message: &[u8]) -> Signature {
        let signature = self.pair.sign(message);
        Signature::Sr25519(signature)
    }

    fn sign_hash(&self, message_hash: &[u8; 32]) -> Signature {
        let signature = self.pair.sign(message_hash);
        Signature::Sr25519(signature)
    }

    fn public_key(&self) -> PublicKey {
        PublicKey::Sr25519(self.public_key)
    }
}

/// Verify a signature for a given key type
pub fn verify_signature(
    signature: &[u8],
    message: &[u8],
    public_key: &[u8],
    key_type: KeyType,
) -> bool {
    match key_type {
        KeyType::Ed25519 => {
            if signature.len() != 64 || public_key.len() != 32 {
                return false;
            }
            let sig = ed25519::Signature::from_raw({
                let mut buf = [0u8; 64];
                buf.copy_from_slice(signature);
                buf
            });
            let pk = ed25519::Public::from_raw({
                let mut buf = [0u8; 32];
                buf.copy_from_slice(public_key);
                buf
            });
            ed25519::Signature::verify(&sig, message, &pk)
        }
        KeyType::Secp256k1 => {
            // secp256k1_ecdsa_recover returns 64-byte uncompressed key (no 0x04 prefix).
            // public_key is expected to be 65 bytes (uncompressed with 0x04 prefix).
            if signature.len() != 65 || public_key.len() != 65 {
                return false;
            }
            let mut sig = [0u8; 65];
            sig.copy_from_slice(signature);
            let mut pk = [0u8; 64];
            pk.copy_from_slice(&public_key[1..]); // skip 0x04 prefix
            match sp_io::crypto::secp256k1_ecdsa_recover(&sig, &blake2_256(message)) {
                Ok(recovered) => recovered.as_ref() == pk.as_ref(),
                Err(_) => false,
            }
        }
        KeyType::Sr25519 => {
            if signature.len() != 64 || public_key.len() != 32 {
                return false;
            }
            let sig = sr25519::Signature::from_raw({
                let mut buf = [0u8; 64];
                buf.copy_from_slice(signature);
                buf
            });
            let pk = sr25519::Public::from_raw({
                let mut buf = [0u8; 32];
                buf.copy_from_slice(public_key);
                buf
            });
            sr25519::Signature::verify(&sig, message, &pk)
        }
    }
}

/// Verify a pre-hashed signature for a given key type
pub fn verify_signature_hash(
    signature: &[u8],
    message_hash: &[u8; 32],
    public_key: &[u8],
    key_type: KeyType,
) -> bool {
    match key_type {
        KeyType::Ed25519 => {
            if signature.len() != 64 || public_key.len() != 32 {
                return false;
            }
            let sig = ed25519::Signature::from_raw({
                let mut buf = [0u8; 64];
                buf.copy_from_slice(signature);
                buf
            });
            let pk = ed25519::Public::from_raw({
                let mut buf = [0u8; 32];
                buf.copy_from_slice(public_key);
                buf
            });
            ed25519::Signature::verify(&sig, &message_hash[..], &pk)
        }
        KeyType::Secp256k1 => {
            // secp256k1_ecdsa_recover returns 64-byte uncompressed key (no 0x04 prefix).
            // public_key is expected to be 65 bytes (uncompressed with 0x04 prefix).
            if signature.len() != 65 || public_key.len() != 65 {
                return false;
            }
            let mut sig = [0u8; 65];
            sig.copy_from_slice(signature);
            let mut pk = [0u8; 64];
            pk.copy_from_slice(&public_key[1..]); // skip 0x04 prefix
            match sp_io::crypto::secp256k1_ecdsa_recover(&sig, message_hash) {
                Ok(recovered) => recovered.as_ref() == pk.as_ref(),
                Err(_) => false,
            }
        }
        KeyType::Sr25519 => {
            if signature.len() != 64 || public_key.len() != 32 {
                return false;
            }
            let sig = sr25519::Signature::from_raw({
                let mut buf = [0u8; 64];
                buf.copy_from_slice(signature);
                buf
            });
            let pk = sr25519::Public::from_raw({
                let mut buf = [0u8; 32];
                buf.copy_from_slice(public_key);
                buf
            });
            sr25519::Signature::verify(&sig, &message_hash[..], &pk)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ed25519_signing() {
        let signer = Ed25519Signer::generate();
        let message = b"test message";
        let signature = signer.sign(message);
        let public_key = signer.public_key();

        assert!(signature.verify(message, &public_key));
        assert!(!signature.verify(b"wrong message", &public_key));
    }

    #[test]
    fn test_secp256k1_signing() {
        let signer = Secp256k1Signer::generate();
        let message = b"test message";
        let signature = signer.sign(message);
        let public_key = signer.public_key();

        assert!(signature.verify(message, &public_key));
        assert!(!signature.verify(b"wrong message", &public_key));
    }

    #[test]
    fn test_sr25519_signing() {
        let signer = Sr25519Signer::generate();
        let message = b"test message";
        let signature = signer.sign(message);
        let public_key = signer.public_key();

        assert!(signature.verify(message, &public_key));
        assert!(!signature.verify(b"wrong message", &public_key));
    }

    #[test]
    fn test_signer_account_id() {
        let ed_signer = Ed25519Signer::generate();
        let secp_signer = Secp256k1Signer::generate();
        let sr_signer = Sr25519Signer::generate();

        // Check that account IDs are valid strings
        assert!(ed_signer.account_id().starts_with("5") || ed_signer.account_id().starts_with("1"));
        assert!(secp_signer.account_id().starts_with("0x"));
        assert!(sr_signer.account_id().starts_with("5") || sr_signer.account_id().starts_with("1"));
    }

    #[test]
    fn test_verify_signature() {
        let message = b"test message";
        let message_hash: [u8; 32] = blake2_256(message);

        // Test ed25519
        let ed_signer = Ed25519Signer::from_seed(&[0u8; 32]);
        let ed_sig = ed_signer.sign(message);
        assert!(verify_signature(
            ed_sig.as_bytes(),
            message,
            ed_signer.public_key().as_bytes(),
            KeyType::Ed25519
        ));

        // Test secp256k1
        let secp_signer = Secp256k1Signer::from_secret_key(&[0u8; 32]).unwrap();
        let secp_sig = secp_signer.sign(message);
        assert!(verify_signature(
            secp_sig.as_bytes(),
            message,
            secp_signer.public_key().as_bytes(),
            KeyType::Secp256k1
        ));

        // Test sr25519
        let sr_signer = Sr25519Signer::from_seed(&[0u8; 32]);
        let sr_sig = sr_signer.sign(message);
        assert!(verify_signature(
            sr_sig.as_bytes(),
            message,
            sr_signer.public_key().as_bytes(),
            KeyType::Sr25519
        ));
    }
}
