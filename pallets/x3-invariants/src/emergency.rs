//! Emergency control types for Phase 0 constitutional controls.
//!
//! Defines the data structures stored in:
//! - `EmergencyAuthorities` — who may activate a module kill switch.
//! - `KillSwitchEvidence`   — sealed incident bundles justifying activation.
//! - `CanonicalTruthMap`    — the single authoritative storage source per domain.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Opaque identifier for a module or subsystem (max 32 bytes).
pub type ModuleId = [u8; 32];

/// Opaque identifier for a canonical truth domain (e.g. `"balances"`, `"receipts"`, `"fee_events"`).
pub type DomainId = [u8; 32];

/// Who holds emergency authority over a module.
///
/// An authority record is stored per `ModuleId`. A holder of the corresponding
/// private key may activate the module's kill switch before `expires_at_block`.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct AuthorityRecord<BlockNumber> {
    /// Hash of the authority key (account public key or committee ID).
    ///
    /// Compared against `blake2_256(origin.encode())` at activation time.
    pub authority_id: [u8; 32],
    /// Block number at which this authority record expires.
    ///
    /// Activation is rejected when `current_block >= expires_at_block`.
    pub expires_at_block: BlockNumber,
    /// Whether a signed evidence bundle hash is required before activating a kill switch.
    pub requires_evidence: bool,
    /// Module name hash (UTF-8 hash, for operator readability; not enforced on-chain).
    pub module_name_hash: [u8; 32],
}

/// Which pallet/storage item is the authoritative source for a given truth domain.
///
/// Stored in `CanonicalTruthMap`. Only one record may exist per `DomainId`.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct TruthSourceRecord {
    /// Domain this record covers.
    pub domain_id: DomainId,
    /// Pallet name hash (identifies which pallet is authoritative).
    pub pallet_name_hash: [u8; 32],
    /// Storage item name hash within that pallet.
    pub storage_item_hash: [u8; 32],
    /// Human-readable description hash.
    pub description_hash: [u8; 32],
}

/// A sealed evidence bundle submitted to justify activating a kill switch.
///
/// Stored in `KillSwitchEvidence` when evidence is provided with the activation call.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
pub struct EvidenceBundle<BlockNumber> {
    /// Hash of the off-chain incident evidence document.
    pub evidence_hash: [u8; 32],
    /// Block at which evidence was submitted.
    pub submitted_at: BlockNumber,
    /// `blake2_256` hash of the submitter's account encoding.
    pub submitter: [u8; 32],
}
