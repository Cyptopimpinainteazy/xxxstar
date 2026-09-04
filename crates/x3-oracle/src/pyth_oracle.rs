//! Pyth Network Price Oracle Integration
//!
//! Real-time price feeds for XL AMM, derivatives, and liquidations
//! On-chain oracle with pulled latest prices from Pyth Network.

use std::collections::HashMap;

/// Price feed from Pyth
#[derive(Clone, Debug)]
pub struct PriceFeed {
    pub id: String,                     // Pyth price feed ID (e.g., "Crypto.BTC/USD")
    pub symbol: String,                 // BTC, ETH, SOL, etc.
    pub price: i64,                     // Price in USD * 10^(decimals)
    pub decimals: u8,                   // -8 for most feeds
    pub confidence: u64,                // Confidence interval
    pub publish_time: u64,              // Unix timestamp
    pub valid_time: u64,                // How long price is valid
    pub prev_publish_time: u64,
    pub prev_price: i64,
}

/// Pyth oracle state
pub struct PythOracle {
    pub price_feeds: HashMap<String, PriceFeed>,
    pub price_history: HashMap<String, Vec<(u64, i64)>>, // symbol → (timestamp, price)
    pub heartbeat_interval_secs: u64,   // Update interval (typically 1-5 seconds)
    pub staleness_threshold_secs: u64,  // Max age of price (e.g., 60 seconds)
}

impl PythOracle {
    pub fn new() -> Self {
        Self {
            price_feeds: HashMap::new(),
            price_history: HashMap::new(),
            heartbeat_interval_secs: 5,
            staleness_threshold_secs: 60,
        }
    }

    /// Register Pyth price feed
    pub fn register_feed(&mut self, feed: PriceFeed) -> Result<(), String> {
        if feed.symbol.is_empty() || feed.id.is_empty() {
            return Err("Feed symbol and ID required".to_string());
        }

        self.price_feeds.insert(feed.symbol.clone(), feed);
        Ok(())
    }

    /// Update price from Pyth network (via off-chain worker)
    pub fn update_price(
        &mut self,
        symbol: &str,
        price: i64,
        confidence: u64,
        publish_time: u64,
        now: u64,
    ) -> Result<(), String> {
        let mut feed = self
            .price_feeds
            .get(symbol)
            .ok_or("Feed not found")?
            .clone();

        // Verify price is fresh (not stale)
        if now > publish_time + self.staleness_threshold_secs {
            return Err("Price too stale".to_string());
        }

        // Update price
        feed.prev_price = feed.price;
        feed.prev_publish_time = feed.publish_time;
        feed.price = price;
        feed.confidence = confidence;
        feed.publish_time = publish_time;

        // Log price
        self.price_history
            .entry(symbol.to_string())
            .or_insert_with(Vec::new)
            .push((publish_time, price));

        // Keep history window (last 1000 prices)
        if let Some(history) = self.price_history.get_mut(symbol) {
            if history.len() > 1000 {
                history.remove(0);
            }
        }

        self.price_feeds.insert(symbol.to_string(), feed);

        Ok(())
    }

    /// Get current price
    pub fn get_price(&self, symbol: &str) -> Result<i64, String> {
        let feed = self.price_feeds.get(symbol).ok_or("Feed not found")?;

        if feed.price == 0 {
            return Err("Price not available".to_string());
        }

        Ok(feed.price)
    }

    /// Get price with decimals (as f64)
    pub fn get_price_decimal(&self, symbol: &str) -> Result<f64, String> {
        let feed = self.price_feeds.get(symbol).ok_or("Feed not found")?;

        let divisor = 10_i64.pow(feed.decimals as u32);
        Ok((feed.price as f64) / (divisor as f64))
    }

    /// Get price with confidence interval
    pub fn get_price_with_confidence(&self, symbol: &str) -> Result<(i64, u64), String> {
        let feed = self.price_feeds.get(symbol).ok_or("Feed not found")?;

        Ok((feed.price, feed.confidence))
    }

    /// Calculate TWAP (Time-Weighted Average Price) over N periods
    pub fn calculate_twap(&self, symbol: &str, periods: usize) -> Result<i64, String> {
        let history = self
            .price_history
            .get(symbol)
            .ok_or("No price history")?;

        if history.is_empty() {
            return Err("Empty price history".to_string());
        }

        let start_idx = history.len().saturating_sub(periods);
        let prices: Vec<i64> = history[start_idx..].iter().map(|(_, p)| *p).collect();

        let avg = prices.iter().sum::<i64>() / prices.len() as i64;

        Ok(avg)
    }

    /// Detect price anomalies (variance detection)
    pub fn is_price_anomaly(&self, symbol: &str, threshold_pct: f64) -> Result<bool, String> {
        let history = self
            .price_history
            .get(symbol)
            .ok_or("No price history")?;

        if history.len() < 10 {
            return Ok(false); // Not enough data
        }

        // Calculate average price
        let prices: Vec<i64> = history.iter().map(|(_, p)| *p).collect();
        let avg = prices.iter().sum::<i64>() / prices.len() as i64;

        // Check if current price deviates > threshold from average
        let current = prices.last().ok_or("No current price available")?;
        let deviation = ((current - avg).abs() as f64 / avg as f64) * 100.0;

        Ok(deviation > threshold_pct)
    }

    /// Get price feed metadata
    pub fn get_feed_info(&self, symbol: &str) -> Option<PriceFeed> {
        self.price_feeds.get(symbol).cloned()
    }

    /// Set staleness threshold
    pub fn set_staleness_threshold(&mut self, secs: u64) {
        self.staleness_threshold_secs = secs;
    }

    /// Get price change percentage
    pub fn get_price_change_pct(&self, symbol: &str) -> Result<f64, String> {
        let feed = self.price_feeds.get(symbol).ok_or("Feed not found")?;

        if feed.prev_price == 0 {
            return Ok(0.0);
        }

        let change = ((feed.price - feed.prev_price) as f64 / feed.prev_price as f64) * 100.0;
        Ok(change)
    }

    /// Get price history window
    pub fn get_price_history(&self, symbol: &str, limit: usize) -> Option<Vec<(u64, i64)>> {
        self.price_history
            .get(symbol)
            .map(|h| h.iter().rev().take(limit).copied().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_creation() {
        let oracle = PythOracle::new();
        assert_eq!(oracle.heartbeat_interval_secs, 5);
    }

    #[test]
    fn test_register_feed() {
        let mut oracle = PythOracle::new();

        let btc_feed = PriceFeed {
            id: "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43".to_string(),
            symbol: "BTC".to_string(),
            price: 4_200_000_000_000i64, // $42,000
            decimals: 8,
            confidence: 1_000_000,
            publish_time: 1000,
            valid_time: 1060,
            prev_publish_time: 0,
            prev_price: 0,
        };

        assert!(oracle.register_feed(btc_feed).is_ok());
    }

    #[test]
    fn test_update_price() {
        let mut oracle = PythOracle::new();

        let btc_feed = PriceFeed {
            id: "0xbtc".to_string(),
            symbol: "BTC".to_string(),
            price: 4_200_000_000_000i64,
            decimals: 8,
            confidence: 1_000_000,
            publish_time: 1000,
            valid_time: 1060,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(btc_feed).ok();

        let result = oracle.update_price("BTC", 4_210_000_000_000i64, 1_000_000, 1005, 1005);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_price() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xeth".to_string(),
            symbol: "ETH".to_string(),
            price: 225_000_000_000i64, // $2,250
            decimals: 8,
            confidence: 500_000,
            publish_time: 1000,
            valid_time: 1060,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(feed).ok();

        let price = oracle.get_price("ETH");
        assert_eq!(price.unwrap(), 225_000_000_000i64);
    }

    #[test]
    fn test_get_price_decimal() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xeth".to_string(),
            symbol: "ETH".to_string(),
            price: 225_000_000_000i64, // $2,250
            decimals: 8,
            confidence: 500_000,
            publish_time: 1000,
            valid_time: 1060,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(feed).ok();

        let price = oracle.get_price_decimal("ETH");
        assert!(price.is_ok());
        assert!(price.unwrap() > 2200.0 && price.unwrap() < 2300.0);
    }

    #[test]
    fn test_price_staleness() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xbtc".to_string(),
            symbol: "BTC".to_string(),
            price: 4_200_000_000_000i64,
            decimals: 8,
            confidence: 1_000_000,
            publish_time: 1000,
            valid_time: 1060,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(feed).ok();

        // Try to update with stale price (publish_time = 500, now = 2000)
        let result = oracle.update_price("BTC", 4_210_000_000_000i64, 1_000_000, 500, 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_price_history() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xsol".to_string(),
            symbol: "SOL".to_string(),
            price: 14_000_000_000i64,
            decimals: 8,
            confidence: 1_000_000,
            publish_time: 1000,
            valid_time: 1005,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(feed).ok();

        for i in 0..5 {
            oracle
                .update_price("SOL", 14_000_000_000i64 + (i * 100_000_000i64), 1_000_000, 1000 + i as u64, 1000 + i as u64)
                .ok();
        }

        let history = oracle.get_price_history("SOL", 3);
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 3);
    }

    #[test]
    fn test_twap_calculation() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xusdc".to_string(),
            symbol: "USDC".to_string(),
            price: 100_000_000i64,
            decimals: 8,
            confidence: 1_000,
            publish_time: 1000,
            valid_time: 1005,
            prev_publish_time: 0,
            prev_price: 0,
        };

        oracle.register_feed(feed).ok();

        for i in 0..10 {
            oracle
                .update_price("USDC", 100_000_000i64 + (i * 10_000_000i64), 1_000, 1000 + i as u64, 1000 + i as u64)
                .ok();
        }

        let twap = oracle.calculate_twap("USDC", 5);
        assert!(twap.is_ok());
    }

    #[test]
    fn test_price_change_pct() {
        let mut oracle = PythOracle::new();

        let feed = PriceFeed {
            id: "0xbnb".to_string(),
            symbol: "BNB".to_string(),
            price: 60_000_000_000i64, // $600
            decimals: 8,
            confidence: 1_000_000,
            publish_time: 1000,
            valid_time: 1005,
            prev_publish_time: 950,
            prev_price: 55_000_000_000i64, // Was $550
        };

        oracle.register_feed(feed).ok();

        let change = oracle.get_price_change_pct("BNB");
        assert!(change.is_ok());
        assert!(change.unwrap() > 8.0 && change.unwrap() < 10.0);
    }
}
