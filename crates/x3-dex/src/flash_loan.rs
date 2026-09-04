/// Flash Loan Engine — Uncollateralized single-transaction loans with 0.09% fee
/// Enables risk-free arbitrage and liquidations within atomic transactions
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FlashLoan {
    pub id: [u8; 32],
    pub borrower: [u8; 32],
    pub token: u128,
    pub amount: u64,
    pub fee: u64,
    pub repayment_required: u64, // amount + fee
    pub initiated_block: u64,
    pub execution_tx_hash: Option<[u8; 32]>,
    pub status: u8, // 0=pending, 1=executing, 2=repaid, 3=defaulted
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FlashLoanPool {
    pub pool_id: [u8; 32],
    pub token: u128,
    pub available_liquidity: u64,
    pub total_loaned: u64,
    pub total_fees_collected: u64,
    pub is_paused: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FlashLoanCallback {
    pub flash_loan_id: [u8; 32],
    pub borrower: [u8; 32],
    pub token: u128,
    pub amount: u64,
    pub fee: u64,
    pub callback_signature: Vec<u8>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FlashLoanStats {
    pub total_loans: u64,
    pub total_repayments: u64,
    pub total_volume: u64,
    pub total_fees_collected: u64,
    pub default_count: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct ArbitrageExecution {
    pub flash_loan_id: [u8; 32],
    pub initial_amount: u64,
    pub profit: u64,
    pub fee_paid: u64,
    pub net_profit: u64,
    pub success: bool,
}

pub struct FlashLoanEngine;

impl FlashLoanEngine {
    const FLASH_LOAN_FEE_BPS: u64 = 9; // 0.09%
    const MIN_LOAN_AMOUNT: u64 = 1;
    const MAX_LOAN_AMOUNT: u64 = u64::MAX / 2; // Prevent overflow
    const DEFAULT_PENALTY_BPS: u64 = 1_000; // 10% penalty on default

    /// Initiate a flash loan
    pub fn initiate_flash_loan(
        borrower: [u8; 32],
        token: u128,
        amount: u64,
        current_block: u64,
    ) -> Result<FlashLoan, &'static str> {
        if !(Self::MIN_LOAN_AMOUNT..=Self::MAX_LOAN_AMOUNT).contains(&amount) {
            return Err("Loan amount out of range");
        }

        let fee = Self::calculate_fee(amount);
        let repayment = amount.saturating_add(fee);

        if repayment.checked_add(amount).is_none() {
            return Err("Repayment amount would overflow");
        }

        let loan = FlashLoan {
            id: Self::derive_loan_id(borrower, token, amount, current_block),
            borrower,
            token,
            amount,
            fee,
            repayment_required: repayment,
            initiated_block: current_block,
            execution_tx_hash: None,
            status: 0, // pending
        };

        Ok(loan)
    }

    /// Execute flash loan (borrower receives funds)
    pub fn execute_flash_loan(
        loan: &mut FlashLoan,
        pool: &mut FlashLoanPool,
        _current_block: u64,
    ) -> Result<(u64, u64), &'static str> {
        if loan.status != 0 {
            return Err("Loan not in pending state");
        }

        if pool.available_liquidity < loan.amount {
            return Err("Insufficient liquidity in pool");
        }

        if pool.is_paused {
            return Err("Flash loan pool is paused");
        }

        // Deduct from pool
        pool.available_liquidity -= loan.amount;
        pool.total_loaned += loan.amount;

        // Update loan status
        loan.status = 1; // executing

        Ok((loan.amount, loan.fee))
    }

    /// Repay flash loan
    pub fn repay_flash_loan(
        loan: &mut FlashLoan,
        pool: &mut FlashLoanPool,
        repayment_amount: u64,
        _current_block: u64,
    ) -> Result<bool, &'static str> {
        if loan.status != 1 {
            return Err("Loan is not executing");
        }

        if repayment_amount < loan.repayment_required {
            return Err("Insufficient repayment amount");
        }

        // Return to pool
        pool.available_liquidity += loan.amount;
        pool.total_loaned -= loan.amount;
        pool.total_fees_collected += loan.fee;

        loan.status = 2; // repaid
        Ok(true)
    }

    /// Handle default (borrower didn't repay)
    pub fn mark_default(
        loan: &mut FlashLoan,
        pool: &mut FlashLoanPool,
    ) -> Result<u64, &'static str> {
        if loan.status != 1 {
            return Err("Loan is not executing");
        }

        // Calculate penalty
        let penalty = (loan.amount * Self::DEFAULT_PENALTY_BPS) / 10_000;

        loan.status = 3; // defaulted
        pool.total_loaned -= loan.amount;

        Ok(penalty)
    }

    /// Calculate flash loan fee
    pub fn calculate_fee(amount: u64) -> u64 {
        (amount * Self::FLASH_LOAN_FEE_BPS) / 10_000
    }

    /// Check if execution completed in time (same block)
    pub fn is_execution_timely(loan: &FlashLoan, current_block: u64) -> bool {
        current_block == loan.initiated_block
    }

    /// Execute arbitrage with flash loan (e.g., buy low, sell high, repay)
    pub fn execute_arbitrage(
        loan: &mut FlashLoan,
        initial_buy_price: u64,
        arbitrage_sell_price: u64,
        quantity: u64,
        current_block: u64,
    ) -> Result<ArbitrageExecution, &'static str> {
        if loan.status != 1 {
            return Err("Loan not executing");
        }

        if !Self::is_execution_timely(loan, current_block) {
            return Err("Execution timeout");
        }

        if arbitrage_sell_price <= initial_buy_price {
            return Err("No arbitrage opportunity");
        }

        // Calculate profit: (sell_price - buy_price) * quantity - fee
        let price_diff = arbitrage_sell_price - initial_buy_price;
        let gross_profit =
            (price_diff as u128 * quantity as u128 / initial_buy_price as u128) as u64;
        let net_profit = gross_profit.saturating_sub(loan.fee);

        let execution = ArbitrageExecution {
            flash_loan_id: loan.id,
            initial_amount: loan.amount,
            profit: gross_profit,
            fee_paid: loan.fee,
            net_profit,
            success: net_profit > 0,
        };

        Ok(execution)
    }

    /// Create or update flash loan pool
    pub fn create_flash_loan_pool(
        token: u128,
        initial_liquidity: u64,
    ) -> Result<FlashLoanPool, &'static str> {
        if initial_liquidity == 0 {
            return Err("Pool must have initial liquidity");
        }

        let pool = FlashLoanPool {
            pool_id: Self::derive_pool_id(token),
            token,
            available_liquidity: initial_liquidity,
            total_loaned: 0,
            total_fees_collected: 0,
            is_paused: false,
        };

        Ok(pool)
    }

    /// Deposit into flash loan pool
    pub fn deposit_to_pool(pool: &mut FlashLoanPool, amount: u64) -> Result<u64, &'static str> {
        if amount == 0 {
            return Err("Deposit amount must be > 0");
        }

        pool.available_liquidity = pool.available_liquidity.saturating_add(amount);
        Ok(pool.available_liquidity)
    }

    /// Withdraw from flash loan pool
    pub fn withdraw_from_pool(pool: &mut FlashLoanPool, amount: u64) -> Result<u64, &'static str> {
        if amount > pool.available_liquidity {
            return Err("Insufficient available liquidity");
        }

        pool.available_liquidity -= amount;
        Ok(pool.available_liquidity)
    }

    /// Pause flash loan pool (emergency)
    pub fn pause_pool(pool: &mut FlashLoanPool) -> Result<bool, &'static str> {
        pool.is_paused = true;
        Ok(true)
    }

    /// Resume flash loan pool
    pub fn resume_pool(pool: &mut FlashLoanPool) -> Result<bool, &'static str> {
        pool.is_paused = false;
        Ok(true)
    }

    /// Calculate statistics
    pub fn calculate_stats(
        total_loans: u64,
        total_repaid: u64,
        volume: u64,
        fees: u64,
        defaults: u32,
    ) -> FlashLoanStats {
        FlashLoanStats {
            total_loans,
            total_repayments: total_repaid,
            total_volume: volume,
            total_fees_collected: fees,
            default_count: defaults,
        }
    }

    /// Check if flashloan is profitable after fee
    pub fn is_profitable(loan_amount: u64, expected_return: u64) -> bool {
        let fee = Self::calculate_fee(loan_amount);
        expected_return > fee
    }

    /// Derive deterministic loan ID
    fn derive_loan_id(borrower: [u8; 32], token: u128, amount: u64, nonce: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in borrower.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let token_bytes = token.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate().take(8) {
            id[i + 8] = *byte;
        }
        let amount_bytes = amount.to_le_bytes();
        for (i, byte) in amount_bytes.iter().enumerate().take(8) {
            id[i + 16] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive pool ID
    fn derive_pool_id(token: u128) -> [u8; 32] {
        let mut id = [0u8; 32];
        let token_bytes = token.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiate_flash_loan() {
        let loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        assert_eq!(loan.amount, 10_000_000);
        assert_eq!(loan.status, 0);
    }

    #[test]
    fn test_calculate_fee() {
        let fee = FlashLoanEngine::calculate_fee(1_000_000);
        assert_eq!(fee, 900); // 0.09% of 1M
    }

    #[test]
    fn test_initiate_flash_loan_invalid_amounts() {
        // Too small
        assert!(FlashLoanEngine::initiate_flash_loan([1; 32], 1, 0, 100).is_err());
        // Too large / overflow guarded
        assert!(FlashLoanEngine::initiate_flash_loan([1; 32], 1, u64::MAX, 100).is_err());
    }

    #[test]
    fn test_execute_flash_loan() {
        let mut loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();

        let (amount, fee) = FlashLoanEngine::execute_flash_loan(&mut loan, &mut pool, 100).unwrap();

        assert_eq!(amount, 10_000_000);
        assert_eq!(fee, 9_000);
        assert_eq!(loan.status, 1);
    }

    #[test]
    fn test_repay_flash_loan() {
        let mut loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();
        FlashLoanEngine::execute_flash_loan(&mut loan, &mut pool, 100).unwrap();

        let repayment_required = loan.repayment_required;
        FlashLoanEngine::repay_flash_loan(&mut loan, &mut pool, repayment_required, 100).unwrap();

        assert_eq!(loan.status, 2);
        assert_eq!(pool.total_fees_collected, 9_000);
    }

    #[test]
    fn test_mark_default() {
        let mut loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();
        FlashLoanEngine::execute_flash_loan(&mut loan, &mut pool, 100).unwrap();

        let penalty = FlashLoanEngine::mark_default(&mut loan, &mut pool).unwrap();

        assert_eq!(loan.status, 3);
        assert!(penalty > 0);
    }

    #[test]
    fn test_execute_arbitrage() {
        let mut loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();
        FlashLoanEngine::execute_flash_loan(&mut loan, &mut pool, 100).unwrap();

        let arb = FlashLoanEngine::execute_arbitrage(
            &mut loan, 5_000, // buy price
            5_100, // sell price (1% profit)
            10_000_000, 100,
        )
        .unwrap();

        assert!(arb.success);
    }

    #[test]
    fn test_execute_flash_loan_paused_pool_fails() {
        let mut loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 1_000_000, 100).unwrap();
        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 2_000_000).unwrap();
        FlashLoanEngine::pause_pool(&mut pool).unwrap();
        assert!(FlashLoanEngine::execute_flash_loan(&mut loan, &mut pool, 100).is_err());
    }

    #[test]
    fn test_create_pool() {
        let pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();

        assert_eq!(pool.available_liquidity, 100_000_000);
        assert!(!pool.is_paused);
    }

    #[test]
    fn test_deposit_to_pool() {
        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();

        let new_balance = FlashLoanEngine::deposit_to_pool(&mut pool, 50_000_000).unwrap();

        assert_eq!(new_balance, 150_000_000);
    }

    #[test]
    fn test_withdraw_from_pool() {
        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();

        let new_balance = FlashLoanEngine::withdraw_from_pool(&mut pool, 30_000_000).unwrap();

        assert_eq!(new_balance, 70_000_000);
    }

    #[test]
    fn test_pause_pool() {
        let mut pool = FlashLoanEngine::create_flash_loan_pool(1, 100_000_000).unwrap();

        FlashLoanEngine::pause_pool(&mut pool).unwrap();
        assert!(pool.is_paused);
    }

    #[test]
    fn test_is_profitable() {
        let is_profitable = FlashLoanEngine::is_profitable(1_000_000, 10_000);

        assert!(is_profitable);
    }

    #[test]
    fn test_is_execution_timely() {
        let loan = FlashLoanEngine::initiate_flash_loan([1; 32], 1, 10_000_000, 100).unwrap();

        assert!(FlashLoanEngine::is_execution_timely(&loan, 100));
        assert!(!FlashLoanEngine::is_execution_timely(&loan, 101));
    }
}
