//! Types for X3 Post-Quantum Cryptography

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQKeyPair {
    pub public_key: PQPublicKey,
    pub private_key: PQPrivateKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQPublicKey(pub Vec<u8>);

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQPrivateKey(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQSignature(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct HybridSignature {
    pub classical: sp_core::sr25519::Signature,
    pub post_quantum: PQSignature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum PQScheme {
    /// CRYSTALS-Dilithium3 (NIST Round 3 finalist)
    Dilithium3,
    /// Falcon-512 (NIST Round 3 finalist)
    Falcon512,
    /// Sphincs+-256 (NIST Round 3 finalist)
    Sphincs256,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct KeyRotationSchedule {
    /// Last rotation timestamp
    pub last_rotation: u64,
    /// Rotation interval (in blocks)
    pub rotation_interval: u64,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQValidatorIdentity {
    /// Validator ID
    pub validator_id: u64,
    /// PQ public key
    pub pq_public_key: PQPublicKey,
    /// Last key rotation
    pub last_rotation: u64,
}

#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PQAccountConfig {
    /// Account address
    pub account: sp_core::H160,
    /// PQ scheme enabled
    pub pq_scheme: Option<PQScheme>,
    /// Hybrid mode enabled
    pub hybrid_enabled: bool,
    /// Last key rotation
    pub last_rotation: u64,
}
