//! X3 Standard Library: native crypto, math, and encoding primitives
//!
//! Provides optimized implementations of common operations for X3-Lang contracts.
//! Includes SHA-256, Blake2, math (sqrt, pow, log), and ABI encoding/decoding.

use sp_core::hashing::{blake2_256, keccak_256, sha256};

/// Cryptographic primitives
pub mod crypto {
    use super::*;

    /// SHA-256 hash (32 bytes output)
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        sha256(data)
    }

    /// Blake2b-256 hash
    pub fn blake2(data: &[u8]) -> [u8; 32] {
        blake2_256(data)
    }

    /// Keccak256 (Ethereum style)
    pub fn keccak256(data: &[u8]) -> [u8; 32] {
        keccak_256(data)
    }

    /// Simple signature verification structure
    pub struct Signature {
        pub r: [u8; 32],
        pub s: [u8; 32],
        pub v: u8, // recovery id (0-3 for ECDSA)
    }

    impl Signature {
        /// Verify ECDSA signature.
        ///
        /// Fail-closed until a full secp256k1 verifier is wired.
        pub fn verify_ecdsa(&self, message_hash: &[u8; 32], public_key: &[u8; 65]) -> bool {
            let _ = self;
            let _ = message_hash;
            let _ = public_key;
            false
        }
    }
}

/// Mathematical operations
pub mod math {
    /// Integer square root (Babylonian method)
    pub fn sqrt(n: u128) -> u128 {
        if n == 0 {
            return 0;
        }

        let mut x = n;
        let mut y = (x + 1) / 2;

        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }

        x
    }

    /// Safe power: x^y with overflow protection
    pub fn pow_safe(x: u128, y: u32) -> Option<u128> {
        let mut result = 1u128;

        for _ in 0..y {
            result = result.checked_mul(x)?;
        }

        Some(result)
    }

    /// Logarithm base 2 (integer)
    pub fn log2(n: u128) -> u32 {
        if n == 0 {
            return 0;
        }

        let mut result = 0u32;
        let mut x = n;

        while x > 1 {
            x /= 2;
            result += 1;
        }

        result
    }

    /// Modular exponentiation: (base ^ exp) % modulus
    pub fn mod_exp(base: u64, exp: u64, modulus: u64) -> u64 {
        if modulus == 1 {
            return 0;
        }

        let mut result = 1u64;
        let mut b = base % modulus;
        let mut e = exp;

        while e > 0 {
            if e % 2 == 1 {
                result = ((result as u128 * b as u128) % modulus as u128) as u64;
            }
            e /= 2;
            b = ((b as u128 * b as u128) % modulus as u128) as u64;
        }

        result
    }
}

/// ABI encoding/decoding (similar to Solidity ABI)
pub mod abi {
    /// Encode uint256 (big-endian, 32 bytes)
    pub fn encode_uint256(n: u128) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[0..16].copy_from_slice(&n.to_be_bytes());
        result
    }

    /// Decode uint256 from 32 bytes
    pub fn decode_uint256(data: &[u8; 32]) -> u128 {
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&data[0..16]);
        u128::from_be_bytes(bytes)
    }

    /// Encode address (20 bytes padded to 32)
    pub fn encode_address(addr: &[u8; 20]) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[12..32].copy_from_slice(addr);
        result
    }

    /// Decode address from 32 bytes
    pub fn decode_address(data: &[u8; 32]) -> [u8; 20] {
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&data[12..32]);
        addr
    }

    /// Encode bool (true = 1, false = 0 in big-endian)
    pub fn encode_bool(b: bool) -> [u8; 32] {
        let mut result = [0u8; 32];
        result[31] = if b { 1 } else { 0 };
        result
    }

    /// Decode bool from 32 bytes
    pub fn decode_bool(data: &[u8; 32]) -> bool {
        data[31] != 0
    }

    /// Encode bytes (with length prefix)
    pub fn encode_bytes(data: &[u8]) -> Vec<u8> {
        let len = (data.len() as u64).to_be_bytes();
        let mut result = Vec::new();
        result.extend_from_slice(&len);
        result.extend_from_slice(data);
        result
    }

    /// Decode bytes (expect length prefix)
    pub fn decode_bytes(data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 8 {
            return None;
        }

        let mut len_bytes = [0u8; 8];
        len_bytes.copy_from_slice(&data[0..8]);
        let len = u64::from_be_bytes(len_bytes) as usize;

        if data.len() < 8 + len {
            return None;
        }

        Some(data[8..8 + len].to_vec())
    }

    /// Packed encoding (for hashing: no padding between elements)
    pub fn pack_encode(elements: &[&[u8]]) -> Vec<u8> {
        let mut result = Vec::new();
        for elem in elements {
            result.extend_from_slice(elem);
        }
        result
    }
}

/// String utilities
pub mod string {
    /// Convert bytes to hex string (without 0x prefix)
    pub fn to_hex(data: &[u8]) -> String {
        data.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Parse hex string to bytes (accepts with or without 0x prefix)
    pub fn from_hex(hex: &str) -> Option<Vec<u8>> {
        let hex_str = hex.trim_start_matches("0x");

        if hex_str.len() % 2 != 0 {
            return None;
        }

        (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).ok())
            .collect()
    }

    /// UTF-8 string validation
    pub fn validate_utf8(data: &[u8]) -> bool {
        std::str::from_utf8(data).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = crypto::sha256(data);
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_sqrt_perfect() {
        assert_eq!(math::sqrt(16), 4);
        assert_eq!(math::sqrt(100), 10);
        assert_eq!(math::sqrt(1), 1);
    }

    #[test]
    fn test_sqrt_imperfect() {
        let sqrt_10 = math::sqrt(10);
        assert!(sqrt_10 == 3 || sqrt_10 == 4); // Floor/ceil
    }

    #[test]
    fn test_pow_safe() {
        assert_eq!(math::pow_safe(2, 3), Some(8));
        assert_eq!(math::pow_safe(10, 2), Some(100));
        assert_eq!(math::pow_safe(u128::MAX, 2), None); // Overflow
    }

    #[test]
    fn test_log2() {
        assert_eq!(math::log2(8), 3);
        assert_eq!(math::log2(16), 4);
        assert_eq!(math::log2(1), 0);
    }

    #[test]
    fn test_mod_exp() {
        assert_eq!(math::mod_exp(2, 3, 5), 3); // 2^3 % 5 = 8 % 5 = 3
        assert_eq!(math::mod_exp(3, 4, 7), 4); // 3^4 % 7 = 81 % 7 = 4
    }

    #[test]
    fn test_encode_decode_uint256() {
        let original: u128 = 12345;
        let encoded = abi::encode_uint256(original);
        let decoded = abi::decode_uint256(&encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_encode_decode_bool() {
        assert_eq!(abi::decode_bool(&abi::encode_bool(true)), true);
        assert_eq!(abi::decode_bool(&abi::encode_bool(false)), false);
    }

    #[test]
    fn test_encode_decode_address() {
        let addr = [1u8; 20];
        let encoded = abi::encode_address(&addr);
        let decoded = abi::decode_address(&encoded);
        assert_eq!(addr, decoded);
    }

    #[test]
    fn test_encode_decode_bytes() {
        let data = b"hello".to_vec();
        let encoded = abi::encode_bytes(&data);
        let decoded = abi::decode_bytes(&encoded);
        assert_eq!(decoded, Some(data));
    }

    #[test]
    fn test_hex_conversion() {
        let data = b"test";
        let hex = string::to_hex(data);
        let recovered = string::from_hex(&hex);
        assert_eq!(recovered, Some(data.to_vec()));
    }

    #[test]
    fn test_hex_with_prefix() {
        let data = b"x3";
        let hex = format!("0x{}", string::to_hex(data));
        let recovered = string::from_hex(&hex);
        assert_eq!(recovered, Some(data.to_vec()));
    }

    #[test]
    fn test_utf8_validation() {
        assert!(string::validate_utf8(b"hello"));
        assert!(!string::validate_utf8(&[0xFF, 0xFE]));
    }
}
