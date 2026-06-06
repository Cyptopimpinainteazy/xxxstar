//! Atomic Swap RPC Endpoints
//!
//! JSON-RPC methods: x3_createSwap, x3_executeSwap, x3_getSwapQuote, x3_estimateSlippage

use std::collections::HashMap;

/// Token pair for swap
#[derive(Clone, Debug)]
pub struct TokenPair {
    pub token_in: String, // Contract address or symbol (X3, USDC, etc.)
    pub token_out: String,
    pub amount_in: u128,
}

/// Swap quote (no state change)
#[derive(Clone, Debug)]
pub struct SwapQuote {
    pub amount_out: u128,
    pub slippage_pct: f64,
    pub price: f64,
    pub execution_time_ms: u64,
    pub route: Vec<String>, // Pool addresses in order
}

/// Swap order
#[derive(Clone, Debug)]
pub struct SwapOrder {
    pub id: String,
    pub from: String, // User wallet
    pub pair: TokenPair,
    pub min_amount_out: u128, // Slippage protection
    pub deadline: u64,        // Block height deadline
    pub status: SwapStatus,
}

#[derive(Clone, Debug)]
pub enum SwapStatus {
    Pending,
    Executed { amount_out: u128, block: u32 },
    Failed { reason: String },
    Expired,
}

/// AMM Pool state
#[derive(Clone, Debug)]
pub struct AMMPool {
    pub id: String,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u128,
    pub reserve_b: u128,
    pub fee_bps: u16, // 30 = 0.3%
    pub tvl_usd: f64,
}

/// RPC Server
pub struct SwapRPCServer {
    pools: HashMap<String, AMMPool>,
    orders: HashMap<String, SwapOrder>,
    price_feed: HashMap<String, f64>, // Symbol → Price in USD
}

impl SwapRPCServer {
    pub fn new() -> Self {
        let mut price_feed = HashMap::new();
        price_feed.insert("X3".to_string(), 10.0); // Mock price
        price_feed.insert("USDC".to_string(), 1.0);
        price_feed.insert("ETH".to_string(), 2500.0);

        Self {
            pools: HashMap::new(),
            orders: HashMap::new(),
            price_feed,
        }
    }

    /// Register an AMM pool (called once per pool)
    pub fn register_pool(&mut self, pool: AMMPool) -> Result<(), String> {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return Err("Pool reserves must be > 0".to_string());
        }
        self.pools.insert(pool.id.clone(), pool);
        Ok(())
    }

    /// x3_getSwapQuote: Get execution quote without state change
    pub fn get_swap_quote(&self, pair: TokenPair) -> Result<SwapQuote, String> {
        let start = std::time::Instant::now();

        // Find best route (simplified: single pool for now)
        let route = self.find_best_route(&pair.token_in, &pair.token_out)?;

        // Calculate amount_out using constant-product formula
        let amount_out = self.calculate_output(&pair.token_in, &pair.token_out, pair.amount_in)?;

        // Calculate slippage vs spot price
        let spot_price = self.get_price(&pair.token_in, &pair.token_out)?;
        let executed_price = (pair.amount_in as f64) / (amount_out as f64);
        let slippage_pct = ((executed_price - spot_price) / spot_price).abs() * 100.0;

        Ok(SwapQuote {
            amount_out,
            slippage_pct,
            price: executed_price,
            execution_time_ms: start.elapsed().as_millis() as u64,
            route,
        })
    }

    /// x3_createSwap: Create a pending swap order
    pub fn create_swap(
        &mut self,
        from: String,
        pair: TokenPair,
        min_amount_out: u128,
        deadline: u64,
    ) -> Result<SwapOrder, String> {
        // Validate: min_amount_out <= estimated_out
        let quote = self.get_swap_quote(pair.clone())?;
        if min_amount_out > quote.amount_out {
            return Err("min_amount_out exceeds maximum possible output".to_string());
        }

        let order = SwapOrder {
            id: format!("swap_{}", self.orders.len()),
            from,
            pair,
            min_amount_out,
            deadline,
            status: SwapStatus::Pending,
        };

        self.orders.insert(order.id.clone(), order.clone());
        Ok(order)
    }

    /// x3_executeSwap: Execute swap on-chain
    pub fn execute_swap(&mut self, order_id: &str, block_height: u32) -> Result<SwapOrder, String> {
        let mut order = self.orders.get(order_id).ok_or("Swap not found")?.clone();

        // Check deadline
        if u64::from(block_height) > order.deadline {
            order.status = SwapStatus::Expired;
            self.orders.insert(order_id.to_string(), order.clone());
            return Err("Swap deadline exceeded".to_string());
        }

        // Execute swap
        match self.calculate_output(
            &order.pair.token_in,
            &order.pair.token_out,
            order.pair.amount_in,
        ) {
            Ok(amount_out) => {
                if amount_out < order.min_amount_out {
                    order.status = SwapStatus::Failed {
                        reason: "Slippage protection triggered".to_string(),
                    };
                    self.orders.insert(order_id.to_string(), order.clone());
                    return Err("Slippage exceeded".to_string());
                }

                // Update pool reserves (constant product)
                self.update_pool_reserves(
                    &order.pair.token_in,
                    &order.pair.token_out,
                    order.pair.amount_in,
                    amount_out,
                );

                order.status = SwapStatus::Executed {
                    amount_out,
                    block: block_height,
                };
                self.orders.insert(order_id.to_string(), order.clone());

                Ok(order)
            }
            Err(e) => {
                order.status = SwapStatus::Failed { reason: e };
                Err("Swap execution failed".to_string())
            }
        }
    }

    /// x3_estimateSlippage: Estimate slippage for given amount
    pub fn estimate_slippage(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
    ) -> Result<f64, String> {
        let quote = self.get_swap_quote(TokenPair {
            token_in: token_in.to_string(),
            token_out: token_out.to_string(),
            amount_in,
        })?;

        Ok(quote.slippage_pct)
    }

    /// Read-only lookup of an order by id
    pub fn get_order(&self, order_id: &str) -> Option<SwapOrder> {
        self.orders.get(order_id).cloned()
    }

    /// Calculate output using constant-product formula: xy = k
    fn calculate_output(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
    ) -> Result<u128, String> {
        // Find pool with both tokens
        let pool = self
            .pools
            .values()
            .find(|p| {
                (p.token_a == token_in && p.token_b == token_out)
                    || (p.token_a == token_out && p.token_b == token_in)
            })
            .ok_or("No pool found for token pair")?;

        let (reserve_in, reserve_out) = if pool.token_a == token_in {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        // formula: output = (input * reserve_out) / (reserve_in + input)
        let numerator = (amount_in as u128) * reserve_out;
        let denominator = reserve_in + amount_in;

        if denominator == 0 {
            return Err("Division by zero".to_string());
        }

        Ok(numerator / denominator)
    }

    /// Update pool reserves after swap
    fn update_pool_reserves(
        &mut self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
        amount_out: u128,
    ) {
        for pool in self.pools.values_mut() {
            if (pool.token_a == token_in && pool.token_b == token_out) {
                pool.reserve_a = pool.reserve_a.saturating_add(amount_in);
                pool.reserve_b = pool.reserve_b.saturating_sub(amount_out);
                return;
            } else if (pool.token_a == token_out && pool.token_b == token_in) {
                pool.reserve_a = pool.reserve_a.saturating_sub(amount_out);
                pool.reserve_b = pool.reserve_b.saturating_add(amount_in);
                return;
            }
        }
    }

    /// Find best route across pools (simplified - direct pool)
    fn find_best_route(&self, token_in: &str, token_out: &str) -> Result<Vec<String>, String> {
        let pool = self
            .pools
            .values()
            .find(|p| {
                (p.token_a == token_in && p.token_b == token_out)
                    || (p.token_a == token_out && p.token_b == token_in)
            })
            .ok_or("No route found")?;

        Ok(vec![pool.id.clone()])
    }

    /// Get price in USD (from price feed)
    fn get_price(&self, token_in: &str, token_out: &str) -> Result<f64, String> {
        let price_in = self
            .price_feed
            .get(token_in)
            .ok_or("Token not in price feed")?;
        let price_out = self
            .price_feed
            .get(token_out)
            .ok_or("Token not in price feed")?;

        Ok(price_in / price_out)
    }

    /// Get all active swaps
    pub fn get_swap_status(&self, order_id: &str) -> Option<SwapOrder> {
        self.orders.get(order_id).cloned()
    }

    /// Get pool info
    pub fn get_pool_info(&self, pool_id: &str) -> Option<AMMPool> {
        self.pools.get(pool_id).cloned()
    }

    /// Set price feed (external oracle)
    pub fn update_price_feed(&mut self, symbol: String, price: f64) {
        self.price_feed.insert(symbol, price);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_creation() {
        let rpc = SwapRPCServer::new();
        assert!(!rpc.price_feed.is_empty());
    }

    #[test]
    fn test_register_pool() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        assert!(rpc.register_pool(pool).is_ok());
    }

    #[test]
    fn test_register_pool_zero_reserves() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 0,
            reserve_b: 0,
            fee_bps: 30,
            tvl_usd: 0.0,
        };

        assert!(rpc.register_pool(pool).is_err());
    }

    #[test]
    fn test_get_swap_quote() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        let quote = rpc.get_swap_quote(TokenPair {
            token_in: "X3".to_string(),
            token_out: "USDC".to_string(),
            amount_in: 1_000_000_000_000,
        });

        assert!(quote.is_ok());
    }

    #[test]
    fn test_create_swap() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        let result = rpc.create_swap(
            "0xuser".to_string(),
            TokenPair {
                token_in: "X3".to_string(),
                token_out: "USDC".to_string(),
                amount_in: 100_000_000,
            },
            1,
            1000,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_swap() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        let order = rpc
            .create_swap(
                "0xuser".to_string(),
                TokenPair {
                    token_in: "X3".to_string(),
                    token_out: "USDC".to_string(),
                    amount_in: 100_000_000,
                },
                1,
                1000,
            )
            .unwrap();

        let result = rpc.execute_swap(&order.id, 500);
        assert!(result.is_ok());
    }

    #[test]
    fn test_slippage_protection() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        // Create swap with excessive min_amount_out
        let result = rpc.create_swap(
            "0xuser".to_string(),
            TokenPair {
                token_in: "X3".to_string(),
                token_out: "USDC".to_string(),
                amount_in: 100_000_000,
            },
            1_000_000_000_000u128, // Impossible amount
            1000,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_slippage() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        let slippage = rpc.estimate_slippage("X3", "USDC", 100_000_000);
        assert!(slippage.is_ok());
        assert!(slippage.unwrap() >= 0.0);
    }

    #[test]
    fn test_deadline_expiry() {
        let mut rpc = SwapRPCServer::new();
        let pool = AMMPool {
            id: "pool_1".to_string(),
            token_a: "X3".to_string(),
            token_b: "USDC".to_string(),
            reserve_a: 1_000_000_000_000u128,
            reserve_b: 10_000_000_000_000u128,
            fee_bps: 30,
            tvl_usd: 10_000.0,
        };

        rpc.register_pool(pool).ok();

        let order = rpc
            .create_swap(
                "0xuser".to_string(),
                TokenPair {
                    token_in: "X3".to_string(),
                    token_out: "USDC".to_string(),
                    amount_in: 100_000_000,
                },
                1,
                100, // Deadline at block 100
            )
            .unwrap();

        // Execute at block 200 (expired)
        let result = rpc.execute_swap(&order.id, 200);
        assert!(result.is_err());
    }
}
