//! Types for the x3-slash pallet.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;

/// Bond state stored on-chain.
#[derive(
    Clone,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
#[scale_info(skip_type_params(AccountId, Balance))]
pub struct BondState<AccountId, Balance> {
    /// Unique bond identifier.
    pub bond_id: H256,
    /// Agent who posted the bond.
    pub agent: AccountId,
    /// Amount bonded.
    pub amount: Balance,
    /// Block at which the bond was posted.
    pub posted_at: u32,
    /// Block at which the bond expires.
    pub expires_at: u32,
    /// Associated intent ID (if any).
    pub intent_id: Option<H256>,
    /// Current status of the bond.
    pub status: BondStatus,
}

/// Bond lifecycle status.
#[derive(
    Clone,
    Copy,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
pub enum BondStatus {
    /// Bond is active and locked.
    Active,
    /// Bond has been fully slashed.
    FullySlashed,
    /// Bond has been released back to the agent.
    Released,
    /// Bond expired without settlement.
    Expired,
}

/// Slash record stored on-chain (immutable history).
#[derive(
    Clone,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
    PartialEq,
    Eq,
)]
#[scale_info(skip_type_params(AccountId))]
pub struct SlashRecord<AccountId> {
    /// Unique slash identifier.
    pub slash_id: u64,
    /// Agent being slashed.
    pub agent: AccountId,
    /// Bond being slashed.
    pub bond_id: H256,
    /// Severity of the slash (0=Minor, 1=Moderate, 2=Major, 3=Critical).
    pub severity: u8,
    /// Amount slashed.
    pub amount_slashed: u128,
    /// Reason for the slash (encoded).
    pub reason: BoundedVec<u8, ConstU32<256>>,
    /// Block at which the slash was executed.
    pub slashed_at: u32,
}
