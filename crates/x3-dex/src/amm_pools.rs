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
            let lp =
                Self::lp_for_deposit(pool, amount_a, amount_b).ok_or("LP calculation overflow")?;

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

        let amount_a = Self::mul_div(pool.reserve_a, lp_amount, pool.total_lp_supply)
            .ok_or("Withdrawal calculation overflow")?;
        let amount_b = Self::mul_div(pool.reserve_b, lp_amount, pool.total_lp_supply)
            .ok_or("Withdrawal calculation overflow")?;

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

        let (reserve_in, reserve_out) = (pool.reserve_a, pool.reserve_b);
        let amount_out = Self::swap_out(amount_in, reserve_in, reserve_out, pool.fee_basis_points)?;

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
            Self::lp_for_deposit(pool, amount_a, amount_b).unwrap_or(0)
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

        // Which leg is limiting, without dividing: `amount_a_desired * reserve_b` against
        // `amount_b_desired * reserve_a`, in the wide intermediate. The float form divided
        // both and compared the rounded quotients (TICKET-094).
        let lhs = sp_core::U256::from(amount_a_desired) * sp_core::U256::from(pool.reserve_b);
        let rhs = sp_core::U256::from(amount_b_desired) * sp_core::U256::from(pool.reserve_a);
        let (amount_a, amount_b) = if lhs <= rhs {
            let b = Self::mul_div(amount_a_desired, pool.reserve_b, pool.reserve_a)
                .ok_or("Liquidity calculation overflow")?;
            (amount_a_desired, b)
        } else {
            let a = Self::mul_div(amount_b_desired, pool.reserve_a, pool.reserve_b)
                .ok_or("Liquidity calculation overflow")?;
            (a, amount_b_desired)
        };

        // Check minimums
        if amount_a < amount_a_min || amount_b < amount_b_min {
            return Err("Output amounts below minimums");
        }

        // Calculate LP tokens
        let lp_tokens =
            Self::lp_for_deposit(pool, amount_a, amount_b).ok_or("LP calculation overflow")?;

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

        let amount_a = Self::mul_div(lp_amount, pool.reserve_a, pool.total_lp_supply)
            .ok_or("Withdrawal calculation overflow")?;
        let amount_b = Self::mul_div(lp_amount, pool.reserve_b, pool.total_lp_supply)
            .ok_or("Withdrawal calculation overflow")?;

        if amount_a < amount_a_min || amount_b < amount_b_min {
            return Err("Output amounts below minimums");
        }

        Ok((amount_a, amount_b))
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

        let amount_out = Self::swap_out(amount_in, reserve_in, reserve_out, pool.fee_basis_points)?;

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
    ///
    /// The same formula `swap` uses, through the same helper, so a preview and an execution
    /// cannot disagree. This was the *ninth* function in this file computing a swap in `f64`
    /// and the enumeration missed it by eye; `preview_swap_agrees_with_swap` pins the two
    /// together, which is the property a caller relies on when they quote a preview
    /// (TICKET-094).
    pub fn preview_swap(pool: &LiquidityPool, amount_in: u128) -> u128 {
        if pool.reserve_a == 0 || pool.reserve_b == 0 {
            return 0;
        }
        Self::swap_out(
            amount_in,
            pool.reserve_a,
            pool.reserve_b,
            pool.fee_basis_points,
        )
        .unwrap_or(0)
    }

    /// Simple integer square root
    /// `a * b / d`, floored, in a 256-bit intermediate. `None` on a zero divisor or a result
    /// that does not fit a `u128`.
    ///
    /// The intermediate *has* to be wider than `u128`, and this is why: a reserve and an
    /// amount are both token quantities and their product is the constant-product formula
    /// itself. `u128` holds about 3.4e38; two quantities of 1e24 multiply to 1e48. The `f64`
    /// this replaces did not overflow — it did something worse, rounding both operands to 53
    /// bits of mantissa *before* the product, so the result was approximate at every size and
    /// the pool's reserves were then updated from it (TICKET-094, PHASE 43).
    ///
    /// Flooring is the AMM's convention: the pool keeps the remainder, so a swap can never
    /// take more out than the formula allows.
    fn mul_div(a: u128, b: u128, d: u128) -> Option<u128> {
        if d == 0 {
            return None;
        }
        let product = sp_core::U256::from(a).checked_mul(sp_core::U256::from(b))?;
        let quotient = product / sp_core::U256::from(d);
        if quotient > sp_core::U256::from(u128::MAX) {
            return None;
        }
        Some(quotient.low_u128())
    }

    /// The input a swap actually prices, after the venue's fee.
    fn net_of_fee(amount_in: u128, fee_basis_points: u32) -> u128 {
        let fee =
            Self::mul_div(amount_in, u128::from(fee_basis_points), 10_000).unwrap_or(amount_in);
        // A fee above the whole input cannot be a fee; `saturating_sub` leaves nothing to
        // price rather than wrapping, which is what the float form did too (`as u128` on a
        // negative float is 0).
        amount_in.saturating_sub(fee)
    }

    /// The output of a constant-product swap: `in_after_fee * reserve_out / (reserve_in +
    /// in_after_fee)`, all integers.
    fn swap_out(
        amount_in: u128,
        reserve_in: u128,
        reserve_out: u128,
        fee_basis_points: u32,
    ) -> Result<u128, &'static str> {
        let net = Self::net_of_fee(amount_in, fee_basis_points);
        let denominator = reserve_in
            .checked_add(net)
            .ok_or("Pool accounting overflow")?;
        Self::mul_div(net, reserve_out, denominator).ok_or("Swap calculation overflow")
    }

    /// LP tokens minted for a deposit, by the pool's own ratio: the **smaller** of the two
    /// legs, so a deposit cannot move the price. The first deposit has no ratio to hold to,
    /// so it mints the geometric mean of the two amounts.
    fn lp_for_deposit(pool: &LiquidityPool, amount_a: u128, amount_b: u128) -> Option<u128> {
        if pool.total_lp_supply == 0 {
            return Some(Self::sqrt(amount_a.saturating_mul(amount_b)));
        }
        let from_a = Self::mul_div(amount_a, pool.total_lp_supply, pool.reserve_a)?;
        let from_b = Self::mul_div(amount_b, pool.total_lp_supply, pool.reserve_b)?;
        Some(from_a.min(from_b))
    }

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

    /// A swap at a realistic size is **exact**, where the float form was not (TICKET-094).
    ///
    /// `10^24` is not representable as a `f64` — it becomes `999999999999999983222784`, off by
    /// `16_777_216` — and the old code converted every reserve and every amount to `f64`,
    /// multiplied them, and cast the quotient back. For a pool of `10^24` on each side taking
    /// a `10^18` swap at 30bps, the exact constant-product answer is
    /// `996_999_005_991_991_025` and the float form produced `996_999_005_991_991_040`:
    /// **fifteen units more than the formula allows**, because the operand was already wrong
    /// before the division. The pool paid the difference, every swap.
    ///
    /// That is not a rounding detail. It is a pool that can be drained fifteen units at a
    /// time, and it is the concrete form of PHASE 43's prohibition.
    #[test]
    fn a_swap_at_a_realistic_size_is_exact() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };
        let mut pool = AMMPool::create_pool(token_a, token_b, 30).unwrap();
        pool.reserve_a = 1_000_000_000_000_000_000_000_000; // 10^24
        pool.reserve_b = 1_000_000_000_000_000_000_000_000; // 10^24
                                                            // The first deposit sets the supply; the second is the swap's counterparty.
        pool.total_lp_supply = 1_000_000_000_000_000_000_000_000;

        let out = AMMPool::swap(&mut pool, 1_000_000_000_000_000_000, 1).unwrap();

        assert_eq!(
            out, 996_999_005_991_991_025,
            "the constant-product answer, exactly — the float form gave ...040, fifteen units \
             more than the formula allows"
        );
    }

    /// A swap never decreases `k`, which is the invariant the pool's solvency rests on.
    ///
    /// The fee is what makes it increase; an integer implementation that rounded the input up
    /// or the output down the wrong way would decrease it, and the float form's fifteen extra
    /// units did exactly that.
    #[test]
    fn a_swap_never_decreases_the_product() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };

        for (reserve, amount_in) in [
            (1_000_000u128, 1_000u128),
            (1_000_000_000_000_000_000, 1_000_000_000),
            (10u128.pow(24), 10u128.pow(18)),
        ] {
            let mut pool = AMMPool::create_pool(token_a.clone(), token_b.clone(), 30).unwrap();
            pool.reserve_a = reserve;
            pool.reserve_b = reserve;
            pool.total_lp_supply = reserve;

            let before = sp_core::U256::from(pool.reserve_a) * sp_core::U256::from(pool.reserve_b);
            AMMPool::swap(&mut pool, amount_in, 1).expect("a funded pool can swap");
            let after = sp_core::U256::from(pool.reserve_a) * sp_core::U256::from(pool.reserve_b);

            assert!(
                after >= before,
                "k must not fall: reserves {reserve}, amount {amount_in}, before {before}, after {after}"
            );
        }
    }

    /// A preview and an execution are the same formula, so they must not disagree.
    ///
    /// They were two separate `f64` computations of the constant-product rule before
    /// TICKET-094 — and `preview_swap` was the one that got missed when the others were
    /// converted, which is exactly the drift this test would have caught.
    #[test]
    fn preview_swap_agrees_with_swap() {
        let token_a = TokenId {
            chain_id: 1,
            asset_id: 1,
        };
        let token_b = TokenId {
            chain_id: 1,
            asset_id: 2,
        };

        for (reserve, amount_in) in [
            (1_000_000u128, 1_000u128),
            (10u128.pow(24), 10u128.pow(18)),
            (999_999_999_999_999_999_999u128, 1u128),
        ] {
            let mut pool = AMMPool::create_pool(token_a.clone(), token_b.clone(), 30).unwrap();
            pool.reserve_a = reserve;
            pool.reserve_b = reserve;
            pool.total_lp_supply = reserve;

            let preview = AMMPool::preview_swap(&pool, amount_in);
            // `min_out: 0` because this asks what the pool pays, not whether it clears a
            // bound — a one-unit swap into a 10^21 pool floors to zero, which the guard would
            // refuse and which is itself correct behaviour.
            let executed = AMMPool::swap(&mut pool, amount_in, 0).expect("a funded pool can swap");
            assert_eq!(
                preview, executed,
                "a preview must be the number the swap pays: reserves {reserve}, amount {amount_in}"
            );
        }
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
        assert_eq!(AMMPool::sqrt(999999), 999);
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
