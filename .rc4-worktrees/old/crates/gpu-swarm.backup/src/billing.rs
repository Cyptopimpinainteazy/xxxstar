//! Billing Engine for DePIN GPU Marketplace
//!
//! Proposal: DEPIN-GPU-001
//!
//! Tracks compute usage per rental and generates billing records for on-chain settlement.
//! Billing is per-second with GPU utilization weighting.
//!
//! ## Revenue Flow
//!
//! ```text
//! User Payment → On-chain Escrow → Job Complete → Revenue Split
//!                                        │
//!                               ┌────────┼────────┐
//!                               ▼        ▼        ▼
//!                         Validator   Burn    Stakers
//!                          (55%)     (25%)    (20%)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A billing record for a rental period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingRecord {
    /// Order ID from pallet.
    pub order_id: [u8; 16],
    /// Total compute seconds.
    pub compute_seconds: u64,
    /// Average GPU utilization (0.0 - 1.0).
    pub avg_gpu_utilization: f64,
    /// Total VRAM-seconds (MB * seconds).
    pub vram_seconds: u64,
    /// Computed fee (micro-tokens).
    pub computed_fee: u64,
    /// Billing start time.
    pub period_start: u64,
    /// Billing end time.
    pub period_end: u64,
    /// Preemption count during period.
    pub preemptions: u32,
    /// Discount applied for preemptions (basis points).
    pub preemption_discount_bps: u32,
}

/// Configuration for the billing engine.
#[derive(Debug, Clone)]
pub struct BillingConfig {
    /// Base rate per GPU-second (micro-tokens).
    pub base_rate_per_second: u64,
    /// VRAM tier multiplier.
    pub vram_tier_multipliers: HashMap<u32, f64>, // VRAM_MB → multiplier
    /// Discount per preemption (basis points).
    pub preemption_discount_per_event_bps: u32,
    /// Maximum total preemption discount (basis points).
    pub max_preemption_discount_bps: u32,
    /// Billing interval (seconds) for periodic billing.
    pub billing_interval_secs: u64,
}

impl Default for BillingConfig {
    fn default() -> Self {
        let mut vram_multipliers = HashMap::new();
        vram_multipliers.insert(4096, 1.0); // 4GB base
        vram_multipliers.insert(8192, 1.5); // 8GB
        vram_multipliers.insert(16384, 2.5); // 16GB
        vram_multipliers.insert(24576, 3.5); // 24GB
        vram_multipliers.insert(40960, 5.0); // 40GB (A100)
        vram_multipliers.insert(81920, 8.0); // 80GB (H100)

        Self {
            base_rate_per_second: 100, // 100 micro-tokens / second
            vram_tier_multipliers: vram_multipliers,
            preemption_discount_per_event_bps: 200, // 2% per preemption
            max_preemption_discount_bps: 1500,      // Max 15% discount
            billing_interval_secs: 60,
        }
    }
}

/// Billing engine that tracks usage and computes fees.
pub struct BillingEngine {
    config: BillingConfig,
    /// Active usage trackers per order.
    trackers: HashMap<[u8; 16], UsageTracker>,
    /// Completed billing records.
    completed_records: Vec<BillingRecord>,
}

/// Tracks real-time usage for an active rental.
#[derive(Debug, Clone)]
pub struct UsageTracker {
    pub order_id: [u8; 16],
    pub start_time: u64,
    pub total_compute_seconds: u64,
    pub total_vram_seconds: u64,
    pub gpu_utilization_samples: Vec<f64>,
    pub vram_mb: u32,
    pub preemptions: u32,
    pub last_sample_time: u64,
}

impl BillingEngine {
    pub fn new(config: BillingConfig) -> Self {
        Self {
            config,
            trackers: HashMap::new(),
            completed_records: Vec::new(),
        }
    }

    /// Start tracking a rental.
    pub fn start_tracking(&mut self, order_id: [u8; 16], vram_mb: u32) {
        let now = now_secs();
        self.trackers.insert(
            order_id,
            UsageTracker {
                order_id,
                start_time: now,
                total_compute_seconds: 0,
                total_vram_seconds: 0,
                gpu_utilization_samples: Vec::new(),
                vram_mb,
                preemptions: 0,
                last_sample_time: now,
            },
        );
    }

    /// Record a usage sample.
    pub fn record_sample(&mut self, order_id: &[u8; 16], gpu_utilization: f64) {
        if let Some(tracker) = self.trackers.get_mut(order_id) {
            let now = now_secs();
            let elapsed = now.saturating_sub(tracker.last_sample_time);

            tracker.total_compute_seconds += elapsed;
            tracker.total_vram_seconds += elapsed * tracker.vram_mb as u64;
            tracker.gpu_utilization_samples.push(gpu_utilization);
            tracker.last_sample_time = now;
        }
    }

    /// Record a preemption event.
    pub fn record_preemption(&mut self, order_id: &[u8; 16]) {
        if let Some(tracker) = self.trackers.get_mut(order_id) {
            tracker.preemptions += 1;
        }
    }

    /// Finalize billing for a completed rental.
    pub fn finalize(&mut self, order_id: &[u8; 16]) -> Option<BillingRecord> {
        let tracker = self.trackers.remove(order_id)?;

        let now = now_secs();

        let avg_util = if tracker.gpu_utilization_samples.is_empty() {
            1.0
        } else {
            tracker.gpu_utilization_samples.iter().sum::<f64>()
                / tracker.gpu_utilization_samples.len() as f64
        };

        // Calculate fee
        let vram_multiplier = self.get_vram_multiplier(tracker.vram_mb);
        let base_fee = tracker.total_compute_seconds * self.config.base_rate_per_second;
        let adjusted_fee = (base_fee as f64 * vram_multiplier * avg_util) as u64;

        // Apply preemption discount
        let discount_bps = std::cmp::min(
            tracker.preemptions * self.config.preemption_discount_per_event_bps,
            self.config.max_preemption_discount_bps,
        );
        let discount = adjusted_fee * discount_bps as u64 / 10_000;
        let final_fee = adjusted_fee.saturating_sub(discount);

        let record = BillingRecord {
            order_id: *order_id,
            compute_seconds: tracker.total_compute_seconds,
            avg_gpu_utilization: avg_util,
            vram_seconds: tracker.total_vram_seconds,
            computed_fee: final_fee,
            period_start: tracker.start_time,
            period_end: now,
            preemptions: tracker.preemptions,
            preemption_discount_bps: discount_bps,
        };

        self.completed_records.push(record.clone());

        Some(record)
    }

    /// Get VRAM tier multiplier.
    fn get_vram_multiplier(&self, vram_mb: u32) -> f64 {
        // Find the closest matching tier
        self.config
            .vram_tier_multipliers
            .iter()
            .filter(|(&tier_vram, _)| tier_vram <= vram_mb)
            .max_by_key(|(&tier_vram, _)| tier_vram)
            .map(|(_, &mult)| mult)
            .unwrap_or(1.0)
    }

    /// Get all completed billing records.
    pub fn completed_records(&self) -> &[BillingRecord] {
        &self.completed_records
    }

    /// Total revenue from completed records.
    pub fn total_revenue(&self) -> u64 {
        self.completed_records.iter().map(|r| r.computed_fee).sum()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_billing_flow() {
        let mut engine = BillingEngine::new(BillingConfig {
            base_rate_per_second: 100,
            ..Default::default()
        });

        let order_id = [0x01; 16];
        engine.start_tracking(order_id, 8192);

        // Simulate usage
        engine.record_sample(&order_id, 0.75);

        let record = engine.finalize(&order_id).unwrap();
        assert!(record.computed_fee > 0 || record.compute_seconds == 0);
        assert_eq!(record.preemptions, 0);
    }

    #[test]
    fn preemption_discount() {
        let mut engine = BillingEngine::new(BillingConfig::default());

        let order_id = [0x02; 16];
        engine.start_tracking(order_id, 4096);
        engine.record_preemption(&order_id);
        engine.record_preemption(&order_id);

        let record = engine.finalize(&order_id).unwrap();
        assert_eq!(record.preemptions, 2);
        assert_eq!(record.preemption_discount_bps, 400); // 2 * 200bps
    }

    #[test]
    fn vram_multiplier() {
        let engine = BillingEngine::new(BillingConfig::default());

        assert_eq!(engine.get_vram_multiplier(8192), 1.5);
        assert_eq!(engine.get_vram_multiplier(40960), 5.0);
        assert_eq!(engine.get_vram_multiplier(2048), 1.0); // Below smallest tier
    }
}
