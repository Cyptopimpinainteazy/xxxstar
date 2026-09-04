//! Arbitrage Bot RPC Configuration
//!
//! Complete setup guide for running an arbitrage bot using the RPC configuration system.
//! 
//! This module demonstrates how to:
//! - Select flash loan providers by fee
//! - Route through DEX aggregators
//! - Monitor gas prices across L2s
//! - Handle RPC failover and rate limiting
//! - Calculate real-time profitability

use external_chains::{
    rpc::{arbitrum_mainnet_config, FlashLoanProvider, DexRouter},
    env_config::{EnvConfig, FlashLoanConfig, BotConfig},
};

/// Complete arbitrage configuration for Arbitrum
pub struct ArbitrageBot {
    pub env: EnvConfig,
    pub flashloan_config: FlashLoanConfig,
    pub bot_config: BotConfig,
    pub rpc_config: external_chains::rpc::ChainRpcConfig,
}

impl ArbitrageBot {
    /// Initialize arbitrage bot with all configurations
    pub fn new() -> Self {
        let env = EnvConfig::from_env();
        let flashloan_config = FlashLoanConfig {
            enabled: true,
            max_slippage_bps: 100,      // 1%
            min_profit_threshold: 50000, // 0.5 USD
            check_interval_ms: 15000,   // 15 seconds
        };
        let bot_config = BotConfig {
            port: 3001,
            check_interval_ms: 15000,
            max_slippage_percent: 1.0,
            min_profit_threshold: 0.5,
            max_gas_price_gwei: 100,
        };
        let rpc_config = arbitrum_mainnet_config();

        Self {
            env,
            flashloan_config,
            bot_config,
            rpc_config,
        }
    }

    /// Get the cheapest flash loan provider for amount
    pub fn select_cheapest_flashloan(&self, amount_wei: u128) -> Option<&FlashLoanProvider> {
        self.rpc_config
            .enabled_flashloans()
            .iter()
            .filter(|p| p.max_liquidity >= amount_wei && p.enabled)
            .min_by_key(|p| p.fee_bps)
            .cloned()
            .as_ref()
    }

    /// Get all viable swap routes (DEX pairs for arbitrage)
    pub fn get_swap_routes(&self) -> Vec<(String, String)> {
        let dexes = self.rpc_config.enabled_dexes();
        let mut routes = Vec::new();

        for i in 0..dexes.len() {
            for j in (i + 1)..dexes.len() {
                routes.push((
                    dexes[i].name.clone(),
                    dexes[j].name.clone(),
                ));
            }
        }

        routes
    }

    /// Calculate estimated execution cost for arbitrage
    pub fn estimate_execution_cost(&self, flash_amount_wei: u128) -> ExecutionCost {
        // Get cheapest flash loan provider
        let flashloan = self.select_cheapest_flashloan(flash_amount_wei);
        let flashloan_fee = flashloan
            .map(|p| (flash_amount_wei * (p.fee_bps as u128)) / 10000)
            .unwrap_or(0);

        // Estimate gas cost (Arbitrum L2)
        // Typical arbitrage: 300-500k gas
        let gas_limit = 400_000u128;
        let gas_price_wei = 1_000_000; // 0.001 gwei (typical L2)
        let gas_cost = gas_limit * gas_price_wei;

        // Slippage (assume 0.5%)
        let max_slippage = (flash_amount_wei * 50) / 10000;

        ExecutionCost {
            flashloan_fee,
            gas_cost,
            max_slippage,
            total: flashloan_fee + gas_cost + max_slippage,
        }
    }

    /// Get RPC endpoint for order book monitoring
    pub fn get_price_feed_rpc(&self) -> Option<String> {
        self.rpc_config
            .primary_rpc()
            .map(|rpc| rpc.url.clone())
    }

    /// Get WebSocket endpoint for real-time updates
    pub fn get_mempool_ws(&self) -> Option<String> {
        self.rpc_config
            .ws_endpoints
            .first()
            .map(|ws| ws.url.clone())
    }

    /// Check if spread is profitable after all fees
    pub fn is_profitable_spread(
        &self,
        spread_percent: f64,
        flash_amount_usd: f64,
    ) -> (bool, f64) {
        let costs = self.estimate_execution_cost(
            (flash_amount_usd * 1_000_000_000_000_000_000f64) as u128
        );
        let cost_percent = (costs.total as f64 / flash_amount_usd) * 100.0;
        let net_profit = spread_percent - cost_percent;

        (
            net_profit >= self.bot_config.min_profit_threshold,
            net_profit,
        )
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║          ARBITRAGE BOT CONFIGURATION SUMMARY               ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");

        println!("Network Configuration:");
        println!("  Network: {}", self.env.network.as_str());
        println!("  Chain ID: {}", self.env.network.chain_id());
        println!("  Primary RPC: {}", 
            self.get_price_feed_rpc().unwrap_or_default());

        println!("\nFlash Loan Configuration:");
        let flashloans = self.rpc_config.enabled_flashloans();
        println!("  Providers: {}", flashloans.len());
        if let Some(cheapest) = self.select_cheapest_flashloan(10u128.pow(18)) {
            println!("  Cheapest: {} ({}bps)", cheapest.name, cheapest.fee_bps);
        }

        println!("\nDEX Swap Routes:");
        let routes = self.get_swap_routes();
        println!("  Available pairs: {}", routes.len());
        for (dex1, dex2) in routes.iter().take(5) {
            println!("    {} ↔ {}", dex1, dex2);
        }

        println!("\nBot Parameters:");
        println!("  Check interval: {}ms", self.bot_config.check_interval_ms);
        println!("  Max slippage: {}%", self.bot_config.max_slippage_percent);
        println!("  Min profit: ${}", self.bot_config.min_profit_threshold);
        println!("  Max gas price: {} gwei", self.bot_config.max_gas_price_gwei);

        println!("\nGas Estimation:");
        let cost = self.estimate_execution_cost(10u128.pow(18));
        println!("  For 1 ETH flashloan:");
        println!("    Flash fee: {} wei", cost.flashloan_fee);
        println!("    Gas cost: {} wei", cost.gas_cost);
        println!("    Slippage: {} wei", cost.max_slippage);
        println!("    Total: {} wei (~${:.4})", cost.total, 
            cost.total as f64 / 1e18);

        println!("\n╚════════════════════════════════════════════════════════════╝");
    }
}

/// Execution cost breakdown
#[derive(Debug, Clone)]
pub struct ExecutionCost {
    pub flashloan_fee: u128,
    pub gas_cost: u128,
    pub max_slippage: u128,
    pub total: u128,
}

impl ExecutionCost {
    pub fn to_usd(&self, eth_price: f64) -> f64 {
        (self.total as f64 / 1e18) * eth_price
    }
}

/// Real-time arbitrage opportunity detection
pub struct OpportunityDetector {
    pub bot: ArbitrageBot,
}

impl OpportunityDetector {
    pub fn new() -> Self {
        let bot = ArbitrageBot::new();
        Self { bot }
    }

    /// Evaluate an arbitrage opportunity
    pub fn evaluate_opportunity(
        &self,
        dex_a: &str,
        dex_b: &str,
        token_pair: &str,
        price_a: f64,
        price_b: f64,
        liquidity_needed: f64,
    ) -> OpportunityResult {
        // Calculate spread
        let spread_percent = ((price_b - price_a) / price_a) * 100.0;

        // Check if profitable
        let flash_amount_usd = liquidity_needed * price_a;
        let (is_profitable, net_profit) = 
            self.bot.is_profitable_spread(spread_percent.abs(), flash_amount_usd);

        OpportunityResult {
            dex_pair: format!("{} ↔ {}", dex_a, dex_b),
            token_pair: token_pair.to_string(),
            price_a,
            price_b,
            spread_percent,
            execution_cost: self.bot.estimate_execution_cost(
                (flash_amount_usd * 1_000_000_000_000_000_000f64) as u128
            ),
            is_profitable,
            net_profit_percent: net_profit,
        }
    }
}

/// Result of opportunity evaluation
#[derive(Debug, Clone)]
pub struct OpportunityResult {
    pub dex_pair: String,
    pub token_pair: String,
    pub price_a: f64,
    pub price_b: f64,
    pub spread_percent: f64,
    pub execution_cost: ExecutionCost,
    pub is_profitable: bool,
    pub net_profit_percent: f64,
}

impl OpportunityResult {
    pub fn print_summary(&self) {
        println!("\n📊 Arbitrage Opportunity");
        println!("  Pair: {}", self.dex_pair);
        println!("  Token: {}", self.token_pair);
        println!("  Price A: ${:.4}", self.price_a);
        println!("  Price B: ${:.4}", self.price_b);
        println!("  Spread: {:.2}%", self.spread_percent);
        println!("  Execution cost: {:.2}%", 
            (self.execution_cost.total as f64 / (self.price_a * 1e18)) * 100.0);
        
        if self.is_profitable {
            println!("  ✅ PROFITABLE: {:.2}% net profit", self.net_profit_percent);
        } else {
            println!("  ❌ NOT PROFITABLE: {:.2}% loss", self.net_profit_percent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrage_bot_initialization() {
        let bot = ArbitrageBot::new();
        assert!(!bot.rpc_config.rpc_endpoints.is_empty());
        assert!(!bot.rpc_config.flashloan_providers.is_empty());
    }

    #[test]
    fn test_flashloan_selection() {
        let bot = ArbitrageBot::new();
        let amount = 1_000_000_000_000_000_000u128; // 1 ETH
        
        let selected = bot.select_cheapest_flashloan(amount);
        assert!(selected.is_some());
        
        let provider = selected.unwrap();
        assert!(provider.max_liquidity >= amount);
        assert!(provider.enabled);
    }

    #[test]
    fn test_swap_routes_generation() {
        let bot = ArbitrageBot::new();
        let routes = bot.get_swap_routes();
        
        assert!(!routes.is_empty());
        // Should have combinations of DEX pairs
        assert!(routes.len() >= 5); // At least 5 DEXs, so C(n,2) combinations
    }

    #[test]
    fn test_execution_cost_calculation() {
        let bot = ArbitrageBot::new();
        let amount = 1_000_000_000_000_000_000u128; // 1 ETH
        
        let cost = bot.estimate_execution_cost(amount);
        assert!(cost.flashloan_fee > 0);
        assert!(cost.gas_cost > 0);
        assert!(cost.max_slippage > 0);
        assert!(cost.total > 0);
    }

    #[test]
    fn test_profitability_calculation() {
        let bot = ArbitrageBot::new();
        
        // 2% spread should be profitable
        let (profitable, _) = bot.is_profitable_spread(2.0, 100000.0);
        assert!(profitable);
        
        // 0.1% spread should not be profitable
        let (not_profitable, _) = bot.is_profitable_spread(0.1, 100000.0);
        assert!(!not_profitable);
    }

    #[test]
    fn test_opportunity_evaluation() {
        let detector = OpportunityDetector::new();
        let result = detector.evaluate_opportunity(
            "Uniswap V3",
            "Camelot V2",
            "USDC/ETH",
            2500.0,
            2525.0, // 1% spread
            100.0,
        );
        
        assert_eq!(result.token_pair, "USDC/ETH");
        assert!(result.spread_percent > 0.0);
    }
}

fn main() {
    // Initialize bot with all configurations
    let bot = ArbitrageBot::new();
    bot.print_summary();

    // Check some example opportunities
    println!("\n\n📈 Example Opportunity Analysis:");
    let detector = OpportunityDetector::new();

    // Opportunity 1: 1% spread
    let opp1 = detector.evaluate_opportunity(
        "Uniswap V3",
        "Camelot V2",
        "USDC/ETH",
        2500.0,
        2525.0,
        100.0,
    );
    opp1.print_summary();

    // Opportunity 2: 0.5% spread (likely not profitable)
    let opp2 = detector.evaluate_opportunity(
        "SushiSwap",
        "Balancer",
        "ETH/DAI",
        1500.0,
        1507.5,
        150.0,
    );
    opp2.print_summary();
}
