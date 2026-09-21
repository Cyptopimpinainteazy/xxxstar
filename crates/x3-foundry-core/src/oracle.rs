//! Price-oracle facade used by Foundry simulations.

pub use x3_oracle::{PriceFeed, PythOracle};

/// Construct a Pyth price oracle with the library's default thresholds.
pub fn default_pyth_oracle() -> PythOracle {
    PythOracle::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundry_can_read_a_feed() {
        let mut oracle = default_pyth_oracle();
        oracle
            .register_feed(PriceFeed {
                id: "BTC/USD".to_string(),
                symbol: "BTC".to_string(),
                price: 0,
                decimals: 8,
                confidence: 0,
                publish_time: 0,
                valid_time: 60,
                prev_publish_time: 0,
                prev_price: 0,
            })
            .expect("feed registers");
        assert_eq!(oracle.price_feeds.len(), 1);
    }
}
