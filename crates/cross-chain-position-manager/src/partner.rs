//! Partner capacity, health scoring, and reservation integration.
//!
//! This module implements Ticket 9 from the Phase 4.5 execution plan.
//!
//! Partners provide lane-specific or route-specific depth, live quote
//! response, and fill reliability.  They remain external counterparties
//! with measurable obligations rather than informal relationships.
//!
//! Reference: [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md]

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sp_core::{H256, U256};
use sp_std::collections::btree_map::BTreeMap;

// ---------------------------------------------------------------------------
// Partner types
// ---------------------------------------------------------------------------

/// Partner operational status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnerStatus {
    Active,
    Degraded,
    Suspended,
    Removed,
}

/// Health metric snapshot for a single partner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartnerHealthMetrics {
    /// Average quote response time in milliseconds.
    pub avg_response_ms: u64,
    /// Fill reliability in basis points (10_000 = 100%).
    pub fill_reliability_bps: u32,
    /// Rejected reservation rate in basis points.
    pub rejected_reservation_bps: u32,
    /// Stale quote rate in basis points.
    pub stale_quote_bps: u32,
    /// Number of settlement disputes.
    pub dispute_count: u32,
    /// Average settlement delay in milliseconds.
    pub avg_settlement_delay_ms: u64,
}

impl Default for PartnerHealthMetrics {
    fn default() -> Self {
        Self {
            avg_response_ms: 0,
            fill_reliability_bps: 10_000,
            rejected_reservation_bps: 0,
            stale_quote_bps: 0,
            dispute_count: 0,
            avg_settlement_delay_ms: 0,
        }
    }
}

/// A full partner capacity record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartnerRecord {
    pub partner_id: String,
    pub status: PartnerStatus,
    /// Composite health score (0–10_000 bps, higher is better).
    pub health_score: u32,
    /// Per-lane exposure limit.
    pub exposure_limit: U256,
    /// Current aggregate exposure across all lanes.
    pub current_exposure: U256,
    /// Lanes this partner supports.
    pub supported_lanes: Vec<H256>,
    /// Health metrics.
    pub metrics: PartnerHealthMetrics,
    /// Last update timestamp.
    pub last_updated_ms: u64,
}

/// Result of a partner reservation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnerReservationResult {
    Accepted {
        partner_id: String,
        reserved_amount: U256,
    },
    Rejected {
        partner_id: String,
        reason: PartnerRejectionReason,
    },
}

/// Typed reasons for partner reservation rejection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnerRejectionReason {
    PartnerNotFound,
    PartnerNotActive,
    LaneNotSupported { lane_id: H256 },
    HealthBelowThreshold { score: u32, threshold: u32 },
    ExposureLimitReached { current: U256, limit: U256 },
    StaleReservation,
}

// ---------------------------------------------------------------------------
// Partner manager
// ---------------------------------------------------------------------------

/// Manages partner records, health scoring, and reservation integration.
#[derive(Debug, Clone)]
pub struct PartnerManager {
    partners: BTreeMap<String, PartnerRecord>,
    /// Minimum health score (bps) to allow reservation.
    min_health_threshold: u32,
}

impl PartnerManager {
    pub fn new(min_health_threshold: u32) -> Self {
        Self {
            partners: BTreeMap::new(),
            min_health_threshold,
        }
    }

    // -- Mutators ---------------------------------------------------------

    /// Register or update a partner.
    pub fn upsert_partner(&mut self, record: PartnerRecord) {
        self.partners.insert(record.partner_id.clone(), record);
    }

    /// Remove a partner.
    pub fn remove_partner(&mut self, partner_id: &str) -> bool {
        self.partners.remove(partner_id).is_some()
    }

    /// Update health metrics and recompute the composite score.
    pub fn update_metrics(&mut self, partner_id: &str, metrics: PartnerHealthMetrics) -> bool {
        if let Some(record) = self.partners.get_mut(partner_id) {
            record.health_score = Self::compute_health_score(&metrics);
            record.metrics = metrics;
            record.last_updated_ms = current_time_ms();

            // Auto-degrade if score drops below threshold
            if record.health_score < self.min_health_threshold
                && record.status == PartnerStatus::Active
            {
                record.status = PartnerStatus::Degraded;
            }
            // Auto-restore if score recovers
            if record.health_score >= self.min_health_threshold
                && record.status == PartnerStatus::Degraded
            {
                record.status = PartnerStatus::Active;
            }

            true
        } else {
            false
        }
    }

    /// Suspend a partner (manual operator action).
    pub fn suspend_partner(&mut self, partner_id: &str) -> bool {
        if let Some(record) = self.partners.get_mut(partner_id) {
            record.status = PartnerStatus::Suspended;
            record.last_updated_ms = current_time_ms();
            true
        } else {
            false
        }
    }

    /// Reactivate a suspended partner.
    pub fn reactivate_partner(&mut self, partner_id: &str) -> bool {
        if let Some(record) = self.partners.get_mut(partner_id) {
            if record.status == PartnerStatus::Suspended {
                record.status = if record.health_score >= self.min_health_threshold {
                    PartnerStatus::Active
                } else {
                    PartnerStatus::Degraded
                };
                record.last_updated_ms = current_time_ms();
                return true;
            }
        }
        false
    }

    // -- Reservation integration ------------------------------------------

    /// Attempt a partner reservation for a lane and amount.
    pub fn request_reservation(
        &mut self,
        partner_id: &str,
        lane_id: H256,
        amount: U256,
    ) -> PartnerReservationResult {
        let record = match self.partners.get_mut(partner_id) {
            Some(r) => r,
            None => {
                return PartnerReservationResult::Rejected {
                    partner_id: partner_id.to_string(),
                    reason: PartnerRejectionReason::PartnerNotFound,
                }
            }
        };

        // Must be Active
        if record.status != PartnerStatus::Active {
            return PartnerReservationResult::Rejected {
                partner_id: partner_id.to_string(),
                reason: PartnerRejectionReason::PartnerNotActive,
            };
        }

        // Lane must be supported
        if !record.supported_lanes.contains(&lane_id) {
            return PartnerReservationResult::Rejected {
                partner_id: partner_id.to_string(),
                reason: PartnerRejectionReason::LaneNotSupported { lane_id },
            };
        }

        // Health check
        if record.health_score < self.min_health_threshold {
            return PartnerReservationResult::Rejected {
                partner_id: partner_id.to_string(),
                reason: PartnerRejectionReason::HealthBelowThreshold {
                    score: record.health_score,
                    threshold: self.min_health_threshold,
                },
            };
        }

        // Exposure limit
        if record.current_exposure.saturating_add(amount) > record.exposure_limit {
            return PartnerReservationResult::Rejected {
                partner_id: partner_id.to_string(),
                reason: PartnerRejectionReason::ExposureLimitReached {
                    current: record.current_exposure,
                    limit: record.exposure_limit,
                },
            };
        }

        // Accept: increase exposure
        record.current_exposure = record.current_exposure.saturating_add(amount);
        record.last_updated_ms = current_time_ms();

        PartnerReservationResult::Accepted {
            partner_id: partner_id.to_string(),
            reserved_amount: amount,
        }
    }

    /// Release partner exposure after settlement or cancellation.
    pub fn release_exposure(&mut self, partner_id: &str, amount: U256) -> bool {
        if let Some(record) = self.partners.get_mut(partner_id) {
            record.current_exposure = record.current_exposure.saturating_sub(amount);
            record.last_updated_ms = current_time_ms();
            true
        } else {
            false
        }
    }

    // -- Queries ----------------------------------------------------------

    pub fn partner(&self, partner_id: &str) -> Option<&PartnerRecord> {
        self.partners.get(partner_id)
    }

    pub fn active_partners(&self) -> Vec<&PartnerRecord> {
        self.partners
            .values()
            .filter(|p| p.status == PartnerStatus::Active)
            .collect()
    }

    pub fn all_partners(&self) -> Vec<&PartnerRecord> {
        self.partners.values().collect()
    }

    pub fn partners_for_lane(&self, lane_id: &H256) -> Vec<&PartnerRecord> {
        self.partners
            .values()
            .filter(|p| p.supported_lanes.contains(lane_id) && p.status == PartnerStatus::Active)
            .collect()
    }

    pub fn partner_count(&self) -> usize {
        self.partners.len()
    }

    pub fn total_exposure(&self) -> U256 {
        self.partners.values().fold(U256::zero(), |acc, p| {
            acc.saturating_add(p.current_exposure)
        })
    }

    // -- Internal ----------------------------------------------------------

    /// Compute a composite health score from metrics.
    /// Weighted average:
    ///   - fill reliability: 40%
    ///   - rejection rate: 20% (inverted: lower is better)
    ///   - stale quote rate: 20% (inverted)
    ///   - dispute penalty: 10%
    ///   - response time penalty: 10%
    fn compute_health_score(metrics: &PartnerHealthMetrics) -> u32 {
        let fill_component = (metrics.fill_reliability_bps as u64 * 40) / 100;
        let rejection_component =
            ((10_000u64.saturating_sub(metrics.rejected_reservation_bps as u64)) * 20) / 100;
        let stale_component =
            ((10_000u64.saturating_sub(metrics.stale_quote_bps as u64)) * 20) / 100;

        // Dispute penalty: lose 500 bps per dispute, capped at 10_000
        let dispute_penalty = (metrics.dispute_count as u64 * 500).min(10_000);
        let dispute_component = ((10_000u64.saturating_sub(dispute_penalty)) * 10) / 100;

        // Response time penalty: lose 100 bps per 100ms above 200ms
        let response_penalty = if metrics.avg_response_ms > 200 {
            ((metrics.avg_response_ms - 200) / 100 * 100).min(10_000)
        } else {
            0
        };
        let response_component = ((10_000u64.saturating_sub(response_penalty)) * 10) / 100;

        let total = fill_component
            + rejection_component
            + stale_component
            + dispute_component
            + response_component;
        (total as u32).min(10_000)
    }
}

impl Default for PartnerManager {
    fn default() -> Self {
        Self::new(7_000) // 70% default threshold
    }
}

// ---------------------------------------------------------------------------
// time helper
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

    fn make_partner(id: &str, health: u32, exposure_limit: u64, lanes: Vec<H256>) -> PartnerRecord {
        PartnerRecord {
            partner_id: id.to_string(),
            status: PartnerStatus::Active,
            health_score: health,
            exposure_limit: U256::from(exposure_limit),
            current_exposure: U256::zero(),
            supported_lanes: lanes,
            metrics: PartnerHealthMetrics::default(),
            last_updated_ms: 1_000,
        }
    }

    #[test]
    fn test_partner_reservation_accepted() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(1);
        mgr.upsert_partner(make_partner("partner_a", 9_000, 1_000, vec![lane_id]));

        let result = mgr.request_reservation("partner_a", lane_id, U256::from(200u64));
        match result {
            PartnerReservationResult::Accepted {
                reserved_amount, ..
            } => {
                assert_eq!(reserved_amount, U256::from(200u64));
            }
            PartnerReservationResult::Rejected { reason, .. } => {
                panic!("unexpected rejection: {:?}", reason)
            }
        }

        assert_eq!(
            mgr.partner("partner_a").unwrap().current_exposure,
            U256::from(200u64)
        );
    }

    #[test]
    fn test_partner_reservation_rejected_not_active() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(2);
        let mut record = make_partner("partner_b", 9_000, 1_000, vec![lane_id]);
        record.status = PartnerStatus::Suspended;
        mgr.upsert_partner(record);

        let result = mgr.request_reservation("partner_b", lane_id, U256::from(50u64));
        assert!(matches!(
            result,
            PartnerReservationResult::Rejected {
                reason: PartnerRejectionReason::PartnerNotActive,
                ..
            }
        ));
    }

    #[test]
    fn test_partner_reservation_rejected_lane_not_supported() {
        let mut mgr = PartnerManager::new(7_000);
        let supported_lane = H256::from_low_u64_be(3);
        let wrong_lane = H256::from_low_u64_be(999);
        mgr.upsert_partner(make_partner(
            "partner_c",
            9_000,
            1_000,
            vec![supported_lane],
        ));

        let result = mgr.request_reservation("partner_c", wrong_lane, U256::from(50u64));
        assert!(matches!(
            result,
            PartnerReservationResult::Rejected {
                reason: PartnerRejectionReason::LaneNotSupported { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_partner_reservation_rejected_exposure_limit() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(4);
        mgr.upsert_partner(make_partner("partner_d", 9_000, 100, vec![lane_id]));

        let result = mgr.request_reservation("partner_d", lane_id, U256::from(200u64));
        assert!(matches!(
            result,
            PartnerReservationResult::Rejected {
                reason: PartnerRejectionReason::ExposureLimitReached { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_partner_reservation_rejected_health_below_threshold() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(5);
        mgr.upsert_partner(make_partner("partner_e", 5_000, 1_000, vec![lane_id])); // below 7000

        let result = mgr.request_reservation("partner_e", lane_id, U256::from(50u64));
        assert!(matches!(
            result,
            PartnerReservationResult::Rejected {
                reason: PartnerRejectionReason::HealthBelowThreshold { .. },
                ..
            }
        ));
    }

    #[test]
    fn test_partner_exposure_release() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(6);
        mgr.upsert_partner(make_partner("partner_f", 9_000, 1_000, vec![lane_id]));

        mgr.request_reservation("partner_f", lane_id, U256::from(300u64));
        assert_eq!(
            mgr.partner("partner_f").unwrap().current_exposure,
            U256::from(300u64)
        );

        mgr.release_exposure("partner_f", U256::from(100u64));
        assert_eq!(
            mgr.partner("partner_f").unwrap().current_exposure,
            U256::from(200u64)
        );
    }

    #[test]
    fn test_partner_auto_degradation() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(7);
        mgr.upsert_partner(make_partner("partner_g", 9_000, 1_000, vec![lane_id]));

        // Degrade with bad metrics
        let bad_metrics = PartnerHealthMetrics {
            fill_reliability_bps: 3_000, // only 30%
            rejected_reservation_bps: 5_000,
            stale_quote_bps: 4_000,
            dispute_count: 10,
            avg_settlement_delay_ms: 5_000,
            avg_response_ms: 1_000,
        };
        mgr.update_metrics("partner_g", bad_metrics);

        let record = mgr.partner("partner_g").unwrap();
        assert_eq!(record.status, PartnerStatus::Degraded);
        assert!(record.health_score < 7_000);
    }

    #[test]
    fn test_partner_suspension_and_reactivation() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(8);
        mgr.upsert_partner(make_partner("partner_h", 9_000, 1_000, vec![lane_id]));

        mgr.suspend_partner("partner_h");
        assert_eq!(
            mgr.partner("partner_h").unwrap().status,
            PartnerStatus::Suspended
        );

        mgr.reactivate_partner("partner_h");
        assert_eq!(
            mgr.partner("partner_h").unwrap().status,
            PartnerStatus::Active
        );
    }

    #[test]
    fn test_partners_for_lane() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_a = H256::from_low_u64_be(10);
        let lane_b = H256::from_low_u64_be(11);

        mgr.upsert_partner(make_partner("p1", 9_000, 1_000, vec![lane_a, lane_b]));
        mgr.upsert_partner(make_partner("p2", 9_000, 1_000, vec![lane_a]));
        mgr.upsert_partner(make_partner("p3", 9_000, 1_000, vec![lane_b]));

        let lane_a_partners = mgr.partners_for_lane(&lane_a);
        assert_eq!(lane_a_partners.len(), 2);

        let lane_b_partners = mgr.partners_for_lane(&lane_b);
        assert_eq!(lane_b_partners.len(), 2);
    }

    #[test]
    fn test_total_exposure() {
        let mut mgr = PartnerManager::new(7_000);
        let lane_id = H256::from_low_u64_be(12);
        mgr.upsert_partner(make_partner("px", 9_000, 1_000, vec![lane_id]));
        mgr.upsert_partner(make_partner("py", 9_000, 1_000, vec![lane_id]));

        mgr.request_reservation("px", lane_id, U256::from(100u64));
        mgr.request_reservation("py", lane_id, U256::from(200u64));

        assert_eq!(mgr.total_exposure(), U256::from(300u64));
    }

    #[test]
    fn test_health_score_computation() {
        // Perfect metrics
        let perfect = PartnerHealthMetrics {
            avg_response_ms: 50,
            fill_reliability_bps: 10_000,
            rejected_reservation_bps: 0,
            stale_quote_bps: 0,
            dispute_count: 0,
            avg_settlement_delay_ms: 100,
        };
        let score = PartnerManager::compute_health_score(&perfect);
        assert_eq!(score, 10_000);

        // Degraded metrics
        let degraded = PartnerHealthMetrics {
            avg_response_ms: 500,
            fill_reliability_bps: 7_000,
            rejected_reservation_bps: 2_000,
            stale_quote_bps: 1_000,
            dispute_count: 2,
            avg_settlement_delay_ms: 2_000,
        };
        let score = PartnerManager::compute_health_score(&degraded);
        assert!(score > 5_000 && score < 9_000, "score was {}", score);
    }
}
