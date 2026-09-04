use ethers::types::U256;
use tracing::info;

pub struct ArbitrageStrategy {
    threshold_bps: u64,
}

impl ArbitrageStrategy {
    pub fn new(threshold_bps: u64) -> Self {
        Self { threshold_bps }
    }

    pub fn scan_opportunity(&self, price_a: U256, price_b: U256) -> Option<U256> {
        if price_a > price_b {
            let diff = price_a - price_b;
            let bps = (diff * U256::from(10000)) / price_b;
            if bps > U256::from(self.threshold_bps) {
                return Some(diff);
            }
        } else if price_b > price_a {
            let diff = price_b - price_a;
            let bps = (diff * U256::from(10000)) / price_a;
            if bps > U256::from(self.threshold_bps) {
                return Some(diff);
            }
        }
        None
    }

    pub fn calculate_trade_size(&self, bankroll: U256, liquidity: U256) -> U256 {
        let b = bankroll / U256::from(10);
        let l = liquidity / U256::from(20);
        if b < l {
            b
        } else {
            l
        }
    }
}
