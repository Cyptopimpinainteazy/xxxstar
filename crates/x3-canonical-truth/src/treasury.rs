//! Canonical treasury model for X3.
//!
//! All types in this module implement SCALE encode/decode, `TypeInfo`, and
//! serde serialization for use across runtime, services, and client SDKs.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

// ─── Treasury account type ────────────────────────────────────────────────────

/// Classification of treasury reserve accounts.
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
pub enum TreasuryAccountType {
    /// Day-to-day settlement float used to fund cross-chain operations.
    OperationalFloat = 0,
    /// Ring-fenced reserve drawn only on protocol-level insurance events.
    InsuranceReserve = 1,
    /// Long-term strategic holdings subject to governance-level approval.
    StrategicReserve = 2,
    /// Reserve dedicated to covering validator and relayer gas costs.
    GasReserve = 3,
}

// ─── Treasury snapshot ────────────────────────────────────────────────────────

/// Point-in-time snapshot of the treasury state at a given block.
///
/// Snapshots are produced by the treasury sidecar on each epoch boundary and
/// committed to the proving harness for reconciliation checks.
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
pub struct TreasurySnapshot {
    /// Blake2-256 hash uniquely identifying this snapshot (includes block number
    /// and all reserve totals to prevent tampering).
    pub snapshot_hash: [u8; 32],
    /// Block number at which the snapshot was taken.
    pub block_number: u32,
    /// Total value in the operational float reserve (atomic units of the reserve asset).
    pub operational_float_total: u128,
    /// Total value in the insurance reserve.
    pub insurance_reserve_total: u128,
    /// Total value in the strategic reserve.
    pub strategic_reserve_total: u128,
    /// Total value in the gas reserve.
    pub gas_reserve_total: u128,
    /// Portion of the operational float currently deployed to settlement vaults.
    pub deployed_settlement_float: u128,
    /// Total exposure currently at risk across all active positions.
    pub at_risk_exposure: u128,
}

// ─── Treasury action kind ─────────────────────────────────────────────────────

/// Classification of treasury actions recorded in the audit trail.
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
pub enum TreasuryActionKind {
    /// Funds were deposited into a treasury vault.
    FundVault = 0,
    /// Funds were withdrawn from a treasury vault.
    WithdrawVault = 1,
    /// Funds were allocated from reserves to a capitalisation target.
    AllocateToCap = 2,
    /// The insurance reserve was drawn to cover a deficit.
    InsuranceDraw = 3,
    /// A treasury action was approved via governance vote.
    GovernanceApproval = 4,
}

// ─── Treasury action ──────────────────────────────────────────────────────────

/// A single treasury action committed to the on-chain audit trail.
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
pub struct TreasuryAction {
    /// What kind of treasury action was performed.
    pub action_kind: TreasuryActionKind,
    /// Asset identifier involved in the action.
    pub asset_id: u32,
    /// Amount of the asset moved (atomic units).
    pub amount: u128,
    /// CAIP-2 chain ID where the action was executed.
    pub chain_id: u32,
    /// Blake2-256 hash of the executor's account (governance or multisig key).
    pub executor_hash: [u8; 32],
    /// Block number at which the action was executed.
    pub block_number: u32,
}

// ─── Treasury reconciliation report ───────────────────────────────────────────

/// Reconciliation report — canonical truth of treasury state after an epoch.
///
/// Produced by the treasury sidecar and committed to the proving harness.
/// A failed reconciliation (divergence > policy threshold) triggers an on-chain
/// alert via the `x3-invariants` pallet.
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
pub struct TreasuryReconciliationReport {
    /// Blake2-256 hash of the snapshot being reconciled.
    pub snapshot_hash: [u8; 32],
    /// Measured divergence from the prior snapshot expressed in basis points.
    /// Values above the governance-configured threshold cause `passed = false`.
    pub divergence_bps: u32,
    /// `true` if all reconciliation checks passed; `false` on any failure.
    pub passed: bool,
    /// Number of treasury actions recorded since the previous reconciliation.
    pub action_count_since_last: u32,
    /// Block number at which the report was generated.
    pub block_number: u32,
}
