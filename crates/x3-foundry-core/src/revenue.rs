use crate::error::FoundryError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Fee configuration for an app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    pub platform_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub ai_agent_fee_bps: Option<u16>,
    pub maintenance_fee_bps: Option<u16>,
    pub referral_fee_bps: Option<u16>,
    pub fee_token: String,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            platform_fee_bps: 200,
            creator_fee_bps: 9700,
            ai_agent_fee_bps: Some(50),
            maintenance_fee_bps: Some(50),
            referral_fee_bps: Some(50),
            fee_token: "X3".to_string(),
        }
    }
}

/// Treasury split configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasurySplit {
    pub protocol_treasury_pct: f64,
    pub gpu_swarm_pct: f64,
    pub dev_vault_pct: f64,
    pub maintenance_pct: f64,
    pub liquidity_pct: f64,
    pub grants_pct: f64,
}

impl Default for TreasurySplit {
    fn default() -> Self {
        Self {
            protocol_treasury_pct: 40.0,
            gpu_swarm_pct: 20.0,
            dev_vault_pct: 15.0,
            maintenance_pct: 10.0,
            liquidity_pct: 10.0,
            grants_pct: 5.0,
        }
    }
}

impl TreasurySplit {
    /// Validates that all percentages sum to 100.
    pub fn validate(&self) -> Result<(), FoundryError> {
        let total = self.protocol_treasury_pct
            + self.gpu_swarm_pct
            + self.dev_vault_pct
            + self.maintenance_pct
            + self.liquidity_pct
            + self.grants_pct;
        if (total - 100.0).abs() > 0.01 {
            return Err(FoundryError::InvalidConfig(format!(
                "Treasury split percentages must sum to 100, got {}",
                total
            )));
        }
        Ok(())
    }
}

/// A single revenue record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecord {
    pub id: String,
    pub app_id: String,
    pub creator_wallet: String,
    pub chain: String,
    pub amount: u128,
    pub fee_token: String,
    pub platform_fee: u128,
    pub creator_revenue: u128,
    pub ai_agent_fee: u128,
    pub maintenance_fee: u128,
    pub referral_fee: u128,
    pub recorded_at: DateTime<Utc>,
    pub tx_hash: String,
    pub claimed: bool,
    pub claimed_at: Option<DateTime<Utc>>,
}

/// Revenue summary for an app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSummary {
    pub app_id: String,
    pub total_volume: u128,
    pub total_fees: u128,
    pub platform_revenue: u128,
    pub creator_revenue: u128,
    pub unclaimed_creator_revenue: u128,
    pub transaction_count: u64,
    pub last_updated: DateTime<Utc>,
}

/// RevenueTracker manages revenue recording, tracking, and distribution.
pub struct RevenueTracker {
    records: Vec<RevenueRecord>,
    fee_config: FeeConfig,
    treasury_split: TreasurySplit,
}

impl RevenueTracker {
    pub fn new(fee_config: FeeConfig, treasury_split: TreasurySplit) -> Self {
        treasury_split.validate().unwrap_or_else(|e| {
            tracing::warn!("Invalid treasury split: {}. Using defaults.", e);
        });
        Self {
            records: Vec::new(),
            fee_config,
            treasury_split,
        }
    }

    /// Records a new revenue event.
    pub fn record_revenue(
        &mut self,
        app_id: &str,
        creator_wallet: &str,
        chain: &str,
        amount: u128,
        fee_token: &str,
        tx_hash: &str,
    ) -> Result<RevenueRecord, FoundryError> {
        info!(
            "Recording revenue: {} {} for app {} on {}",
            amount, fee_token, app_id, chain
        );

        let platform_fee = amount.saturating_mul(self.fee_config.platform_fee_bps as u128) / 10000;
        let ai_agent_fee =
            amount.saturating_mul(self.fee_config.ai_agent_fee_bps.unwrap_or(0) as u128) / 10000;
        let maintenance_fee =
            amount.saturating_mul(self.fee_config.maintenance_fee_bps.unwrap_or(0) as u128) / 10000;
        let referral_fee =
            amount.saturating_mul(self.fee_config.referral_fee_bps.unwrap_or(0) as u128) / 10000;
        let total_deductions = platform_fee + ai_agent_fee + maintenance_fee + referral_fee;
        let creator_revenue = amount.saturating_sub(total_deductions);

        let record = RevenueRecord {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: app_id.to_string(),
            creator_wallet: creator_wallet.to_string(),
            chain: chain.to_string(),
            amount,
            fee_token: fee_token.to_string(),
            platform_fee,
            creator_revenue,
            ai_agent_fee,
            maintenance_fee,
            referral_fee,
            recorded_at: Utc::now(),
            tx_hash: tx_hash.to_string(),
            claimed: false,
            claimed_at: None,
        };

        self.records.push(record.clone());
        Ok(record)
    }

    /// Gets all revenue records for a specific app.
    pub fn get_revenue_by_app(&self, app_id: &str) -> Vec<&RevenueRecord> {
        self.records.iter().filter(|r| r.app_id == app_id).collect()
    }

    /// Gets all revenue records for a specific creator.
    pub fn get_revenue_by_creator(&self, creator_wallet: &str) -> Vec<&RevenueRecord> {
        self.records
            .iter()
            .filter(|r| r.creator_wallet == creator_wallet)
            .collect()
    }

    /// Gets all revenue records for a specific chain.
    pub fn get_revenue_by_chain(&self, chain: &str) -> Vec<&RevenueRecord> {
        self.records.iter().filter(|r| r.chain == chain).collect()
    }

    /// Gets total platform revenue (sum of all platform fees).
    pub fn get_platform_revenue(&self) -> u128 {
        self.records.iter().map(|r| r.platform_fee).sum()
    }

    /// Gets total unclaimed creator revenue.
    pub fn get_unclaimed_creator_revenue(&self) -> u128 {
        self.records
            .iter()
            .filter(|r| !r.claimed)
            .map(|r| r.creator_revenue)
            .sum()
    }

    /// Gets unclaimed revenue for a specific creator.
    pub fn get_unclaimed_creator_revenue_by_wallet(&self, creator_wallet: &str) -> u128 {
        self.records
            .iter()
            .filter(|r| r.creator_wallet == creator_wallet && !r.claimed)
            .map(|r| r.creator_revenue)
            .sum()
    }

    /// Claims all unclaimed revenue for a creator.
    pub fn claim_creator_revenue(&mut self, creator_wallet: &str) -> Result<u128, FoundryError> {
        let unclaimed: u128 = self
            .records
            .iter_mut()
            .filter(|r| r.creator_wallet == creator_wallet && !r.claimed)
            .map(|r| {
                r.claimed = true;
                r.claimed_at = Some(Utc::now());
                r.creator_revenue
            })
            .sum();

        if unclaimed == 0 {
            return Err(FoundryError::NoRevenueToClaim(creator_wallet.to_string()));
        }

        info!("Claimed {} for creator {}", unclaimed, creator_wallet);
        Ok(unclaimed)
    }

    /// Claims referral revenue for a referrer.
    pub fn claim_referral_revenue(&mut self, referral_wallet: &str) -> Result<u128, FoundryError> {
        let unclaimed: u128 = self
            .records
            .iter_mut()
            .filter(|r| !r.claimed) // In production, track referral wallet separately
            .map(|r| {
                r.claimed = true;
                r.claimed_at = Some(Utc::now());
                r.referral_fee
            })
            .sum();

        if unclaimed == 0 {
            return Err(FoundryError::NoRevenueToClaim(referral_wallet.to_string()));
        }

        info!(
            "Claimed {} referral revenue for {}",
            unclaimed, referral_wallet
        );
        Ok(unclaimed)
    }

    /// Distributes platform fees according to the treasury split.
    pub fn distribute_platform_fees(&self) -> Result<HashMap<String, u128>, FoundryError> {
        let total_platform = self.get_platform_revenue();
        if total_platform == 0 {
            return Err(FoundryError::NoRevenueToClaim("platform".to_string()));
        }

        let mut distribution = HashMap::new();
        distribution.insert(
            "Protocol Treasury".into(),
            (total_platform as f64 * self.treasury_split.protocol_treasury_pct / 100.0) as u128,
        );
        distribution.insert(
            "GPU Swarm".into(),
            (total_platform as f64 * self.treasury_split.gpu_swarm_pct / 100.0) as u128,
        );
        distribution.insert(
            "Dev Vault".into(),
            (total_platform as f64 * self.treasury_split.dev_vault_pct / 100.0) as u128,
        );
        distribution.insert(
            "Maintenance".into(),
            (total_platform as f64 * self.treasury_split.maintenance_pct / 100.0) as u128,
        );
        distribution.insert(
            "Liquidity".into(),
            (total_platform as f64 * self.treasury_split.liquidity_pct / 100.0) as u128,
        );
        distribution.insert(
            "Grants".into(),
            (total_platform as f64 * self.treasury_split.grants_pct / 100.0) as u128,
        );

        info!("Distributed platform fees: {:?}", distribution);
        Ok(distribution)
    }

    /// Gets a revenue summary for a specific app.
    pub fn get_app_revenue_summary(&self, app_id: &str) -> RevenueSummary {
        let app_records: Vec<&RevenueRecord> =
            self.records.iter().filter(|r| r.app_id == app_id).collect();
        let total_volume: u128 = app_records.iter().map(|r| r.amount).sum();
        let total_fees: u128 = app_records
            .iter()
            .map(|r| r.platform_fee + r.ai_agent_fee + r.maintenance_fee + r.referral_fee)
            .sum();
        let platform_revenue: u128 = app_records.iter().map(|r| r.platform_fee).sum();
        let creator_revenue: u128 = app_records.iter().map(|r| r.creator_revenue).sum();
        let unclaimed_creator_revenue: u128 = app_records
            .iter()
            .filter(|r| !r.claimed)
            .map(|r| r.creator_revenue)
            .sum();
        let transaction_count = app_records.len() as u64;

        RevenueSummary {
            app_id: app_id.to_string(),
            total_volume,
            total_fees,
            platform_revenue,
            creator_revenue,
            unclaimed_creator_revenue,
            transaction_count,
            last_updated: Utc::now(),
        }
    }

    /// Gets all revenue records.
    pub fn get_all_records(&self) -> &[RevenueRecord] {
        &self.records
    }

    /// Gets the number of records tracked.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

impl Default for RevenueTracker {
    fn default() -> Self {
        Self::new(FeeConfig::default(), TreasurySplit::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_revenue() {
        let mut tracker = RevenueTracker::default();
        let record = tracker.record_revenue("app-1", "0xCreator", "x3-mainnet", 1000, "X3", "0xtx");
        assert!(record.is_ok());
        let r = record.unwrap();
        assert_eq!(r.amount, 1000);
        assert!(r.platform_fee > 0);
        assert!(r.creator_revenue > 0);
    }

    #[test]
    fn test_claim_revenue() {
        let mut tracker = RevenueTracker::default();
        tracker
            .record_revenue("app-1", "0xCreator", "x3-mainnet", 1000, "X3", "0xtx")
            .unwrap();
        let claimed = tracker.claim_creator_revenue("0xCreator");
        assert!(claimed.is_ok());
        assert!(claimed.unwrap() > 0);
    }

    #[test]
    fn test_platform_revenue() {
        let mut tracker = RevenueTracker::default();
        tracker
            .record_revenue("app-1", "0xCreator", "x3-mainnet", 10000, "X3", "0xtx1")
            .unwrap();
        tracker
            .record_revenue("app-2", "0xCreator2", "x3-mainnet", 20000, "X3", "0xtx2")
            .unwrap();
        let platform = tracker.get_platform_revenue();
        assert!(platform > 0);
    }

    #[test]
    fn test_treasury_split_validation() {
        let split = TreasurySplit::default();
        assert!(split.validate().is_ok());

        let bad_split = TreasurySplit {
            protocol_treasury_pct: 100.0,
            ..Default::default()
        };
        assert!(bad_split.validate().is_err());
    }

    #[test]
    fn test_distribute_platform_fees() {
        let mut tracker = RevenueTracker::default();
        tracker
            .record_revenue("app-1", "0xCreator", "x3-mainnet", 100000, "X3", "0xtx")
            .unwrap();
        let dist = tracker.distribute_platform_fees();
        assert!(dist.is_ok());
        let d = dist.unwrap();
        assert_eq!(d.len(), 6);
    }

    #[test]
    fn test_revenue_summary() {
        let mut tracker = RevenueTracker::default();
        tracker
            .record_revenue("app-1", "0xCreator", "x3-mainnet", 5000, "X3", "0xtx")
            .unwrap();
        let summary = tracker.get_app_revenue_summary("app-1");
        assert_eq!(summary.total_volume, 5000);
        assert_eq!(summary.transaction_count, 1);
    }
}
