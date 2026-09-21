//! Fee Distribution — Revenue splitting and royalty management
//!
//! Manages publisher revenue, marketplace fees, and payment distribution

use crate::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Fee split configuration, in **basis points**.
///
/// It was two `f64` percentages and the publisher's share was `(total as f64 * pct / 100.0) as u128`.
/// Measured, that loses money at the amounts a token balance reaches, because `f64` holds 53 bits and
/// a wei amount needs more:
///
/// ```text
/// 80% of 10^18 + 7              -> 800000000000000000  (exact 800000000000000005, 5 short)
/// 80% of 12345678901234567890   -> 9876543120987654144 (exact 9876543120987654312, 168 short)
/// ```
///
/// Basis points are what this repository's fee splits already use — `x3-accounting-events`'s
/// `FeeSplits` says "the spine may verify that all entries sum to 10 000" — so this is one convention
/// rather than a third, and the share is integer arithmetic that cannot lose a unit to a mantissa.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeSplit {
    pub publisher_bps: u32,
    pub marketplace_bps: u32,
}

/// One hundred percent, in the basis points a split is stated in.
pub const FULL_SPLIT_BPS: u32 = 10_000;

impl FeeSplit {
    pub fn new(publisher_bps: u32, marketplace_bps: u32) -> Self {
        FeeSplit {
            publisher_bps,
            marketplace_bps,
        }
    }

    pub fn validate(&self) -> bool {
        // Exactly 10 000, not "within a hundredth of 100": a split that does not add up is a split
        // whose remainder nobody has decided, and `process_payment` gives the remainder to the
        // marketplace only because the publisher's share is the one computed.
        self.publisher_bps + self.marketplace_bps == FULL_SPLIT_BPS
            && self.publisher_bps > 0
            && self.marketplace_bps > 0
    }
}

/// Fee pool entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEntry {
    pub id: String,
    pub from_account: String,
    pub plugin_id: String,
    pub amount: u128,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Fee pool tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeePool {
    pub total_fees: u128,
    pub unclaimed_fees: u128,
    pub entries: Vec<FeeEntry>,
    pub collection_start: DateTime<Utc>,
}

impl FeePool {
    pub fn new() -> Self {
        FeePool {
            total_fees: 0,
            unclaimed_fees: 0,
            entries: Vec::new(),
            collection_start: Utc::now(),
        }
    }

    /// Record fee
    pub fn record_fee(&mut self, from: &str, plugin_id: &str, amount: u128, reason: &str) {
        let entry = FeeEntry {
            id: format!("fee_{}", self.entries.len()),
            from_account: from.to_string(),
            plugin_id: plugin_id.to_string(),
            amount,
            reason: reason.to_string(),
            timestamp: Utc::now(),
        };

        self.entries.push(entry);
        self.total_fees += amount;
        self.unclaimed_fees += amount;
    }

    /// Get fees by plugin
    pub fn fees_by_plugin(&self, plugin_id: &str) -> u128 {
        self.entries
            .iter()
            .filter(|e| e.plugin_id == plugin_id)
            .map(|e| e.amount)
            .sum()
    }

    /// Clear unclaimed fees (mark as claimed)
    pub fn claim_fees(&mut self) -> u128 {
        let claimed = self.unclaimed_fees;
        self.unclaimed_fees = 0;
        claimed
    }
}

impl Default for FeePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Payment distribution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentDistribution {
    pub id: String,
    pub plugin_id: String,
    pub publisher: String,
    pub total_amount: u128,
    pub publisher_share: u128,
    pub marketplace_share: u128,
    pub source: String, // "downloads", "endorsement", "license"
    pub timestamp: DateTime<Utc>,
    pub claimed: bool,
}

/// Fee Distribution Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeDistribution {
    fee_split: FeeSplit,
    fee_pool: FeePool,
    distributions: HashMap<String, PaymentDistribution>,
    distribution_counter: u32,
    publisher_balances: HashMap<String, u128>,
    marketplace_balance: u128,
}

impl FeeDistribution {
    pub fn new(fee_split: FeeSplit) -> Self {
        assert!(fee_split.validate(), "Invalid fee split percentages");

        FeeDistribution {
            fee_split,
            fee_pool: FeePool::new(),
            distributions: HashMap::new(),
            distribution_counter: 0,
            publisher_balances: HashMap::new(),
            marketplace_balance: 0,
        }
    }

    /// Default 80/20 split
    pub fn default_split() -> Self {
        Self::new(FeeSplit::new(8_000, 2_000))
    }

    /// Process payment (distribute to publisher and marketplace)
    pub fn process_payment(
        &mut self,
        plugin_id: &str,
        publisher: &str,
        total_amount: u128,
        source: &str,
    ) -> Result<String> {
        // Integer arithmetic on the total, so no unit is lost to a mantissa: the publisher's share is
        // floored to a whole unit and the *marketplace takes the remainder*, which is what makes the
        // two sum to the amount that was paid in.
        let publisher_share =
            total_amount * u128::from(self.fee_split.publisher_bps) / u128::from(FULL_SPLIT_BPS);
        let marketplace_share = total_amount - publisher_share;

        self.distribution_counter += 1;
        let distribution_id = format!("dist_{}", self.distribution_counter);

        let distribution = PaymentDistribution {
            id: distribution_id.clone(),
            plugin_id: plugin_id.to_string(),
            publisher: publisher.to_string(),
            total_amount,
            publisher_share,
            marketplace_share,
            source: source.to_string(),
            timestamp: Utc::now(),
            claimed: false,
        };

        self.distributions
            .insert(distribution_id.clone(), distribution);

        // Update balances
        *self
            .publisher_balances
            .entry(publisher.to_string())
            .or_insert(0) += publisher_share;
        self.marketplace_balance += marketplace_share;

        // Record in fee pool
        self.fee_pool
            .record_fee(publisher, plugin_id, total_amount, "plugin_revenue");

        Ok(distribution_id)
    }

    /// Get publisher balance
    pub fn publisher_balance(&self, publisher: &str) -> u128 {
        self.publisher_balances.get(publisher).copied().unwrap_or(0)
    }

    /// Get marketplace balance
    pub fn marketplace_balance(&self) -> u128 {
        self.marketplace_balance
    }

    /// Claim earnings for publisher
    pub fn claim_earnings(&mut self, publisher: &str) -> u128 {
        self.publisher_balances.remove(publisher).unwrap_or(0)
    }

    /// Get payment distribution
    pub fn get_distribution(&self, distribution_id: &str) -> Option<PaymentDistribution> {
        self.distributions.get(distribution_id).cloned()
    }

    /// Get distributions for plugin
    pub fn distributions_by_plugin(&self, plugin_id: &str) -> Vec<PaymentDistribution> {
        self.distributions
            .values()
            .filter(|d| d.plugin_id == plugin_id)
            .cloned()
            .collect()
    }

    /// Total earned by publisher (claimed + unclaimed)
    pub fn total_earned(&self, publisher: &str) -> u128 {
        self.distributions
            .values()
            .filter(|d| d.publisher == publisher)
            .map(|d| d.publisher_share)
            .sum()
    }

    /// Total claimed by publisher
    pub fn total_claimed(&self, publisher: &str) -> u128 {
        self.distributions
            .values()
            .filter(|d| d.publisher == publisher && d.claimed)
            .map(|d| d.publisher_share)
            .sum()
    }

    /// Mark distribution as claimed
    pub fn mark_claimed(&mut self, distribution_id: &str) -> Result<()> {
        if let Some(dist) = self.distributions.get_mut(distribution_id) {
            dist.claimed = true;
            Ok(())
        } else {
            Err(crate::MarketplaceError::PluginNotFound)
        }
    }

    /// Revenue split info, in basis points.
    pub fn fee_split_info(&self) -> (u32, u32) {
        (self.fee_split.publisher_bps, self.fee_split.marketplace_bps)
    }

    /// Cumulative statistics
    pub fn statistics(&self) -> DistributionStats {
        let total_distributed: u128 = self.distributions.values().map(|d| d.total_amount).sum();
        let total_publisher_earned: u128 =
            self.distributions.values().map(|d| d.publisher_share).sum();

        let publishers: std::collections::HashSet<_> =
            self.distributions.values().map(|d| &d.publisher).collect();

        DistributionStats {
            total_distributed,
            total_publisher_earned,
            total_marketplace_earned: self.marketplace_balance,
            distribution_count: self.distributions.len() as u32,
            publisher_count: publishers.len() as u32,
        }
    }
}

impl Default for FeeDistribution {
    fn default() -> Self {
        Self::default_split()
    }
}

/// Distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStats {
    pub total_distributed: u128,
    pub total_publisher_earned: u128,
    pub total_marketplace_earned: u128,
    pub distribution_count: u32,
    pub publisher_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_split_validation() {
        let valid = FeeSplit::new(8_000, 2_000);
        assert!(valid.validate());

        let invalid = FeeSplit::new(8_000, 2_100);
        assert!(!invalid.validate());
    }

    #[test]
    fn a_share_of_a_token_amount_is_exact_and_the_two_shares_are_the_whole() {
        // The defect this replaced, measured: the publisher's share was
        // `(total as f64 * 80.0 / 100.0) as u128`, and `f64` holds 53 bits — so for the amounts a
        // token balance actually reaches the share came out short:
        //
        //   80% of 10^18 + 7              -> 800000000000000000  (exact 800000000000000005)
        //   80% of 12345678901234567890   -> 9876543120987654144 (exact 9876543120987654312)
        //
        // The integer form is exact, and the marketplace takes the remainder, so the two shares are
        // always the amount that was paid in.
        let mut distributor = FeeDistribution::default_split();
        for (total, exact_publisher) in [
            (10u128.pow(18) + 7, 800_000_000_000_000_005u128),
            (12_345_678_901_234_567_890, 9_876_543_120_987_654_312),
            (10u128.pow(24), 800_000_000_000_000_000_000_000),
            (7, 5),
        ] {
            let id = distributor
                .process_payment("plugin", "publisher", total, "test")
                .expect("a payment is distributed");
            let distribution = distributor
                .distributions
                .get(&id)
                .expect("the distribution is recorded");
            assert_eq!(
                distribution.publisher_share, exact_publisher,
                "80% of {total} must be {exact_publisher}, not the float's approximation"
            );
            assert_eq!(
                distribution.publisher_share + distribution.marketplace_share,
                total,
                "the two shares are the whole amount: {:?}",
                distribution
            );
        }
    }

    #[test]
    fn a_split_that_does_not_add_up_is_refused() {
        // The remainder has to be somebody's: `process_payment` gives it to the marketplace because the
        // publisher's share is the one computed, and a split that leaves a hundredth unassigned is a
        // configuration nobody decided.
        assert!(!FeeSplit::new(8_000, 1_999).validate());
        assert!(
            !FeeSplit::new(0, 10_000).validate(),
            "a zero share is not a split"
        );
        assert!(!FeeSplit::new(10_000, 0).validate());
        assert!(FeeSplit::new(9_999, 1).validate());
    }

    #[test]
    fn test_process_payment() {
        let mut distributor = FeeDistribution::default_split();
        let dist_id = distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();

        assert!(!dist_id.is_empty());
        assert_eq!(distributor.publisher_balance("publisher1"), 800); // 80% of 1000
        assert_eq!(distributor.marketplace_balance(), 200); // 20% of 1000
    }

    #[test]
    fn test_claim_earnings() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();

        let claimed = distributor.claim_earnings("publisher1");
        assert_eq!(claimed, 800);
        assert_eq!(distributor.publisher_balance("publisher1"), 0);
    }

    #[test]
    fn test_multiple_payments() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();
        distributor
            .process_payment("plugin1", "publisher1", 500, "license")
            .unwrap();

        assert_eq!(distributor.publisher_balance("publisher1"), 1200); // (800 + 400)
    }

    #[test]
    fn test_distributions_by_plugin() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();
        distributor
            .process_payment("plugin2", "publisher2", 500, "download")
            .unwrap();

        let plugin1_dists = distributor.distributions_by_plugin("plugin1");
        assert_eq!(plugin1_dists.len(), 1);
    }

    #[test]
    fn test_total_earned() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();
        distributor
            .process_payment("plugin2", "publisher1", 500, "download")
            .unwrap();

        let total = distributor.total_earned("publisher1");
        assert_eq!(total, 1200); // 800 + 400
    }

    #[test]
    fn test_statistics() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "pub1", 1000, "download")
            .unwrap();
        distributor
            .process_payment("plugin2", "pub2", 1000, "download")
            .unwrap();

        let stats = distributor.statistics();
        assert_eq!(stats.total_distributed, 2000);
        assert_eq!(stats.total_publisher_earned, 1600); // 80% x 2
        assert_eq!(stats.total_marketplace_earned, 400); // 20% x 2
        assert_eq!(stats.publisher_count, 2);
    }

    #[test]
    fn test_custom_fee_split() {
        let split = FeeSplit::new(7_000, 3_000);
        let mut distributor = FeeDistribution::new(split);
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();

        assert_eq!(distributor.publisher_balance("publisher1"), 700); // 70% of 1000
        assert_eq!(distributor.marketplace_balance(), 300); // 30% of 1000
    }

    #[test]
    fn test_fee_pool_tracking() {
        let mut distributor = FeeDistribution::default_split();
        distributor
            .process_payment("plugin1", "publisher1", 1000, "download")
            .unwrap();

        let fees = distributor.fee_pool.fees_by_plugin("plugin1");
        assert_eq!(fees, 1000);
    }
}
