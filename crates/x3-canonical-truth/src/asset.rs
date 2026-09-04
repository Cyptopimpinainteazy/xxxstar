//! Canonical asset model for X3.
//!
//! All types in this module implement SCALE encode/decode, `TypeInfo`, and
//! serde serialization for use across runtime, services, and client SDKs.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

// ─── Asset origin ─────────────────────────────────────────────────────────────

/// Asset origin classification.
///
/// Determines how supply accounting and lock-proof requirements are applied.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub enum AssetOrigin {
    /// Natively minted on the X3 chain (e.g. ATOM3).
    Native = 0,
    /// Wrapped representation of an asset locked on another chain.
    Wrapped = 1,
    /// Bridged via an external cross-chain bridge protocol.
    Bridged = 2,
    /// Synthetically minted against collateral; no external lock required.
    Synthetic = 3,
}

// ─── Canonical asset ──────────────────────────────────────────────────────────

/// Canonical asset descriptor — chain-agnostic identifier for any X3-tracked asset.
///
/// Implements `MaxEncodedLen` because it contains only fixed-size fields and is
/// suitable for use as a Substrate `StorageMap` value.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub struct CanonicalAsset {
    /// Numeric asset identifier, unique within the X3 asset registry.
    pub asset_id: u32,
    /// How this asset came to exist on X3.
    pub origin: AssetOrigin,
    /// CAIP-2 numeric chain ID where the asset is natively minted or locked.
    pub canonical_chain_id: u32,
    /// Token decimal places (e.g. 18 for most EVM tokens, 6 for USDC).
    pub decimals: u8,
    /// Blake2-256 hash of the ticker symbol encoded as UTF-8.
    pub symbol_hash: [u8; 32],
    /// Whether the asset is currently active and tradeable on X3.
    pub is_active: bool,
}

// ─── Chain supply entry ───────────────────────────────────────────────────────

/// Cross-chain supply entry for one (chain, asset) pair.
///
/// The net circulating supply on a given chain is `minted - burned - locked`.
/// `pending_in` and `pending_out` track in-flight bridge transfers.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub struct ChainSupplyEntry {
    /// CAIP-2 numeric chain identifier.
    pub chain_id: u32,
    /// Numeric asset identifier (matches [`CanonicalAsset::asset_id`]).
    pub asset_id: u32,
    /// Total amount minted on this chain (atomic units).
    pub minted: u128,
    /// Total amount permanently burned on this chain (atomic units).
    pub burned: u128,
    /// Amount currently locked as collateral for cross-chain wraps.
    pub locked: u128,
    /// Amount pending inbound bridging (initiated but not yet finalised).
    pub pending_in: u128,
    /// Amount pending outbound bridging (initiated but not yet finalised).
    pub pending_out: u128,
}

// ─── Canonical lock proof ─────────────────────────────────────────────────────

/// Proof that a canonical lock exists on the source chain justifying a wrapped mint.
///
/// The `proof_hash` is a blake2-256 digest of the lock transaction receipt,
/// committed to the X3 chain by the bridge oracle.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Debug,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    Serialize,
    Deserialize,
)]
pub struct CanonicalLockProof {
    /// Asset identifier for the locked asset.
    pub asset_id: u32,
    /// CAIP-2 chain ID where the asset is locked.
    pub source_chain_id: u32,
    /// CAIP-2 chain ID where the wrapped token is minted.
    pub dest_chain_id: u32,
    /// Locked amount in atomic units.
    pub locked_amount: u128,
    /// Blake2-256 hash of the on-chain lock receipt used to verify the proof.
    pub proof_hash: [u8; 32],
    /// Block number on the source chain at which the lock was confirmed.
    pub block_number: u32,
}
