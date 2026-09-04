//! Solvency engine — pre-quote, pre-reservation, and pre-submission gates.
//!
//! This module implements Ticket 4, Ticket 7 band-freeze integration, and
//! Ticket 12 failure-recovery hooks from the Phase 4.5 execution plan.
//!
//! The solvency engine enforces hard gates that block unsafe execution even
//! when a technically valid route exists.  Every firm route must pass a
//! solvency check.  Indicative routes remain possible when technical pathing
//! exists but solvency does not.
//!
//! Reference: [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md]

use crate::accounting::{InventoryKey, InventoryManager, ObligationStatus};
use crate::router::{InventoryBand, LanePolicy, LaneStatus, ReservationRecord, ReservationStatus};
use crate::{PositionManagerError, Result};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};

// ---------------------------------------------------------------------------
// Rejection taxonomy
// ---------------------------------------------------------------------------

/// Typed rejection reason returned by every solvency gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolvencyRejection {
    /// Lane is frozen — no firm execution allowed.
    LaneFrozen { lane_id: H256 },
    /// Lane policy is missing.
    LanePolicyMissing { lane_id: H256 },
    /// Source chain is unhealthy.
    SourceChainUnhealthy { chain_id: u64, reason: String },
    /// Destination chain is unhealthy.
    DestinationChainUnhealthy { chain_id: u64, reason: String },
    /// Insufficient source inventory.
    InsufficientSourceBalance {
        chain_id: u64,
        asset: H160,
        available: U256,
        required: U256,
    },
    /// Gas reserve below minimum threshold.
    InsufficientGasReserve {
        chain_id: u64,
        available: U256,
        required: U256,
    },
    /// Exposure cap would be breached.
    ExposureCapBreach {
        lane_id: H256,
        current_exposure: U256,
        requested: U256,
        cap: U256,
    },
    /// Unsettled notional exceeds safety budget.
    UnsettledNotionalExceeded {
        current_unsettled: U256,
        max_unsettled: U256,
    },
    /// Source inventory below critical_min band — auto-freeze triggered.
    InventoryBelowCriticalMin {
        chain_id: u64,
        asset: H160,
        available: U256,
        critical_min: U256,
    },
    /// Source inventory below min band — rebalance needed.
    InventoryBelowMin {
        chain_id: u64,
        asset: H160,
        available: U256,
        min: U256,
    },
    /// Reservation has expired.
    ReservationExpired { reservation_id: H256 },
    /// Reservation is not active.
    ReservationNotActive { reservation_id: H256 },
    /// Quote is stale (exceeded freshness window).
    QuoteStale { age_ms: u64, max_age_ms: u64 },
    /// Partner is unhealthy.
    PartnerUnhealthy {
        partner_id: String,
        health_score: u32,
    },
}

// ---------------------------------------------------------------------------
// Solvency check result
// ---------------------------------------------------------------------------

/// Outcome of a solvency gate check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolvencyCheckResult {
    /// `true` when all checks pass.
    pub passed: bool,
    /// Typed rejection reasons — empty when `passed` is `true`.
    pub rejections: Vec<SolvencyRejection>,
    /// Snapshot hash that proves the state the check was evaluated against.
    pub snapshot_hash: H256,
}

impl SolvencyCheckResult {
    pub fn pass(snapshot_hash: H256) -> Self {
        Self {
            passed: true,
            rejections: Vec::new(),
            snapshot_hash,
        }
    }

    pub fn fail(rejections: Vec<SolvencyRejection>, snapshot_hash: H256) -> Self {
        Self {
            passed: false,
            rejections,
            snapshot_hash,
        }
    }
}

// ---------------------------------------------------------------------------
// Solvency policy (configurable caps)
// ---------------------------------------------------------------------------

/// Per-lane or global solvency policy knobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolvencyPolicy {
    /// Maximum unsettled notional across all lanes.
    pub max_unsettled_notional: U256,
    /// Per-lane exposure cap (default applied when lane has no override).
    pub default_exposure_cap: U256,
    /// Minimum gas reserve per chain (in native token wei).
    pub min_gas_reserve: U256,
    /// Quote freshness window in milliseconds.
    pub quote_freshness_ms: u64,
    /// Minimum partner health score (0–10_000 bps scale).
    pub min_partner_health_bps: u32,
}

impl Default for SolvencyPolicy {
    fn default() -> Self {
        Self {
            max_unsettled_notional: U256::from(100_000_000_000_000_000_000u128), // 100 ETH equiv
            default_exposure_cap: U256::from(50_000_000_000_000_000_000u128),    // 50 ETH equiv
            min_gas_reserve: U256::from(500_000_000_000_000_000u128),            // 0.5 ETH
            quote_freshness_ms: 30_000,                                          // 30 seconds
            min_partner_health_bps: 7_000,                                       // 70 %
        }
    }
}

// ---------------------------------------------------------------------------
// Chain health record
// ---------------------------------------------------------------------------

/// Lightweight per-chain health record consumed by pre-quote checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainHealthRecord {
    pub chain_id: u64,
    pub healthy: bool,
    pub reason: String,
    pub gas_balance: U256,
}

// ---------------------------------------------------------------------------
// Solvency engine
// ---------------------------------------------------------------------------

/// The solvency engine runs pre-quote, pre-reservation, and pre-submission
/// checks.  It consults the inventory manager and lane policies to decide
/// whether a route is safe to execute.
#[derive(Debug, Clone)]
pub struct SolvencyEngine {
    policy: SolvencyPolicy,
    chain_health: sp_std::collections::btree_map::BTreeMap<u64, ChainHealthRecord>,
    lane_exposure: sp_std::collections::btree_map::BTreeMap<H256, U256>,
    lane_exposure_caps: sp_std::collections::btree_map::BTreeMap<H256, U256>,
    total_unsettled: U256,
}

impl SolvencyEngine {
    pub fn new(policy: SolvencyPolicy) -> Self {
        Self {
            policy,
            chain_health: sp_std::collections::btree_map::BTreeMap::new(),
            lane_exposure: sp_std::collections::btree_map::BTreeMap::new(),
            lane_exposure_caps: sp_std::collections::btree_map::BTreeMap::new(),
            total_unsettled: U256::zero(),
        }
    }

    // -- Mutators ---------------------------------------------------------

    /// Update chain health state.
    pub fn update_chain_health(&mut self, record: ChainHealthRecord) {
        self.chain_health.insert(record.chain_id, record);
    }

    /// Set per-lane exposure cap (overrides default from policy).
    pub fn set_lane_exposure_cap(&mut self, lane_id: H256, cap: U256) {
        self.lane_exposure_caps.insert(lane_id, cap);
    }

    /// Record additional unsettled notional (called after reservation).
    pub fn add_unsettled(&mut self, lane_id: H256, amount: U256) {
        let current = self.lane_exposure.entry(lane_id).or_insert(U256::zero());
        *current = current.saturating_add(amount);
        self.total_unsettled = self.total_unsettled.saturating_add(amount);
    }

    /// Release settled notional (called after settlement).
    pub fn release_settled(&mut self, lane_id: H256, amount: U256) {
        if let Some(current) = self.lane_exposure.get_mut(&lane_id) {
            *current = current.saturating_sub(amount);
        }
        self.total_unsettled = self.total_unsettled.saturating_sub(amount);
    }

    /// Read the global unsettled notional.
    pub fn total_unsettled(&self) -> U256 {
        self.total_unsettled
    }

    /// Read per-lane exposure.
    pub fn lane_exposure(&self, lane_id: &H256) -> U256 {
        self.lane_exposure
            .get(lane_id)
            .copied()
            .unwrap_or(U256::zero())
    }

    pub fn policy(&self) -> &SolvencyPolicy {
        &self.policy
    }

    // -- Gates ------------------------------------------------------------

    /// **Pre-quote gate**  (Ticket 4, Spec §Solvency gates – Pre-quote)
    ///
    /// Checks:
    /// 1. Source chain health
    /// 2. Destination chain health
    /// 3. Lane freeze status
    /// 4. Tentative capacity (source inventory above critical_min)
    pub fn pre_quote_check(
        &self,
        source_chain: u64,
        dest_chain: u64,
        source_asset: H160,
        lane_policy: Option<&LanePolicy>,
        inventory: &InventoryManager,
    ) -> SolvencyCheckResult {
        let mut rejections = Vec::new();

        // 1. Source chain health
        if let Some(health) = self.chain_health.get(&source_chain) {
            if !health.healthy {
                rejections.push(SolvencyRejection::SourceChainUnhealthy {
                    chain_id: source_chain,
                    reason: health.reason.clone(),
                });
            }
        }

        // 2. Destination chain health
        if let Some(health) = self.chain_health.get(&dest_chain) {
            if !health.healthy {
                rejections.push(SolvencyRejection::DestinationChainUnhealthy {
                    chain_id: dest_chain,
                    reason: health.reason.clone(),
                });
            }
        }

        // 3. Lane freeze status
        if let Some(policy) = lane_policy {
            if !policy.status.allows_firm_execution() {
                rejections.push(SolvencyRejection::LaneFrozen {
                    lane_id: policy.lane_id,
                });
            }

            // 4. Tentative capacity: source inventory vs critical_min
            if let Some(balance) = inventory.balance(source_chain, source_asset) {
                if let Some(band) = &balance.band {
                    if balance.available < band.critical_min {
                        rejections.push(SolvencyRejection::InventoryBelowCriticalMin {
                            chain_id: source_chain,
                            asset: source_asset,
                            available: balance.available,
                            critical_min: band.critical_min,
                        });
                    }
                }
            }
        } else {
            // No policy registered — will be indicative only, not a hard rejection.
        }

        let snapshot_hash = inventory.snapshot().snapshot_id;
        if rejections.is_empty() {
            SolvencyCheckResult::pass(snapshot_hash)
        } else {
            SolvencyCheckResult::fail(rejections, snapshot_hash)
        }
    }

    /// **Pre-reservation gate** (Ticket 4, Spec §Solvency gates – Pre-reservation)
    ///
    /// Checks:
    /// 1. Source vault sufficiency
    /// 2. Gas reserve sufficiency
    /// 3. Exposure cap
    /// 4. Pending unsettled notional
    /// 5. Inventory band checks (critical_min freeze, min rebalance)
    pub fn pre_reservation_check(
        &self,
        source_chain: u64,
        source_asset: H160,
        required_amount: U256,
        lane_id: H256,
        inventory: &InventoryManager,
    ) -> SolvencyCheckResult {
        let mut rejections = Vec::new();

        // 1. Source vault sufficiency
        let available = inventory.free_balance(source_chain, source_asset);
        if available < required_amount {
            rejections.push(SolvencyRejection::InsufficientSourceBalance {
                chain_id: source_chain,
                asset: source_asset,
                available,
                required: required_amount,
            });
        }

        // 2. Gas reserve sufficiency
        if let Some(health) = self.chain_health.get(&source_chain) {
            if health.gas_balance < self.policy.min_gas_reserve {
                rejections.push(SolvencyRejection::InsufficientGasReserve {
                    chain_id: source_chain,
                    available: health.gas_balance,
                    required: self.policy.min_gas_reserve,
                });
            }
        }

        // 3. Exposure cap
        let current_exposure = self.lane_exposure(&lane_id);
        let cap = self
            .lane_exposure_caps
            .get(&lane_id)
            .copied()
            .unwrap_or(self.policy.default_exposure_cap);
        if current_exposure.saturating_add(required_amount) > cap {
            rejections.push(SolvencyRejection::ExposureCapBreach {
                lane_id,
                current_exposure,
                requested: required_amount,
                cap,
            });
        }

        // 4. Pending unsettled notional
        if self.total_unsettled.saturating_add(required_amount) > self.policy.max_unsettled_notional
        {
            rejections.push(SolvencyRejection::UnsettledNotionalExceeded {
                current_unsettled: self.total_unsettled,
                max_unsettled: self.policy.max_unsettled_notional,
            });
        }

        // 5. Inventory band checks
        if let Some(balance) = inventory.balance(source_chain, source_asset) {
            if let Some(band) = &balance.band {
                let post_available = available.saturating_sub(required_amount);
                if post_available < band.critical_min {
                    rejections.push(SolvencyRejection::InventoryBelowCriticalMin {
                        chain_id: source_chain,
                        asset: source_asset,
                        available: post_available,
                        critical_min: band.critical_min,
                    });
                } else if post_available < band.min {
                    // Not a hard rejection — but record for rebalance signaling.
                    rejections.push(SolvencyRejection::InventoryBelowMin {
                        chain_id: source_chain,
                        asset: source_asset,
                        available: post_available,
                        min: band.min,
                    });
                }
            }
        }

        let snapshot_hash = inventory.snapshot().snapshot_id;
        if rejections.is_empty() {
            SolvencyCheckResult::pass(snapshot_hash)
        } else {
            SolvencyCheckResult::fail(rejections, snapshot_hash)
        }
    }

    /// **Pre-submission gate**  (Spec §Solvency gates – Pre-submission)
    ///
    /// Checks:
    /// 1. Reservation is still active and not expired.
    /// 2. Quote freshness.
    /// 3. All pre-reservation checks still hold.
    #[allow(clippy::too_many_arguments)]
    pub fn pre_submission_check(
        &self,
        reservation: &ReservationRecord,
        now_ms: u64,
        quote_timestamp_ms: u64,
        source_chain: u64,
        source_asset: H160,
        required_amount: U256,
        lane_id: H256,
        inventory: &InventoryManager,
    ) -> SolvencyCheckResult {
        let mut rejections = Vec::new();

        // 1. Reservation validity
        if reservation.status != ReservationStatus::Active {
            rejections.push(SolvencyRejection::ReservationNotActive {
                reservation_id: reservation.reservation_id,
            });
        }
        if now_ms > reservation.expiry_ts_ms {
            rejections.push(SolvencyRejection::ReservationExpired {
                reservation_id: reservation.reservation_id,
            });
        }

        // 2. Quote freshness
        let age_ms = now_ms.saturating_sub(quote_timestamp_ms);
        if age_ms > self.policy.quote_freshness_ms {
            rejections.push(SolvencyRejection::QuoteStale {
                age_ms,
                max_age_ms: self.policy.quote_freshness_ms,
            });
        }

        // 3. Re-run pre-reservation checks (gas, exposure, unsettled)
        let pre_res = self.pre_reservation_check(
            source_chain,
            source_asset,
            required_amount,
            lane_id,
            inventory,
        );
        if !pre_res.passed {
            rejections.extend(pre_res.rejections);
        }

        let snapshot_hash = inventory.snapshot().snapshot_id;
        if rejections.is_empty() {
            SolvencyCheckResult::pass(snapshot_hash)
        } else {
            SolvencyCheckResult::fail(rejections, snapshot_hash)
        }
    }

    // -- Band-aware freeze helpers (Ticket 7) -----------------------------

    /// Check whether a `(chain, asset)` pair should be frozen based on
    /// its inventory band.  Returns `Some(LaneStatus)` indicating the
    /// recommended lane status.
    pub fn evaluate_band_status(
        &self,
        inventory: &InventoryManager,
        chain_id: u64,
        asset: H160,
    ) -> BandEvaluation {
        let balance = match inventory.balance(chain_id, asset) {
            Some(b) => b,
            None => {
                return BandEvaluation {
                    recommended_status: LaneStatus::Active,
                    rebalance_needed: false,
                    reason: None,
                }
            }
        };
        let band = match &balance.band {
            Some(b) => b,
            None => {
                return BandEvaluation {
                    recommended_status: LaneStatus::Active,
                    rebalance_needed: false,
                    reason: None,
                }
            }
        };

        if balance.available < band.critical_min {
            BandEvaluation {
                recommended_status: LaneStatus::Frozen,
                rebalance_needed: true,
                reason: Some("available below critical_min".to_string()),
            }
        } else if balance.available < band.min {
            BandEvaluation {
                recommended_status: LaneStatus::Warning,
                rebalance_needed: true,
                reason: Some("available below min — rebalance needed".to_string()),
            }
        } else if balance.available > band.max {
            BandEvaluation {
                recommended_status: LaneStatus::Active,
                rebalance_needed: true,
                reason: Some("available above max — sweep recommended".to_string()),
            }
        } else {
            BandEvaluation {
                recommended_status: LaneStatus::Active,
                rebalance_needed: false,
                reason: None,
            }
        }
    }

    // -- Failure recovery helpers (Ticket 12) -----------------------------

    /// Record a settlement failure and decide whether the lane should freeze.
    pub fn record_settlement_failure(
        &mut self,
        lane_id: H256,
        failed_amount: U256,
    ) -> FailureAction {
        // Release the unsettled notional for the failed amount.
        self.release_settled(lane_id, failed_amount);

        // If remaining lane exposure is still high, recommend freeze.
        let remaining = self.lane_exposure(&lane_id);
        let cap = self
            .lane_exposure_caps
            .get(&lane_id)
            .copied()
            .unwrap_or(self.policy.default_exposure_cap);

        if remaining > cap / 2 {
            FailureAction::FreezeLane {
                lane_id,
                reason: "settlement failure with high remaining exposure".to_string(),
            }
        } else {
            FailureAction::DegradeOnly {
                lane_id,
                reason: "settlement failure — degraded but not frozen".to_string(),
            }
        }
    }

    /// Record a loss event for accounting visibility.
    pub fn record_loss_event(&mut self, lane_id: H256, amount: U256) -> LossEvent {
        self.release_settled(lane_id, amount);
        LossEvent {
            lane_id,
            amount,
            timestamp_ms: current_time_ms(),
        }
    }
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Result of evaluating inventory bands against thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandEvaluation {
    pub recommended_status: LaneStatus,
    pub rebalance_needed: bool,
    pub reason: Option<String>,
}

/// Action recommended after a settlement failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureAction {
    FreezeLane { lane_id: H256, reason: String },
    DegradeOnly { lane_id: H256, reason: String },
}

/// Recorded loss event for accounting and operator visibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossEvent {
    pub lane_id: H256,
    pub amount: U256,
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// time helper (reuse pattern from router/accounting)
// ---------------------------------------------------------------------------

fn current_time_ms() -> u64 {
    #[cfg(test)]
    {
        1_000
    }

    #[cfg(not(test))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounting::InventoryManager;
    use crate::router::{InventoryBand, LaneClass, LanePolicy, LaneStatus, LiquiditySourceType};

    fn make_inventory_with_balance(chain: u64, asset: H160, avail: u64) -> InventoryManager {
        let mut mgr = InventoryManager::new();
        mgr.set_available_balance(chain, asset, U256::from(avail));
        mgr.set_inventory_band(
            chain,
            asset,
            InventoryBand {
                critical_min: U256::from(10u64),
                min: U256::from(50u64),
                target: U256::from(500u64),
                max: U256::from(1_000u64),
            },
        );
        mgr
    }

    fn make_policy(lane_id: H256, status: LaneStatus) -> LanePolicy {
        LanePolicy {
            lane_id,
            source_chain: 1,
            target_chain: 137,
            source_asset: H160::repeat_byte(0xAA),
            target_asset: H160::repeat_byte(0xBB),
            lane_class: LaneClass::MarketOnly,
            status,
            allowed_liquidity_sources: vec![LiquiditySourceType::ExternalMarket],
            inventory_band: InventoryBand {
                critical_min: U256::from(10u64),
                min: U256::from(50u64),
                target: U256::from(500u64),
                max: U256::from(1_000u64),
            },
        }
    }

    // -- Pre-quote tests --------------------------------------------------

    #[test]
    fn test_pre_quote_passes_healthy_lane() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 200);
        let lane_id = H256::from_low_u64_be(1);
        let policy = make_policy(lane_id, LaneStatus::Active);

        let result = engine.pre_quote_check(1, 137, asset, Some(&policy), &inventory);
        assert!(result.passed, "rejections: {:?}", result.rejections);
    }

    #[test]
    fn test_pre_quote_rejects_frozen_lane() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 200);
        let lane_id = H256::from_low_u64_be(2);
        let policy = make_policy(lane_id, LaneStatus::Frozen);

        let result = engine.pre_quote_check(1, 137, asset, Some(&policy), &inventory);
        assert!(!result.passed);
        assert!(result
            .rejections
            .iter()
            .any(|r| matches!(r, SolvencyRejection::LaneFrozen { .. })));
    }

    #[test]
    fn test_pre_quote_rejects_unhealthy_source_chain() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy::default());
        engine.update_chain_health(ChainHealthRecord {
            chain_id: 1,
            healthy: false,
            reason: "RPC down".to_string(),
            gas_balance: U256::zero(),
        });
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 200);

        let result = engine.pre_quote_check(1, 137, asset, None, &inventory);
        assert!(!result.passed);
        assert!(result
            .rejections
            .iter()
            .any(|r| matches!(r, SolvencyRejection::SourceChainUnhealthy { .. })));
    }

    // -- Pre-reservation tests --------------------------------------------

    #[test]
    fn test_pre_reservation_passes_sufficient_balance() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy::default());
        engine.update_chain_health(ChainHealthRecord {
            chain_id: 1,
            healthy: true,
            reason: String::new(),
            gas_balance: U256::from(1_000_000_000_000_000_000u128),
        });
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 500);
        let lane_id = H256::from_low_u64_be(10);

        let result =
            engine.pre_reservation_check(1, asset, U256::from(100u64), lane_id, &inventory);
        assert!(result.passed, "rejections: {:?}", result.rejections);
    }

    #[test]
    fn test_pre_reservation_rejects_insufficient_gas() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy::default());
        engine.update_chain_health(ChainHealthRecord {
            chain_id: 1,
            healthy: true,
            reason: String::new(),
            gas_balance: U256::from(100u64), // way below min
        });
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 500);
        let lane_id = H256::from_low_u64_be(11);

        let result =
            engine.pre_reservation_check(1, asset, U256::from(100u64), lane_id, &inventory);
        assert!(!result.passed);
        assert!(result
            .rejections
            .iter()
            .any(|r| matches!(r, SolvencyRejection::InsufficientGasReserve { .. })));
    }

    #[test]
    fn test_pre_reservation_rejects_exposure_cap_breach() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy {
            default_exposure_cap: U256::from(200u64),
            ..SolvencyPolicy::default()
        });
        engine.update_chain_health(ChainHealthRecord {
            chain_id: 1,
            healthy: true,
            reason: String::new(),
            gas_balance: U256::from(1_000_000_000_000_000_000u128),
        });
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 500);
        let lane_id = H256::from_low_u64_be(12);

        // Pre-load 150 exposure
        engine.add_unsettled(lane_id, U256::from(150u64));

        // Request 100 more => 250 > cap of 200
        let result =
            engine.pre_reservation_check(1, asset, U256::from(100u64), lane_id, &inventory);
        assert!(!result.passed);
        assert!(result
            .rejections
            .iter()
            .any(|r| matches!(r, SolvencyRejection::ExposureCapBreach { .. })));
    }

    #[test]
    fn test_pre_reservation_rejects_unsettled_notional() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy {
            max_unsettled_notional: U256::from(300u64),
            ..SolvencyPolicy::default()
        });
        engine.update_chain_health(ChainHealthRecord {
            chain_id: 1,
            healthy: true,
            reason: String::new(),
            gas_balance: U256::from(1_000_000_000_000_000_000u128),
        });
        let asset = H160::repeat_byte(0xAA);
        let inventory = make_inventory_with_balance(1, asset, 500);
        let lane_id = H256::from_low_u64_be(13);

        engine.add_unsettled(lane_id, U256::from(250u64));

        // Request 100 more => 350 > 300 cap
        let result =
            engine.pre_reservation_check(1, asset, U256::from(100u64), lane_id, &inventory);
        assert!(!result.passed);
        assert!(result
            .rejections
            .iter()
            .any(|r| matches!(r, SolvencyRejection::UnsettledNotionalExceeded { .. })));
    }

    // -- Band evaluation tests (Ticket 7) ---------------------------------

    #[test]
    fn test_band_freeze_below_critical_min() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xCC);
        let inventory = make_inventory_with_balance(1, asset, 5); // below critical_min=10

        let eval = engine.evaluate_band_status(&inventory, 1, asset);
        assert_eq!(eval.recommended_status, LaneStatus::Frozen);
        assert!(eval.rebalance_needed);
    }

    #[test]
    fn test_band_warning_below_min() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xCC);
        let inventory = make_inventory_with_balance(1, asset, 30); // between critical_min=10 and min=50

        let eval = engine.evaluate_band_status(&inventory, 1, asset);
        assert_eq!(eval.recommended_status, LaneStatus::Warning);
        assert!(eval.rebalance_needed);
    }

    #[test]
    fn test_band_active_in_normal_range() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xCC);
        let inventory = make_inventory_with_balance(1, asset, 200); // between min=50 and max=1000

        let eval = engine.evaluate_band_status(&inventory, 1, asset);
        assert_eq!(eval.recommended_status, LaneStatus::Active);
        assert!(!eval.rebalance_needed);
    }

    #[test]
    fn test_band_sweep_above_max() {
        let engine = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xCC);
        let inventory = make_inventory_with_balance(1, asset, 2_000); // above max=1000

        let eval = engine.evaluate_band_status(&inventory, 1, asset);
        assert_eq!(eval.recommended_status, LaneStatus::Active);
        assert!(eval.rebalance_needed);
    }

    // -- Failure recovery tests (Ticket 12) --------------------------------

    #[test]
    fn test_settlement_failure_freeze_high_exposure() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy {
            default_exposure_cap: U256::from(200u64),
            ..SolvencyPolicy::default()
        });
        let lane_id = H256::from_low_u64_be(20);
        engine.add_unsettled(lane_id, U256::from(180u64));

        let action = engine.record_settlement_failure(lane_id, U256::from(30u64));
        // remaining = 150, cap/2 = 100, 150 > 100 => freeze
        assert!(matches!(action, FailureAction::FreezeLane { .. }));
    }

    #[test]
    fn test_settlement_failure_degrade_low_exposure() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy {
            default_exposure_cap: U256::from(200u64),
            ..SolvencyPolicy::default()
        });
        let lane_id = H256::from_low_u64_be(21);
        engine.add_unsettled(lane_id, U256::from(80u64));

        let action = engine.record_settlement_failure(lane_id, U256::from(40u64));
        // remaining = 40, cap/2 = 100, 40 < 100 => degrade only
        assert!(matches!(action, FailureAction::DegradeOnly { .. }));
    }

    #[test]
    fn test_loss_event_recording() {
        let mut engine = SolvencyEngine::new(SolvencyPolicy::default());
        let lane_id = H256::from_low_u64_be(22);
        engine.add_unsettled(lane_id, U256::from(100u64));

        let loss = engine.record_loss_event(lane_id, U256::from(25u64));
        assert_eq!(loss.amount, U256::from(25u64));
        assert_eq!(engine.lane_exposure(&lane_id), U256::from(75u64));
    }
}
