//! Settlement Engine Integration for Limit Order Book
//!
//! Wires the off-chain LimitOrderBookEngine to the on-chain X3 Settlement Engine
//! for atomic execution of matched orders.
//!
//! ## Flow
//! 1. Orders matched off-chain by LimitOrderBookEngine
//! 2. Settlement intent created on X3
//! 3. Assets locked in escrow
//! 4. Cross-VM execution (if needed)
//! 5. Finalize settlement or refund on timeout
//!
//! ## Integration Workflow
//!
//! ### Step 1: Off-Chain Order Matching (limit_order_book.rs)
//! ```ignore
//! use x3_dex::limit_order_book::LimitOrderBookEngine;
//!
//! let (buy_execution, sell_execution) = LimitOrderBookEngine::match_orders(
//!     &buy_order,
//!     &sell_order,
//!     execution_price,
//!     timestamp,
//! );
//! ```
//!
//! ### Step 2: Create Settlement Intent (settlement_bridge.rs)
//! ```ignore
//! use x3_dex::settlement_bridge::LimitOrderSettlementBridge;
//!
//! let settlement_intent = LimitOrderSettlementBridge::create_settlement_intent_from_execution(
//!     &buy_execution,
//!     &sell_execution,
//!     &buy_order,
//!     &sell_order,
//!     current_block,
//! )?;
//! ```
//!
//! ### Step 3: Submit to On-Chain Settlement Engine (RPC layer - node/src/rpc.rs)
//! ```ignore
//! // Map OrderSettlementIntent → pallet_x3_settlement_engine::SettlementIntent
//! use pallet_x3_settlement_engine::types::{SettlementIntent, AssetSpec, TokenId};
//!
//! let on_chain_intent = SettlementIntent {
//!     intent_id: settlement_intent.intent_id,
//!     maker: AccountId32::from(settlement_intent.seller), // Seller is maker (limit order)
//!     taker: AccountId32::from(settlement_intent.buyer),  // Buyer is taker (market order)
//!     asset_a: AssetSpec {
//!         chain: ExternalChainId::X3,
//!         token: TokenId::X3Asset(settlement_intent.token_out as u32),
//!         amount: settlement_intent.settlement_amount as u128,
//!     },
//!     asset_b: AssetSpec {
//!         chain: ExternalChainId::X3,
//!         token: TokenId::X3Asset(settlement_intent.token_in as u32),
//!         amount: calculate_output_amount(settlement_intent.settlement_amount, settlement_intent.execution_price),
//!     },
//!     secret_hash: H256::zero(), // Not used for DEX atomic swaps
//!     timeout: settlement_intent.deadline,
//!     created_at: settlement_intent.created_at,
//!     legs_total: 2,
//!     legs_locked: 0,
//!     legs_claimed: 0,
//! };
//!
//! // Submit via extrinsic
//! pallet_x3_settlement_engine::Pallet::<Runtime>::create_intent(origin, on_chain_intent)?;
//! ```
//!
//! ### Step 4: Asset Locking & Finalization (pallet-x3-settlement-engine)
//! The settlement engine handles:
//! - Locking buyer's asset_b (token_in) in escrow
//! - Locking seller's asset_a (token_out) in escrow
//! - State transition: Pending → Locked → Executing → Finalized
//! - Timeout refund if deadline exceeded
//! - Atomic guarantee: both assets transferred or both refunded
//!
//! ## RPC Endpoint Integration
//!
//! Add to `node/src/rpc.rs`:
//! ```ignore
//! module.register_async_method("walletDex_executeLimitOrder", |params, _ctx| async move {
//!     // 1. Parse order parameters
//!     // 2. Call LimitOrderBookEngine::match_orders
//!     // 3. Call LimitOrderSettlementBridge::create_settlement_intent_from_execution
//!     // 4. Submit to pallet_x3_settlement_engine via extrinsic
//!     // 5. Return settlement intent ID to caller
//! })?;
//! ```

use crate::limit_order_book::{LimitOrder, OrderExecution};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::H256;
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OrderSettlementIntent {
    /// Unique settlement intent ID
    pub intent_id: H256,
    /// Buy order ID
    pub buy_order_id: [u8; 32],
    /// Sell order ID
    pub sell_order_id: [u8; 32],
    /// Execution price (in basis points)
    pub execution_price: u64,
    /// Amount to be settled
    pub settlement_amount: u64,
    /// Buyer account
    pub buyer: [u8; 32],
    /// Seller account
    pub seller: [u8; 32],
    /// Token being bought
    pub token_in: u128,
    /// Token being sold
    pub token_out: u128,
    /// Settlement status
    pub status: SettlementStatus,
    /// Created at block
    pub created_at: u64,
    /// Settlement deadline (blocks)
    pub deadline: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum SettlementStatus {
    /// Intent created, awaiting asset lock
    Pending,
    /// Assets locked in escrow
    Locked,
    /// Cross-VM execution in progress
    Executing,
    /// Settlement finalized successfully
    Finalized,
    /// Settlement failed, refund initiated
    Refunded,
    /// Timed out, automatic refund
    TimedOut,
}

pub struct LimitOrderSettlementBridge;

impl LimitOrderSettlementBridge {
    /// Create settlement intent from matched orders
    ///
    /// This is the bridge function between off-chain order matching
    /// and on-chain settlement execution.
    ///
    /// # Arguments
    /// * `buy_order` - The buy limit order
    /// * `sell_order` - The sell limit order
    /// * `execution_price` - Agreed execution price (in basis points)
    /// * `execution_amount` - Amount to settle
    /// * `current_block` - Current block number
    ///
    /// # Returns
    /// Settlement intent ready for on-chain submission
    pub fn create_settlement_intent(
        buy_order: &LimitOrder,
        sell_order: &LimitOrder,
        execution_price: u64,
        execution_amount: u64,
        current_block: u64,
    ) -> Result<OrderSettlementIntent, &'static str> {
        // Validation: orders must be compatible
        if buy_order.token_out != sell_order.token_in {
            return Err("Token mismatch: buy_order.token_out != sell_order.token_in");
        }

        if buy_order.token_in != sell_order.token_out {
            return Err("Token mismatch: buy_order.token_in != sell_order.token_out");
        }

        // Buy order must be willing to pay execution_price or more
        if buy_order.limit_price < execution_price {
            return Err("Buy order limit price below execution price");
        }

        // Sell order must accept execution_price or less
        if sell_order.limit_price > execution_price {
            return Err("Sell order limit price above execution price");
        }

        // Amount must not exceed remaining capacity
        let buy_remaining = buy_order.amount_in.saturating_sub(buy_order.filled_amount);
        let sell_remaining = sell_order
            .amount_in
            .saturating_sub(sell_order.filled_amount);

        if execution_amount > buy_remaining || execution_amount > sell_remaining {
            return Err("Execution amount exceeds remaining order capacity");
        }

        // Generate intent ID from order IDs + execution price + block
        let intent_id = Self::derive_intent_id(
            buy_order.id,
            sell_order.id,
            execution_price,
            execution_amount,
            current_block,
        );

        // Settlement deadline: 100 blocks (~10 minutes at 6s/block)
        const SETTLEMENT_TIMEOUT_BLOCKS: u64 = 100;

        Ok(OrderSettlementIntent {
            intent_id,
            buy_order_id: buy_order.id,
            sell_order_id: sell_order.id,
            execution_price,
            settlement_amount: execution_amount,
            buyer: buy_order.user,
            seller: sell_order.user,
            token_in: buy_order.token_in,
            token_out: buy_order.token_out,
            status: SettlementStatus::Pending,
            created_at: current_block,
            deadline: current_block + SETTLEMENT_TIMEOUT_BLOCKS,
        })
    }

    /// Derive deterministic intent ID from order parameters
    fn derive_intent_id(
        buy_order_id: [u8; 32],
        sell_order_id: [u8; 32],
        execution_price: u64,
        execution_amount: u64,
        block: u64,
    ) -> H256 {
        // Simple deterministic hash for no_std compatibility
        // XOR all inputs together for a basic deterministic ID
        let mut hash = [0u8; 32];

        for (i, &byte) in buy_order_id.iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        for (i, &byte) in sell_order_id.iter().enumerate() {
            hash[(i + 8) % 32] ^= byte;
        }

        let price_bytes = execution_price.to_le_bytes();
        for (i, &byte) in price_bytes.iter().enumerate() {
            hash[(i + 16) % 32] ^= byte;
        }

        let amount_bytes = execution_amount.to_le_bytes();
        for (i, &byte) in amount_bytes.iter().enumerate() {
            hash[(i + 20) % 32] ^= byte;
        }

        let block_bytes = block.to_le_bytes();
        for (i, &byte) in block_bytes.iter().enumerate() {
            hash[(i + 24) % 32] ^= byte;
        }

        H256::from(hash)
    }

    /// Check if settlement intent can be finalized
    ///
    /// Called by settlement engine to verify all conditions met:
    /// - Both orders still valid
    /// - Assets successfully locked
    /// - No timeout occurred
    pub fn can_finalize_intent(
        intent: &OrderSettlementIntent,
        current_block: u64,
    ) -> Result<bool, &'static str> {
        // Check timeout
        if current_block > intent.deadline {
            return Ok(false);
        }

        // Check status progression
        match intent.status {
            SettlementStatus::Locked | SettlementStatus::Executing => Ok(true),
            SettlementStatus::Finalized => Err("Intent already finalized"),
            SettlementStatus::Refunded | SettlementStatus::TimedOut => Err("Intent failed"),
            SettlementStatus::Pending => Err("Assets not yet locked"),
        }
    }

    /// Create order execution record from finalized intent
    pub fn create_execution_record(
        intent: &OrderSettlementIntent,
        timestamp: u64,
    ) -> (OrderExecution, OrderExecution) {
        let buy_execution = OrderExecution {
            order_id: intent.buy_order_id,
            executed_at_price: intent.execution_price,
            execution_amount: intent.settlement_amount,
            execution_fee: Self::calculate_taker_fee(intent.settlement_amount),
            matched_against: Some(intent.sell_order_id),
            timestamp,
        };

        let sell_execution = OrderExecution {
            order_id: intent.sell_order_id,
            executed_at_price: intent.execution_price,
            execution_amount: intent.settlement_amount,
            execution_fee: 0, // Maker rebate applied elsewhere
            matched_against: Some(intent.buy_order_id),
            timestamp,
        };

        (buy_execution, sell_execution)
    }

    /// Calculate taker fee (0.25%)
    const TAKER_FEE_BPS: u64 = 25;

    fn calculate_taker_fee(amount: u64) -> u64 {
        amount.saturating_mul(Self::TAKER_FEE_BPS) / 10_000
    }

    /// Create settlement intent from matched order execution
    ///
    /// This is the bridge function to be called after limit_order_book::match_orders succeeds.
    /// It takes the matched OrderExecution pair and creates an OrderSettlementIntent ready
    /// for on-chain submission to pallet-x3-settlement-engine.
    ///
    /// # Workflow
    /// 1. limit_order_book::LimitOrderBookEngine::match_orders() → (OrderExecution, OrderExecution)
    /// 2. settlement_bridge::create_settlement_intent_from_execution() → OrderSettlementIntent
    /// 3. Submit to chain via RPC/extrinsic → pallet_x3_settlement_engine::SettlementIntent
    ///
    /// # Arguments
    /// * `buy_execution` - OrderExecution for buy order (from match_orders)
    /// * `sell_execution` - OrderExecution for sell order (from match_orders)
    /// * `buy_order` - Original buy LimitOrder
    /// * `sell_order` - Original sell LimitOrder
    /// * `current_block` - Current block number for deadline calculation
    ///
    /// # Returns
    /// OrderSettlementIntent ready for on-chain submission
    pub fn create_settlement_intent_from_execution(
        buy_execution: &OrderExecution,
        sell_execution: &OrderExecution,
        buy_order: &LimitOrder,
        sell_order: &LimitOrder,
        current_block: u64,
    ) -> Result<OrderSettlementIntent, &'static str> {
        // Validate executions are matched
        if buy_execution.matched_against != Some(sell_execution.order_id) {
            return Err("Buy execution not matched to sell execution");
        }

        if sell_execution.matched_against != Some(buy_execution.order_id) {
            return Err("Sell execution not matched to buy execution");
        }

        // Use execution price and amount from matched orders
        let execution_price = buy_execution.executed_at_price;
        let execution_amount = buy_execution.execution_amount;

        // Validate amounts match
        if execution_amount != sell_execution.execution_amount {
            return Err("Execution amounts do not match");
        }

        // Generate deterministic intent ID
        let intent_id = Self::derive_intent_id(
            buy_execution.order_id,
            sell_execution.order_id,
            execution_price,
            execution_amount,
            current_block,
        );

        // Settlement deadline: 100 blocks (~10 minutes at 6s/block)
        const SETTLEMENT_TIMEOUT_BLOCKS: u64 = 100;

        Ok(OrderSettlementIntent {
            intent_id,
            buy_order_id: buy_execution.order_id,
            sell_order_id: sell_execution.order_id,
            execution_price,
            settlement_amount: execution_amount,
            buyer: buy_order.user,
            seller: sell_order.user,
            token_in: buy_order.token_in,
            token_out: buy_order.token_out,
            status: SettlementStatus::Pending,
            created_at: current_block,
            deadline: current_block + SETTLEMENT_TIMEOUT_BLOCKS,
        })
    }

    /// Convert OrderSettlementIntent to on-chain SettlementIntent format
    ///
    /// Maps off-chain DEX settlement intent to pallet-x3-settlement-engine format.
    /// This function provides the structure for RPC/extrinsic submission.
    ///
    /// Note: Actual on-chain submission requires runtime integration via extrinsic.
    /// This function documents the mapping for implementation in node/src/rpc.rs
    ///
    /// # On-Chain Mapping
    /// ```ignore
    /// OrderSettlementIntent → pallet_x3_settlement_engine::SettlementIntent {
    ///     intent_id: intent.intent_id,
    ///     maker: intent.seller (limit order maker receives rebate),
    ///     taker: intent.buyer (market order taker pays fee),
    ///     asset_a: AssetSpec { chain: X3, token: X3Asset(token_in), amount: settlement_amount },
    ///     asset_b: AssetSpec { chain: X3, token: X3Asset(token_out), amount: calculated_output },
    ///     secret_hash: H256::zero(), // Not used for DEX settlements
    ///     timeout: intent.deadline,
    ///     created_at: intent.created_at,
    ///     legs_total: 2, // Always 2 legs for simple swap
    ///     legs_locked: 0, // Will be set by settlement engine
    ///     legs_claimed: 0,
    /// }
    /// ```
    pub fn get_on_chain_mapping_doc() -> &'static str {
        "OrderSettlementIntent maps to pallet_x3_settlement_engine::SettlementIntent \
         via RPC extrinsic. See settlement_bridge.rs for detailed field mapping."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_buy_order() -> LimitOrder {
        LimitOrder {
            id: [1u8; 32],
            user: [10u8; 32],
            order_type: 0, // buy
            token_in: 100,
            token_out: 200,
            amount_in: 1_000_000,
            limit_price: 10_500, // Willing to pay up to 1.05x
            filled_amount: 0,
            status: 0,
            created_at: 1000,
            expires_at: 2000,
        }
    }

    fn create_test_sell_order() -> LimitOrder {
        LimitOrder {
            id: [2u8; 32],
            user: [20u8; 32],
            order_type: 1, // sell
            token_in: 200,
            token_out: 100,
            amount_in: 1_000_000,
            limit_price: 9_500, // Willing to accept 0.95x or more
            filled_amount: 0,
            status: 0,
            created_at: 1000,
            expires_at: 2000,
        }
    }

    #[test]
    fn test_create_settlement_intent_valid() {
        let buy_order = create_test_buy_order();
        let sell_order = create_test_sell_order();

        let result = LimitOrderSettlementBridge::create_settlement_intent(
            &buy_order,
            &sell_order,
            10_000,  // 1.0x price (between buy limit 1.05x and sell limit 0.95x)
            500_000, // Half the order size
            1500,
        );

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.buyer, buy_order.user);
        assert_eq!(intent.seller, sell_order.user);
        assert_eq!(intent.settlement_amount, 500_000);
        assert_eq!(intent.status, SettlementStatus::Pending);
    }

    #[test]
    fn test_create_settlement_intent_price_mismatch() {
        let buy_order = create_test_buy_order();
        let sell_order = create_test_sell_order();

        // Execution price too high for buy order
        let result = LimitOrderSettlementBridge::create_settlement_intent(
            &buy_order,
            &sell_order,
            11_000, // Above buy limit of 10_500
            500_000,
            1500,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Buy order limit price below execution price"
        );
    }

    #[test]
    fn test_can_finalize_intent_timeout() {
        let intent = OrderSettlementIntent {
            intent_id: H256::zero(),
            buy_order_id: [1u8; 32],
            sell_order_id: [2u8; 32],
            execution_price: 10_000,
            settlement_amount: 500_000,
            buyer: [10u8; 32],
            seller: [20u8; 32],
            token_in: 100,
            token_out: 200,
            status: SettlementStatus::Locked,
            created_at: 1000,
            deadline: 1100,
        };

        // Before deadline: can finalize
        assert_eq!(
            LimitOrderSettlementBridge::can_finalize_intent(&intent, 1050),
            Ok(true)
        );

        // After deadline: cannot finalize
        assert_eq!(
            LimitOrderSettlementBridge::can_finalize_intent(&intent, 1150),
            Ok(false)
        );
    }

    #[test]
    fn test_create_execution_record() {
        let intent = OrderSettlementIntent {
            intent_id: H256::zero(),
            buy_order_id: [1u8; 32],
            sell_order_id: [2u8; 32],
            execution_price: 10_000,
            settlement_amount: 1_000_000,
            buyer: [10u8; 32],
            seller: [20u8; 32],
            token_in: 100,
            token_out: 200,
            status: SettlementStatus::Finalized,
            created_at: 1000,
            deadline: 1100,
        };

        let (buy_exec, sell_exec) =
            LimitOrderSettlementBridge::create_execution_record(&intent, 1050);

        assert_eq!(buy_exec.order_id, intent.buy_order_id);
        assert_eq!(buy_exec.execution_amount, 1_000_000);
        assert_eq!(buy_exec.execution_fee, 2_500); // 0.25% of 1M = 2500

        assert_eq!(sell_exec.order_id, intent.sell_order_id);
        assert_eq!(sell_exec.execution_fee, 0); // Maker rebate
    }

    #[test]
    fn test_create_settlement_intent_from_execution() {
        let buy_order = create_test_buy_order();
        let sell_order = create_test_sell_order();

        // Create matched executions (simulating match_orders output)
        let buy_execution = OrderExecution {
            order_id: buy_order.id,
            executed_at_price: 10_000,
            execution_amount: 500_000,
            execution_fee: 1_250, // 0.25% taker fee
            matched_against: Some(sell_order.id),
            timestamp: 1500,
        };

        let sell_execution = OrderExecution {
            order_id: sell_order.id,
            executed_at_price: 10_000,
            execution_amount: 500_000,
            execution_fee: 0,
            matched_against: Some(buy_order.id),
            timestamp: 1500,
        };

        let result = LimitOrderSettlementBridge::create_settlement_intent_from_execution(
            &buy_execution,
            &sell_execution,
            &buy_order,
            &sell_order,
            1500,
        );

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.buyer, buy_order.user);
        assert_eq!(intent.seller, sell_order.user);
        assert_eq!(intent.settlement_amount, 500_000);
        assert_eq!(intent.execution_price, 10_000);
        assert_eq!(intent.status, SettlementStatus::Pending);
        assert_eq!(intent.deadline, 1500 + 100); // SETTLEMENT_TIMEOUT_BLOCKS
    }

    #[test]
    fn test_create_settlement_intent_from_execution_mismatch() {
        let buy_order = create_test_buy_order();
        let sell_order = create_test_sell_order();

        // Create unmatched executions
        let buy_execution = OrderExecution {
            order_id: buy_order.id,
            executed_at_price: 10_000,
            execution_amount: 500_000,
            execution_fee: 1_250,
            matched_against: None, // NOT MATCHED
            timestamp: 1500,
        };

        let sell_execution = OrderExecution {
            order_id: sell_order.id,
            executed_at_price: 10_000,
            execution_amount: 500_000,
            execution_fee: 0,
            matched_against: Some(buy_order.id),
            timestamp: 1500,
        };

        let result = LimitOrderSettlementBridge::create_settlement_intent_from_execution(
            &buy_execution,
            &sell_execution,
            &buy_order,
            &sell_order,
            1500,
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Buy execution not matched to sell execution"
        );
    }
}
