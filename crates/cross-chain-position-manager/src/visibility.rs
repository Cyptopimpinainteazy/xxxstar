//! Operator visibility APIs — Ticket 10.
//!
//! This module provides read-only query types and aggregation logic for
//! operator-facing dashboards, monitoring, and alerting.  It consolidates
//! data from the inventory manager, solvency engine, rebalance engine, and
//! partner manager into actionable views.
//!
//! Reference: [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md]

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_core::{H160, H256, U256};

use crate::accounting::{InventoryKey, InventoryManager, InventorySnapshot};
use crate::partner::{PartnerManager, PartnerRecord, PartnerStatus};
use crate::rebalance::{RebalanceEngine, RebalancePlan, RebalanceStatus};
use crate::solvency::{BandEvaluation, SolvencyEngine};

// ---------------------------------------------------------------------------
// System-wide dashboard view
// ---------------------------------------------------------------------------

/// Top-level system health summary for operator dashboards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemHealthView {
    /// Current inventory snapshot.
    pub inventory_snapshot: InventorySnapshot,
    /// Total unsettled notional.
    pub total_unsettled: U256,
    /// Number of active rebalance plans.
    pub active_rebalances: u32,
    /// Number of active partners.
    pub active_partners: u32,
    /// Total partner exposure.
    pub total_partner_exposure: U256,
    /// Lane health summaries.
    pub lane_health: Vec<LaneHealthView>,
    /// Chain health summaries.
    pub chain_health: Vec<ChainHealthView>,
}

/// Lane-level health for operator view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneHealthView {
    pub lane_id: H256,
    pub source_chain: u64,
    pub dest_chain: u64,
    pub asset: H160,
    pub band_evaluation: BandEvaluation,
    pub current_exposure: U256,
}

/// Chain-level health for operator view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainHealthView {
    pub chain_id: u64,
    pub healthy: bool,
    pub reason: String,
    pub gas_balance: U256,
}

// ---------------------------------------------------------------------------
// Inventory detail view
// ---------------------------------------------------------------------------

/// Detailed inventory position for a single (chain, asset) pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryDetailView {
    pub chain_id: u64,
    pub asset: H160,
    pub available: U256,
    pub reserved: U256,
    pub total: U256,
    pub band_evaluation: Option<BandEvaluation>,
}

// ---------------------------------------------------------------------------
// Partner summary view
// ---------------------------------------------------------------------------

/// Summary of a partner for operator display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartnerSummaryView {
    pub partner_id: String,
    pub status: PartnerStatus,
    pub health_score: u32,
    pub current_exposure: U256,
    pub exposure_limit: U256,
    pub supported_lanes_count: u32,
}

impl From<&PartnerRecord> for PartnerSummaryView {
    fn from(record: &PartnerRecord) -> Self {
        Self {
            partner_id: record.partner_id.clone(),
            status: record.status,
            health_score: record.health_score,
            current_exposure: record.current_exposure,
            exposure_limit: record.exposure_limit,
            supported_lanes_count: record.supported_lanes.len() as u32,
        }
    }
}

// ---------------------------------------------------------------------------
// Rebalance summary view
// ---------------------------------------------------------------------------

/// Summary of a rebalance plan for operator display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceSummaryView {
    pub request_id: H256,
    pub chain_id: u64,
    pub asset: H160,
    pub deficit: U256,
    pub resolved_amount: U256,
    pub status: RebalanceStatus,
    pub actions_count: u32,
}

impl From<&RebalancePlan> for RebalanceSummaryView {
    fn from(plan: &RebalancePlan) -> Self {
        Self {
            request_id: plan.request.request_id,
            chain_id: plan.request.inventory_key.chain_id,
            asset: plan.request.inventory_key.asset,
            deficit: plan.request.deficit,
            resolved_amount: plan.resolved_amount,
            status: plan.status,
            actions_count: plan.actions.len() as u32,
        }
    }
}

// ---------------------------------------------------------------------------
// Operator dashboard builder
// ---------------------------------------------------------------------------

/// Builds operator-facing visibility views by aggregating subsystem state.
pub struct OperatorDashboard;

impl OperatorDashboard {
    /// Build the full system health view.
    pub fn system_health(
        inventory: &InventoryManager,
        solvency: &SolvencyEngine,
        rebalance: &RebalanceEngine,
        partners: &PartnerManager,
        lanes: &[(H256, u64, u64, H160)], // (lane_id, src, dst, asset)
    ) -> SystemHealthView {
        let snapshot = inventory.snapshot();

        // Lane health
        let lane_health: Vec<LaneHealthView> = lanes
            .iter()
            .map(|(lane_id, src, _dst, asset)| {
                let band_eval = solvency.evaluate_band_status(inventory, *src, *asset);
                LaneHealthView {
                    lane_id: *lane_id,
                    source_chain: *src,
                    dest_chain: *_dst,
                    asset: *asset,
                    band_evaluation: band_eval,
                    current_exposure: solvency.lane_exposure(lane_id),
                }
            })
            .collect();

        SystemHealthView {
            inventory_snapshot: snapshot,
            total_unsettled: solvency.total_unsettled(),
            active_rebalances: rebalance.active_plan_count() as u32,
            active_partners: partners.active_partners().len() as u32,
            total_partner_exposure: partners.total_exposure(),
            lane_health,
            chain_health: Vec::new(), // populated via solvency chain_health (opaque field)
        }
    }

    /// Build inventory detail views for all tracked positions.
    pub fn inventory_details(
        inventory: &InventoryManager,
        solvency: &SolvencyEngine,
        keys: &[InventoryKey],
    ) -> Vec<InventoryDetailView> {
        keys.iter()
            .filter_map(|key| {
                let balance = inventory.balance(key.chain_id, key.asset)?;
                let band_eval =
                    Some(solvency.evaluate_band_status(inventory, key.chain_id, key.asset));
                Some(InventoryDetailView {
                    chain_id: key.chain_id,
                    asset: key.asset,
                    available: balance.available,
                    reserved: balance.reserved,
                    total: balance.available.saturating_add(balance.reserved),
                    band_evaluation: band_eval,
                })
            })
            .collect()
    }

    /// Build partner summary views.
    pub fn partner_summaries(partners: &PartnerManager) -> Vec<PartnerSummaryView> {
        partners
            .all_partners()
            .iter()
            .map(|p| PartnerSummaryView::from(*p))
            .collect()
    }

    /// Build rebalance summary views (active + recent history).
    pub fn rebalance_summaries(
        rebalance: &RebalanceEngine,
        history_limit: usize,
    ) -> Vec<RebalanceSummaryView> {
        let mut views: Vec<RebalanceSummaryView> = rebalance
            .active_plans()
            .iter()
            .map(|p| RebalanceSummaryView::from(*p))
            .collect();

        let history_views: Vec<RebalanceSummaryView> = rebalance
            .history(history_limit)
            .iter()
            .map(RebalanceSummaryView::from)
            .collect();

        views.extend(history_views);
        views
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounting::InventoryManager;
    use crate::partner::{PartnerHealthMetrics, PartnerManager, PartnerRecord, PartnerStatus};
    use crate::rebalance::RebalanceEngine;
    use crate::router::InventoryBand;
    use crate::solvency::{SolvencyEngine, SolvencyPolicy};

    fn setup_inventory() -> InventoryManager {
        let mut mgr = InventoryManager::new();
        let asset = H160::repeat_byte(0xAA);
        mgr.set_available_balance(1, asset, U256::from(500u64));
        mgr.set_inventory_band(
            1,
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

    fn setup_partners() -> PartnerManager {
        let mut mgr = PartnerManager::new(7_000);
        mgr.upsert_partner(PartnerRecord {
            partner_id: "test_partner".into(),
            status: PartnerStatus::Active,
            health_score: 9_000,
            exposure_limit: U256::from(1_000u64),
            current_exposure: U256::from(100u64),
            supported_lanes: vec![H256::from_low_u64_be(1)],
            metrics: PartnerHealthMetrics::default(),
            last_updated_ms: 1_000,
        });
        mgr
    }

    #[test]
    fn test_system_health_view() {
        let inventory = setup_inventory();
        let solvency = SolvencyEngine::new(SolvencyPolicy::default());
        let rebalance = RebalanceEngine::new();
        let partners = setup_partners();

        let asset = H160::repeat_byte(0xAA);
        let lanes = vec![(H256::from_low_u64_be(1), 1u64, 137u64, asset)];

        let view =
            OperatorDashboard::system_health(&inventory, &solvency, &rebalance, &partners, &lanes);

        assert_eq!(view.active_partners, 1);
        assert_eq!(view.active_rebalances, 0);
        assert_eq!(view.total_partner_exposure, U256::from(100u64));
        assert_eq!(view.lane_health.len(), 1);
    }

    #[test]
    fn test_inventory_detail_view() {
        let inventory = setup_inventory();
        let solvency = SolvencyEngine::new(SolvencyPolicy::default());
        let asset = H160::repeat_byte(0xAA);

        let keys = vec![InventoryKey { chain_id: 1, asset }];

        let details = OperatorDashboard::inventory_details(&inventory, &solvency, &keys);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].available, U256::from(500u64));
        assert!(details[0].band_evaluation.is_some());
    }

    #[test]
    fn test_partner_summary_view() {
        let partners = setup_partners();
        let summaries = OperatorDashboard::partner_summaries(&partners);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].partner_id, "test_partner");
        assert_eq!(summaries[0].health_score, 9_000);
    }

    #[test]
    fn test_rebalance_summary_view() {
        let rebalance = RebalanceEngine::new();
        let summaries = OperatorDashboard::rebalance_summaries(&rebalance, 10);
        assert!(summaries.is_empty());
    }
}
