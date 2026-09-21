//! X3 Price Oracles
//!
//! Pyth Network price feeds and TWAP calculation for on-chain oracle.

pub mod pyth_oracle;

pub use pyth_oracle::{PriceFeed, PythOracle};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registered_feed_is_updateable() {
        let mut oracle = PythOracle::new();
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
        oracle
            .update_price("BTC", 50_000_000_000, 1, 10, 11)
            .expect("fresh price updates");
    }

    #[test]
    fn a_stale_price_is_refused() {
        let mut oracle = PythOracle::new();
        oracle
            .register_feed(PriceFeed {
                id: "ETH/USD".to_string(),
                symbol: "ETH".to_string(),
                price: 0,
                decimals: 8,
                confidence: 0,
                publish_time: 0,
                valid_time: 60,
                prev_publish_time: 0,
                prev_price: 0,
            })
            .expect("feed registers");
        assert_eq!(
            oracle.update_price("ETH", 1_000_000_000, 1, 1, 999),
            Err("Price too stale".to_string())
        );
    }
}
