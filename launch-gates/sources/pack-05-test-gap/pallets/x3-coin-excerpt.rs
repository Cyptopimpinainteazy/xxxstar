#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]

//! # X3 Coin Pallet
//!
//! The X3 Coin pallet manages the canonical X3 token on the X3 Chain.
//! It provides:
//! - Genesis issuance and treasury allocation
//! - Runtime API for canonical balance and total supply
//! - Bonus pool claim ledger and vesting hooks
//! - Integration with X3 Kernel for cross-VM operations

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod cross_chain;
pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::UnixTime;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::traits::{SaturatedConversion, Zero};
use sp_std::{vec, vec::Vec};

const FINALITY_ENVELOPE_MAGIC: &[u8; 4] = b"X3PF";
const FINALITY_ENVELOPE_VERSION: u8 = 1;
const FINALITY_CHAIN_EVM: u8 = 1;
const FINALITY_CHAIN_SVM: u8 = 2;
const FINALITY_CHAIN_BTC: u8 = 3;
const FINALITY_ENVELOPE_LEN: usize = 74;
const FINALITY_WITNESS_LEN: usize = 96;
const EVM_FINALITY_TAIL_LEN: usize = 52;
const SVM_FINALITY_TAIL_LEN: usize = 41;
const BTC_FINALITY_PREFIX_LEN: usize = 40;
const MIN_EVM_CONFIRMATIONS: u32 = 12;
const MIN_SVM_CONFIRMATIONS: u32 = 32;
const MIN_BTC_CONFIRMATIONS: u32 = 6;

pub use weights::WeightInfo;

/// X3 token identifier (canonical asset ID)
pub const X3_ASSET_ID: u32 = 0;

/// X3 token symbol
pub const X3_SYMBOL: &[u8] = b"X3";

/// X3 token decimals
pub const X3_DECIMALS: u8 = 12;

/// Total X3 supply (2 billion tokens with 12 decimals)
pub const X3_TOTAL_SUPPLY: u128 = 2_000_000_000 * 1_000_000_000_000; // 2B * 10^12

/// Treasury allocation (20% of total supply)
pub const X3_TREASURY_ALLOCATION: u128 = X3_TOTAL_SUPPLY / 5;

/// Community bonus pool (10% of total supply)
pub const X3_BONUS_POOL_ALLOCATION: u128 = X3_TOTAL_SUPPLY / 10;

/// Genesis allocation for team and advisors (15% of total supply)
pub const X3_TEAM_ALLOCATION: u128 = X3_TOTAL_SUPPLY * 15 / 100;

/// Genesis allocation for ecosystem development (25% of total supply)
pub const X3_ECOSYSTEM_ALLOCATION: u128 = X3_TOTAL_SUPPLY * 25 / 100;

/// Genesis allocation for liquidity and exchanges (30% of total supply)
pub const X3_LIQUIDITY_ALLOCATION: u128 = X3_TOTAL_SUPPLY * 30 / 100;

/// Vesting period for team allocation (in blocks, ~1 year at 200ms blocks)
pub const TEAM_VESTING_BLOCKS: u64 = 15_768_000; // 15,768,000 blocks ≈ 1 year

/// Vesting cliff for team allocation (in blocks, ~6 months)
pub const TEAM_VESTING_CLIFF: u64 = 7_884_000; // 7,884,000 blocks ≈ 6 months

/// Bonus pool claim period (in blocks, ~3 months)
pub const BONUS_CLAIM_PERIOD: u64 = 3_942_000; // 3,942,000 blocks ≈ 3 months

/// Maximum number of bonus claims per account
pub const MAX_BONUS_CLAIMS: u32 = 10;

/// Vesting schedule for team allocation
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct VestingSchedule {
    /// Total amount to be vested
    pub total_amount: u128,
    /// Amount already claimed
    pub claimed: u128,
    /// Block when vesting starts
    pub start_block: u64,
    /// Cliff period before first claim
    pub cliff_blocks: u64,
    /// Total vesting period
    pub vesting_blocks: u64,
}

/// Bonus pool claim record
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct BonusClaim {
    /// Amount claimed
    pub amount: u128,
    /// Block when claim was made
    pub claimed_at: u64,
    /// Whether claim is locked (for vesting)
    pub locked: bool,
}

/// Runtime relayer configuration entry
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct RelayerRuntimeConfig<AccountId, Balance> {
    pub relayer: AccountId,
    pub enabled_chains: Vec<u32>,
    pub min_confirmations: u32,
    pub max_gas_price: Balance,
}

/// Runtime cross-chain event record
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CrossChainRuntimeEvent {
    pub operation_id: H256,
    pub chain_id: u32,
    pub event_type: u8,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Proof types for cross-chain operations
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum X3Proof {
    /// No proof
    None,
    /// EVM transaction proof
    EvmProof {
        tx_hash: H256,
        block_number: u64,
        proof_data: Vec<u8>,
    },
    /// SVM transaction proof
    SvmProof {
        signature: Vec<u8>,
        block_number: u64,
        proof_data: Vec<u8>,
    },
    /// BTC transaction proof
    BtcProof {
        txid: H256,
        block_height: u64,
        merkle_proof: Vec<u8>,
    },
}

/// Cross-chain operation types
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum CrossChainOperation {
    /// Mint X3 tokens from external chain
    Mint {
        target_account: Vec<u8>,
        amount: u128,
        proof: X3Proof,
    },
    /// Burn X3 tokens for external chain
    Burn {
        source_account: Vec<u8>,
        amount: u128,
        proof: X3Proof,
    },
    /// Transfer between chains
    Transfer {
        source_account: Vec<u8>,
        target_account: Vec<u8>,
        amount: u128,
        proof: X3Proof,
    },
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::Get;
    use sp_runtime::traits::Saturating;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_x3_kernel::Config {
        /// Aggregated runtime event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Unix time for vesting calculations
        type UnixTime: UnixTime;

        /// Weight information provider
        type WeightInfo: WeightInfo;

        /// Treasury account for X3 allocations
        #[pallet::constant]
        type TreasuryAccount: Get<Self::AccountId>;

        /// Maximum number of bonus claims per account
        #[pallet::constant]
        type MaxBonusClaims: Get<u32>;

        /// Vesting period for team allocation (blocks)
        #[pallet::constant]
        type TeamVestingBlocks: Get<u64>;

        /// Vesting cliff for team allocation (blocks)
        #[pallet::constant]
        type TeamVestingCliff: Get<u64>;

        /// Bonus claim period (blocks)
        #[pallet::constant]
        type BonusClaimPeriod: Get<u64>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Total X3 supply
    #[pallet::storage]
    #[pallet::getter(fn total_supply)]
    pub type TotalSupply<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    /// Treasury allocation balance
    #[pallet::storage]
    #[pallet::getter(fn treasury_balance)]
    pub type TreasuryBalance<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    /// Community bonus pool balance
    #[pallet::storage]
    #[pallet::getter(fn bonus_pool_balance)]
    pub type BonusPoolBalance<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    /// Team vesting schedules
    #[pallet::storage]
    #[pallet::getter(fn team_vesting)]
    pub type TeamVesting<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, VestingSchedule, OptionQuery>;

    /// Bonus claims per account
    #[pallet::storage]
    #[pallet::getter(fn bonus_claims)]
    pub type BonusClaims<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<BonusClaim, T::MaxBonusClaims>,
        ValueQuery,
    >;

    /// Cross-chain operation registry
    #[pallet::storage]
    #[pallet::getter(fn cross_chain_operations)]
    pub type CrossChainOperations<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, CrossChainOperation, OptionQuery>;

    /// Proof hash registry for replay protection
    #[pallet::storage]
    #[pallet::getter(fn proof_registry)]
    pub type ProofRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>, OptionQuery>;

    /// Nonce for cross-chain operations
    #[pallet::storage]
    #[pallet::getter(fn cross_chain_nonce)]
    pub type CrossChainNonce<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    /// Relayer configuration registry
    /// Value tuple: (enabled_chains, min_confirmations, max_gas_price)
    #[pallet::storage]
    #[pallet::getter(fn relayer_registry_store)]
    pub type RelayerRegistryStore<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (Vec<u32>, u32, T::Balance), OptionQuery>;

    /// Cross-chain event history by chain
    /// Event tuple: (operation_id, event_type, timestamp, data)
    #[pallet::storage]
    #[pallet::getter(fn cross_chain_event_history_store)]
    pub type CrossChainEventHistoryStore<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, Vec<(H256, u8, u64, Vec<u8>)>, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Team allocation accounts and amounts
        pub team_allocations: Vec<(T::AccountId, u128)>,
        /// Ecosystem allocation accounts and amounts
        pub ecosystem_allocations: Vec<(T::AccountId, u128)>,
        /// Liquidity allocation accounts and amounts
        pub liquidity_allocations: Vec<(T::AccountId, u128)>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Set total supply
            let total_supply: T::Balance = X3_TOTAL_SUPPLY.saturated_into();
            TotalSupply::<T>::put(total_supply);

            // Set treasury balance
            let treasury_balance: T::Balance = X3_TREASURY_ALLOCATION.saturated_into();
            TreasuryBalance::<T>::put(treasury_balance);

            // Set bonus pool balance
            let bonus_pool_balance: T::Balance = X3_BONUS_POOL_ALLOCATION.saturated_into();
            BonusPoolBalance::<T>::put(bonus_pool_balance);

            // Distribute team allocations with vesting
            for (account, amount) in &self.team_allocations {
                let schedule = VestingSchedule {
                    total_amount: *amount,
                    claimed: 0,
                    start_block: 0,
                    cliff_blocks: T::TeamVestingCliff::get(),
                    vesting_blocks: T::TeamVestingBlocks::get(),
                };
                TeamVesting::<T>::insert(account, schedule);
            }

            // Distribute ecosystem allocations (no vesting)
            for (account, amount) in &self.ecosystem_allocations {
                Pallet::<T>::increase_canonical_balance(account, (*amount).saturated_into());
            }

            // Distribute liquidity allocations (no vesting)
            for (account, amount) in &self.liquidity_allocations {
                Pallet::<T>::increase_canonical_balance(account, (*amount).saturated_into());
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// X3 tokens minted
        Minted {
            account: T::AccountId,
            amount: T::Balance,
            operation_id: H256,
        },
        /// X3 tokens burned
        Burned {
            account: T::AccountId,
            amount: T::Balance,
            operation_id: H256,
        },
        /// Team vesting claim
        TeamVestingClaimed {
            account: T::AccountId,
            amount: T::Balance,
        },
        /// Bonus pool claim
        BonusClaimed {
            account: T::AccountId,
            amount: T::Balance,
            claim_id: u32,
        },
        /// Cross-chain operation submitted
        CrossChainOperationSubmitted {
            operation_id: H256,
            operation_type: u8, // 0: Mint, 1: Burn, 2: Transfer
            source_account: Vec<u8>,
            target_account: Vec<u8>,
            amount: T::Balance,
        },
        /// Cross-chain operation finalized
        CrossChainOperationFinalized { operation_id: H256, success: bool },
        /// Treasury allocation updated
        TreasuryAllocationUpdated { new_balance: T::Balance },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient treasury balance
        InsufficientTreasuryBalance,
        /// Insufficient bonus pool balance
        InsufficientBonusPoolBalance,
        /// Invalid proof
        InvalidProof,
        /// Proof already used (replay attack)
        ProofAlreadyUsed,
        /// No vesting schedule found
        NoVestingSchedule,
        /// Vesting period not started
        VestingNotStarted,
        /// Vesting cliff not reached
        VestingCliffNotReached,
        /// No vested amount available
        NoVestedAmount,
        /// Maximum bonus claims reached
        MaxBonusClaimsReached,
        /// Bonus claim period expired
        BonusClaimPeriodExpired,
        /// Invalid cross-chain operation
        InvalidCrossChainOperation,
        /// Cross-chain operation already exists
        CrossChainOperationExists,
        /// Cross-chain operation not found
        CrossChainOperationNotFound,
        /// Insufficient balance for burn
        InsufficientBalance,
        /// Invalid target account
        InvalidTargetAccount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Mint X3 tokens from cross-chain operation
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            target_account: Vec<u8>,
            amount: T::Balance,
            proof: X3Proof,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Validate proof
            Self::validate_proof(&proof)?;

            // Check treasury balance
            let treasury_balance = TreasuryBalance::<T>::get();
            ensure!(
                treasury_balance >= amount,
                Error::<T>::InsufficientTreasuryBalance
            );

            // Generate operation ID
            let operation_id = Self::generate_operation_id(&target_account, amount, &proof);

            // Check for replay attacks
            ensure!(
                !ProofRegistry::<T>::contains_key(operation_id),
                Error::<T>::ProofAlreadyUsed
            );

            // Register proof
            ProofRegistry::<T>::insert(operation_id, frame_system::Pallet::<T>::block_number());

            // Update treasury balance
            TreasuryBalance::<T>::mutate(|balance| *balance = balance.saturating_sub(amount));

            // Update canonical balance via X3 Kernel
            let account_id = Self::decode_account_id(&target_account)?;
            Self::increase_canonical_balance(&account_id, amount);

            // Emit event
            Self::deposit_event(Event::Minted {
                account: account_id,
                amount,
                operation_id,
            });

            Ok(())
        }

        /// Burn X3 tokens for cross-chain operation
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            source_account: Vec<u8>,
            amount: T::Balance,
            proof: X3Proof,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Validate proof
            Self::validate_proof(&proof)?;

            // Generate operation ID
            let operation_id = Self::generate_operation_id(&source_account, amount, &proof);

            // Check for replay attacks
            ensure!(
                !ProofRegistry::<T>::contains_key(operation_id),
                Error::<T>::ProofAlreadyUsed
            );

            // Register proof
            ProofRegistry::<T>::insert(operation_id, frame_system::Pallet::<T>::block_number());

            // Check canonical balance
            let account_id = Self::decode_account_id(&source_account)?;
            let current_balance = Self::canonical_balance(&account_id);

            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            Self::decrease_canonical_balance(&account_id, amount)?;

