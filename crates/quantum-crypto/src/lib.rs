#![warn(unused_imports, unused_variables)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::should_implement_trait)]

//! Quantum-Resistant Cryptography for X3 Chain
//!
//! This crate provides post-quantum cryptographic primitives to future-proof
//! X3 Chain against quantum computing attacks.
//!
//! # Algorithms
//!
//! - **SPHINCS+**: research implementation with standard-conforming sizes
//! - **Kyber (research/simulated)**: `kyber_research` — deterministic hash
//!   expansion, NOT the CRYSTALS-Kyber lattice algorithm
//! - **Dilithium (research/simulated)**: `dilithium_research` — deterministic
//!   hash expansion, NOT CRYSTALS-Dilithium lattice signatures
//! - **BLAKE3+**: extended hash functions for quantum resistance
//!
//! # Security status
//!
//! **This crate is a research/simulated implementation, not production
//! post-quantum cryptography.** No standard PQC library (`pqcrypto`,
//! `liboqs`, etc.) is linked here. Byte sizes mimic real parameter sets so
//! downstream layout experiments are possible, but any use for real security
//! is forbidden until audited bindings are integrated. The runtime exposes
//! this crate only behind its `pq` feature, which is disabled by default and
//! intentionally fails closed in release configuration.

#![allow(dead_code)]
#![allow(unused_variables)]

#[path = "dilithium.rs"]
pub mod dilithium_research;
pub mod blake3ext;
pub mod error;
pub mod hash;
#[path = "kyber.rs"]
pub mod kyber_research;
pub mod sphincs;
pub mod types;

pub use error::{QuantumError, QuantumResult};
pub use types::*;

/// Quantum Crypto version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Security level for quantum-resistant operations
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// NIST Level 1 (AES-128 equivalent)
    Level1,
    /// NIST Level 3 (AES-192 equivalent)
    #[default]
    Level3,
    /// NIST Level 5 (AES-256 equivalent)
    Level5,
}

impl SecurityLevel {
    /// Convert to numeric level (1, 3, or 5)
    pub fn to_u8(self) -> u8 {
        match self {
            SecurityLevel::Level1 => 1,
            SecurityLevel::Level3 => 3,
            SecurityLevel::Level5 => 5,
        }
    }
    /// Convert from numeric level (1, 3, or 5)
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => SecurityLevel::Level1,
            5 => SecurityLevel::Level5,
            _ => SecurityLevel::Level3,
        }
    }
}

/// Combined quantum-resistant keypair
pub struct QuantumKeypair {
    /// SPHINCS+ keypair for signatures
    pub sphincs: sphincs::SphincsKeypair,
    /// Research/simulated Kyber-shaped keypair for key encapsulation
    pub kyber_research: kyber_research::KyberKeypair,
    /// Research/simulated Dilithium-shaped keypair for signatures
    pub dilithium_research: dilithium_research::DilithiumKeypair,
}

impl QuantumKeypair {
    /// Generate a new quantum-resistant keypair
    pub fn generate(level: SecurityLevel) -> Self {
        Self {
            sphincs: sphincs::SphincsKeypair::generate(level),
            kyber_research: kyber_research::KyberKeypair::generate(level),
            dilithium_research: dilithium_research::DilithiumKeypair::generate(level),
        }
    }

    /// Sign a message using SPHINCS+ (most secure, larger signatures)
    pub fn sign_sphincs(&self, message: &[u8]) -> sphincs::SphincsSignature {
        self.sphincs.sign(message)
    }

    /// Sign a message using the simulated Dilithium-shaped research keypair
    pub fn sign_dilithium_research(&self, message: &[u8]) -> dilithium_research::DilithiumSignature {
        self.dilithium_research.sign(message)
    }

    /// Encapsulate a shared secret using the simulated Kyber-shaped research keypair
    pub fn encapsulate(
        &self,
        recipient_pk: &kyber_research::KyberPublicKey,
    ) -> (kyber_research::KyberCiphertext, kyber_research::SharedSecret) {
        kyber_research::encapsulate(recipient_pk)
    }

    /// Decapsulate a shared secret using Kyber
    pub fn decapsulate(
        &self,
        ciphertext: &kyber_research::KyberCiphertext,
    ) -> QuantumResult<kyber_research::SharedSecret> {
        self.kyber_research.decapsulate(ciphertext)
    }
}

/// Quantum-resistant hash function wrapper
pub fn quantum_hash(data: &[u8]) -> hash::QuantumHash {
    hash::quantum_hash(data)
}

/// Quantum-resistant address derivation
pub fn derive_quantum_address(pubkey: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(pubkey);
    let mut address = [0u8; 32];
    address.copy_from_slice(hash.as_bytes());
    address
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = QuantumKeypair::generate(SecurityLevel::Level3);
        assert!(!keypair.sphincs.public_key.as_bytes().is_empty());
    }

    #[test]
    fn test_quantum_hash() {
        let data = b"test data";
        let hash1 = quantum_hash(data);
        let hash2 = quantum_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_security_level_conversion() {
        assert_eq!(SecurityLevel::from_u8(1), SecurityLevel::Level1);
        assert_eq!(SecurityLevel::from_u8(3), SecurityLevel::Level3);
        assert_eq!(SecurityLevel::from_u8(5), SecurityLevel::Level5);
        assert_eq!(SecurityLevel::Level1.to_u8(), 1);
        assert_eq!(SecurityLevel::Level3.to_u8(), 3);
        assert_eq!(SecurityLevel::Level5.to_u8(), 5);
    }
}
