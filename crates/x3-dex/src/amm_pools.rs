/// AMM Liquidity Pools — ConstantProduct (Uniswap V2-style) pool implementation with LP token management.
/// Enables AMM-based trading across X3, supports multi-pool routing, and governs LP rewards.
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use sp_runtime::scale_info::TypeInfo;
use sp_std::prelude::*;

#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq,
)]
pub struct LiquidityPool {
    pub pool_id: u64,
    pub token_a: TokenId,
    pub token_b: TokenId,
    pub reserve_a: u128,
    pub reserve_b: u128,
    pub total_lp_supply: u128,
    pub fee_basis_points: u32,
    pub created_block: u32,
}

#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq,
)]
pub struct TokenId {
    pub chain_id: u32,
    pub asset_id: u128,
}

#[derive(
    Clone, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo, Debug, PartialEq, Eq,
)]
pub struct LPPosition {
    pub position_id: u64,
    pub pool_id: u64,
    pub lp_balance: u128,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SwapEvent {
    pub pool_id: u64,
    pub amount_in: u128,
    pub amount_out: u128,
    pub user: [u8; 32],
    pub token_in: TokenId,
    pub token_out: TokenId,
}

pub trait AMMPoolManager {
    fn create_pool(token_a: TokenId, token_b: TokenId, fee_bp: u32) -> Result<u64, &'static str>;
    fn add_liquidity(
        pool_id: u64,
        amount_a: u128,
        amount_b: u128,
        user: [u8; 32],
    ) -> Result<u128, &'static str>;
    fn remove_liquidity(
        pool_id: u64,
        lp_amount: u128,
        user: [u8; 32],
    ) -> Result<(u128, u128), &'static str>;
    fn swap(
        pool_id: u64,
        token_in_amount: u128,
        min_out: u128,
        user: [u8; 32],
        token_in: TokenId,
    ) -> Result<u128, &'static str>;
    fn get_pool(pool_id: u64) -> Option<LiquidityPool>;
    fn get_lp_position(position_id: u64) -> Option<LPPosition>;
}

pub struct AMMPool;

impl AMMPool {
    /// Create a new liquidity pool with specified fee tier
    pub fn create_pool(
        token_a: TokenId,
        token_b: TokenId,
        fee_bp: u32,
    ) -> Result<LiquidityPool, &'static str> {
        if fee_bp > 10000 {
            return Err("Fee cannot exceed 100%");
        }
        if token_a == token_b {
            return Err("Cannot create pool with identical tokens");
        }

        let pool_id = Self::generate_pool_id(&token_a, &token_b);
        Ok(LiquidityPool {
            pool_id,
            token_a,
            token_b,
            reserve_a: 0,
            reserve_b: 0,
            total_lp_supply: 0,
            fee_basis_points: fee_bp,
            created_block: 0,
        })
    }

    /// Add liquidity to an existing pool (mints LP tokens proportional to deposit ratio)
    pub fn add_liquidity(
        pool: &mut LiquidityPool,
        amount_a: u128,
        amount_b: u128,
    ) -> Result<u128, &'static str> {
        if amount_a == 0 || amount_b == 0 {
            return Err("Liquidity amounts must be positive");
        }

        let lp_minted = if pool.total_lp_supply == 0 {
            // First liquidity: use geometric mean of deposits
            Self::sqrt(amount_a.saturating_mul(amount_b))
        } else {
            // LP tokens = min(amount_a * total_supply / reserve_a, amount_b * total_supply / reserve_b)
            let lp_from_a =
                (amount_a as f64) * (pool.total_lp_supply as f64) / (pool.reserve_a as f64);
            let lp_from_b =
                (amount_b as f64) * (pool.total_lp_supply as f64) / (pool.reserve_b as f64);
            let lp = lp_from_a.min(lp_from_b) as u128;

            if lp == 0 {
                return Err("LP amount too small");
            }
            lp
        };

        pool.reserve_a = pool.reserve_a.saturating_add(amount_a);
        pool.reserve_b = pool.reserve_b.saturating_add(amount_b);
        pool.total_lp_supply = pool.total_lp_supply.saturating_add(lp_minted);

        Ok(lp_minted)
    }

    /// Remove liquidity from pool (burns LP tokens, returns proportional reserves)
    pub fn remove_liquidity(
        pool: &mut LiquidityPool,
        lp_amount: u128,
    ) -> Result<(u128, u128), &'static str> {
        if lp_amount == 0 || lp_amount > pool.total_lp_supply {
            return Err("Invalid LP amount");
        }

        let share = (lp_amount as f64) / (pool.total_lp_supply as f64);
        let amount_a = ((pool.reserve_a as f64) * share) as u128;
        let amount_b = ((pool.reserve_b as f64) * share) as u128;

        if amount_a == 0 || amount_b == 0 {
            return Err("Withdrawal too small");
        }

        pool.reserve_a = pool.reserve_a.saturating_sub(amount_a);
        pool.reserve_b = pool.reserve_b.saturating_sub(amount_b);
        pool.total_lp_supply = pool.total_lp_supply.saturating_sub(lp_amount);

        Ok((amount_a, amount_b))
    }

    /// Execute swap using constant-product formula: reserve_a * reserve_b = k
    /// amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
    pub fn swap(
        pool: &mut LiquidityPool,
        amount_in: u128,
        min_out: u128,
    ) -> Result<u128, &'static str> {
        if amount_in == 0 {
            return Err("Input amount must be positive");
        }
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return Err("Pool has no liquidity");
        }

        // Apply fee
        let fee = (amount_in as f64) * (pool.fee_basis_points as f64) / 10000.0;
        let amount_in_after_fee = (amount_in as f64) - fee;

        // Constant product: amount_out = (amount_in * reserve_out) / (reserve_in + amount_in)
        let amount_out = ((amount_in_after_fee * (pool.reserve_b as f64))
            / ((pool.reserve_a as f64) + amount_in_after_fee)) as u128;

        if amount_out < min_out {
            return Err("Slippage exceeds limit");
        }
        if amount_out > pool.reserve_b {
            return Err("Insufficient liquidity");
        }

        pool.reserve_a = pool.reserve_a.saturating_add(amount_in);
        pool.reserve_b = pool.reserve_b.saturating_sub(amount_out);

        Ok(amount_out)
    }

    /// Calculate LP tokens for deposited amounts (for preview)
    pub fn calculate_lp_for_deposit(pool: &LiquidityPool, amount_a: u128, amount_b: u128) -> u128 {
        if pool.total_lp_supply == 0 {
            Self::sqrt(amount_a.saturating_mul(amount_b))
        } else {
            let lp_from_a =
                (amount_a as f64) * (pool.total_lp_supply as f64) / (pool.reserve_a as f64);
            let lp_from_b =
                (amount_b as f64) * (pool.total_lp_supply as f64) / (pool.reserve_b as f64);
            lp_from_a.min(lp_from_b) as u128
        }
    }

    /// Calculate liquidity addition with optimal amounts
    pub fn add_liquidity_calculate(
        pool: &LiquidityPool,
        amount_a_desired: u128,
        amount_b_desired: u128,
        amount_a_min: u128,
        amount_b_min: u128,
    ) -> Result<(u128, u128, u128), &'static str> {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            // First liquidity provision - accept provided amounts
            if amount_a_desired < amount_a_min || amount_b_desired < amount_b_min {
                return Err("Insufficient amounts for first liquidity provision");
            }
            let lp_tokens = Self::sqrt(amount_a_desired.saturating_mul(amount_b_desired));
            return Ok((amount_a_desired, amount_b_desired, lp_tokens));
        }

        // Calculate optimal amounts based on current ratio
        let amount_b_optimal =
            (amount_a_desired as f64) * (pool.reserve_b as f64) / (pool.reserve_a as f64);
        let amount_a_optimal =
            (amount_b_desired as f64) * (pool.reserve_a as f64) / (pool.reserve_b as f64);

        let (amount_a, amount_b) = if amount_b_optimal <= amount_b_desired as f64 {
            (amount_a_desired, amount_b_optimal as u128)
        } else {
            (amount_a_optimal as u128, amount_b_desired)
        };

        // Check minimums
        if (amount_a as u128) < amount_a_min || (amount_b as u128) < amount_b_min {
            return Err("Output amounts below minimums");
        }

        // Calculate LP tokens
        let lp_tokens = if pool.total_lp_supply == 0 {
            Self::sqrt(amount_a.saturating_mul(amount_b))
        } else {
            let lp_from_a =
                (amount_a as f64) * (pool.total_lp_supply as f64) / (pool.reserve_a as f64);
            let lp_from_b =
                (amount_b as f64) * (pool.total_lp_supply as f64) / (pool.reserve_b as f64);
            lp_from_a.min(lp_from_b) as u128
        };

        Ok((amount_a, amount_b, lp_tokens))
    }

    /// Calculate liquidity removal amounts
    pub fn remove_liquidity_calculate(
        pool: &LiquidityPool,
        lp_amount: u128,
        amount_a_min: u128,
        amount_b_min: u128,
    ) -> Result<(u128, u128), &'static str> {
        if lp_amount == 0 {
            return Err("LP amount must be positive");
        }
        if lp_amount > pool.total_lp_supply {
            return Err("Insufficient LP balance");
        }

        let amount_a = (lp_amount as f64) * (pool.reserve_a as f64) / (pool.total_lp_supply as f64);
        let amount_b = (lp_amount as f64) * (pool.reserve_b as f64) / (pool.total_lp_supply as f64);

        if (amount_a as u128) < amount_a_min || (amount_b as u128) < amount_b_min {
            return Err("Output amounts below minimums");
        }

        Ok((amount_a as u128, amount_b as u128))
    }

    /// Calculate swap output amount
    pub fn swap_calculate(
        pool: &LiquidityPool,
        token_in: &TokenId,
        amount_in: u128,
        min_out: u128,
    ) -> Result<u128, &'static str> {
        if amount_in == 0 {
            return Err("Input amount must be positive");
        }
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return Err("Pool has no liquidity");
        }

        // Determine which token is being swapped
        let (reserve_in, reserve_out) = if *token_in == pool.token_a {
            (pool.reserve_a, pool.reserve_b)
        } else if *token_in == pool.token_b {
            (pool.reserve_b, pool.reserve_a)
        } else {
            return Err("Token not in pool");
        };

        // Apply fee
        let fee = (amount_in as f64) * (pool.fee_basis_points as f64) / 10000.0;
        let amount_in_after_fee = (amount_in as f64) - fee;

        // Constant product formula
        let amount_out = ((amount_in_after_fee * (reserve_out as f64))
            / ((reserve_in as f64) + amount_in_after_fee)) as u128;

        if amount_out < min_out {
            return Err("Slippage exceeds limit");
        }
        if amount_out > reserve_out {
            return Err("Insufficient liquidity");
        }

        Ok(amount_out)
    }

    /// Get current pool state
    pub fn get_pool_state(pool: &LiquidityPool) -> (u128, u128, u128, u32) {
        (
            pool.reserve_a,
            pool.reserve_b,
            pool.total_lp_supply,
            pool.fee_basis_points,
        )
    }

    /// Calculate output amount for given input (without executing swap)
    pub fn preview_swap(pool: &LiquidityPool, amount_in: u128) -> u128 {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return 0;
        }

        let fee = (amount_in as f64) * (pool.fee_basis_points as f64) / 10000.0;
        let amount_in_after_fee = (amount_in as f64) - fee;
        ((amount_in_after_fee * (pool.reserve_b as f64))
            / ((pool.reserve_a as f64) + amount_in_after_fee)) as u128
    }

    /// Simple integer square root
    fn sqrt(n: u128) -> u128 {
        if n == 0 {
            return 0;
        }
        let mut x = n;
        let mut y = x.div_ceil(2);
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        x
    }

    /// Generate deterministic pool ID from token pair
    fn generate_pool_id(token_a: &TokenId, token_b: &TokenId) -> u64 {
        let mut hash = 0u64;
        hash = hash.wrapping_mul(31).wrapping_add(token_a.chain_id as u64);
        hash = hash
            .wrapping_mul(31)
            .wrapping_add((token_a.asset_id >> 64) as u64);
        hash = hash.wrapping_mul(31).wrapping_add(token_b.chain_id as u64);
        hash = hash
            .wrapping_mul(31)
            .wrapping_add((token_b.asset_id >> 64) as u64);
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pool() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let pool = AMMPool::create_pool(token_a.clone(), token_b.clone(), 30).unwrap();

        assert_eq!(pool.token_a, token_a);
        assert_eq!(pool.token_b, token_b);
        assert_eq!(pool.fee_basis_points, 30);
        assert_eq!(pool.reserve_a, 0);
        assert_eq!(pool.reserve_b, 0);
    }

    #[test]
    fn test_fee_exceeds_max() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let result = AMMPool::create_pool(token_a, token_b, 10001);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Fee cannot exceed 100%");
    }

    #[test]
    fn test_identical_tokens() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let result = AMMPool::create_pool(token_a.clone(), token_a, 30);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Cannot create pool with identical tokens"
        );
    }

    #[test]
    fn test_add_liquidity_first() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        let lp = AMMPool::add_liquidity(&mut pool, 1000000, 500000).unwrap();

        assert!(lp > 0);
        assert_eq!(pool.reserve_a, 1000000);
        assert_eq!(pool.reserve_b, 500000);
        assert_eq!(pool.total_lp_supply, lp);
    }

    #[test]
    fn test_add_liquidity_second() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        let lp1 = AMMPool::add_liquidity(&mut pool, 1000000, 500000).unwrap();
        let lp2 = AMMPool::add_liquidity(&mut pool, 1000000, 500000).unwrap();

        assert_eq!(lp1, lp2); // Equal deposits should yield equal LP
        assert_eq!(pool.total_lp_supply, lp1 * 2);
    }

    #[test]
    fn test_swap_constant_product() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let amount_out = AMMPool::swap(&mut pool, 100000, 0).unwrap();

        assert!(amount_out > 0);
        assert!(amount_out < 100000); // Slippage due to price impact
        assert_eq!(pool.reserve_a, 1100000);
    }

    #[test]
    fn test_swap_slippage_protection() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let result = AMMPool::swap(&mut pool, 100000, 100000);

        assert!(result.is_err()); // min_out is unrealistic
    }

    #[test]
    fn test_remove_liquidity() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        let lp = AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let (amount_a, amount_b) = AMMPool::remove_liquidity(&mut pool, lp).unwrap();

        assert_eq!(amount_a, 1000000);
        assert_eq!(amount_b, 1000000);
        assert_eq!(pool.total_lp_supply, 0);
    }

    #[test]
    fn test_remove_partial_liquidity() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        let lp = AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let (amount_a, amount_b) = AMMPool::remove_liquidity(&mut pool, lp / 2).unwrap();

        assert!(amount_a > 0);
        assert!(amount_b > 0);
        assert_eq!(pool.total_lp_supply, lp / 2);
    }

    #[test]
    fn test_preview_swap() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let preview = AMMPool::preview_swap(&pool, 100000);

        assert!(preview > 0);
        assert!(preview < 100000);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(AMMPool::sqrt(0), 0);
        assert_eq!(AMMPool::sqrt(1), 1);
        assert_eq!(AMMPool::sqrt(4), 2);
        assert_eq!(AMMPool::sqrt(1000000), 1000);
        assert!(AMMPool::sqrt(999999) > 999);
        assert!(AMMPool::sqrt(1000001) < 1001);
    }

    #[test]
    fn test_zero_liquidity_swap_fails() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        let result = AMMPool::swap(&mut pool.clone(), 100000, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_input_swap_fails() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();

        AMMPool::add_liquidity(&mut pool, 1000000, 1000000).unwrap();
        let result = AMMPool::swap(&mut pool, 0, 0);

        assert!(result.is_err());
    }

    #[test]
    fn test_pool_id_deterministic() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };

        let id1 = AMMPool::generate_pool_id(&token_a, &token_b);
        let id2 = AMMPool::generate_pool_id(&token_a, &token_b);

        assert_eq!(id1, id2);
    }
}
