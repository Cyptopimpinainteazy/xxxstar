/// Limit Order Book — Offchain order book with on-chain settlement
/// Enables users to place buy/sell limit orders that execute when price reaches target
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct LimitOrder {
    pub id: [u8; 32],
    pub user: [u8; 32],
    pub order_type: u8, // 0=buy, 1=sell
    pub token_in: u128,
    pub token_out: u128,
    pub amount_in: u64,
    pub limit_price: u64, // Price in basis points (1 bp = 0.01%)
    pub filled_amount: u64,
    pub status: u8, // 0=pending, 1=partially_filled, 2=filled, 3=cancelled
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OrderBook {
    pub token_pair_id: [u8; 32],
    pub buy_orders: Vec<[u8; 32]>,  // Order IDs sorted by price desc
    pub sell_orders: Vec<[u8; 32]>, // Order IDs sorted by price asc
    pub total_buy_volume: u64,
    pub total_sell_volume: u64,
    pub last_trade_price: u64,
    pub last_update_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OrderExecution {
    pub order_id: [u8; 32],
    pub executed_at_price: u64,
    pub execution_amount: u64,
    pub execution_fee: u64,
    pub matched_against: Option<[u8; 32]>, // Counter-order ID if matched
    pub timestamp: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct OrderBookStats {
    pub total_orders: u32,
    pub pending_orders: u32,
    pub filled_orders: u32,
    pub cancelled_orders: u32,
    pub total_volume_24h: u64,
    pub avg_execution_price: u64,
}

type DepthLevel = (u64, u64);
type OrderBookDepth = (Vec<DepthLevel>, Vec<DepthLevel>);

pub struct LimitOrderBookEngine;

impl LimitOrderBookEngine {
    const MIN_ORDER_AMOUNT: u64 = 1;
    const MAX_ORDER_AMOUNT: u64 = 1_000_000_000_000; // 1 trillion base units
    const ORDER_EXPIRY_BLOCKS: u64 = 86_400; // ~1 day at 1 block/sec
    const TAKER_FEE_BPS: u64 = 25; // 0.25%
    const MAKER_REBATE_BPS: u64 = 10; // 0.1% rebate

    /// Create a new limit order
    pub fn create_limit_order(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        amount_in: u64,
        limit_price: u64,
        is_buy: bool,
        current_block: u64,
    ) -> Result<LimitOrder, &'static str> {
        if !(Self::MIN_ORDER_AMOUNT..=Self::MAX_ORDER_AMOUNT).contains(&amount_in) {
            return Err("Order amount out of range");
        }

        if limit_price == 0 {
            return Err("Limit price cannot be zero");
        }

        let order = LimitOrder {
            id: Self::derive_order_id(user, token_in, token_out, amount_in, current_block),
            user,
            order_type: if is_buy { 0 } else { 1 },
            token_in,
            token_out,
            amount_in,
            limit_price,
            filled_amount: 0,
            status: 0, // pending
            created_at: current_block,
            expires_at: current_block + Self::ORDER_EXPIRY_BLOCKS,
        };

        Ok(order)
    }

    /// Cancel an existing limit order
    pub fn cancel_order(order: &mut LimitOrder, _current_block: u64) -> Result<bool, &'static str> {
        if order.status == 3 {
            return Err("Order already cancelled");
        }

        if order.status == 2 {
            return Err("Cannot cancel fully filled order");
        }

        order.status = 3; // cancelled
        Ok(true)
    }

    /// Execute a limit order (match against current market or another order)
    pub fn execute_order(
        order: &mut LimitOrder,
        execution_price: u64,
        execution_amount: u64,
        timestamp: u64,
    ) -> Result<OrderExecution, &'static str> {
        if order.status == 2 {
            return Err("Order already fully filled");
        }

        if order.status == 3 {
            return Err("Order is cancelled");
        }

        // Validate price meets limit
        if order.order_type == 0 && execution_price > order.limit_price {
            // Buy order: price must be <= limit
            return Err("Execution price exceeds buy limit");
        }

        if order.order_type == 1 && execution_price < order.limit_price {
            // Sell order: price must be >= limit
            return Err("Execution price below sell limit");
        }

        let execution_fee = Self::calculate_execution_fee(execution_amount);
        let remaining = order.amount_in - order.filled_amount;

        if execution_amount > remaining {
            return Err("Execution amount exceeds remaining");
        }

        order.filled_amount += execution_amount;
        if order.filled_amount == order.amount_in {
            order.status = 2; // fully filled
        } else {
            order.status = 1; // partially filled
        }

        let execution = OrderExecution {
            order_id: order.id,
            executed_at_price: execution_price,
            execution_amount,
            execution_fee,
            matched_against: None,
            timestamp,
        };

        Ok(execution)
    }

    /// Match buy and sell orders (offchain matching, on-chain settlement)
    pub fn match_orders(
        buy_order: &mut LimitOrder,
        sell_order: &mut LimitOrder,
        current_price: u64,
        timestamp: u64,
    ) -> Result<(OrderExecution, OrderExecution), &'static str> {
        if buy_order.order_type != 0 {
            return Err("First order must be a buy order");
        }

        if sell_order.order_type != 1 {
            return Err("Second order must be a sell order");
        }

        if buy_order.token_in != sell_order.token_out {
            return Err("Token mismatch between orders");
        }

        // Validate price overlap
        if current_price > buy_order.limit_price || current_price < sell_order.limit_price {
            return Err("Orders do not overlap at current price");
        }

        let match_amount = sp_std::cmp::min(
            buy_order.amount_in - buy_order.filled_amount,
            sell_order.amount_in - sell_order.filled_amount,
        );

        let mut buy_exec = Self::execute_order(buy_order, current_price, match_amount, timestamp)?;
        let mut sell_exec =
            Self::execute_order(sell_order, current_price, match_amount, timestamp)?;

        buy_exec.matched_against = Some(sell_order.id);
        sell_exec.matched_against = Some(buy_order.id);

        Ok((buy_exec, sell_exec))
    }

    /// Get best bid/ask prices from order book
    pub fn get_best_prices(book: &OrderBook) -> Result<(u64, u64), &'static str> {
        if book.buy_orders.is_empty() || book.sell_orders.is_empty() {
            return Err("Order book has no orders");
        }

        let best_bid = book.buy_orders.first().copied().ok_or("No buy orders")?;
        let best_ask = book.sell_orders.first().copied().ok_or("No sell orders")?;

        let bid_id = best_bid;
        let ask_id = best_ask;

        Ok((
            u64::from_le_bytes(bid_id[..8].try_into().unwrap()),
            u64::from_le_bytes(ask_id[..8].try_into().unwrap()),
        ))
    }

    /// Calculate execution fee (taker fee - maker gets rebate)
    pub fn calculate_execution_fee(amount: u64) -> u64 {
        (amount * Self::TAKER_FEE_BPS) / 10_000
    }

    /// Calculate maker rebate
    pub fn calculate_maker_rebate(amount: u64) -> u64 {
        (amount * Self::MAKER_REBATE_BPS) / 10_000
    }

    /// Check if order has expired
    pub fn is_order_expired(order: &LimitOrder, current_block: u64) -> bool {
        current_block > order.expires_at
    }

    /// Expire stale orders
    pub fn expire_order(order: &mut LimitOrder) -> Result<bool, &'static str> {
        if order.status == 3 || order.status == 2 {
            return Err("Cannot expire cancelled or filled order");
        }

        order.status = 3; // cancelled due to expiry
        Ok(true)
    }

    /// Get order book depth (aggregated volume at price levels)
    pub fn get_order_book_depth(
        buy_orders: &[LimitOrder],
        sell_orders: &[LimitOrder],
        levels: u32,
    ) -> Result<OrderBookDepth, &'static str> {
        if levels == 0 {
            return Err("Depth levels must be > 0");
        }

        let mut bid_depth = Vec::new();
        let mut ask_depth = Vec::new();

        for (i, order) in buy_orders.iter().enumerate() {
            if i >= levels as usize {
                break;
            }
            if order.status == 0 || order.status == 1 {
                let remaining = order.amount_in - order.filled_amount;
                bid_depth.push((order.limit_price, remaining));
            }
        }

        for (i, order) in sell_orders.iter().enumerate() {
            if i >= levels as usize {
                break;
            }
            if order.status == 0 || order.status == 1 {
                let remaining = order.amount_in - order.filled_amount;
                ask_depth.push((order.limit_price, remaining));
            }
        }

        Ok((bid_depth, ask_depth))
    }

    /// Calculate weighted average executed price
    pub fn calculate_vwap(executions: &[OrderExecution]) -> Result<u64, &'static str> {
        if executions.is_empty() {
            return Err("No executions to calculate VWAP");
        }

        let mut total_volume: u64 = 0;
        let mut total_price_volume: u128 = 0;

        for exec in executions {
            total_volume = total_volume.saturating_add(exec.execution_amount);
            total_price_volume = total_price_volume.saturating_add(
                (exec.executed_at_price as u128).saturating_mul(exec.execution_amount as u128),
            );
        }

        if total_volume == 0 {
            return Err("Total volume is zero");
        }

        let vwap = (total_price_volume / total_volume as u128) as u64;
        Ok(vwap)
    }

    /// User's total open order volume
    pub fn get_user_open_volume(user: [u8; 32], orders: &[LimitOrder]) -> u64 {
        orders
            .iter()
            .filter(|o| o.user == user && (o.status == 0 || o.status == 1))
            .map(|o| o.amount_in - o.filled_amount)
            .fold(0u64, |acc, v| acc.saturating_add(v))
    }

    /// Derive deterministic order ID
    fn derive_order_id(
        user: [u8; 32],
        token_in: u128,
        _token_out: u128,
        amount: u64,
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate() {
            id[i] = *byte;
        }
        let token_in_bytes = token_in.to_le_bytes();
        for (i, byte) in token_in_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        let amount_bytes = amount.to_le_bytes();
        for (i, byte) in amount_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        let nonce_bytes = nonce.to_le_bytes();
        for (i, byte) in nonce_bytes.iter().enumerate().take(8) {
            id[i + 24] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_limit_order_buy() {
        let user = [1; 32];
        let order =
            LimitOrderBookEngine::create_limit_order(user, 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        assert_eq!(order.user, user);
        assert_eq!(order.order_type, 0); // buy
        assert_eq!(order.status, 0); // pending
    }

    #[test]
    fn test_create_limit_order_sell() {
        let user = [2; 32];
        let order =
            LimitOrderBookEngine::create_limit_order(user, 2, 1, 500_000, 4_500, false, 100)
                .unwrap();

        assert_eq!(order.user, user);
        assert_eq!(order.order_type, 1); // sell
    }

    #[test]
    fn test_create_order_invalid_amount_zero() {
        let result = LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 0, 5_000, true, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_order() {
        let mut order =
            LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        LimitOrderBookEngine::cancel_order(&mut order, 100).unwrap();
        assert_eq!(order.status, 3); // cancelled
    }

    #[test]
    fn test_execute_order() {
        let mut order =
            LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        let exec = LimitOrderBookEngine::execute_order(
            &mut order, 4_500, // price within limit
            500_000, 100,
        )
        .unwrap();

        assert_eq!(order.filled_amount, 500_000);
        assert_eq!(order.status, 1); // partially filled
        assert_eq!(exec.execution_amount, 500_000);
    }

    #[test]
    fn test_execute_order_price_violation() {
        let mut order =
            LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        let result = LimitOrderBookEngine::execute_order(
            &mut order, 5_500, // price exceeds buy limit
            500_000, 100,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_execute_order_full_fill() {
        let mut order =
            LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        LimitOrderBookEngine::execute_order(&mut order, 4_800, 1_000_000, 100).unwrap();

        assert_eq!(order.filled_amount, 1_000_000);
        assert_eq!(order.status, 2); // fully filled
    }

    #[test]
    fn test_is_order_expired() {
        let order =
            LimitOrderBookEngine::create_limit_order([1; 32], 1, 2, 1_000_000, 5_000, true, 100)
                .unwrap();

        assert!(!LimitOrderBookEngine::is_order_expired(&order, 1000));
        assert!(LimitOrderBookEngine::is_order_expired(&order, 200_000));
    }

    #[test]
    fn test_calculate_execution_fee() {
        let fee = LimitOrderBookEngine::calculate_execution_fee(1_000_000);
        assert_eq!(fee, 2_500); // 0.25% of 1M
    }

    #[test]
    fn test_calculate_maker_rebate() {
        let rebate = LimitOrderBookEngine::calculate_maker_rebate(1_000_000);
        assert_eq!(rebate, 1_000); // 0.1% of 1M
    }

    #[test]
    fn test_get_user_open_volume() {
        let user = [1; 32];
        let orders = vec![
            LimitOrder {
                id: [0; 32],
                user,
                order_type: 0,
                token_in: 1,
                token_out: 2,
                amount_in: 1_000_000,
                limit_price: 5_000,
                filled_amount: 250_000,
                status: 1, // partially filled
                created_at: 0,
                expires_at: 1000,
            },
            LimitOrder {
                id: [1; 32],
                user,
                order_type: 1,
                token_in: 2,
                token_out: 1,
                amount_in: 500_000,
                limit_price: 4_800,
                filled_amount: 0,
                status: 0, // pending
                created_at: 0,
                expires_at: 1000,
            },
        ];

        let volume = LimitOrderBookEngine::get_user_open_volume(user, &orders);
        assert_eq!(volume, 750_000 + 500_000); // remaining from both orders
    }

    #[test]
    fn test_calculate_vwap() {
        let executions = vec![
            OrderExecution {
                order_id: [0; 32],
                executed_at_price: 5_000,
                execution_amount: 100_000,
                execution_fee: 250,
                matched_against: None,
                timestamp: 100,
            },
            OrderExecution {
                order_id: [1; 32],
                executed_at_price: 5_100,
                execution_amount: 150_000,
                execution_fee: 375,
                matched_against: None,
                timestamp: 101,
            },
        ];

        let vwap = LimitOrderBookEngine::calculate_vwap(&executions).unwrap();
        // (5000 * 100k + 5100 * 150k) / 250k = (500M + 765M) / 250k = 1265M / 250k = 5060
        assert_eq!(vwap, 5_060);
    }

    #[test]
    fn test_calculate_vwap_empty() {
        let result = LimitOrderBookEngine::calculate_vwap(&[]);
        assert!(result.is_err());
    }
}
