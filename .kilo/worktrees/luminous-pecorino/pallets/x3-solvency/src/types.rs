use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::{BoundedVec, Parameter};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use pallet_x3_inventory::types::{LaneId, ReservationId, RouteId, VaultId};

// ──────────────────────────────────────────────
// Type aliases
// ──────────────────────────────────────────────

/// Represents a solvency snapshot identifier (hash of context).
pub type SnapshotHash = [u8; 32];

// ──────────────────────────────────────────────
// Gate-check dimension enum
// ──────────────────────────────────────────────

/// Each value represents one dimension the solvency engine evaluated.
/// Failed checks are surfaced in `SolvencyResult::failed_checks`.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub enum SolvencyCheck {
    // Pre-quote / pre-reservation
    LaneFrozen,
    InsufficientVault,
    UnsettledCapBreached,
    RouteDuplicate,
    // Pre-submission
    ReservationExpired,
    ReservationNotActive,
    QuoteStale,
    SlippageExceeded,
    SignerPathUnhealthy,
    IncidentFlagged,
    ReconciliationLagged,
    PartnerReservationMissing,
    BridgePathMissing,
}

// ──────────────────────────────────────────────
// Gate result
// ──────────────────────────────────────────────

/// Returned by every gate function.  `passed == true` iff `failed_checks` is empty.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
#[scale_info(skip_type_params(MaxChecks))]
pub struct SolvencyResult<MaxChecks: frame_support::traits::Get<u32> + Clone> {
    pub passed: bool,
    pub failed_checks: BoundedVec<SolvencyCheck, MaxChecks>,
    pub snapshot_hash: SnapshotHash,
}

impl<MaxChecks: frame_support::traits::Get<u32> + Clone> SolvencyResult<MaxChecks> {
    pub fn pass(snapshot_hash: SnapshotHash) -> Self {
        Self { passed: true, failed_checks: BoundedVec::default(), snapshot_hash }
    }

    pub fn fail(
        failed_checks: BoundedVec<SolvencyCheck, MaxChecks>,
        snapshot_hash: SnapshotHash,
    ) -> Self {
        Self { passed: false, failed_checks, snapshot_hash }
    }
}

// ──────────────────────────────────────────────
// Input contexts
// ──────────────────────────────────────────────

/// Context supplied by callers of `check_pre_quote`.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct QuoteContext<Balance: Parameter + MaxEncodedLen> {
    pub lane_id: LaneId,
    pub vault_id: VaultId,
    pub amount: Balance,
    pub route_id: RouteId,
}

/// Context supplied by callers of `check_pre_reservation`.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct ReservationContext<Balance: Parameter + MaxEncodedLen> {
    pub lane_id: LaneId,
    pub vault_id: VaultId,
    pub amount: Balance,
    pub route_id: RouteId,
}

/// Context supplied by callers of `check_pre_submission`.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct SubmissionContext<Balance: Parameter + MaxEncodedLen, BlockNumber: Parameter + MaxEncodedLen>
{
    pub reservation_id: ReservationId,
    pub route_id: RouteId,
    pub vault_id: VaultId,
    pub lane_id: LaneId,
    pub amount: Balance,
    pub quote_block: BlockNumber,
    pub slippage_bps: u32,
    pub max_slippage_bps: u32,
}

/// Context supplied by callers of `record_post_submission`.
#[derive(Clone, PartialEq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct PostSubmissionContext<Balance: Parameter + MaxEncodedLen, BlockNumber: Parameter + MaxEncodedLen>
{
    pub reservation_id: ReservationId,
    pub route_id: RouteId,
    pub vault_id: VaultId,
    pub lane_id: LaneId,
    pub amount: Balance,
    pub submission_block: BlockNumber,
    pub submission_hash: SnapshotHash,
}

// ──────────────────────────────────────────────
// Snapshot record (TICKET-4.5-010)
// ──────────────────────────────────────────────

/// Sealed record of a solvency gate evaluation stored on-chain.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
#[scale_info(skip_type_params(MaxChecks))]
pub struct SolvencySnapshotRecord<BlockNumber: Parameter + MaxEncodedLen, MaxChecks: frame_support::traits::Get<u32> + Clone> {
    pub block_number: BlockNumber,
    pub passed: bool,
    pub failed_checks: BoundedVec<SolvencyCheck, MaxChecks>,
    pub route_id: RouteId,
    pub reservation_id: ReservationId,
    pub context_hash: SnapshotHash,
    /// true while a live reservation or pending obligation references this snapshot
    pub referenced: bool,
}

// ──────────────────────────────────────────────
// Pending obligation record (TICKET-4.5-009)
// ──────────────────────────────────────────────

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct PendingObligation<Balance: Parameter + MaxEncodedLen, BlockNumber: Parameter + MaxEncodedLen> {
    pub route_id: RouteId,
    pub reservation_id: ReservationId,
    pub amount: Balance,
    pub timeout_block: BlockNumber,
    pub snapshot_hash: SnapshotHash,
    pub submission_hash: SnapshotHash,
}

/// Evidence record sealed after a successful submission.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
pub struct EvidenceRecord<BlockNumber: Parameter + MaxEncodedLen> {
    pub route_id: RouteId,
    pub reservation_id: ReservationId,
    pub submission_hash: SnapshotHash,
    pub block_timestamp: BlockNumber,
    pub snapshot_hash: SnapshotHash,
}
