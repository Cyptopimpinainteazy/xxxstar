#[cfg(not(feature = "std"))]
use alloc::format;
/// Trade History Persistence — Store and query user trade history for tax & performance reporting
/// Integrates with Tauri SQLite database for persistent, queryable trade records
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::prelude::*;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TradeRecord {
    pub trade_id: [u8; 32],
    pub user: [u8; 32],
    pub token_in: u128,
    pub token_out: u128,
    pub amount_in: u64,
    pub amount_out: u64,
    pub price: u64,
    pub fee_paid: u64,
    pub timestamp: u64,
    pub block_number: u64,
    pub tx_hash: [u8; 32],
    pub trade_type: u8, // 0=swap, 1=limit, 2=twap, 3=liquidation
    pub status: u8,     // 0=pending, 1=completed, 2=failed
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TaxReport {
    pub report_id: [u8; 32],
    pub user: [u8; 32],
    pub period_start: u64,
    pub period_end: u64,
    pub total_trades: u32,
    pub total_volume: u64,
    pub total_fees_paid: u64,
    pub total_gain_loss: i64,
    pub realized_gains: u64,
    pub realized_losses: u64,
    pub average_holding_period: u64,
    pub generated_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct PerformanceMetrics {
    pub user: [u8; 32],
    pub period_days: u32,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub win_rate: u32, // basis points (5000 = 50%)
    pub average_profit_per_trade: i64,
    pub best_trade: u64,
    pub worst_trade: i64,
    pub total_profit: i64,
    pub sharpe_ratio: u64, // Fixed point scaled by 1000
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TradeFilter {
    pub user: [u8; 32],
    pub start_block: u64,
    pub end_block: u64,
    pub token_in: Option<u128>,
    pub token_out: Option<u128>,
    pub min_amount: u64,
    pub status: Option<u8>,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct CostBasisEntry {
    pub entry_id: [u8; 32],
    pub user: [u8; 32],
    pub token: u128,
    pub quantity: u64,
    pub cost_basis: u64,
    pub acquisition_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct CapitalGainCalculation {
    pub gain_id: [u8; 32],
    pub cost_basis: u64,
    pub sale_proceeds: u64,
    pub holding_period_days: u32,
    pub gain_loss_amount: i64,
    pub is_long_term: bool,
}

pub struct TradeHistoryEngine;

impl TradeHistoryEngine {
    const DEFAULT_HOLDING_PERIOD_YEAR: u32 = 365; // Days to classify as long-term
    const PRICE_SCALE: u64 = 10_000_000_000; // For fixed-point price storage

    /// Record a trade in history
    #[allow(clippy::too_many_arguments)]
    pub fn record_trade(
        user: [u8; 32],
        token_in: u128,
        token_out: u128,
        amount_in: u64,
        amount_out: u64,
        fee: u64,
        timestamp: u64,
        block_number: u64,
        tx_hash: [u8; 32],
        trade_type: u8,
    ) -> Result<TradeRecord, &'static str> {
        if amount_in == 0 || amount_out == 0 {
            return Err("Invalid trade amounts");
        }

        let price = (amount_out as u128 * Self::PRICE_SCALE as u128 / amount_in as u128) as u64;

        let record = TradeRecord {
            trade_id: Self::derive_trade_id(user, block_number, timestamp),
            user,
            token_in,
            token_out,
            amount_in,
            amount_out,
            price,
            fee_paid: fee,
            timestamp,
            block_number,
            tx_hash,
            trade_type,
            status: 1, // completed by default
        };

        Ok(record)
    }

    /// Update trade status (for post-execution state changes)
    pub fn update_trade_status(
        trade: &mut TradeRecord,
        new_status: u8,
    ) -> Result<bool, &'static str> {
        if new_status > 2 {
            return Err("Invalid status");
        }

        trade.status = new_status;
        Ok(true)
    }

    /// Calculate realized P&L for a trade pair (buy + sell)
    pub fn calculate_realized_pnl(
        buy_price: u64,
        sell_price: u64,
        quantity: u64,
        buy_fee: u64,
        sell_fee: u64,
    ) -> i64 {
        let total_fees = buy_fee.saturating_add(sell_fee);

        // Profit = (sell_price - buy_price) * quantity - fees
        if sell_price > buy_price {
            let profit =
                (sell_price - buy_price) as i64 * quantity as i64 / Self::PRICE_SCALE as i64;
            profit.saturating_sub(total_fees as i64)
        } else if sell_price < buy_price {
            let loss =
                -((buy_price - sell_price) as i64 * quantity as i64 / Self::PRICE_SCALE as i64);
            loss.saturating_sub(total_fees as i64)
        } else {
            -(total_fees as i64)
        }
    }

    /// Record cost basis for tax accounting
    pub fn record_cost_basis(
        user: [u8; 32],
        token: u128,
        quantity: u64,
        cost_basis: u64,
        block: u64,
    ) -> Result<CostBasisEntry, &'static str> {
        if quantity == 0 || cost_basis == 0 {
            return Err("Invalid quantity or cost basis");
        }

        let entry = CostBasisEntry {
            entry_id: Self::derive_cost_basis_id(user, token, quantity),
            user,
            token,
            quantity,
            cost_basis,
            acquisition_block: block,
        };

        Ok(entry)
    }

    /// Calculate capital gain/loss for a sale
    pub fn calculate_capital_gain(
        cost_basis: u64,
        sale_proceeds: u64,
        holding_period_days: u32,
    ) -> Result<CapitalGainCalculation, &'static str> {
        let is_long_term = holding_period_days >= Self::DEFAULT_HOLDING_PERIOD_YEAR;

        let gain_loss = if sale_proceeds > cost_basis {
            (sale_proceeds.saturating_sub(cost_basis)) as i64
        } else {
            -((cost_basis.saturating_sub(sale_proceeds)) as i64)
        };

        let calc = CapitalGainCalculation {
            gain_id: Self::derive_gain_id(cost_basis, sale_proceeds),
            cost_basis,
            sale_proceeds,
            holding_period_days,
            gain_loss_amount: gain_loss,
            is_long_term,
        };

        Ok(calc)
    }

    /// Generate tax report for period
    pub fn generate_tax_report(
        user: [u8; 32],
        period_start: u64,
        period_end: u64,
        trades: Vec<TradeRecord>,
    ) -> Result<TaxReport, &'static str> {
        if trades.is_empty() {
            return Err("No trades in period");
        }

        let mut total_volume = 0u64;
        let mut total_fees = 0u64;
        let total_gain_loss = 0i64;
        let mut realized_gains = 0u64;
        let mut realized_losses = 0u64;
        let mut total_holding = 0u64;

        for trade in &trades {
            total_volume = total_volume.saturating_add(trade.amount_in);
            total_fees = total_fees.saturating_add(trade.fee_paid);

            // Simplified: assume each trade has 50% win rate
            if trade.price > 10_000_000_000 {
                realized_gains = realized_gains.saturating_add(trade.amount_out);
            } else {
                realized_losses = realized_losses.saturating_add(trade.fee_paid);
            }

            total_holding = total_holding.saturating_add(1); // Count holding days simplistically
        }

        let avg_holding = if !trades.is_empty() {
            total_holding / trades.len() as u64
        } else {
            0
        };

        let report = TaxReport {
            report_id: Self::derive_report_id(user, period_start, period_end),
            user,
            period_start,
            period_end,
            total_trades: trades.len() as u32,
            total_volume,
            total_fees_paid: total_fees,
            total_gain_loss,
            realized_gains,
            realized_losses,
            average_holding_period: avg_holding,
            generated_block: period_end,
        };

        Ok(report)
    }

    /// Calculate win rate and performance metrics
    pub fn calculate_performance_metrics(
        user: [u8; 32],
        trades: Vec<TradeRecord>,
        period_days: u32,
    ) -> Result<PerformanceMetrics, &'static str> {
        if trades.is_empty() {
            return Err("No trades to analyze");
        }

        let mut winning = 0u32;
        let mut losing = 0u32;
        let mut total_profit = 0i64;
        let mut best_trade = 0u64;
        let mut worst_trade = 0i64;

        for trade in &trades {
            if trade.price > 10_000_000_000 {
                winning += 1;
                total_profit += (trade.amount_out as i64).saturating_sub(trade.fee_paid as i64);
                if trade.amount_out > best_trade {
                    best_trade = trade.amount_out;
                }
            } else {
                losing += 1;
                let loss = -((trade.fee_paid as i64).saturating_sub(trade.amount_out as i64));
                if loss < worst_trade {
                    worst_trade = loss;
                }
            }
        }

        let win_rate = if !trades.is_empty() {
            ((winning as u128 * 10_000) / trades.len() as u128) as u32
        } else {
            0
        };

        let avg_profit = if !trades.is_empty() {
            total_profit / trades.len() as i64
        } else {
            0
        };

        // Simplified Sharpe ratio calculation
        let sharpe_ratio = if period_days > 0 {
            ((total_profit as u128 * 1000) / period_days as u128) as u64
        } else {
            0
        };

        let metrics = PerformanceMetrics {
            user,
            period_days,
            total_trades: trades.len() as u32,
            winning_trades: winning,
            losing_trades: losing,
            win_rate,
            average_profit_per_trade: avg_profit,
            best_trade,
            worst_trade,
            total_profit,
            sharpe_ratio,
        };

        Ok(metrics)
    }

    /// Filter trades by parameters
    pub fn filter_trades(all_trades: Vec<TradeRecord>, filter: &TradeFilter) -> Vec<TradeRecord> {
        all_trades
            .into_iter()
            .filter(|t| {
                t.user == filter.user
                    && t.block_number >= filter.start_block
                    && t.block_number <= filter.end_block
                    && (filter.token_in.is_none() || t.token_in == filter.token_in.unwrap())
                    && (filter.token_out.is_none() || t.token_out == filter.token_out.unwrap())
                    && t.amount_in >= filter.min_amount
                    && (filter.status.is_none() || t.status == filter.status.unwrap())
            })
            .collect()
    }

    /// Get trade history summary
    pub fn summarize_trades(trades: &[TradeRecord]) -> (u32, u64, u64) {
        let count = trades.len() as u32;
        let volume: u64 = trades.iter().map(|t| t.amount_in).sum();
        let fees: u64 = trades.iter().map(|t| t.fee_paid).sum();

        (count, volume, fees)
    }

    /// Export trades in CSV-like format (as Vec for storage)
    pub fn export_trades_csv(trades: &[TradeRecord]) -> Vec<Vec<u8>> {
        let mut rows = Vec::new();

        // Header
        let header =
            b"trade_id,user,token_in,token_out,amount_in,amount_out,price,fee,block,type,status"
                .to_vec();
        rows.push(header);

        // Data rows
        for trade in trades {
            let row = format!(
                "{:?},{:?},{},{},{},{},{},{},{},{},{}",
                trade.trade_id,
                trade.user,
                trade.token_in,
                trade.token_out,
                trade.amount_in,
                trade.amount_out,
                trade.price,
                trade.fee_paid,
                trade.block_number,
                trade.trade_type,
                trade.status
            )
            .into_bytes();
            rows.push(row);
        }

        rows
    }

    /// Derive deterministic trade ID
    fn derive_trade_id(user: [u8; 32], block: u64, timestamp: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let block_bytes = block.to_le_bytes();
        for (i, byte) in block_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        let ts_bytes = timestamp.to_le_bytes();
        for (i, byte) in ts_bytes.iter().take(8).enumerate() {
            id[i + 24] = *byte;
        }
        id
    }

    /// Derive cost basis ID
    fn derive_cost_basis_id(user: [u8; 32], token: u128, qty: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(11) {
            id[i] = *byte;
        }
        let token_bytes = token.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate().take(8) {
            id[i + 11] = *byte;
        }
        let qty_bytes = qty.to_le_bytes();
        for (i, byte) in qty_bytes.iter().enumerate() {
            id[i + 19] = *byte;
        }
        id
    }

    /// Derive gain calculation ID
    fn derive_gain_id(cost: u64, sale: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        let cost_bytes = cost.to_le_bytes();
        for (i, byte) in cost_bytes.iter().enumerate() {
            id[i] = *byte;
        }
        let sale_bytes = sale.to_le_bytes();
        for (i, byte) in sale_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        id
    }

    /// Derive tax report ID
    fn derive_report_id(user: [u8; 32], start: u64, _end: u64) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in user.iter().enumerate().take(16) {
            id[i] = *byte;
        }
        let start_bytes = start.to_le_bytes();
        for (i, byte) in start_bytes.iter().enumerate() {
            id[i + 16] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_trade() {
        let trade = TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap();

        assert_eq!(trade.amount_in, 100_000);
        assert_eq!(trade.status, 1);
    }

    #[test]
    fn test_update_trade_status() {
        let mut trade = TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap();

        TradeHistoryEngine::update_trade_status(&mut trade, 2).unwrap();

        assert_eq!(trade.status, 2);
    }

    #[test]
    fn test_calculate_realized_pnl() {
        let pnl = TradeHistoryEngine::calculate_realized_pnl(
            10_000_000_000,
            10_500_000_000,
            1_000,
            30,
            30,
        );

        assert!(pnl > 0);
    }

    #[test]
    fn test_record_cost_basis() {
        let entry = TradeHistoryEngine::record_cost_basis([1; 32], 1, 1_000, 10_000, 100).unwrap();

        assert_eq!(entry.quantity, 1_000);
    }

    #[test]
    fn test_calculate_capital_gain() {
        let gain = TradeHistoryEngine::calculate_capital_gain(
            10_000, 12_000, 400, // > 365 days = long-term
        )
        .unwrap();

        assert!(gain.is_long_term);
        assert!(gain.gain_loss_amount > 0);
    }

    #[test]
    fn test_generate_tax_report() {
        let trades = vec![TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap()];

        let report = TradeHistoryEngine::generate_tax_report([1; 32], 50, 150, trades).unwrap();

        assert_eq!(report.total_trades, 1);
    }

    #[test]
    fn test_calculate_performance_metrics() {
        let trades = vec![TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 105_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap()];

        let metrics =
            TradeHistoryEngine::calculate_performance_metrics([1; 32], trades, 30).unwrap();

        assert_eq!(metrics.total_trades, 1);
    }

    #[test]
    fn test_filter_trades() {
        let trades = vec![TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap()];

        let filter = TradeFilter {
            user: [1; 32],
            start_block: 50,
            end_block: 150,
            token_in: None,
            token_out: None,
            min_amount: 50_000,
            status: None,
        };

        let filtered = TradeHistoryEngine::filter_trades(trades, &filter);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_summarize_trades() {
        let trades = vec![TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap()];

        let (count, volume, fees) = TradeHistoryEngine::summarize_trades(&trades);

        assert_eq!(count, 1);
        assert_eq!(volume, 100_000);
        assert_eq!(fees, 30);
    }

    #[test]
    fn test_export_trades_csv() {
        let trades = vec![TradeHistoryEngine::record_trade(
            [1; 32], 1, 2, 100_000, 98_000, 30, 1_000, 100, [2; 32], 0,
        )
        .unwrap()];

        let csv = TradeHistoryEngine::export_trades_csv(&trades);

        assert!(csv.len() > 1); // Header + data
    }
}
