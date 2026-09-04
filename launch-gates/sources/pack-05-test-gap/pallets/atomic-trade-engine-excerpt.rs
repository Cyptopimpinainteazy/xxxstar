#![deny(unsafe_code)]
//! # Atomic Trade Engine Pallet
//!
//! ## Overview
//!
//! The Atomic Trade Engine enables atomic arbitrage and multi-hop trades across EVM and SVM
//! state machines inside X3 Chain. It provides:
//!
//! - **Cross-VM Call Batching**: Execute multiple VM operations atomically
//! - **Failure Atomicity**: All-or-nothing execution with automatic rollback
//! - **State Checkpointing**: Intermediate state snapshots for complex multi-leg trades
//! - **Trade Graph Resolution**: Deterministic pathfinding for multi-hop swaps
//! - **AMM Integration**: Unified interface for Uniswap, Raydium, Orca, and custom DEXes
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                     Atomic Trade Engine                            │
//! ├────────────────────────────────────────────────────────────────────┤
//! │  TradeBatch                                                        │
//! │  ├── TradeLegs (Vec<TradeLeg>)                                    │
//! │  ├── Checkpoints (Vec<StateCheckpoint>)                           │
//! │  └── ExecutionPlan (Vec<ExecutionStep>)                           │
//! ├────────────────────────────────────────────────────────────────────┤
//! │  TradeGraphResolver                                                │
//! │  ├── find_optimal_path()                                          │
//! │  ├── calculate_expected_output()                                  │
//! │  └── detect_arbitrage_opportunity()                               │
//! ├────────────────────────────────────────────────────────────────────┤
//! │  AMM Adapters                                                      │
//! │  ├── UniswapV2Adapter                                             │
//! │  ├── UniswapV3Adapter                                             │
//! │  ├── RaydiumAdapter                                               │
//! │  └── OrcaWhirlpoolAdapter                                         │
//! └────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                       X3 Kernel                                 │
//! │  ├── EVM Adapter                                                  │
//! │  └── SVM Adapter                                                  │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
// FRAME pallet macros generate functions with many parameters; suppress the lint.
#![allow(clippy::too_many_arguments)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod amm;
pub mod graph;
pub mod runtime_api;
pub mod types;
pub mod weights;

pub use runtime_api::*;
pub use types::{AmmProtocol, VmType};
pub use weights::WeightInfo;

use codec::{Decode, Encode};
use frame_support::{
    pallet_prelude::*,
    traits::{Currency, UnixTime},
};
use frame_system::pallet_prelude::*;
use pallet_x3_kernel::{EvmExecutorAdapter, SvmExecutorAdapter, X3ExecutorAdapter};
use scale_info::TypeInfo;
use sp_core::{H256, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{DispatchError, RuntimeDebug, SaturatedConversion};
use sp_std::prelude::*;

/// Bridge to the settlement engine.
///
/// The trade engine calls these methods after a batch completes
/// successfully.  The runtime wires this to `pallet_x3_settlement`
/// in production and a no-op adapter in tests.
pub trait SettlementBridge<AccountId> {
    /// Register a completed batch with the settlement layer.
    ///
    /// Implementations should create a settlement intent and lock
    /// escrow for each leg.  `secret_hash` is blake2-256 of
    /// `(batch_id, block_number)` — callers generate it.
    fn register_completed_batch(
        maker: &AccountId,
        batch_id: H256,
        secret_hash: H256,
        legs: &[(VmType, u128)], // (vm_type, amount_out) per leg
    ) -> Result<H256, DispatchError>; // returns intent_id

    /// Release / refund escrow for a failed or rolled-back batch.
    fn release_batch(_batch_id: H256) -> Result<(), DispatchError> {
        Ok(())
    }
}

/// No-op settlement bridge — used in tests or when settlement is disabled.
pub struct NoOpSettlementBridge;
impl<AccountId> SettlementBridge<AccountId> for NoOpSettlementBridge {
    fn register_completed_batch(
        _maker: &AccountId,
        _batch_id: H256,
        _secret_hash: H256,
        _legs: &[(VmType, u128)],
    ) -> Result<H256, DispatchError> {
        Ok(H256::zero())
    }
}

/// Maximum number of legs in a single trade batch
pub const MAX_TRADE_LEGS: u32 = 16;

/// Maximum number of checkpoints per trade batch
pub const MAX_CHECKPOINTS: u32 = 8;

/// Maximum slippage tolerance in basis points (100 = 1%)
pub const MAX_SLIPPAGE_BPS: u32 = 5000; // 50%

/// Minimum slippage tolerance in basis points
pub const MIN_SLIPPAGE_BPS: u32 = 1; // 0.01%

/// Maximum length for addresses (enough for EVM 20 bytes or Solana 32 bytes)
pub const MAX_ADDRESS_LEN: u32 = 64;

/// Maximum length for route data in a trade leg
pub const MAX_ROUTE_DATA_LEN: u32 = 256;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    use frame_support::storage::{with_transaction, TransactionOutcome};
    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_timestamp::Config + pallet_x3_kernel::Config
    {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;

        /// The currency type for fee handling.
        type Currency: Currency<<Self as frame_system::Config>::AccountId>;

        /// EVM execution adapter (from x3-kernel or custom).
        type EvmAdapter: EvmExecutorAdapter;

        /// SVM execution adapter (from x3-kernel or custom).
        type SvmAdapter: SvmExecutorAdapter;

        /// X3 execution adapter (from x3-kernel or custom).
        type X3Adapter: X3ExecutorAdapter;

        /// Maximum number of trade legs per batch.
        #[pallet::constant]
        type MaxTradeLegs: Get<u32>;

        /// Maximum number of checkpoints per trade batch.
        #[pallet::constant]
        type MaxCheckpoints: Get<u32>;

        /// Maximum pending batches per account.
        #[pallet::constant]
        type MaxPendingBatchesPerAccount: Get<u32>;

        /// Default gas limit for EVM trade operations.
        #[pallet::constant]
        type DefaultTradeEvmGasLimit: Get<u64>;

        /// Default compute limit for SVM trade operations.
        #[pallet::constant]
        type DefaultTradeSvmComputeLimit: Get<u64>;

        /// Default gas limit for X3 trade operations.
        #[pallet::constant]
        type DefaultTradeX3GasLimit: Get<u64>;

        /// Origin allowed to register AMM adapters.
        type AmmRegistrarOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

        /// Settlement bridge for post-execution settlement intent creation.
        /// Use `NoOpSettlementBridge` in tests or when settlement is not needed.
        type Settlement: SettlementBridge<<Self as frame_system::Config>::AccountId>;
    }

    /// Active trade batches indexed by batch_id.
    #[pallet::storage]
    #[pallet::getter(fn trade_batches)]
    pub type TradeBatches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // batch_id
        TradeBatch<T::AccountId, BalanceOf<T>, T::MaxTradeLegs>,
        OptionQuery,
    >;

    /// Checkpoints for active trade batches.
    #[pallet::storage]
    #[pallet::getter(fn checkpoints)]
    pub type Checkpoints<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        H256, // batch_id
        BoundedVec<StateCheckpoint, T::MaxCheckpoints>,
        ValueQuery,
    >;

    /// Pending batch IDs per account for rate limiting.
    #[pallet::storage]
    #[pallet::getter(fn pending_batches)]
    pub type PendingBatches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<H256, T::MaxPendingBatchesPerAccount>,
        ValueQuery,
    >;

    /// Registered AMM adapters by protocol identifier.
    #[pallet::storage]
    #[pallet::getter(fn amm_adapters)]
    pub type AmmAdapters<T: Config> =
        StorageMap<_, Blake2_128Concat, types::AmmProtocol, AmmAdapterConfig, OptionQuery>;

    /// Registered liquidity pools used by the route solver and oracle.
    #[pallet::storage]
    #[pallet::getter(fn liquidity_pools)]
    pub type LiquidityPools<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, types::LiquidityPool, OptionQuery>;

    /// Trade execution nonces per account.
    #[pallet::storage]
    #[pallet::getter(fn trade_nonces)]
    pub type TradeNonces<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Completed batch count for metrics.
    #[pallet::storage]
    #[pallet::getter(fn completed_batch_count)]
    pub type CompletedBatchCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Failed batch count for metrics.
    #[pallet::storage]
    #[pallet::getter(fn failed_batch_count)]
    pub type FailedBatchCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Total volume traded (in native units) for metrics.
    #[pallet::storage]
    #[pallet::getter(fn total_volume)]
    pub type TotalVolume<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Price observations for TWAP oracle (token_pair => observations).
    #[pallet::storage]
    #[pallet::getter(fn price_observations)]
    pub type PriceObservations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (H256, H256), // (token_a, token_b)
        BoundedVec<types::PricePoint, ConstU32<256>>,
        ValueQuery,
    >;

    /// TWAP data for price pairs.
    #[pallet::storage]
    #[pallet::getter(fn twap_data)]
    pub type TwapData<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (H256, H256), // (token_a, token_b)
        types::TwapData,
        OptionQuery,
    >;

    /// Offchain worker price aggregator submissions.
    #[pallet::storage]
    pub type PendingPriceUpdates<T: Config> =
        StorageValue<_, BoundedVec<types::PricePoint, ConstU32<64>>, ValueQuery>;

    /// Type alias to disambiguate AccountId from frame_system::Config
    pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

    pub type BalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new trade batch was created.
        TradeBatchCreated {
            batch_id: H256,
            origin: T::AccountId,
            legs_count: u32,
        },
        /// A trade leg execution started.
        TradeLegStarted {
            batch_id: H256,
            leg_index: u32,
            vm_type: types::VmType,
        },
        /// A trade leg completed successfully.
        TradeLegCompleted {
            batch_id: H256,
            leg_index: u32,
            amount_out: u128,
        },
        /// A trade leg failed.
        TradeLegFailed {
            batch_id: H256,
            leg_index: u32,
            reason: TradeLegFailureReason,
        },
        /// A checkpoint was created during trade execution.
        CheckpointCreated {
            batch_id: H256,
            checkpoint_index: u32,
            state_root: H256,
        },
        /// A rollback to checkpoint was executed.
        RollbackExecuted {
            batch_id: H256,
            checkpoint_index: u32,
        },
        /// A trade batch completed successfully.
        TradeBatchCompleted {
            batch_id: H256,
            total_input: u128,
            total_output: u128,
            gas_used: u64,
        },
        /// A trade batch failed and was rolled back.
        TradeBatchFailed {
            batch_id: H256,
            failed_leg_index: u32,
            reason: BatchFailureReason,
        },
        /// An AMM adapter was registered.
        AmmAdapterRegistered {
            protocol: types::AmmProtocol,
            vm_type: types::VmType,
        },
        /// An AMM adapter was removed.
        AmmAdapterRemoved { protocol: types::AmmProtocol },
        /// A liquidity pool was registered or updated.
        LiquidityPoolRegistered {
            pool_id: H256,
            token_a: H256,
            token_b: H256,
            protocol: types::AmmProtocol,
            vm_type: types::VmType,
        },
        /// Liquidity reserves were refreshed for a registered pool.
        LiquidityPoolUpdated {
            pool_id: H256,
            reserve_a: u128,
            reserve_b: u128,
        },
        /// A pool-derived oracle observation was synced.
        PoolPriceSynced {
            pool_id: H256,
            token_a: H256,
            token_b: H256,
            price: u128,
        },
        /// Arbitrage opportunity detected.
        ArbitrageOpportunityDetected {
            path: Vec<AssetPair>,
            expected_profit_bps: u32,
        },
        /// Price observation recorded.
        PriceObservationRecorded {
            token_a: H256,
            token_b: H256,
            price: u128,
            source: types::AmmProtocol,
        },
        /// TWAP updated for a token pair.
        TwapUpdated {
            token_a: H256,
            token_b: H256,
            twap_price: u128,
        },

        /// A trade batch was executed via X3 Kernel v2 comit.
        TradeBatchExecutedViaKernelComitV2 { batch_id: H256, comit_id: H256 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Trade batch not found.
        BatchNotFound,
        /// Trade batch already exists.
        BatchAlreadyExists,
        /// Too many trade legs in batch.
        TooManyTradeLegs,
        /// Empty trade batch.
        EmptyTradeBatch,
        /// Invalid leg index.
        InvalidLegIndex,
        /// Checkpoint not found.
        CheckpointNotFound,
        /// Too many checkpoints.
        TooManyCheckpoints,
        /// Slippage tolerance exceeded.
        SlippageExceeded,
        /// Invalid slippage tolerance.
        InvalidSlippageTolerance,
        /// Insufficient input amount.
        InsufficientInputAmount,
        /// AMM adapter not registered.
        AmmNotRegistered,
        /// AMM adapter already registered.
        AmmAlreadyRegistered,
        /// Liquidity pool not registered.
        PoolNotFound,
        /// Invalid AMM protocol.
        InvalidAmmProtocol,
        /// Trade nonce mismatch.
        InvalidTradeNonce,
        /// Too many pending batches for account.
        TooManyPendingBatches,
        /// Trade batch is not pending.
        BatchNotPending,
        /// Trade batch already completed.
        BatchAlreadyCompleted,
        /// EVM execution failed during trade.
        EvmTradeFailed,
        /// SVM execution failed during trade.
        SvmTradeFailed,
        /// Cross-VM bridging failed.
        CrossVmBridgeFailed,
        /// X3 execution failed during trade.
        X3TradeFailed,
        /// Arithmetic overflow.
        ArithmeticOverflow,
        /// Invalid asset pair.
        InvalidAssetPair,
        /// Path not found.
        PathNotFound,
        /// Circular path detected.
        CircularPathDetected,
        /// Deadline expired.
        DeadlineExpired,
        /// Unauthorized operation.
        Unauthorized,
        /// Invalid batch status transition.
        InvalidStatusTransition,
        /// Price observation limit exceeded.
        TooManyPriceObservations,
        /// Invalid price data.
        InvalidPriceData,
        /// Invalid liquidity pool configuration.
        InvalidPoolConfiguration,
        /// Price oracle not initialized for pair.
        PriceOracleNotInitialized,

        /// Batch contains a VM type not supported by kernel v2 comits.
        KernelComitUnsupportedVm,
        /// Batch contains more than one leg for the same VM.
        KernelComitDuplicateVmLeg,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new atomic trade batch with multiple legs.
        ///
        /// # Arguments
        /// * `legs` - Vector of trade legs to execute atomically
        /// * `slippage_tolerance_bps` - Maximum acceptable slippage in basis points
        /// * `deadline` - Block number deadline for execution
        /// * `nonce` - Trade nonce for replay protection
        ///
        /// # Events
        /// * `TradeBatchCreated` - When batch is successfully created
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::create_trade_batch(legs.len() as u32))]
        pub fn create_trade_batch(
            origin: OriginFor<T>,
            legs: Vec<TradeLegInput>,
            slippage_tolerance_bps: u32,
            deadline: BlockNumberFor<T>,
