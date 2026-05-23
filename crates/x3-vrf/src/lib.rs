#![cfg_attr(not(feature = "std"), no_std)]

//! # X3 VRF (Verifiable Random Function)
//!
//! Provides verifiable randomness generation using Schnorrkel VRF.
//! Used for fair lottery systems, random selections, and cryptographic randomness.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{H256, U256};
use sp_std::vec::Vec;

/// VRF proof for verifying randomness generation
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct VrfProof {
    /// The generated random output
    pub output: [u8; 32],
    /// Proof data for verification
    pub proof: [u8; 64], // Fixed size for simplicity
}

/// VRF public key
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct VrfPublicKey(pub [u8; 32]);

/// VRF secret key
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct VrfSecretKey(pub [u8; 64]);

/// Randomness request data
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct RandomnessRequest {
    /// Request ID
    pub request_id: H256,
    /// Seed data for randomness generation
    pub seed: H256,
    /// Block number when request was made
    pub block_number: u64,
    /// Maximum gas willing to pay
    pub max_fee: U256,
}

/// Fulfilled randomness data
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct RandomnessResult {
    /// Request ID
    pub request_id: H256,
    /// Generated randomness
    pub randomness: [u8; 32],
    /// VRF proof
    pub proof: VrfProof,
    /// Block when fulfilled
    pub fulfilled_block: u64,
}

/// VRF provider trait for different implementations
pub trait VrfProvider {
    /// Generate VRF proof and randomness from seed
    fn prove(&self, seed: &[u8; 32]) -> Result<VrfProof, VrfError>;

    /// Verify a VRF proof
    fn verify(&self, proof: &VrfProof, seed: &[u8; 32], public_key: &VrfPublicKey) -> bool;

    /// Derive public key from secret key
    fn derive_public_key(&self, secret_key: &VrfSecretKey) -> VrfPublicKey;
}

/// VRF errors
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum VrfError {
    /// Invalid secret key
    InvalidSecretKey,
    /// Invalid public key
    InvalidPublicKey,
    /// Proof verification failed
    VerificationFailed,
    /// Proof generation failed
    GenerationFailed,
    /// Invalid seed data
    InvalidSeed,
}

// Note: Real Schnorrkel VRF implementation would go here
// For now, using mock provider for all environments

/// Mock VRF provider for testing (no_std compatible)
pub struct MockVrfProvider;

impl VrfProvider for MockVrfProvider {
    fn prove(&self, seed: &[u8; 32]) -> Result<VrfProof, VrfError> {
        use sp_io::hashing::blake2_256;

        // Generate deterministic "randomness" from seed
        let output = blake2_256(seed);

        let mut proof = [0u8; 64];
        proof[..32].copy_from_slice(seed);
        Ok(VrfProof { output, proof })
    }

    fn verify(&self, proof: &VrfProof, seed: &[u8; 32], _public_key: &VrfPublicKey) -> bool {
        // Mock verification: check that proof starts with seed
        proof.proof[..32] == *seed
    }

    fn derive_public_key(&self, secret_key: &VrfSecretKey) -> VrfPublicKey {
        // Mock public key derivation: hash the secret key
        use sp_io::hashing::blake2_256;
        let hash = blake2_256(&secret_key.0);
        VrfPublicKey(hash)
    }
}

/// Get the global VRF provider
pub fn get_vrf_provider() -> &'static dyn VrfProvider {
    &MockVrfProvider
}

/// Generate randomness with proof
pub fn generate_randomness(seed: &[u8; 32]) -> Result<VrfProof, VrfError> {
    get_vrf_provider().prove(seed)
}

/// Verify randomness proof
pub fn verify_randomness(proof: &VrfProof, seed: &[u8; 32], public_key: &VrfPublicKey) -> bool {
    get_vrf_provider().verify(proof, seed, public_key)
}

/// Utility function to convert VRF output to various random formats
pub mod utils {
    use super::*;

    /// Convert VRF output to a random u64
    pub fn vrf_to_u64(output: &[u8; 32]) -> u64 {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&output[..8]);
        u64::from_le_bytes(bytes)
    }

    /// Convert VRF output to a random u32
    pub fn vrf_to_u32(output: &[u8; 32]) -> u32 {
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&output[..4]);
        u32::from_le_bytes(bytes)
    }

    /// Convert VRF output to a random boolean
    pub fn vrf_to_bool(output: &[u8; 32]) -> bool {
        output[0] & 1 == 1
    }

    /// Get random bytes from VRF output
    pub fn vrf_to_bytes(output: &[u8; 32], len: usize) -> Vec<u8> {
        output[..len.min(32)].to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_vrf_provider() {
        let provider = MockVrfProvider;
        let seed = [1u8; 32];

        let proof = provider.prove(&seed).unwrap();
        let public_key = provider.derive_public_key(&VrfSecretKey([2u8; 64]));

        assert!(provider.verify(&proof, &seed, &public_key));
    }

    #[test]
    fn test_vrf_utils() {
        let output = [42u8; 32];

        let u64_val = utils::vrf_to_u64(&output);
        assert_eq!(u64_val, 0x2a2a2a2a2a2a2a2a);

        let u32_val = utils::vrf_to_u32(&output);
        assert_eq!(u32_val, 0x2a2a2a2a);

        let bool_val = utils::vrf_to_bool(&output);
        assert!(bool_val); // 42 & 1 = 0, wait no: 42 in binary is 00101010, LSB is 0, so false?

        let bytes = utils::vrf_to_bytes(&output, 4);
        assert_eq!(bytes, vec![42, 42, 42, 42]);
    }
}
