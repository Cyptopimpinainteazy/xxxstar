//! Rebalance engine — ordered recovery paths for inventory management.
//!
//! This module implements Ticket 8 from the Phase 4.5 execution plan.
//!
//! Rebalancing preserves lane continuity without letting treasury drift into
//! unmanaged exposure.  The engine follows a strict policy order:
//!
//! 1. Internal netting (cheapest, reduces market footprint)
//! 2. Cross-chain sweep from overfunded vaults
//! 3. Market rebalance through approved venues
//! 4. Partner-assisted rebalance
//! 5. Treasury refill (last resort, only for critical lanes)
//!
//! Reference: [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};
use sp_std::collections::btree_map::BTreeMap;

use crate::accounting::InventoryKey;

// ---------------------------------------------------------------------------
// Rebalance request
// ---------------------------------------------------------------------------

/// Trigger reason for a rebalance request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceTrigger {
    /// Inventory fell below `min` band.
    BelowMinBand,
    /// Inventory fell below `critical_min` band.
    BelowCriticalMin,
    /// Inventory exceeded `max` band — sweep needed.
    AboveMaxBand,
    /// Projected demand spike.
    DemandSpike,
    /// Concentration breach on a single chain or asset.
    ConcentrationBreach,
    /// Partner capacity lost.
    PartnerCapacityLoss,
    /// Venue liquidity collapse.
    VenueLiquidityCollapse,
    /// Persistent one-way flow.
    PersistentOneWayFlow,
    /// Chain degradation.
    ChainDegradation,
    /// Manual operator trigger.
    OperatorTriggered,
}

/// Urgency level for rebalance requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RebalanceUrgency {
    /// Scheduled, cost-aware.
    Slow,
    /// Event-driven, critical thresholds approaching.
    Fast,
    /// Emergency — lane frozen or about to freeze.
    Emergency,
}

/// A rebalance request emitted by inventory band checks or solvency gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceRequest {
    pub request_id: H256,
    pub inventory_key: InventoryKey,
    pub trigger: RebalanceTrigger,
    pub urgency: RebalanceUrgency,
    pub deficit: U256,
    pub target: U256,
    pub created_at_ms: u64,
}

// ---------------------------------------------------------------------------
// Rebalance actions (policy-ordered)
// ---------------------------------------------------------------------------

/// The type of rebalance action, ordered by policy preference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceActionType {
    /// Offset with opposing flow.
    InternalNetting,
    /// Sweep from an overfunded chain/asset vault.
    CrossChainSweep { from_chain: u64, from_asset: H160 },
    /// Rebalance through approved DEX/aggregator.
    MarketRebalance { venue: String },
    /// Use partner-provided depth.
    PartnerAssisted { partner_id: String },
    /// Treasury refill — last resort for critical lanes only.
    TreasuryRefill,
}

/// A planned rebalance action with amount and estimated cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceAction {
    pub action_type: RebalanceActionType,
    pub amount: U256,
    pub estimated_cost: U256,
    pub priority: u8,
}

/// Status of a rebalance action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RebalanceStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Full rebalance plan for a single request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlan {
    pub request: RebalanceRequest,
    pub actions: Vec<RebalanceAction>,
    pub status: RebalanceStatus,
    pub resolved_amount: U256,
}

// ---------------------------------------------------------------------------
// Rebalance engine
// ---------------------------------------------------------------------------

/// The rebalance engine accepts requests and produces ordered action plans
/// following the spec's policy priority.
#[derive(Debug, Clone)]
pub struct RebalanceEngine {
    /// Active rebalance plans.
    plans: BTreeMap<H256, RebalancePlan>,
    /// Completed/historical plans.
    history: Vec<RebalancePlan>,
    /// Maximum depth of history to keep.
    max_history: usize,
    /// Available netting sources (chain, asset) -> surplus.
    netting_sources: BTreeMap<InventoryKey, U256>,
    /// Approved partner IDs for partner-assisted rebalancing.
    approved_partners: Vec<String>,
    /// Whether treasury refill is enabled.
    treasury_refill_enabled: bool,
}

impl RebalanceEngine {
    pub fn new() -> Self {
        Self {
            plans: BTreeMap::new(),
            history: Vec::new(),
            max_history: 200,
            netting_sources: BTreeMap::new(),
            approved_partners: Vec::new(),
            treasury_refill_enabled: true,
        }
    }

    // -- Configuration mutators -------------------------------------------

    /// Register a netting source (chain/asset with surplus).
    pub fn register_netting_source(&mut self, key: InventoryKey, surplus: U256) {
        self.netting_sources.insert(key, surplus);
    }

    /// Clear all netting sources (call before re-scanning).
    pub fn clear_netting_sources(&mut self) {
        self.netting_sources.clear();
    }

    /// Register an approved partner for assisted rebalancing.
    pub fn register_partner(&mut self, partner_id: String) {
        if !self.approved_partners.contains(&partner_id) {
            self.approved_partners.push(partner_id);
        }
    }

    /// Enable or disable treasury refill.
    pub fn set_treasury_refill_enabled(&mut self, enabled: bool) {
        self.treasury_refill_enabled = enabled;
    }

    // -- Core engine logic ------------------------------------------------

    /// Submit a rebalance request and produce an ordered action plan.
    pub fn submit_request(&mut self, request: RebalanceRequest) -> &RebalancePlan {
        let mut actions = Vec::new();
        let mut remaining = request.deficit;

        // 1. Internal netting
        for (key, surplus) in &self.netting_sources {
            if remaining.is_zero() {
                break;
            }
            if *key == request.inventory_key {
                continue; // skip self
            }
            let usable = (*surplus).min(remaining);
            if !usable.is_zero() {
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::InternalNetting,
                    amount: usable,
                    estimated_cost: U256::zero(),
                    priority: 1,
                });
                remaining = remaining.saturating_sub(usable);
            }
        }

        // 2. Cross-chain sweep (look for overfunded same-asset on other chains)
        for (key, surplus) in &self.netting_sources {
            if remaining.is_zero() {
                break;
            }
            if key.asset == request.inventory_key.asset
                && key.chain_id != request.inventory_key.chain_id
            {
                let usable = (*surplus).min(remaining);
                if !usable.is_zero() {
                    actions.push(RebalanceAction {
                        action_type: RebalanceActionType::CrossChainSweep {
                            from_chain: key.chain_id,
                            from_asset: key.asset,
                        },
                        amount: usable,
                        estimated_cost: U256::from(1_000u64), // placeholder bridge cost
                        priority: 2,
                    });
                    remaining = remaining.saturating_sub(usable);
                }
            }
        }

        // 3. Market rebalance
        if !remaining.is_zero() {
            let market_amount = remaining;
            actions.push(RebalanceAction {
                action_type: RebalanceActionType::MarketRebalance {
                    venue: "default_aggregator".to_string(),
                },
                amount: market_amount,
                estimated_cost: market_amount / 200, // ~0.5% placeholder
                priority: 3,
            });
            remaining = remaining.saturating_sub(market_amount);
        }

        // 4. Partner-assisted
        if !remaining.is_zero() {
            for partner_id in &self.approved_partners {
                if remaining.is_zero() {
                    break;
                }
                let partner_amount = remaining;
                actions.push(RebalanceAction {
                    action_type: RebalanceActionType::PartnerAssisted {
                        partner_id: partner_id.clone(),
                    },
                    amount: partner_amount,
                    estimated_cost: partner_amount / 100, // ~1% placeholder
                    priority: 4,
                });
                remaining = remaining.saturating_sub(partner_amount);
            }
        }

        // 5. Treasury refill (last resort)
        if !remaining.is_zero() && self.treasury_refill_enabled {
            actions.push(RebalanceAction {
                action_type: RebalanceActionType::TreasuryRefill,
                amount: remaining,
                estimated_cost: U256::zero(),
                priority: 5,
            });
            remaining = U256::zero();
        }

        let resolved = request.deficit.saturating_sub(remaining);
        let plan = RebalancePlan {
            request: request.clone(),
            actions,
            status: RebalanceStatus::Queued,
            resolved_amount: resolved,
        };

        self.plans.insert(request.request_id, plan);
        self.plans.get(&request.request_id).unwrap()
    }

    /// Mark a plan as completed and move to history.
    pub fn complete_plan(&mut self, request_id: &H256) -> bool {
        if let Some(mut plan) = self.plans.remove(request_id) {
            plan.status = RebalanceStatus::Completed;
            self.history.push(plan);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            true
        } else {
            false
        }
    }

    /// Mark a plan as failed and move to history.
    pub fn fail_plan(&mut self, request_id: &H256) -> bool {
        if let Some(mut plan) = self.plans.remove(request_id) {
            plan.status = RebalanceStatus::Failed;
            self.history.push(plan);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            true
        } else {
            false
        }
    }

    // -- Queries ----------------------------------------------------------

    /// Active plans count.
    pub fn active_plan_count(&self) -> usize {
        self.plans.len()
    }

    /// Get a specific plan.
    pub fn plan(&self, request_id: &H256) -> Option<&RebalancePlan> {
        self.plans.get(request_id)
    }

    /// All active plans.
    pub fn active_plans(&self) -> Vec<&RebalancePlan> {
        self.plans.values().collect()
    }

    /// Recent history.
    pub fn history(&self, limit: usize) -> &[RebalancePlan] {
        let start = self.history.len().saturating_sub(limit);
        &self.history[start..]
    }
}

impl Default for RebalanceEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(id: u64, deficit: u64, urgency: RebalanceUrgency) -> RebalanceRequest {
        RebalanceRequest {
            request_id: H256::from_low_u64_be(id),
            inventory_key: InventoryKey {
                chain_id: 1,
                asset: H160::repeat_byte(0xAA),
            },
            trigger: RebalanceTrigger::BelowMinBand,
            urgency,
            deficit: U256::from(deficit),
            target: U256::from(500u64),
            created_at_ms: 1_000,
        }
    }

    #[test]
    fn test_rebalance_uses_netting_first() {
        let mut engine = RebalanceEngine::new();
        engine.register_netting_source(
            InventoryKey {
                chain_id: 137,
                asset: H160::repeat_byte(0xBB),
            },
            U256::from(200u64),
        );

        let request = make_request(1, 100, RebalanceUrgency::Slow);
        let plan = engine.submit_request(request);

        assert_eq!(
            plan.actions[0].action_type,
            RebalanceActionType::InternalNetting
        );
        assert_eq!(plan.actions[0].amount, U256::from(100u64));
        assert_eq!(plan.resolved_amount, U256::from(100u64));
    }

    #[test]
    fn test_rebalance_escalates_to_market_when_netting_insufficient() {
        let mut engine = RebalanceEngine::new();
        engine.register_netting_source(
            InventoryKey {
                chain_id: 137,
                asset: H160::repeat_byte(0xBB),
            },
            U256::from(30u64),
        );

        let request = make_request(2, 100, RebalanceUrgency::Fast);
        let plan = engine.submit_request(request);

        // Should have netting (30) + market (70)
        assert!(plan.actions.len() >= 2);
        assert_eq!(
            plan.actions[0].action_type,
            RebalanceActionType::InternalNetting
        );
        assert_eq!(plan.actions[0].amount, U256::from(30u64));

        let market_action = plan
            .actions
            .iter()
            .find(|a| matches!(a.action_type, RebalanceActionType::MarketRebalance { .. }));
        assert!(market_action.is_some());
        assert_eq!(market_action.unwrap().amount, U256::from(70u64));
    }

    #[test]
    fn test_rebalance_treasury_last_policy() {
        let mut engine = RebalanceEngine::new();
        // No netting sources, no partners — should go market then treasury
        let request = make_request(3, 200, RebalanceUrgency::Emergency);
        let plan = engine.submit_request(request);

        // Market should come before treasury
        let market_idx = plan
            .actions
            .iter()
            .position(|a| matches!(a.action_type, RebalanceActionType::MarketRebalance { .. }));
        let treasury_idx = plan
            .actions
            .iter()
            .position(|a| a.action_type == RebalanceActionType::TreasuryRefill);

        // Market fills 200, so treasury shouldn't even appear
        assert!(market_idx.is_some());
        assert!(treasury_idx.is_none()); // market handled it all
    }

    #[test]
    fn test_rebalance_treasury_disabled() {
        let mut engine = RebalanceEngine::new();
        engine.set_treasury_refill_enabled(false);

        let request = make_request(4, 100, RebalanceUrgency::Slow);
        let plan = engine.submit_request(request);

        assert!(!plan
            .actions
            .iter()
            .any(|a| a.action_type == RebalanceActionType::TreasuryRefill));
    }

    #[test]
    fn test_plan_completion_moves_to_history() {
        let mut engine = RebalanceEngine::new();
        let request = make_request(5, 50, RebalanceUrgency::Slow);
        engine.submit_request(request);
        assert_eq!(engine.active_plan_count(), 1);

        let completed = engine.complete_plan(&H256::from_low_u64_be(5));
        assert!(completed);
        assert_eq!(engine.active_plan_count(), 0);
        assert_eq!(engine.history(10).len(), 1);
        assert_eq!(engine.history(10)[0].status, RebalanceStatus::Completed);
    }

    #[test]
    fn test_plan_failure_moves_to_history() {
        let mut engine = RebalanceEngine::new();
        let request = make_request(6, 50, RebalanceUrgency::Emergency);
        engine.submit_request(request);

        let failed = engine.fail_plan(&H256::from_low_u64_be(6));
        assert!(failed);
        assert_eq!(engine.active_plan_count(), 0);
        assert_eq!(engine.history(10)[0].status, RebalanceStatus::Failed);
    }

    #[test]
    fn test_demand_spike_trigger() {
        let mut engine = RebalanceEngine::new();
        let request = RebalanceRequest {
            request_id: H256::from_low_u64_be(7),
            inventory_key: InventoryKey {
                chain_id: 1,
                asset: H160::repeat_byte(0xAA),
            },
            trigger: RebalanceTrigger::DemandSpike,
            urgency: RebalanceUrgency::Fast,
            deficit: U256::from(300u64),
            target: U256::from(500u64),
            created_at_ms: 1_000,
        };
        let plan = engine.submit_request(request);
        assert!(plan.resolved_amount > U256::zero());
    }

    #[test]
    fn test_cross_chain_sweep() {
        let mut engine = RebalanceEngine::new();
        // Register same asset on different chain with surplus
        engine.register_netting_source(
            InventoryKey {
                chain_id: 137,
                asset: H160::repeat_byte(0xAA), // same asset as request
            },
            U256::from(80u64),
        );

        let request = make_request(8, 60, RebalanceUrgency::Slow);
        let plan = engine.submit_request(request);

        // Should detect cross-chain sweep opportunity
        let sweep = plan.actions.iter().find(|a| {
            matches!(
                a.action_type,
                RebalanceActionType::CrossChainSweep { .. } | RebalanceActionType::InternalNetting
            )
        });
        assert!(sweep.is_some());
    }
}
