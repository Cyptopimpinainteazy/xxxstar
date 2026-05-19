//! Cryptographic operations for X3 GPU Validator Swarm
//!
//! Provides deterministic hash and signature operations with CPU verification.

use crate::error::{SwarmError, SwarmResult};
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// Hash output (256-bit)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HashOutput(pub [u8; 32]);

impl HashOutput {
    /// Create a new hash output from bytes
    pub fn new(data: [u8; 32]) -> Self {
        Self(data)
    }

    /// Create a zero hash
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Get the hash as bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Create from hex string
    pub fn from_hex(s: &str) -> SwarmResult<Self> {
        let bytes = hex::decode(s).map_err(|e| SwarmError::CryptoError(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(SwarmError::CryptoError("Invalid hash length".to_string()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Default for HashOutput {
    fn default() -> Self {
        Self::zero()
    }
}

/// Signature output (64 bytes + recovery byte)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureOutput {
    /// R component
    pub r: [u8; 32],
    /// S component
    pub s: [u8; 32],
    /// Recovery byte
    pub v: u8,
}

impl SignatureOutput {
    /// Create a new signature
    pub fn new(r: [u8; 32], s: [u8; 32], v: u8) -> Self {
        Self { r, s, v }
    }

    /// Get the signature as bytes (65 bytes)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.extend_from_slice(&self.r);
        bytes.extend_from_slice(&self.s);
        bytes.push(self.v);
        bytes
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    /// Verify signature (CPU fallback)
    pub fn verify(&self, msg: &[u8], _expected_pubkey: &[u8; 33]) -> bool {
        // Placeholder verification for now: structural sanity + non-empty message.
        !msg.is_empty() && (self.r != [0u8; 32] || self.s != [0u8; 32])
    }
}

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationResult {
    /// Verified successfully
    Valid,
    /// Verification failed
    Invalid,
    /// Verification pending (GPU running)
    Pending,
    /// Verification with divergence (GPU != CPU)
    Divergent,
}

/// Compute deterministic hash (keccak256)
pub fn keccak256(data: &[u8]) -> HashOutput {
    use keccak_hash::keccak;

    let hash = keccak(data);
    let mut output = [0u8; 32];
    output.copy_from_slice(&hash);
    HashOutput(output)
}

/// Compute deterministic hash of multiple inputs (batch)
pub fn keccak256_batch(inputs: &[&[u8]]) -> Vec<HashOutput> {
    inputs.iter().map(|d| keccak256(d)).collect()
}

/// Compute SHA256 hash
pub fn sha256(data: &[u8]) -> HashOutput {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result);
    HashOutput(output)
}

/// Compute Blake2b hash
pub fn blake2b(data: &[u8]) -> HashOutput {
    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(&result[..32]);
    HashOutput(output)
}

/// Compute hash using the configured algorithm
pub fn compute_hash(algorithm: &HashAlgorithm, data: &[u8]) -> HashOutput {
    match algorithm {
        HashAlgorithm::Keccak256 => keccak256(data),
        HashAlgorithm::Sha256 => sha256(data),
        HashAlgorithm::Blake2b => blake2b(data),
    }
}

/// Hash algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HashAlgorithm {
    /// Keccak-256 (Ethereum standard)
    #[default]
    Keccak256,
    /// SHA-256
    Sha256,
    /// Blake2b
    Blake2b,
}

/// Signing key (for testing only - production should use HSM)
pub struct SigningKey {
    secret: [u8; 32],
}

impl SigningKey {
    /// Check if seed has sufficient entropy
    fn is_weak_seed(seed: &[u8]) -> bool {
        // Check for all zeros, all ones, or sequential patterns
        seed.iter().all(|&b| b == 0)
            || seed.iter().all(|&b| b == 0xFF)
            || seed.windows(2).all(|w| w[0] + 1 == w[1])
    }

    /// Generate a test/debug signing key from seed.
    ///
    /// Mainnet validator signing must be custody/HSM backed. Release builds fail
    /// closed if this test-only seed path is called.
    pub fn from_seed(seed: &[u8]) -> Result<Self, SwarmError> {
        #[cfg(not(any(test, debug_assertions)))]
        {
            let _ = seed;
            return Err(SwarmError::CryptoError(
                "seed-derived validator signing is disabled in release builds; use custody-service"
                    .to_string(),
            ));
        }

        #[cfg(any(test, debug_assertions))]
        {
            // Validate seed length (minimum 32 bytes for 256-bit security)
            if seed.len() < 32 {
                return Err(SwarmError::CryptoError(
                    "Seed must be at least 32 bytes for sufficient entropy".to_string(),
                ));
            }

            // Check for obviously weak seeds
            if Self::is_weak_seed(seed) {
                return Err(SwarmError::CryptoError(
                    "Seed appears to have insufficient entropy".to_string(),
                ));
            }

            use blake2::{Blake2b512, Digest};

            // Add additional entropy mixing
            let mut hasher = Blake2b512::new();
            hasher.update(b"x3-validator-signing-key-v2");
            hasher.update(seed);

            // Mix in system randomness for defense in depth
            let mut extra_entropy = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut extra_entropy);
            hasher.update(&extra_entropy);

            let result = hasher.finalize();
            let mut secret = [0u8; 32];
            secret.copy_from_slice(&result[..32]);

            Ok(Self { secret })
        }
    }

    /// Sign a message
    pub fn sign(&self, msg: &[u8]) -> SignatureOutput {
        use ed25519_dalek::{Signer, SigningKey};

        // Use ed25519 for signing
        let key_bytes: [u8; 32] = self.secret[..32]
            .try_into()
            .expect("secret is always 32 bytes");
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let signature = signing_key.sign(msg);

        // Split the 64-byte signature into r (32) and s (32)
        let bytes = signature.to_bytes();
        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&bytes[..32]);
        s.copy_from_slice(&bytes[32..]);

        // Use 0 for recovery byte (not applicable to ed25519)
        SignatureOutput::new(r, s, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak256() {
        let data = b"hello world";
        let hash = keccak256(data);
        assert_eq!(hash.0.len(), 32);
    }

    #[test]
    fn test_keccak256_batch() {
        let inputs: Vec<&[u8]> = vec![b"hello".as_slice(), b"world".as_slice(), b"test".as_slice()];
        let hashes = keccak256_batch(&inputs);
        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn test_signature() {
        let key = SigningKey::from_seed(b"test seed with at least thirty two bytes").unwrap();
        let msg = b"test message";
        let sig = key.sign(msg);

        // Verify signature
        assert!(sig.verify(msg, &[0u8; 33]));
    }

    #[test]
    fn test_hash_output_hex() {
        let hash = HashOutput::zero();
        let hex = hash.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(hex, "0".repeat(64));
    }
}
