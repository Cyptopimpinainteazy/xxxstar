#![cfg_attr(not(feature = "std"), no_std)]
#![warn(unused_imports, unused_variables)]

//! X3 DEX Core
//!
//! Multi-hop routing, liquidity pools, AMM execution, and advanced trading features.

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod amm_pools;
pub mod arb_bot_events;
pub mod batch_swap_router;
pub mod concentrated_liquidity;
pub mod flash_loan;
pub mod limit_order_book;
pub mod liquidity_mining;
pub mod lp_position_nft;
pub mod options;
pub mod perpetuals;
pub mod pool_analytics;
pub mod real_slippage;
pub mod route_finder;
pub mod settlement_bridge; // NEW: Limit order → settlement engine integration
pub mod stop_loss_trigger;
pub mod trade_history;
pub mod twap_executor;
pub mod ve_governance;

#[cfg(test)]
#[path = "tests/attack_sandwich.rs"]
mod attack_sandwich;

#[cfg(test)]
#[path = "tests/attack_liquidation_frontrun.rs"]
mod attack_liquidation_frontrun;

#[cfg(test)]
#[path = "tests/attack_twap_manipulation.rs"]
mod attack_twap_manipulation;

#[cfg(test)]
#[path = "tests/attack_oracle_frontrun.rs"]
mod attack_oracle_frontrun;

pub use amm_pools::{AMMPool, LPPosition, LiquidityPool, SwapEvent, TokenId};
pub use arb_bot_events::{
    ArbBotEventSystem, ArbOpportunity, BotPerformance, BotSubscription, ExecutedArb,
};
pub use route_finder::{Path, PoolEdge, RouteFinder};

// TIER 3: Core Trading & Liquidity Features
pub use batch_swap_router::{
    AtomicSwapExecution, BatchSwap, BatchSwapRouter, MEVProtection, SwapRoute,
};
pub use concentrated_liquidity::{
    ConcentratedLiquidityEngine, ConcentratedPool, ConcentratedPosition, TickLiquidity,
};
pub use flash_loan::{
    ArbitrageExecution, FlashLoan, FlashLoanCallback, FlashLoanEngine, FlashLoanPool,
};
pub use limit_order_book::{LimitOrder, LimitOrderBookEngine, OrderBook, OrderExecution};
pub use liquidity_mining::{
    EpochRewards, LPRewardAccumulator, LiquidityMiningEngine, LiquidityMiningReward,
};
pub use lp_position_nft::{
    LPPositionNFT, LPPositionNFTEngine, NFTCollateral, NFTListing, NFTMetadata, NFTTransfer,
};
pub use options::{Option, OptionExercise, OptionGreeks, OptionQuote, OptionsEngine};
pub use perpetuals::{
    FundingRate, PerpetualFuturesEngine, PerpetualLiquidation, PerpetualMetrics, PerpetualPosition,
};
pub use pool_analytics::{
    LiquidityProviderStats, PoolAnalytics, PoolAnalyticsEngine, PoolMetrics, TokenMetrics,
};
pub use real_slippage::{
    PoolReserves, PriceImpact, RealSlippageCalculator, SlippageProtection, SlippageQuote,
};
pub use settlement_bridge::{LimitOrderSettlementBridge, OrderSettlementIntent, SettlementStatus};
pub use stop_loss_trigger::{
    GridTradingConfig, StopLossTakeProfitEngine, StopLossTrigger, TakeProfitTrigger,
    TrailingStopTrigger,
};
pub use trade_history::{
    CostBasisEntry, PerformanceMetrics, TaxReport, TradeHistoryEngine, TradeRecord,
};
pub use twap_executor::{TWAPExecutor, TWAPOrder, TWAPSliceExecution, TWAPStatistics};
pub use ve_governance::{
    LiquidityMiningAllocation, Proposal, VeX3GovernanceEngine, VeX3Lock, Vote,
};
