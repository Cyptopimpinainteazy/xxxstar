#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    non_snake_case,
    unexpected_cfgs,
    unused_parens,
    non_camel_case_types,
    clippy::all
)]

//! X3 Atomic Trade Engine
//!
//! RPC endpoints for swaps, quotes, and DEX operations.

pub mod rollback_listener;
pub mod swap_rpc;

pub use rollback_listener::{
    FailureNotification, FailureReason, RollbackEventListener, RollbackLog, SeverityLevel,
    TradeBatchFailure,
};
pub use swap_rpc::{AMMPool, SwapOrder, SwapQuote, SwapRPCServer, SwapStatus, TokenPair};

/// Billing enforcement for atomic trade operations
pub mod billing {
    use std::collections::HashMap;

    /// Protocol fee collector address
    pub const PROTOCOL_FEE_COLLECTOR: [u8; 32] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xFE, 0xE5,
    ];

    /// Billing plan tiers
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum BillingPlan {
        Free,       // 100 requests/month
        Basic,      // 10,000 requests/month
        Pro,        // 100,000 requests/month
        Enterprise, // Unlimited
    }

    impl BillingPlan {
        pub fn monthly_quota(&self) -> u64 {
            match self {
                BillingPlan::Free => 100,
                BillingPlan::Basic => 10_000,
                BillingPlan::Pro => 100_000,
                BillingPlan::Enterprise => u64::MAX,
            }
        }
    }

    /// Billing account state
    #[derive(Clone, Debug)]
    pub struct BillingAccount {
        pub account_id: [u8; 32],
        pub plan: BillingPlan,
        pub used_this_month: u64,
        pub last_reset_epoch: u64,
        /// Protocol fee units reserved for the current in-flight request.
        pub pending_fee: u128,
        /// Cumulative protocol fees committed across all lifetime requests.
        pub total_fees_charged: u128,
    }

    impl BillingAccount {
        pub fn new(account_id: [u8; 32], plan: BillingPlan) -> Self {
            Self {
                account_id,
                plan,
                used_this_month: 0,
                last_reset_epoch: 0,
                pending_fee: 0,
                total_fees_charged: 0,
            }
        }

        pub fn remaining_quota(&self) -> u64 {
            self.plan
                .monthly_quota()
                .saturating_sub(self.used_this_month)
        }

        pub fn increment_usage(&mut self) -> Result<(), &'static str> {
            if self.used_this_month >= self.plan.monthly_quota() {
                return Err("Monthly quota exceeded");
            }
            self.used_this_month += 1;
            Ok(())
        }

        pub fn reset_if_new_month(&mut self, current_epoch: u64) {
            // Reset usage if we're in a new month (assuming 30-day epochs)
            const MONTH_IN_EPOCHS: u64 = 30 * 24 * 60; // ~30 days in minutes
            if current_epoch > self.last_reset_epoch + MONTH_IN_EPOCHS {
                self.used_this_month = 0;
                self.last_reset_epoch = current_epoch;
            }
        }
    }

    /// API key middleware for billing enforcement
    pub struct BillingMiddleware {
        accounts: HashMap<String, BillingAccount>,
    }

    impl BillingMiddleware {
        pub fn new() -> Self {
            Self {
                accounts: HashMap::new(),
            }
        }

        pub fn register_account(&mut self, api_key: String, account: BillingAccount) {
            self.accounts.insert(api_key, account);
        }

        pub fn validate_request(
            &mut self,
            api_key: &str,
            current_epoch: u64,
        ) -> Result<(), &'static str> {
            let account = self.accounts.get_mut(api_key).ok_or("Invalid API key")?;
            account.reset_if_new_month(current_epoch);
            account.increment_usage()
        }

        pub fn get_usage(&self, api_key: &str) -> Option<(u64, u64)> {
            self.accounts
                .get(api_key)
                .map(|a| (a.used_this_month, a.plan.monthly_quota()))
        }

        /// Reserve `fee` units in the named account's pending-fee counter before
        /// executing a billable operation.  Returns `Err` if the key is unknown.
        pub fn reserve_fee(&mut self, api_key: &str, fee: u128) -> Result<(), &'static str> {
            let account = self.accounts.get_mut(api_key).ok_or("Invalid API key")?;
            account.pending_fee = account.pending_fee.saturating_add(fee);
            Ok(())
        }

        /// Release a previously-reserved fee without charging it (call on failure).
        pub fn unreserve_fee(&mut self, api_key: &str, fee: u128) {
            if let Some(account) = self.accounts.get_mut(api_key) {
                account.pending_fee = account.pending_fee.saturating_sub(fee);
            }
        }

        /// Commit a previously-reserved fee to the lifetime total (call on success).
        pub fn commit_fee(&mut self, api_key: &str, fee: u128) {
            if let Some(account) = self.accounts.get_mut(api_key) {
                account.pending_fee = account.pending_fee.saturating_sub(fee);
                account.total_fees_charged = account.total_fees_charged.saturating_add(fee);
            }
        }
    }

    impl Default for BillingMiddleware {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Fee calculation for atomic trades
    pub fn calculate_trade_fee(legs: u32, capital: u128, cross_chain_hops: u32) -> u128 {
        const BASE_FEE: u128 = 10_000;
        const COMPLEXITY_MULTIPLIER: u128 = 500;
        const CAPITAL_COEFFICIENT: u128 = 1_000;
        const CROSS_CHAIN_SURCHARGE: u128 = 5_000;

        let complexity_fee = (legs as u128) * COMPLEXITY_MULTIPLIER;
        let capital_fee = if capital > 0 {
            (capital.ilog2() as u128) * CAPITAL_COEFFICIENT
        } else {
            0
        };
        let hop_fee = (cross_chain_hops as u128) * CROSS_CHAIN_SURCHARGE;

        BASE_FEE + complexity_fee + capital_fee + hop_fee
    }
}
