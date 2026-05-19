//! Canonical identity model for X3.
//!
//! All types in this module implement SCALE encode/decode, `TypeInfo`, and
//! serde serialization for use across runtime, services, and client SDKs.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

// ─── Serde helper for [u8; 64] ────────────────────────────────────────────────
//
// serde_core (the crate that backs `serde` in this workspace) implements
// Serialize/Deserialize for fixed arrays only up to size 32.  The 64-byte
// address buffer requires a manual `with`-module so we keep the derive macro
// on ChainAddress while still satisfying the trait bounds.

mod serde_array64 {
    use core::fmt;
    use serde::{
        de::{self, SeqAccess, Visitor},
        ser::SerializeTuple,
        Deserializer, Serializer,
    };

    pub fn serialize<S: Serializer>(arr: &[u8; 64], ser: S) -> Result<S::Ok, S::Error> {
        let mut tup = ser.serialize_tuple(64)?;
        for b in arr {
            tup.serialize_element(b)?;
        }
        tup.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deser: D) -> Result<[u8; 64], D::Error> {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = [u8; 64];

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a sequence of exactly 64 bytes")
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 64], A::Error> {
                let mut arr = [0u8; 64];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq.next_element()?.ok_or_else(|| {
                        de::Error::invalid_length(i, &"exactly 64 bytes")
                    })?;
                }
                Ok(arr)
            }
        }

        deser.deserialize_tuple(64, ArrayVisitor)
    }
}

// ─── Chain family ─────────────────────────────────────────────────────────────

/// Chain families supported by X3.
///
/// SCALE-encoded as a single byte (0–3). Add new variants at the end to preserve
/// backward-compatible encoding.
#[derive(
    Clone, PartialEq, Eq, Debug,
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    Serialize, Deserialize,
)]
pub enum ChainFamily {
    Substrate = 0,
    Evm = 1,
    Svm = 2,
    Bitcoin = 3,
}

// ─── Chain address ────────────────────────────────────────────────────────────

/// A chain-specific account address (raw bytes, up to 64 bytes).
///
/// `address_bytes` is a fixed 64-byte buffer; `address_len` records the number
/// of bytes actually used (e.g. 20 for an EVM address, 32 for a Substrate key).
#[derive(
    Clone, PartialEq, Eq, Debug,
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    Serialize, Deserialize,
)]
pub struct ChainAddress {
    /// CAIP-2 numeric chain identifier. 0 means the native X3 chain.
    pub chain_id: u32,
    /// Chain family that determines how `address_bytes` should be interpreted.
    pub family: ChainFamily,
    /// Raw address bytes. Only the first `address_len` bytes are meaningful.
    #[serde(with = "serde_array64")]
    pub address_bytes: [u8; 64],
    /// Number of bytes used in `address_bytes` (must be <= 64).
    pub address_len: u8,
}

// ─── KYC tier ─────────────────────────────────────────────────────────────────

/// KYC verification tier for a canonical identity.
#[derive(
    Clone, PartialEq, Eq, Debug,
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    Serialize, Deserialize,
)]
pub enum KycTier {
    None = 0,
    Basic = 1,
    Enhanced = 2,
    Institutional = 3,
}

// ─── Canonical identity ────────────────────────────────────────────────────────

/// Canonical cross-chain identity for a user.
///
/// The primary account is the X3 Substrate account. All other linked addresses
/// on foreign chains are summarised by `identity_hash` (a blake2-256 digest of
/// the primary account concatenated with every linked `ChainAddress` encoding).
///
/// # Generic Parameter
///
/// `AccountId` is the Substrate `AccountId32` type in runtime contexts and can
/// be substituted with `[u8; 32]` in off-chain / test contexts.
#[derive(
    Clone, PartialEq, Eq, Debug,
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    Serialize, Deserialize,
)]
pub struct CanonicalIdentity<AccountId> {
    /// Primary X3 Substrate account.
    pub primary: AccountId,
    /// Blake2-256 hash of the primary account concatenated with all linked addresses.
    pub identity_hash: [u8; 32],
    /// Whether this identity is registered with the on-chain governance pallet.
    pub governance_registered: bool,
    /// Highest KYC tier reached by this identity.
    pub kyc_tier: KycTier,
}

// ─── Governance record ────────────────────────────────────────────────────────

/// Governance registration record — maps an identity hash to voting weight.
#[derive(
    Clone, PartialEq, Eq, Debug,
    Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    Serialize, Deserialize,
)]
pub struct GovernanceRecord {
    /// Blake2-256 identity hash linking this record to a [`CanonicalIdentity`].
    pub identity_hash: [u8; 32],
    /// Total voting power derived from staked/locked tokens at registration.
    pub total_voting_power: u128,
    /// Block number at which the identity registered for governance.
    pub registration_block: u32,
}
