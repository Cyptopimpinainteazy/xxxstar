//! Multi-Hop Pathfinding Router
//!
//! Find best routes across multiple AMM pools for optimal prices.
//! Uses Bellman-Ford longest-path variant to maximize output.

use sp_std::{collections::btree_map::BTreeMap, prelude::*};
#[cfg(feature = "std")]
use std::collections::VecDeque;
#[cfg(not(feature = "std"))]
use alloc::collections::VecDeque;
#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

/// Pool for routing
#[derive(Clone, Debug)]
pub struct PoolEdge {
    pub pool_id: String,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u128,
    pub reserve_b: u128,
    pub fee_bps: u16,
}

/// Routing path (sequence of pools)
#[derive(Clone, Debug)]
pub struct Path {
    pub hops: Vec<String>,   // Pool IDs in order
    pub tokens: Vec<String>, // Token addresses in order
    pub input_amount: u128,
    pub output_amount: u128,
    pub slippage_pct: f64,
    pub gas_cost: u64,
}

/// Route finder (graph-based)
pub struct RouteFinder {
    pub pools: BTreeMap<String, PoolEdge>,
    pub graph: BTreeMap<String, Vec<String>>, // token → connected pools
}

impl Default for RouteFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl RouteFinder {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            graph: BTreeMap::new(),
        }
    }

    /// Register pool for routing
    pub fn register_pool(&mut self, pool: PoolEdge) -> Result<(), String> {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return Err("Pool reserves must be > 0".to_string());
        }

        // Add to graph
        self.graph
            .entry(pool.token_a.clone())
            .or_default()
            .push(pool.pool_id.clone());

        self.graph
            .entry(pool.token_b.clone())
            .or_default()
            .push(pool.pool_id.clone());

        self.pools.insert(pool.pool_id.clone(), pool);

        Ok(())
    }

    /// Find best multi-hop path from token_in to token_out
    pub fn find_best_path(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
        max_hops: usize,
    ) -> Result<Path, String> {
        if token_in == token_out {
            return Err("Input and output tokens are the same".to_string());
        }

        // BFS to find all paths up to max_hops
        let all_paths = self.find_all_paths(token_in, token_out, max_hops)?;

        if all_paths.is_empty() {
            return Err("No path found".to_string());
        }

        // Simulate each path and pick best output
        let mut best_path: Option<Path> = None;

        for path_spec in all_paths {
            if let Ok(path) = self.simulate_path(token_in, token_out, amount_in, &path_spec) {
                if best_path.is_none()
                    || path.output_amount > best_path.as_ref().unwrap().output_amount
                {
                    best_path = Some(path);
                }
            }
        }

        best_path.ok_or_else(|| "No viable path after simulation".to_string())
    }

    /// Find all possible paths (BFS)
    fn find_all_paths(
        &self,
        start: &str,
        end: &str,
        max_hops: usize,
    ) -> Result<Vec<Vec<String>>, String> {
        let mut paths = Vec::new();

        // Queue: (current_token, current_path_of_pools)
        let mut queue: VecDeque<(String, Vec<String>, Vec<String>)> = VecDeque::new();

        // Start: find all pools that have start token
        if let Some(start_pools) = self.graph.get(start) {
            for pool_id in start_pools {
                queue.push_back((
                    start.to_string(),
                    vec![pool_id.clone()],
                    vec![start.to_string()],
                ));
            }
        }

        while let Some((current_token, current_pools, visited_tokens)) = queue.pop_front() {
            if current_pools.len() > max_hops {
                continue;
            }

            if current_token == end {
                paths.push(current_pools);
                continue;
            }

            // Find next pools
            if let Some(next_pools) = self.graph.get(&current_token) {
                for next_pool_id in next_pools {
                    // Avoid cycles
                    if current_pools.contains(next_pool_id) {
                        continue;
                    }

                    let pool = &self.pools[next_pool_id];

                    // Determine next token
                    let next_token = if pool.token_a == current_token {
                        pool.token_b.clone()
                    } else {
                        pool.token_a.clone()
                    };

                    // Avoid cycles in token path
                    if visited_tokens.contains(&next_token) {
                        continue;
                    }

                    let mut new_visited = visited_tokens.clone();
                    new_visited.push(next_token.clone());

                    let mut new_pools = current_pools.clone();
                    new_pools.push(next_pool_id.clone());

                    queue.push_back((next_token, new_pools, new_visited));
                }
            }
        }

        Ok(paths)
    }

    /// Simulate path execution
    fn simulate_path(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
        pool_path: &[String],
    ) -> Result<Path, String> {
        let mut amount = amount_in;
        let mut current_token = token_in.to_string();
        let mut tokens = vec![token_in.to_string()];
        let mut _total_fees = 0u64;

        for pool_id in pool_path {
            let pool = self.pools.get(pool_id).ok_or("Pool not found")?;

            // Determine input/output tokens
            let (reserve_in, reserve_out, next_token) = if pool.token_a == current_token {
                (pool.reserve_a, pool.reserve_b, pool.token_b.clone())
            } else if pool.token_b == current_token {
                (pool.reserve_b, pool.reserve_a, pool.token_a.clone())
            } else {
                return Err("Pool does not contain current token".to_string());
            };

            // Apply fee
            let fee_amount = (amount as u64 * pool.fee_bps as u64) / 10_000;
            amount = amount.saturating_sub(fee_amount as u128);
            _total_fees += fee_amount;

            // Constant product formula
            let numerator = amount.wrapping_mul(reserve_out);
            let denominator = reserve_in.saturating_add(amount);

            if denominator == 0 {
                return Err("Division by zero in pool".to_string());
            }

            amount = numerator / denominator;
            current_token = next_token;
            tokens.push(current_token.clone());
        }

        if current_token != token_out {
            return Err("Final token does not match expected output".to_string());
        }

        let slippage_pct = 0.0; // Simplified

        Ok(Path {
            hops: pool_path.to_vec(),
            tokens,
            input_amount: amount_in,
            output_amount: amount,
            slippage_pct,
            gas_cost: (pool_path.len() as u64) * 100_000, // ~100k gas per hop
        })
    }

    /// Get all registered pools
    pub fn get_all_pools(&self) -> Vec<PoolEdge> {
        self.pools.values().cloned().collect()
    }

    /// Check if path exists
    pub fn has_path(&self, token_in: &str, token_out: &str) -> bool {
        self.find_all_paths(token_in, token_out, 5)
            .map(|p| !p.is_empty())
            .unwrap_or(false)
    }

    /// Get liquidity for direct pair
    pub fn get_liquidity(&self, token_a: &str, token_b: &str) -> Option<u128> {
        self.pools
            .values()
            .find(|p| {
                (p.token_a == token_a && p.token_b == token_b)
                    || (p.token_a == token_b && p.token_b == token_a)
            })
            .map(|p| p.reserve_a.min(p.reserve_b))
    }

    /// Estimate slippage for multi-hop
    pub fn estimate_slippage_for_path(&self, path: &Path) -> f64 {
        // Rough estimate: slippage increases with hops
        ((path.hops.len() as f64) * 0.1) + 0.01 // 0.1% per hop + 0.01% base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = RouteFinder::new();
        assert!(router.pools.is_empty());
    }

    #[test]
    fn test_register_pool() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 1_000_000_000_000u128,
            fee_bps: 30,
        };

        assert!(router.register_pool(pool).is_ok());
    }

    #[test]
    fn test_register_zero_reserves() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 0,
            reserve_b: 0,
            fee_bps: 30,
        };

        assert!(router.register_pool(pool).is_err());
    }

    #[test]
    fn test_find_direct_path() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 1_000_000_000_000u128,
            fee_bps: 30,
        };

        router.register_pool(pool).ok();

        let path = router.find_best_path("USDC", "X3", 1_000_000_000_000u128, 5);
        assert!(path.is_ok());
    }

    #[test]
    fn test_find_multihop_path() {
        let mut router = RouteFinder::new();

        let pool1 = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "ETH".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 5_000_000_000_000u128,
            fee_bps: 30,
        };

        let pool2 = PoolEdge {
            pool_id: "pool_2".to_string(),
            token_a: "ETH".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 5_000_000_000_000u128,
            reserve_b: 1_000_000_000_000u128,
            fee_bps: 30,
        };

        router.register_pool(pool1).ok();
        router.register_pool(pool2).ok();

        let path = router.find_best_path("USDC", "X3", 1_000_000_000_000u128, 5);
        assert!(path.is_ok());
        assert!(!path.unwrap().hops.is_empty());
    }

    #[test]
    fn test_no_path_found() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "ETH".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 5_000_000_000_000u128,
            fee_bps: 30,
        };

        router.register_pool(pool).ok();

        let path = router.find_best_path("USDC", "X3", 1_000_000_000_000u128, 5);
        assert!(path.is_err());
    }

    #[test]
    fn test_has_path() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 1_000_000_000_000u128,
            fee_bps: 30,
        };

        router.register_pool(pool).ok();

        assert!(router.has_path("USDC", "X3"));
        assert!(!router.has_path("USDC", "BTC"));
    }

    #[test]
    fn test_get_liquidity() {
        let mut router = RouteFinder::new();

        let pool = PoolEdge {
            pool_id: "pool_1".to_string(),
            token_a: "USDC".to_string(),
            token_b: "X3".to_string(),
            reserve_a: 10_000_000_000_000u128,
            reserve_b: 1_000_000_000_000u128,
            fee_bps: 30,
        };

        router.register_pool(pool).ok();

        let liq = router.get_liquidity("USDC", "X3");
        assert!(liq.is_some());
    }

    #[test]
    fn test_slippage_estimation() {
        let path = Path {
            hops: vec!["p1".to_string(), "p2".to_string()],
            tokens: vec!["USDC".to_string(), "ETH".to_string(), "X3".to_string()],
            input_amount: 1_000_000_000_000u128,
            output_amount: 100_000_000_000u128,
            slippage_pct: 0.2,
            gas_cost: 300_000,
        };

        let router = RouteFinder::new();
        let slippage = router.estimate_slippage_for_path(&path);
        assert!(slippage > 0.0);
    }
}
