//! Core types shared across all Phase 4.5 modules.
//!
//! No storage, no logic, no extrinsics — types only.
//! Every type must implement Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Clone, PartialEq.

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Primitive aliases
// ---------------------------------------------------------------------------

/// Opaque chain identifier (u32 wrapping a CAIP-2 numeric ID).
pub type ChainId = u32;

/// Opaque asset identifier (u32; mapped to a registered asset record).
pub type AssetId = u32;

/// Opaque route identifier.
pub type RouteId = [u8; 32];

// ---------------------------------------------------------------------------
// Vault types
// ---------------------------------------------------------------------------

/// Unique vault identifier: hash of (chain_id, asset_id, vault_type).
pub type VaultId = [u8; 32];

/// Purpose class of a vault.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VaultType {
    /// Chain gas tokens and operational fee assets.
    Gas,
    /// Route settlement balances for approved corridors.
    SettlementFloat,
    /// Strategic reserve capital — not accessible by the router.
    TreasuryReserve,
    /// Funds for approved loss events and failed settlement recovery.
    InsuranceLoss,
}

/// Ownership class of a vault.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OwnerType {
    Protocol,
    Treasury,
    Partner,
}

/// Operational health of a vault.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VaultStatus {
    /// Normal operation; all bands satisfied.
    Active,
    /// Below `min` band; rebalance triggered.
    Degraded,
    /// Below `critical_min`; new reservations rejected.
    Frozen,
}

// ---------------------------------------------------------------------------
// Lane types
// ---------------------------------------------------------------------------

/// Unique lane identifier.
pub type LaneId = [u8; 32];

/// Risk / execution class of a lane.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LaneClass {
    /// Public liquidity only; no protocol backstop.
    A,
    /// Partner-backed; approved counterparties with depth commitments.
    B,
    /// Protocol-backed strategic corridor; strictest monitoring.
    C,
}

/// Operational health of a lane.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LaneStatus {
    Active,
    Degraded,
    Frozen,
}

// ---------------------------------------------------------------------------
// Reservation types
// ---------------------------------------------------------------------------

/// Unique reservation identifier.
pub type ReservationId = [u8; 32];

/// Lifecycle state of a reservation.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ReservationStatus {
    /// Inventory is held; route may proceed.
    Active,
    /// Expiry block passed before consumption.
    Expired,
    /// Explicitly released before expiry.
    Released,
    /// Consumed by a successful submission.
    Consumed,
}

// ---------------------------------------------------------------------------
// Partner types
// ---------------------------------------------------------------------------

/// Unique partner identifier.
pub type PartnerId = [u8; 32];

/// Lifecycle state of a partner.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum PartnerStatus {
    Active,
    /// Temporarily suspended; routes blocked.
    Suspended,
    /// Permanently removed from the control plane.
    Terminated,
}

// ---------------------------------------------------------------------------
// Liquidity source types
// ---------------------------------------------------------------------------

/// Which liquidity bucket a route consumes.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum LiquiditySourceType {
    /// Offsetting internal user demand; no external movement.
    InternalNetting,
    /// DEX pool, aggregator, order book, or bridge liquidity.
    ExternalMarket,
    /// Named partner LP or market-maker.
    Partner,
    /// Protocol-owned settlement float (approved corridors only).
    ProtocolFloat,
}

// ---------------------------------------------------------------------------
// Freeze / unfreeze types
// ---------------------------------------------------------------------------

/// Why a lane was frozen.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum FreezeReason {
    BalanceBelowCriticalMin,
    UnresolvedSettlementFailure,
    PartnerDefault,
    QuoteIntegrityFailure,
    ChainHealthDegraded,
    ReconciliationMismatch,
    OperatorManual,
}

/// Evidence bundle submitted when unfreezing a lane.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OperatorEvidence {
    /// Free-form UTF-8 description (bounded to 256 bytes).
    pub description_hash: [u8; 32],
    /// Block number at which the operator submitted the evidence.
    pub submitted_at_block: u32,
    /// Operator account hash (32-byte public key or account ID).
    pub operator_id: [u8; 32],
}

// ---------------------------------------------------------------------------
// Solvency types
// ---------------------------------------------------------------------------

/// A single dimension that the solvency engine evaluates.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum SolvencyCheck {
    SourceChainHealth,
    DestChainHealth,
    RouteComponentHealth,
    AssetActivation,
    LaneFrozen,
    QuoteFreshness,
    TentativeCapacity,
    SourceVaultSufficiency,
    DestinationCapacity,
    GasReserveSufficiency,
    ExposureCap,
    UnsettledNotional,
    PartnerHealth,
    RouteProfitability,
    QuarantineStatus,
    ReservationValidity,
    SlippageBounds,
    SignerPathHealth,
    IncidentFlag,
    ReconciliationLag,
    PartnerReservationLive,
    BridgePathExists,
}

/// Result returned by every solvency gate evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SolvencyResult {
    pub passed: bool,
    /// All checks that failed; empty when `passed` is true.
    pub failed_checks: sp_std::vec::Vec<SolvencyCheck>,
    /// Deterministic hash of the full evaluation context.
    pub snapshot_hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// Rebalance types
// ---------------------------------------------------------------------------

/// Unique venue identifier.
pub type VenueId = [u8; 32];

/// What triggered a rebalance.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RebalanceTrigger {
    BelowMinBand { vault_id: VaultId },
    DemandSpike { lane_id: LaneId },
    ConcentrationBreach { chain_id: ChainId },
    PartnerCapacityLoss { partner_id: PartnerId },
    VenueLiquidityCollapse { venue_id: VenueId },
    PersistentOneWayFlow { lane_id: LaneId },
    ChainDegradation { chain_id: ChainId },
}

/// Which rebalance step successfully resolved the shortage.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum RebalanceMethod {
    InternalNetting,
    CrossChainSweep,
    MarketRebalance,
    PartnerAssisted,
    TreasuryRefill,
}

// ---------------------------------------------------------------------------
// Chain health
// ---------------------------------------------------------------------------

/// Observed health state of an external chain.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ChainHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
}

// ---------------------------------------------------------------------------
// Composite structs
// ---------------------------------------------------------------------------

/// Full state record for a single vault.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct VaultState<
    Balance: Encode + Decode + MaxEncodedLen + TypeInfo + Clone + core::fmt::Debug + PartialEq + Eq,
> {
    pub vault_id: VaultId,
    pub vault_type: VaultType,
    pub owner_type: OwnerType,
    pub chain_id: ChainId,
    pub asset_id: AssetId,
    pub available_balance: Balance,
    pub reserved_balance: Balance,
    pub pending_out_balance: Balance,
    pub pending_in_balance: Balance,
    pub critical_min: Balance,
    pub min_band: Balance,
    pub target_band: Balance,
    pub max_band: Balance,
    pub status: VaultStatus,
}

/// Full state record for a single lane.
/// `MaxSources` is a const-generic bound for the `BoundedVec`.
#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
)]
#[scale_info(skip_type_params(MaxSources))]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LaneState<
    Balance: Encode + Decode + MaxEncodedLen + TypeInfo + Clone + core::fmt::Debug + PartialEq + Eq,
    MaxSources: frame_support::traits::Get<u32>,
> {
    pub lane_id: LaneId,
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    pub source_asset: AssetId,
    pub dest_asset: AssetId,
    pub lane_class: LaneClass,
    pub allowed_liquidity_sources: frame_support::BoundedVec<LiquiditySourceType, MaxSources>,
    pub status: LaneStatus,
    pub exposure_cap: Balance,
    pub unsettled_cap: Balance,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use codec::{Decode, Encode};

    /// Confirm round-trip codec for every type defined in this module.
    macro_rules! assert_codec_roundtrip {
        ($val:expr) => {{
            let encoded = $val.encode();
            let decoded = Decode::decode(&mut &encoded[..])
                .expect("decode must succeed for a well-formed value");
            assert_eq!($val, decoded);
        }};
    }

    #[test]
    fn vault_type_roundtrip() {
        assert_codec_roundtrip!(VaultType::Gas);
        assert_codec_roundtrip!(VaultType::SettlementFloat);
        assert_codec_roundtrip!(VaultType::TreasuryReserve);
        assert_codec_roundtrip!(VaultType::InsuranceLoss);
    }

    #[test]
    fn owner_type_roundtrip() {
        assert_codec_roundtrip!(OwnerType::Protocol);
        assert_codec_roundtrip!(OwnerType::Treasury);
        assert_codec_roundtrip!(OwnerType::Partner);
    }

    #[test]
    fn vault_status_roundtrip() {
        assert_codec_roundtrip!(VaultStatus::Active);
        assert_codec_roundtrip!(VaultStatus::Degraded);
        assert_codec_roundtrip!(VaultStatus::Frozen);
    }

    #[test]
    fn lane_class_roundtrip() {
        assert_codec_roundtrip!(LaneClass::A);
        assert_codec_roundtrip!(LaneClass::B);
        assert_codec_roundtrip!(LaneClass::C);
    }

    #[test]
    fn lane_status_roundtrip() {
        assert_codec_roundtrip!(LaneStatus::Active);
        assert_codec_roundtrip!(LaneStatus::Degraded);
        assert_codec_roundtrip!(LaneStatus::Frozen);
    }

    #[test]
    fn reservation_status_roundtrip() {
        assert_codec_roundtrip!(ReservationStatus::Active);
        assert_codec_roundtrip!(ReservationStatus::Expired);
        assert_codec_roundtrip!(ReservationStatus::Released);
        assert_codec_roundtrip!(ReservationStatus::Consumed);
    }

    #[test]
    fn partner_status_roundtrip() {
        assert_codec_roundtrip!(PartnerStatus::Active);
        assert_codec_roundtrip!(PartnerStatus::Suspended);
        assert_codec_roundtrip!(PartnerStatus::Terminated);
    }

    #[test]
    fn liquidity_source_type_roundtrip() {
        assert_codec_roundtrip!(LiquiditySourceType::InternalNetting);
        assert_codec_roundtrip!(LiquiditySourceType::ExternalMarket);
        assert_codec_roundtrip!(LiquiditySourceType::Partner);
        assert_codec_roundtrip!(LiquiditySourceType::ProtocolFloat);
    }

    #[test]
    fn freeze_reason_roundtrip() {
        assert_codec_roundtrip!(FreezeReason::BalanceBelowCriticalMin);
        assert_codec_roundtrip!(FreezeReason::UnresolvedSettlementFailure);
        assert_codec_roundtrip!(FreezeReason::PartnerDefault);
        assert_codec_roundtrip!(FreezeReason::QuoteIntegrityFailure);
        assert_codec_roundtrip!(FreezeReason::ChainHealthDegraded);
        assert_codec_roundtrip!(FreezeReason::ReconciliationMismatch);
        assert_codec_roundtrip!(FreezeReason::OperatorManual);
    }

    #[test]
    fn operator_evidence_roundtrip() {
        let ev = OperatorEvidence {
            description_hash: [0xab; 32],
            submitted_at_block: 42,
            operator_id: [0xcd; 32],
        };
        assert_codec_roundtrip!(ev);
    }

    #[test]
    fn solvency_check_roundtrip() {
        let checks = [
            SolvencyCheck::SourceChainHealth,
            SolvencyCheck::DestChainHealth,
            SolvencyCheck::RouteComponentHealth,
            SolvencyCheck::AssetActivation,
            SolvencyCheck::LaneFrozen,
            SolvencyCheck::QuoteFreshness,
            SolvencyCheck::TentativeCapacity,
            SolvencyCheck::SourceVaultSufficiency,
            SolvencyCheck::DestinationCapacity,
            SolvencyCheck::GasReserveSufficiency,
            SolvencyCheck::ExposureCap,
            SolvencyCheck::UnsettledNotional,
            SolvencyCheck::PartnerHealth,
            SolvencyCheck::RouteProfitability,
            SolvencyCheck::QuarantineStatus,
            SolvencyCheck::ReservationValidity,
            SolvencyCheck::SlippageBounds,
            SolvencyCheck::SignerPathHealth,
            SolvencyCheck::IncidentFlag,
            SolvencyCheck::ReconciliationLag,
            SolvencyCheck::PartnerReservationLive,
            SolvencyCheck::BridgePathExists,
        ];
        for check in checks {
            assert_codec_roundtrip!(check);
        }
    }

    #[test]
    fn solvency_result_roundtrip() {
        let result = SolvencyResult {
            passed: false,
            failed_checks: vec![SolvencyCheck::LaneFrozen, SolvencyCheck::QuoteFreshness],
            snapshot_hash: [0x11; 32],
        };
        let encoded = result.encode();
        let decoded: SolvencyResult =
            Decode::decode(&mut &encoded[..]).expect("decode must succeed");
        assert_eq!(result, decoded);
    }

    #[test]
    fn rebalance_trigger_roundtrip() {
        assert_codec_roundtrip!(RebalanceTrigger::BelowMinBand {
            vault_id: [0x01; 32]
        });
        assert_codec_roundtrip!(RebalanceTrigger::DemandSpike {
            lane_id: [0x02; 32]
        });
        assert_codec_roundtrip!(RebalanceTrigger::ConcentrationBreach { chain_id: 1 });
        assert_codec_roundtrip!(RebalanceTrigger::PartnerCapacityLoss {
            partner_id: [0x03; 32]
        });
        assert_codec_roundtrip!(RebalanceTrigger::VenueLiquidityCollapse {
            venue_id: [0x04; 32]
        });
        assert_codec_roundtrip!(RebalanceTrigger::PersistentOneWayFlow {
            lane_id: [0x05; 32]
        });
        assert_codec_roundtrip!(RebalanceTrigger::ChainDegradation { chain_id: 2 });
    }

    #[test]
    fn rebalance_method_roundtrip() {
        assert_codec_roundtrip!(RebalanceMethod::InternalNetting);
        assert_codec_roundtrip!(RebalanceMethod::CrossChainSweep);
        assert_codec_roundtrip!(RebalanceMethod::MarketRebalance);
        assert_codec_roundtrip!(RebalanceMethod::PartnerAssisted);
        assert_codec_roundtrip!(RebalanceMethod::TreasuryRefill);
    }

    #[test]
    fn chain_health_status_roundtrip() {
        assert_codec_roundtrip!(ChainHealthStatus::Healthy);
        assert_codec_roundtrip!(ChainHealthStatus::Degraded);
        assert_codec_roundtrip!(ChainHealthStatus::Unreachable);
    }
}
