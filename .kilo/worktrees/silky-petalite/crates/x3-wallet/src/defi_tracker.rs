/// DeFi Position Tracker — Unified view of LP positions, staking, and borrows
/// Track liquidity pool shares, staking positions, and open borrows
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LPPosition {
    pub id: [u8; 32],
    pub holder: [u8; 32],
    pub pool_address: [u8; 32],
    pub token_a: [u8; 32],
    pub token_b: [u8; 32],
    pub liquidity_share: u128, // user's LP token balance
    pub pool_size_token_a: u128,
    pub pool_size_token_b: u128,
    pub created_block: u64,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct StakingPosition {
    pub id: [u8; 32],
    pub staker: [u8; 32],
    pub validator: [u8; 32],
    pub amount_staked: u128,
    pub rewards_earned: u128,
    pub stake_started_block: u64,
    pub lock_duration_blocks: u64,
    pub is_locked: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BorrowPosition {
    pub id: [u8; 32],
    pub borrower: [u8; 32],
    pub collateral_token: [u8; 32],
    pub collateral_amount: u128,
    pub borrowed_token: [u8; 32],
    pub borrowed_amount: u128,
    pub interest_accrued: u128,
    pub borrow_block: u64,
    pub health_factor: u32, // scaled by 1000 (e.g., 1500 = 1.5x)
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct DeFiPortfolio {
    pub owner: [u8; 32],
    pub lp_positions: Vec<[u8; 32]>,      // LP position IDs
    pub staking_positions: Vec<[u8; 32]>, // Staking position IDs
    pub borrow_positions: Vec<[u8; 32]>,  // Borrow position IDs
    pub total_value_usd: u128,
    pub last_updated_block: u64,
}

pub struct DeFiTracker;

impl DeFiTracker {
    /// Create LP position
    pub fn create_lp_position(
        holder: [u8; 32],
        pool_address: [u8; 32],
        token_a: [u8; 32],
        token_b: [u8; 32],
        liquidity_share: u128,
        pool_size_a: u128,
        pool_size_b: u128,
        current_block: u64,
    ) -> Result<LPPosition, &'static str> {
        if liquidity_share == 0 {
            return Err("Liquidity share must be > 0");
        }
        if pool_size_a == 0 || pool_size_b == 0 {
            return Err("Pool must have liquidity");
        }

        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&holder[0..8]);
        id[8..16].copy_from_slice(&pool_address[0..8]);

        Ok(LPPosition {
            id,
            holder,
            pool_address,
            token_a,
            token_b,
            liquidity_share,
            pool_size_token_a: pool_size_a,
            pool_size_token_b: pool_size_b,
            created_block: current_block,
            is_active: true,
        })
    }

    /// Update LP position (rebalance)
    pub fn update_lp_position(
        position: &mut LPPosition,
        new_share: u128,
        new_pool_size_a: u128,
        new_pool_size_b: u128,
    ) -> Result<(), &'static str> {
        if !position.is_active {
            return Err("Position not active");
        }
        if new_share == 0 {
            return Err("Share must be > 0");
        }

        position.liquidity_share = new_share;
        position.pool_size_token_a = new_pool_size_a;
        position.pool_size_token_b = new_pool_size_b;
        Ok(())
    }

    /// Calculate LP position value (simplified)
    pub fn calculate_lp_value(position: &LPPosition) -> u128 {
        // value = (user_share / total_liquidity) * pool_size_a + pool_size_b
        if position.pool_size_token_a == 0 || position.pool_size_token_b == 0 {
            return 0;
        }
        position.liquidity_share // simplified: just return share for now
    }

    /// Create staking position
    pub fn create_staking_position(
        staker: [u8; 32],
        validator: [u8; 32],
        amount: u128,
        lock_duration: u64,
        current_block: u64,
    ) -> Result<StakingPosition, &'static str> {
        if amount == 0 {
            return Err("Stake amount must be > 0");
        }
        if lock_duration > 365 * 24 * 10 {
            return Err("Lock duration too long");
        }

        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&staker[0..8]);
        id[8..16].copy_from_slice(&validator[0..8]);

        Ok(StakingPosition {
            id,
            staker,
            validator,
            amount_staked: amount,
            rewards_earned: 0,
            stake_started_block: current_block,
            lock_duration_blocks: lock_duration,
            is_locked: true,
        })
    }

    /// Add rewards to staking position
    pub fn add_staking_rewards(
        position: &mut StakingPosition,
        reward_amount: u128,
    ) -> Result<(), &'static str> {
        if !position.is_locked {
            return Err("Position not locked");
        }
        if reward_amount == 0 {
            return Err("Reward must be > 0");
        }

        position.rewards_earned += reward_amount;
        Ok(())
    }

    /// Unlock staking position
    pub fn unlock_staking_position(
        position: &mut StakingPosition,
        current_block: u64,
    ) -> Result<(), &'static str> {
        if !position.is_locked {
            return Err("Position not locked");
        }
        if current_block < position.stake_started_block + position.lock_duration_blocks {
            return Err("Lock period not elapsed");
        }

        position.is_locked = false;
        Ok(())
    }

    /// Create borrow position
    pub fn create_borrow_position(
        borrower: [u8; 32],
        collateral_token: [u8; 32],
        collateral_amount: u128,
        borrowed_token: [u8; 32],
        borrowed_amount: u128,
        current_block: u64,
    ) -> Result<BorrowPosition, &'static str> {
        if collateral_amount == 0 || borrowed_amount == 0 {
            return Err("Amounts must be > 0");
        }

        // Simple health factor: (collateral_amount * 1000) / (borrowed_amount * 2)
        // threshold is 1000 (1.0x), liquidation at < 1000
        let health_factor: u32 = if borrowed_amount > 0 {
            (((collateral_amount / 2) * 1000) / borrowed_amount).min(u32::MAX as u128) as u32
        } else {
            u32::MAX
        };

        if health_factor < 1000 {
            return Err("Health factor too low");
        }

        let mut id = [0u8; 32];
        id[0..8].copy_from_slice(&borrower[0..8]);
        id[8..16].copy_from_slice(&collateral_token[0..8]);

        Ok(BorrowPosition {
            id,
            borrower,
            collateral_token,
            collateral_amount,
            borrowed_token,
            borrowed_amount,
            interest_accrued: 0,
            borrow_block: current_block,
            health_factor,
        })
    }

    /// Accrue interest on borrow
    pub fn accrue_interest(
        position: &mut BorrowPosition,
        rate_per_block: u32, // e.g., 10 = 0.01% per block
    ) -> Result<(), &'static str> {
        if position.borrowed_amount == 0 {
            return Err("Nothing borrowed");
        }

        let interest = (position.borrowed_amount as u32 * rate_per_block / 100000) as u128;
        position.interest_accrued += interest;
        Ok(())
    }

    /// Repay borrow
    pub fn repay_borrow(
        position: &mut BorrowPosition,
        repay_amount: u128,
    ) -> Result<(), &'static str> {
        if position.borrowed_amount == 0 {
            return Err("Nothing to repay");
        }
        if repay_amount == 0 {
            return Err("Repay amount must be > 0");
        }

        let total_debt = position.borrowed_amount + position.interest_accrued;
        if repay_amount > total_debt {
            return Err("Repay amount exceeds debt");
        }

        position.borrowed_amount = if total_debt >= repay_amount {
            total_debt - repay_amount
        } else {
            0
        };
        Ok(())
    }

    /// Update health factor
    pub fn update_health_factor(position: &mut BorrowPosition) -> Result<(), &'static str> {
        if position.borrowed_amount == 0 {
            position.health_factor = u32::MAX;
            return Ok(());
        }

        let health = ((position.collateral_amount / 2) * 1000) / position.borrowed_amount;
        position.health_factor = health as u32;

        if position.health_factor < 1000 {
            return Err("Position at risk of liquidation");
        }

        Ok(())
    }

    /// Create portfolio summary
    pub fn create_portfolio(owner: [u8; 32], current_block: u64) -> DeFiPortfolio {
        DeFiPortfolio {
            owner,
            lp_positions: vec![],
            staking_positions: vec![],
            borrow_positions: vec![],
            total_value_usd: 0,
            last_updated_block: current_block,
        }
    }

    /// Add LP position to portfolio
    pub fn add_lp_to_portfolio(
        portfolio: &mut DeFiPortfolio,
        lp_id: [u8; 32],
    ) -> Result<(), &'static str> {
        if portfolio.lp_positions.contains(&lp_id) {
            return Err("Position already in portfolio");
        }
        portfolio.lp_positions.push(lp_id);
        Ok(())
    }

    /// Add staking position to portfolio
    pub fn add_staking_to_portfolio(
        portfolio: &mut DeFiPortfolio,
        stake_id: [u8; 32],
    ) -> Result<(), &'static str> {
        if portfolio.staking_positions.contains(&stake_id) {
            return Err("Position already in portfolio");
        }
        portfolio.staking_positions.push(stake_id);
        Ok(())
    }

    /// Add borrow position to portfolio
    pub fn add_borrow_to_portfolio(
        portfolio: &mut DeFiPortfolio,
        borrow_id: [u8; 32],
    ) -> Result<(), &'static str> {
        if portfolio.borrow_positions.contains(&borrow_id) {
            return Err("Position already in portfolio");
        }
        portfolio.borrow_positions.push(borrow_id);
        Ok(())
    }

    /// Get portfolio position count
    pub fn get_position_count(portfolio: &DeFiPortfolio) -> usize {
        portfolio.lp_positions.len()
            + portfolio.staking_positions.len()
            + portfolio.borrow_positions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_lp_position() {
        let result = DeFiTracker::create_lp_position(
            [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], 1000, 10000, 10000, 100,
        );
        assert!(result.is_ok());
        let pos = result.unwrap();
        assert_eq!(pos.liquidity_share, 1000);
    }

    #[test]
    fn test_create_lp_position_zero_share() {
        let result = DeFiTracker::create_lp_position(
            [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], 0, 10000, 10000, 100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_update_lp_position() {
        let mut pos = DeFiTracker::create_lp_position(
            [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], 1000, 10000, 10000, 100,
        )
        .unwrap();

        let result = DeFiTracker::update_lp_position(&mut pos, 2000, 12000, 12000);
        assert!(result.is_ok());
        assert_eq!(pos.liquidity_share, 2000);
    }

    #[test]
    fn test_calculate_lp_value() {
        let pos = DeFiTracker::create_lp_position(
            [1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], 1000, 10000, 10000, 100,
        )
        .unwrap();

        let value = DeFiTracker::calculate_lp_value(&pos);
        assert!(value > 0);
    }

    #[test]
    fn test_create_staking_position() {
        let result = DeFiTracker::create_staking_position([1u8; 32], [2u8; 32], 5000, 720, 100);
        assert!(result.is_ok());
        let stake = result.unwrap();
        assert_eq!(stake.amount_staked, 5000);
        assert!(stake.is_locked);
    }

    #[test]
    fn test_create_staking_position_zero_amount() {
        let result = DeFiTracker::create_staking_position([1u8; 32], [2u8; 32], 0, 720, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_staking_rewards() {
        let mut stake =
            DeFiTracker::create_staking_position([1u8; 32], [2u8; 32], 5000, 720, 100).unwrap();

        let result = DeFiTracker::add_staking_rewards(&mut stake, 250);
        assert!(result.is_ok());
        assert_eq!(stake.rewards_earned, 250);
    }

    #[test]
    fn test_unlock_staking_position() {
        let mut stake =
            DeFiTracker::create_staking_position([1u8; 32], [2u8; 32], 5000, 100, 0).unwrap();

        let result = DeFiTracker::unlock_staking_position(&mut stake, 101);
        assert!(result.is_ok());
        assert!(!stake.is_locked);
    }

    #[test]
    fn test_unlock_staking_position_not_ready() {
        let mut stake =
            DeFiTracker::create_staking_position([1u8; 32], [2u8; 32], 5000, 100, 0).unwrap();

        let result = DeFiTracker::unlock_staking_position(&mut stake, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_borrow_position() {
        let result =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 10000, [3u8; 32], 5000, 100);
        assert!(result.is_ok());
        let borrow = result.unwrap();
        assert!(borrow.health_factor >= 1000);
    }

    #[test]
    fn test_create_borrow_position_low_health() {
        let result =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 1000, [3u8; 32], 5000, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_accrue_interest() {
        let mut borrow =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 10000, [3u8; 32], 5000, 100)
                .unwrap();

        let result = DeFiTracker::accrue_interest(&mut borrow, 10);
        assert!(result.is_ok());
        assert!(borrow.interest_accrued > 0);
    }

    #[test]
    fn test_repay_borrow() {
        let mut borrow =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 10000, [3u8; 32], 5000, 100)
                .unwrap();

        let result = DeFiTracker::repay_borrow(&mut borrow, 2000);
        assert!(result.is_ok());
        assert!(borrow.borrowed_amount < 5000);
    }

    #[test]
    fn test_repay_borrow_exceeds_debt() {
        let mut borrow =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 10000, [3u8; 32], 5000, 100)
                .unwrap();

        let result = DeFiTracker::repay_borrow(&mut borrow, 10000);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_health_factor() {
        let mut borrow =
            DeFiTracker::create_borrow_position([1u8; 32], [2u8; 32], 10000, [3u8; 32], 5000, 100)
                .unwrap();

        borrow.collateral_amount = 100;
        let result = DeFiTracker::update_health_factor(&mut borrow);
        assert!(result.is_err()); // health factor too low now
    }

    #[test]
    fn test_create_portfolio() {
        let portfolio = DeFiTracker::create_portfolio([1u8; 32], 100);
        assert_eq!(portfolio.owner, [1u8; 32]);
        assert_eq!(DeFiTracker::get_position_count(&portfolio), 0);
    }

    #[test]
    fn test_add_lp_to_portfolio() {
        let mut portfolio = DeFiTracker::create_portfolio([1u8; 32], 100);

        let result = DeFiTracker::add_lp_to_portfolio(&mut portfolio, [99u8; 32]);
        assert!(result.is_ok());
        assert_eq!(portfolio.lp_positions.len(), 1);
    }

    #[test]
    fn test_add_staking_to_portfolio() {
        let mut portfolio = DeFiTracker::create_portfolio([1u8; 32], 100);

        let result = DeFiTracker::add_staking_to_portfolio(&mut portfolio, [99u8; 32]);
        assert!(result.is_ok());
        assert_eq!(portfolio.staking_positions.len(), 1);
    }

    #[test]
    fn test_add_borrow_to_portfolio() {
        let mut portfolio = DeFiTracker::create_portfolio([1u8; 32], 100);

        let result = DeFiTracker::add_borrow_to_portfolio(&mut portfolio, [99u8; 32]);
        assert!(result.is_ok());
        assert_eq!(portfolio.borrow_positions.len(), 1);
    }

    #[test]
    fn test_get_position_count() {
        let mut portfolio = DeFiTracker::create_portfolio([1u8; 32], 100);

        DeFiTracker::add_lp_to_portfolio(&mut portfolio, [99u8; 32]).unwrap();
        DeFiTracker::add_staking_to_portfolio(&mut portfolio, [98u8; 32]).unwrap();

        assert_eq!(DeFiTracker::get_position_count(&portfolio), 2);
    }
}
