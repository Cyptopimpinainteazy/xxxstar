//! # X3 Foundry Revenue
//!
//! Revenue calculation and distribution for the X3 Foundry ecosystem.
//! All fee calculations use basis points (bps) where 10000 bps = 100%.

use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Maximum basis points (100%).
pub const MAX_BASIS_POINTS: u64 = 10_000;

/// Default platform fee in basis points (5%).
pub const DEFAULT_PLATFORM_FEE_BPS: u64 = 500;

/// Default creator fee in basis points (85%).
pub const DEFAULT_CREATOR_FEE_BPS: u64 = 8_500;

/// Default referral fee in basis points (5%).
pub const DEFAULT_REFERRAL_FEE_BPS: u64 = 500;

/// Default treasury fee in basis points (5%).
pub const DEFAULT_TREASURY_FEE_BPS: u64 = 500;

/// Minimum fee in basis points (0.1%).
pub const MIN_FEE_BPS: u64 = 10;

/// Maximum fee in basis points (50%).
pub const MAX_FEE_BPS: u64 = 5_000;

/// Errors that can occur during revenue operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RevenueError {
    #[error("Invalid basis points: {0}. Must be between 0 and 10000")]
    InvalidBasisPoints(u64),
    #[error("Fee exceeds maximum allowed: {0} bps > {1} bps")]
    FeeExceedsMax(u64, u64),
    #[error("Fee below minimum allowed: {0} bps < {1} bps")]
    FeeBelowMin(u64, u64),
    #[error("Total fee split does not equal 10000 bps: {0}")]
    InvalidFeeSplit(u64),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Calculation overflow")]
    CalculationOverflow,
    #[error("Decimal conversion error: {0}")]
    DecimalError(String),
}

impl From<RevenueError> for anyhow::Error {
    fn from(e: RevenueError) -> Self {
        anyhow::anyhow!("{}", e)
    }
}

/// Configuration for treasury fee splitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasurySplitConfig {
    /// Percentage of treasury share allocated to operations (basis points).
    pub operations_bps: u64,
    /// Percentage of treasury share allocated to development (basis points).
    pub development_bps: u64,
    /// Percentage of treasury share allocated to marketing (basis points).
    pub marketing_bps: u64,
    /// Percentage of treasury share allocated to reserves (basis points).
    pub reserves_bps: u64,
    /// Percentage of treasury share allocated to community rewards (basis points).
    pub community_bps: u64,
}

impl Default for TreasurySplitConfig {
    fn default() -> Self {
        Self {
            operations_bps: 3_000,
            development_bps: 2_500,
            marketing_bps: 2_000,
            reserves_bps: 1_500,
            community_bps: 1_000,
        }
    }
}

impl TreasurySplitConfig {
    /// Validate that the split sums to 10000 bps.
    pub fn validate(&self) -> Result<(), RevenueError> {
        let total = self.operations_bps
            + self.development_bps
            + self.marketing_bps
            + self.reserves_bps
            + self.community_bps;
        if total != MAX_BASIS_POINTS {
            Err(RevenueError::InvalidFeeSplit(total))
        } else {
            Ok(())
        }
    }

    /// Create a new TreasurySplitConfig with validation.
    pub fn new(
        operations_bps: u64,
        development_bps: u64,
        marketing_bps: u64,
        reserves_bps: u64,
        community_bps: u64,
    ) -> Result<Self, RevenueError> {
        let config = Self {
            operations_bps,
            development_bps,
            marketing_bps,
            reserves_bps,
            community_bps,
        };
        config.validate()?;
        Ok(config)
    }
}

/// Configuration for fee distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    pub platform_fee_bps: u64,
    pub creator_fee_bps: u64,
    pub referral_fee_bps: u64,
    pub treasury_fee_bps: u64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            platform_fee_bps: DEFAULT_PLATFORM_FEE_BPS,
            creator_fee_bps: DEFAULT_CREATOR_FEE_BPS,
            referral_fee_bps: DEFAULT_REFERRAL_FEE_BPS,
            treasury_fee_bps: DEFAULT_TREASURY_FEE_BPS,
        }
    }
}

impl FeeConfig {
    /// Validate that all fees sum to 10000 bps.
    pub fn validate(&self) -> Result<(), RevenueError> {
        let total = self.platform_fee_bps
            + self.creator_fee_bps
            + self.referral_fee_bps
            + self.treasury_fee_bps;
        if total != MAX_BASIS_POINTS {
            Err(RevenueError::InvalidFeeSplit(total))
        } else {
            Ok(())
        }
    }

    /// Create a new FeeConfig with validation.
    pub fn new(
        platform_fee_bps: u64,
        creator_fee_bps: u64,
        referral_fee_bps: u64,
        treasury_fee_bps: u64,
    ) -> Result<Self, RevenueError> {
        let config = Self {
            platform_fee_bps,
            creator_fee_bps,
            referral_fee_bps,
            treasury_fee_bps,
        };
        config.validate()?;
        Ok(config)
    }
}

/// A revenue report for a specific period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReport {
    pub report_id: String,
    pub dapp_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_revenue: Decimal,
    pub platform_share: Decimal,
    pub creator_share: Decimal,
    pub referral_share: Decimal,
    pub treasury_share: Decimal,
    pub treasury_split: TreasurySplit,
    pub transaction_count: u64,
    pub generated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Breakdown of treasury allocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasurySplit {
    pub total: Decimal,
    pub operations: Decimal,
    pub development: Decimal,
    pub marketing: Decimal,
    pub reserves: Decimal,
    pub community: Decimal,
}

/// All calculated shares for a given amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllShares {
    pub platform: Decimal,
    pub creator: Decimal,
    pub referral: Decimal,
    pub treasury: Decimal,
    pub treasury_split: TreasurySplit,
}

/// The main revenue calculator.
pub struct RevenueCalculator {
    fee_config: FeeConfig,
    treasury_split: TreasurySplitConfig,
}

impl RevenueCalculator {
    /// Create a new RevenueCalculator with default fee configuration.
    pub fn new() -> Self {
        Self {
            fee_config: FeeConfig::default(),
            treasury_split: TreasurySplitConfig::default(),
        }
    }

    /// Create a new RevenueCalculator with custom fee configuration.
    pub fn with_config(fee_config: FeeConfig, treasury_split: TreasurySplitConfig) -> Self {
        Self {
            fee_config,
            treasury_split,
        }
    }

    /// Get the current fee configuration.
    pub fn fee_config(&self) -> &FeeConfig {
        &self.fee_config
    }

    /// Get the current treasury split configuration.
    pub fn treasury_split_config(&self) -> &TreasurySplitConfig {
        &self.treasury_split
    }

    /// Calculate the platform share of a given amount.
    pub fn calculate_platform_share(&self, amount: &Decimal) -> Result<Decimal, RevenueError> {
        Self::calculate_share(amount, self.fee_config.platform_fee_bps)
    }

    /// Calculate the creator share of a given amount.
    pub fn calculate_creator_share(&self, amount: &Decimal) -> Result<Decimal, RevenueError> {
        Self::calculate_share(amount, self.fee_config.creator_fee_bps)
    }

    /// Calculate the referral share of a given amount.
    pub fn calculate_referral_share(&self, amount: &Decimal) -> Result<Decimal, RevenueError> {
        Self::calculate_share(amount, self.fee_config.referral_fee_bps)
    }

    /// Calculate the treasury share of a given amount.
    pub fn calculate_treasury_share(&self, amount: &Decimal) -> Result<Decimal, RevenueError> {
        Self::calculate_share(amount, self.fee_config.treasury_fee_bps)
    }

    /// Calculate the treasury split breakdown.
    pub fn calculate_treasury_split(&self, treasury_amount: &Decimal) -> Result<TreasurySplit, RevenueError> {
        Ok(TreasurySplit {
            total: *treasury_amount,
            operations: Self::calculate_share(treasury_amount, self.treasury_split.operations_bps)?,
            development: Self::calculate_share(treasury_amount, self.treasury_split.development_bps)?,
            marketing: Self::calculate_share(treasury_amount, self.treasury_split.marketing_bps)?,
            reserves: Self::calculate_share(treasury_amount, self.treasury_split.reserves_bps)?,
            community: Self::calculate_share(treasury_amount, self.treasury_split.community_bps)?,
        })
    }

    /// Calculate all shares for a given amount at once.
    pub fn calculate_all_shares(&self, amount: &Decimal) -> Result<AllShares, RevenueError> {
        let platform = self.calculate_platform_share(amount)?;
        let creator = self.calculate_creator_share(amount)?;
        let referral = self.calculate_referral_share(amount)?;
        let treasury = self.calculate_treasury_share(amount)?;
        let treasury_split = self.calculate_treasury_split(&treasury)?;

        Ok(AllShares {
            platform,
            creator,
            referral,
            treasury,
            treasury_split,
        })
    }

    /// Generate a revenue report for a dApp over a period.
    pub fn generate_report(
        &self,
        dapp_id: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        total_revenue: &Decimal,
        transaction_count: u64,
    ) -> Result<RevenueReport, RevenueError> {
        let all_shares = self.calculate_all_shares(total_revenue)?;

        Ok(RevenueReport {
            report_id: format!("rev-{}-{}", dapp_id, Utc::now().timestamp()),
            dapp_id: dapp_id.to_string(),
            period_start,
            period_end,
            total_revenue: *total_revenue,
            platform_share: all_shares.platform,
            creator_share: all_shares.creator,
            referral_share: all_shares.referral,
            treasury_share: all_shares.treasury,
            treasury_split: all_shares.treasury_split,
            transaction_count,
            generated_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Internal: calculate a share using basis points.
    fn calculate_share(amount: &Decimal, bps: u64) -> Result<Decimal, RevenueError> {
        if bps > MAX_BASIS_POINTS {
            return Err(RevenueError::InvalidBasisPoints(bps));
        }
        let bps_decimal = Decimal::from_u64(bps)
            .ok_or(RevenueError::CalculationOverflow)?;
        let max_bps = Decimal::from_u64(MAX_BASIS_POINTS)
            .ok_or(RevenueError::CalculationOverflow)?;
        let result = amount.checked_mul(bps_decimal)
            .ok_or(RevenueError::CalculationOverflow)?
            .checked_div(max_bps)
            .ok_or(RevenueError::CalculationOverflow)?;
        Ok(result.round_dp(18))
    }
}

impl Default for RevenueCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates fee configurations against platform rules.
pub struct FeeValidator;

impl FeeValidator {
    /// Validate a complete fee configuration.
    pub fn validate_fee_config(config: &FeeConfig) -> Result<(), Vec<RevenueError>> {
        let mut errors = Vec::new();

        if let Err(e) = Self::validate_basis_points(config.platform_fee_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.creator_fee_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.referral_fee_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.treasury_fee_bps) {
            errors.push(e);
        }

        if let Err(e) = config.validate() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate a single basis points value.
    pub fn validate_basis_points(bps: u64) -> Result<(), RevenueError> {
        if bps > MAX_BASIS_POINTS {
            return Err(RevenueError::InvalidBasisPoints(bps));
        }
        if bps > MAX_FEE_BPS {
            return Err(RevenueError::FeeExceedsMax(bps, MAX_FEE_BPS));
        }
        if bps > 0 && bps < MIN_FEE_BPS {
            return Err(RevenueError::FeeBelowMin(bps, MIN_FEE_BPS));
        }
        Ok(())
    }

    /// Validate a treasury split configuration.
    pub fn validate_treasury_split(config: &TreasurySplitConfig) -> Result<(), Vec<RevenueError>> {
        let mut errors = Vec::new();

        if let Err(e) = Self::validate_basis_points(config.operations_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.development_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.marketing_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.reserves_bps) {
            errors.push(e);
        }
        if let Err(e) = Self::validate_basis_points(config.community_bps) {
            errors.push(e);
        }

        if let Err(e) = config.validate() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_default_fee_config() {
        let config = FeeConfig::default();
        assert_eq!(config.platform_fee_bps, DEFAULT_PLATFORM_FEE_BPS);
        assert_eq!(config.creator_fee_bps, DEFAULT_CREATOR_FEE_BPS);
        assert_eq!(config.referral_fee_bps, DEFAULT_REFERRAL_FEE_BPS);
        assert_eq!(config.treasury_fee_bps, DEFAULT_TREASURY_FEE_BPS);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_fee_config() {
        let config = FeeConfig::new(5000, 5000, 0, 0);
        assert!(config.is_err());
    }

    #[test]
    fn test_default_treasury_split() {
        let config = TreasurySplitConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_calculate_platform_share() {
        let calc = RevenueCalculator::new();
        let amount = dec!(1000);
        let share = calc.calculate_platform_share(&amount).unwrap();
        assert_eq!(share, dec!(50)); // 5% of 1000
    }

    #[test]
    fn test_calculate_creator_share() {
        let calc = RevenueCalculator::new();
        let amount = dec!(1000);
        let share = calc.calculate_creator_share(&amount).unwrap();
        assert_eq!(share, dec!(850)); // 85% of 1000
    }

    #[test]
    fn test_calculate_referral_share() {
        let calc = RevenueCalculator::new();
        let amount = dec!(1000);
        let share = calc.calculate_referral_share(&amount).unwrap();
        assert_eq!(share, dec!(50)); // 5% of 1000
    }

    #[test]
    fn test_calculate_treasury_share() {
        let calc = RevenueCalculator::new();
        let amount = dec!(1000);
        let share = calc.calculate_treasury_share(&amount).unwrap();
        assert_eq!(share, dec!(50)); // 5% of 1000
    }

    #[test]
    fn test_calculate_all_shares() {
        let calc = RevenueCalculator::new();
        let amount = dec!(10000);
        let shares = calc.calculate_all_shares(&amount).unwrap();
        assert_eq!(shares.platform, dec!(500));
        assert_eq!(shares.creator, dec!(8500));
        assert_eq!(shares.referral, dec!(500));
        assert_eq!(shares.treasury, dec!(500));
    }

    #[test]
    fn test_calculate_treasury_split() {
        let calc = RevenueCalculator::new();
        let treasury = dec!(1000);
        let split = calc.calculate_treasury_split(&treasury).unwrap();
        assert_eq!(split.total, dec!(1000));
        assert_eq!(split.operations, dec!(300));
        assert_eq!(split.development, dec!(250));
        assert_eq!(split.marketing, dec!(200));
        assert_eq!(split.reserves, dec!(150));
        assert_eq!(split.community, dec!(100));
    }

    #[test]
    fn test_generate_report() {
        let calc = RevenueCalculator::new();
        let start = Utc::now();
        let end = Utc::now();
        let report = calc.generate_report("dapp-1", start, end, &dec!(5000), 10).unwrap();
        assert_eq!(report.dapp_id, "dapp-1");
        assert_eq!(report.total_revenue, dec!(5000));
        assert_eq!(report.transaction_count, 10);
        assert_eq!(report.platform_share, dec!(250));
        assert_eq!(report.creator_share, dec!(4250));
    }

    #[test]
    fn test_fee_validator_valid() {
        let config = FeeConfig::default();
        assert!(FeeValidator::validate_fee_config(&config).is_ok());
    }

    #[test]
    fn test_fee_validator_invalid_bps() {
        assert!(FeeValidator::validate_basis_points(MAX_BASIS_POINTS + 1).is_err());
        assert!(FeeValidator::validate_basis_points(MAX_FEE_BPS + 1000).is_err());
    }

    #[test]
    fn test_zero_amount() {
        let calc = RevenueCalculator::new();
        let share = calc.calculate_platform_share(&dec!(0)).unwrap();
        assert_eq!(share, dec!(0));
    }

    #[test]
    fn test_large_amount() {
        let calc = RevenueCalculator::new();
        let amount = dec!(1000000000000);
        let share = calc.calculate_platform_share(&amount).unwrap();
        assert_eq!(share, dec!(50000000000));
    }

    #[test]
    fn test_custom_config() {
        let fee_config = FeeConfig::new(1000, 7000, 1000, 1000).unwrap();
        let treasury = TreasurySplitConfig::new(2000, 2000, 2000, 2000, 2000).unwrap();
        let calc = RevenueCalculator::with_config(fee_config, treasury);
        let amount = dec!(1000);
        let shares = calc.calculate_all_shares(&amount).unwrap();
        assert_eq!(shares.platform, dec!(100));
        assert_eq!(shares.creator, dec!(700));
        assert_eq!(shares.referral, dec!(100));
        assert_eq!(shares.treasury, dec!(100));
    }
}
