//! Thin Foundry-facing bridge over the standalone revenue crate.

pub use x3_foundry_revenue::{FeeConfig, RevenueError, TreasurySplitConfig};

/// Construct the revenue crate's default fee configuration.
pub fn default_fee_config() -> FeeConfig {
    FeeConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundry_can_build_the_revenue_defaults() {
        let config = default_fee_config();
        assert!(config.validate().is_ok());
        assert!(TreasurySplitConfig::default().validate().is_ok());
    }
}
