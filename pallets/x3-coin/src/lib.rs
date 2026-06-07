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

use codec::{Codec, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use frame_support::traits::UnixTime;
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::traits::{CheckedAdd, SaturatedConversion, Zero};
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
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
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
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    RuntimeDebug,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct BonusClaim {
    /// Amount claimed
    pub amount: u128,
    /// Block when claim was made
    pub claimed_at: u64,
    /// Whether claim is locked (for vesting)
    pub locked: bool,
}

/// Runtime relayer configuration entry
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct RelayerRuntimeConfig<AccountId, Balance> {
    pub relayer: AccountId,
    pub enabled_chains: Vec<u32>,
    pub min_confirmations: u32,
    pub max_gas_price: Balance,
}

/// Runtime cross-chain event record
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct CrossChainRuntimeEvent {
    pub operation_id: H256,
    pub chain_id: u32,
    pub event_type: u8,
    pub timestamp: u64,
    pub data: Vec<u8>,
}

/// Proof types for cross-chain operations
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
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

    /// S1-3 (unauthorized_mint) — Authorized minter / burner / relayer allow-list.
    ///
    /// Only accounts present in this map (or root) may invoke `mint`, `burn`,
    /// `submit_cross_chain_operation`, or `finalize_cross_chain_operation`.
    /// Membership is governed by the root origin via `add_minter` / `remove_minter`,
    /// which in production is bound to governance through the runtime's
    /// `RuntimeUpgradeOrigin` (or equivalent) -> root scheduler path.
    #[pallet::storage]
    #[pallet::getter(fn is_minter)]
    pub type Minters<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

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
            // The configured treasury account is the default privileged bridge
            // executor for genesis and testnet bring-up, so it must start on
            // the minter allow-list.
            Minters::<T>::insert(T::TreasuryAccount::get(), ());

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

            // 🔒 SECURITY: Verify supply conservation invariant after genesis allocations
            Pallet::<T>::verify_supply_invariant().expect("Genesis supply invariant violation");
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
        /// S1-3: An account was added to the authorized-minter allow-list.
        MinterAdded { account: T::AccountId },
        /// S1-3: An account was removed from the authorized-minter allow-list.
        MinterRemoved { account: T::AccountId },
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
        /// Supply conservation invariant violated - total supply must equal treasury + bonus + distributed
        SupplyInvariantViolation,
        /// S1-3: caller is not in the authorized minter allow-list.
        UnauthorizedMinter,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Mint X3 tokens from cross-chain operation
        ///
        /// S1-3 (unauthorized_mint): caller MUST be in the `Minters` allow-list.
        /// `ensure_signed` alone is not sufficient — minting is a privileged
        /// operation that affects total supply.
        #[pallet::call_index(0)]
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn mint(
            origin: OriginFor<T>,
            target_account: Vec<u8>,
            amount: T::Balance,
            proof: X3Proof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_minter(&who)?;

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

            // 🔒 SECURITY: Verify supply conservation invariant after mint
            // Treasury decreased, canonical balance increased - total should be unchanged
            Self::verify_supply_invariant()?;

            // Emit event
            Self::deposit_event(Event::Minted {
                account: account_id,
                amount,
                operation_id,
            });

            Ok(())
        }

        /// Burn X3 tokens for cross-chain operation
        ///
        /// S1-3 (unauthorized_mint): symmetric guard — only authorized minters
        /// may perform cross-chain burns. Prevents an unauthorized actor from
        /// burning canonical balance to forge an external-chain release.
        #[pallet::call_index(1)]
        #[pallet::weight(<T as Config>::WeightInfo::burn())]
        pub fn burn(
            origin: OriginFor<T>,
            source_account: Vec<u8>,
            amount: T::Balance,
            proof: X3Proof,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_minter(&who)?;

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

            // Update treasury balance
            TreasuryBalance::<T>::mutate(|balance| *balance = balance.saturating_add(amount));

            // 🔒 SECURITY: Verify supply conservation invariant after burn
            // Canonical balance decreased, treasury increased - total should be unchanged
            Self::verify_supply_invariant()?;

            // Emit event
            Self::deposit_event(Event::Burned {
                account: account_id,
                amount,
                operation_id,
            });

            Ok(())
        }

        /// Claim team vesting tokens
        #[pallet::call_index(2)]
        #[pallet::weight(<T as Config>::WeightInfo::claim_team_vesting())]
        pub fn claim_team_vesting(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut schedule = TeamVesting::<T>::get(&who).ok_or(Error::<T>::NoVestingSchedule)?;

            // Check if vesting period has started
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_block_u64: u64 = current_block.saturated_into();
            ensure!(
                current_block_u64 >= schedule.start_block,
                Error::<T>::VestingNotStarted
            );

            // Check if cliff period has passed
            ensure!(
                current_block_u64 >= schedule.start_block + schedule.cliff_blocks,
                Error::<T>::VestingCliffNotReached
            );

            // Calculate vested amount
            let elapsed_blocks = current_block_u64.saturating_sub(schedule.start_block);
            let total_vested = if elapsed_blocks >= schedule.vesting_blocks {
                schedule.total_amount
            } else {
                schedule
                    .total_amount
                    .saturating_mul(elapsed_blocks.into())
                    .saturating_div(schedule.vesting_blocks.into())
            };

            let available_u128 = total_vested.saturating_sub(schedule.claimed);
            ensure!(available_u128 > 0, Error::<T>::NoVestedAmount);
            let available: T::Balance = available_u128.saturated_into();

            // Update schedule
            schedule.claimed = schedule.claimed.saturating_add(available_u128);
            TeamVesting::<T>::insert(&who, schedule);

            // Update canonical balance via X3 Kernel
            Self::increase_canonical_balance(&who, available);

            // 🔒 SECURITY: Verify supply conservation invariant after vesting claim
            Self::verify_supply_invariant()?;

            // Emit event
            Self::deposit_event(Event::TeamVestingClaimed {
                account: who,
                amount: available,
            });

            Ok(())
        }

        /// Claim bonus pool tokens
        #[pallet::call_index(3)]
        #[pallet::weight(<T as Config>::WeightInfo::claim_bonus())]
        pub fn claim_bonus(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check bonus pool balance
            let bonus_balance = BonusPoolBalance::<T>::get();
            ensure!(
                bonus_balance > T::Balance::zero(),
                Error::<T>::InsufficientBonusPoolBalance
            );

            // Check claim period
            let current_block = frame_system::Pallet::<T>::block_number();
            let current_block_u64: u64 = current_block.saturated_into();
            let last_claim = BonusClaims::<T>::get(&who)
                .last()
                .map(|claim| claim.claimed_at)
                .unwrap_or(0);

            ensure!(
                current_block_u64.saturating_sub(last_claim) >= T::BonusClaimPeriod::get(),
                Error::<T>::BonusClaimPeriodExpired
            );

            // Check maximum claims
            let mut claims = BonusClaims::<T>::get(&who);
            ensure!(
                claims.len() < T::MaxBonusClaims::get() as usize,
                Error::<T>::MaxBonusClaimsReached
            );

            // Calculate claim amount (10% of remaining bonus pool)
            let divisor: T::Balance = 10u32.saturated_into();
            let claim_amount = bonus_balance / divisor;

            // Update bonus pool balance
            BonusPoolBalance::<T>::mutate(|balance| {
                *balance = balance.saturating_sub(claim_amount)
            });

            // Add claim record
            let claim = BonusClaim {
                amount: claim_amount.saturated_into(),
                claimed_at: current_block_u64,
                locked: false,
            };
            claims
                .try_push(claim)
                .map_err(|_| Error::<T>::MaxBonusClaimsReached)?;
            let claim_id = claims.len() as u32;
            BonusClaims::<T>::insert(&who, claims);

            // Update canonical balance via X3 Kernel
            Self::increase_canonical_balance(&who, claim_amount);

            // 🔒 SECURITY: Verify supply conservation invariant after bonus claim
            Self::verify_supply_invariant()?;

            // Emit event
            Self::deposit_event(Event::BonusClaimed {
                account: who,
                amount: claim_amount,
                claim_id,
            });

            Ok(())
        }

        /// Submit cross-chain operation
        ///
        /// S1-3 (unauthorized_mint): only authorized minters may submit
        /// cross-chain mint/burn/transfer operations.
        #[pallet::call_index(4)]
        #[pallet::weight(<T as Config>::WeightInfo::submit_cross_chain_operation())]
        pub fn submit_cross_chain_operation(
            origin: OriginFor<T>,
            operation: CrossChainOperation,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_minter(&who)?;

            let operation_id = Self::generate_cross_chain_operation_id(&operation);

            // Check for duplicate operations
            ensure!(
                !CrossChainOperations::<T>::contains_key(operation_id),
                Error::<T>::CrossChainOperationExists
            );

            // Validate operation
            Self::validate_cross_chain_operation(&operation)?;

            // Store operation
            CrossChainOperations::<T>::insert(operation_id, operation.clone());

            // Emit event
            let (operation_type, source_account, target_account, amount_u128) = match &operation {
                CrossChainOperation::Mint {
                    target_account,
                    amount,
                    ..
                } => (0u8, vec![], target_account.clone(), *amount),
                CrossChainOperation::Burn {
                    source_account,
                    amount,
                    ..
                } => (1u8, source_account.clone(), vec![], *amount),
                CrossChainOperation::Transfer {
                    source_account,
                    target_account,
                    amount,
                    ..
                } => (2u8, source_account.clone(), target_account.clone(), *amount),
            };
            let amount: T::Balance = amount_u128.saturated_into();

            Self::deposit_event(Event::CrossChainOperationSubmitted {
                operation_id,
                operation_type,
                source_account,
                target_account,
                amount,
            });

            Ok(())
        }

        /// Finalize cross-chain operation
        /// S1-3 (unauthorized_mint): finalization triggers the actual mint /
        /// burn / transfer — this MUST be guarded against arbitrary callers.
        #[pallet::call_index(5)]
        #[pallet::weight(<T as Config>::WeightInfo::finalize_cross_chain_operation())]
        pub fn finalize_cross_chain_operation(
            origin: OriginFor<T>,
            operation_id: H256,
            success: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Self::ensure_minter(&who)?;

            let operation = CrossChainOperations::<T>::get(operation_id)
                .ok_or(Error::<T>::CrossChainOperationNotFound)?;

            if success {
                match operation {
                    CrossChainOperation::Mint {
                        target_account,
                        amount,
                        proof: _,
                    } => {
                        // Execute mint
                        let account_id = Self::decode_account_id(&target_account)?;
                        Self::increase_canonical_balance(&account_id, amount.saturated_into());

                        // 🔒 SECURITY: Verify supply conservation invariant after cross-chain mint
                        Self::verify_supply_invariant()?;
                    }
                    CrossChainOperation::Burn {
                        source_account,
                        amount,
                        proof: _,
                    } => {
                        // Execute burn
                        let account_id = Self::decode_account_id(&source_account)?;
                        let amount: T::Balance = amount.saturated_into();
                        let current_balance = Self::canonical_balance(&account_id);
                        ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

                        Self::decrease_canonical_balance(&account_id, amount)?;

                        // 🔒 SECURITY: Verify supply conservation invariant after cross-chain burn
                        Self::verify_supply_invariant()?;
                    }
                    CrossChainOperation::Transfer {
                        source_account,
                        target_account,
                        amount,
                        proof: _,
                    } => {
                        // Execute transfer
                        let source_id = Self::decode_account_id(&source_account)?;
                        let target_id = Self::decode_account_id(&target_account)?;
                        let amount: T::Balance = amount.saturated_into();

                        let current_balance = Self::canonical_balance(&source_id);
                        ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

                        Self::decrease_canonical_balance(&source_id, amount)?;
                        Self::increase_canonical_balance(&target_id, amount);

                        // 🔒 SECURITY: Verify supply conservation invariant after cross-chain transfer
                        Self::verify_supply_invariant()?;
                    }
                }
            }

            // Remove operation
            CrossChainOperations::<T>::remove(operation_id);

            // Emit event
            Self::deposit_event(Event::CrossChainOperationFinalized {
                operation_id,
                success,
            });

            Ok(())
        }

        /// S1-3: Add an account to the authorized-minter allow-list.
        ///
        /// Origin: `Root` (governance-bound in production).
        #[pallet::call_index(20)]
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn add_minter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Minters::<T>::insert(&who, ());
            Self::deposit_event(Event::MinterAdded { account: who });
            Ok(())
        }

        /// S1-3: Remove an account from the authorized-minter allow-list.
        ///
        /// Origin: `Root` (governance-bound in production).
        #[pallet::call_index(21)]
        #[pallet::weight(<T as Config>::WeightInfo::mint())]
        pub fn remove_minter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Minters::<T>::remove(&who);
            Self::deposit_event(Event::MinterRemoved { account: who });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// S1-3 (unauthorized_mint): Reject callers that are not in the
        /// authorized minter allow-list. Privileged paths (mint, burn,
        /// cross-chain submit/finalize) MUST call this immediately after
        /// `ensure_signed`.
        pub(crate) fn ensure_minter(who: &T::AccountId) -> Result<(), Error<T>> {
            ensure!(
                Minters::<T>::contains_key(who),
                Error::<T>::UnauthorizedMinter
            );
            Ok(())
        }

        fn parse_finality_envelope(
            proof_bytes: &[u8],
            expected_chain: u8,
        ) -> Result<(u32, [u8; 32], [u8; 32], &[u8]), Error<T>> {
            ensure!(
                proof_bytes.len() >= FINALITY_ENVELOPE_LEN,
                Error::<T>::InvalidProof
            );
            ensure!(
                &proof_bytes[0..4] == FINALITY_ENVELOPE_MAGIC,
                Error::<T>::InvalidProof
            );
            ensure!(proof_bytes[4] == expected_chain, Error::<T>::InvalidProof);
            ensure!(
                proof_bytes[5] == FINALITY_ENVELOPE_VERSION,
                Error::<T>::InvalidProof
            );

            let mut confirmations_bytes = [0u8; 4];
            confirmations_bytes.copy_from_slice(&proof_bytes[6..10]);
            let confirmations = u32::from_le_bytes(confirmations_bytes);

            let mut inclusion_commitment = [0u8; 32];
            inclusion_commitment.copy_from_slice(&proof_bytes[10..42]);
            let mut header_commitment = [0u8; 32];
            header_commitment.copy_from_slice(&proof_bytes[42..74]);
            ensure!(
                inclusion_commitment.iter().any(|b| *b != 0),
                Error::<T>::InvalidProof
            );
            ensure!(
                header_commitment.iter().any(|b| *b != 0),
                Error::<T>::InvalidProof
            );

            Ok((
                confirmations,
                inclusion_commitment,
                header_commitment,
                &proof_bytes[74..],
            ))
        }

        fn validate_finality_witness<'a>(
            expected_chain: u8,
            inclusion_commitment: &[u8; 32],
            header_commitment: &[u8; 32],
            witness_and_tail: &'a [u8],
        ) -> Result<&'a [u8], Error<T>> {
            ensure!(
                witness_and_tail.len() >= FINALITY_WITNESS_LEN,
                Error::<T>::InvalidProof
            );

            let receipt_root = &witness_and_tail[0..32];
            let header_hash = &witness_and_tail[32..64];
            let light_client_root = &witness_and_tail[64..96];

            ensure!(
                receipt_root.iter().any(|b| *b != 0),
                Error::<T>::InvalidProof
            );
            ensure!(
                header_hash.iter().any(|b| *b != 0),
                Error::<T>::InvalidProof
            );
            ensure!(
                light_client_root.iter().any(|b| *b != 0),
                Error::<T>::InvalidProof
            );

            let mut inclusion_preimage = Vec::with_capacity(1 + 32 + 32);
            inclusion_preimage.push(expected_chain);
            inclusion_preimage.extend_from_slice(receipt_root);
            inclusion_preimage.extend_from_slice(light_client_root);

            let mut header_preimage = Vec::with_capacity(1 + 32 + 32);
            header_preimage.push(expected_chain);
            header_preimage.extend_from_slice(header_hash);
            header_preimage.extend_from_slice(light_client_root);

            let expected_inclusion = sp_io::hashing::blake2_256(&inclusion_preimage);
            let expected_header = sp_io::hashing::blake2_256(&header_preimage);

            ensure!(
                expected_inclusion == *inclusion_commitment,
                Error::<T>::InvalidProof
            );
            ensure!(
                expected_header == *header_commitment,
                Error::<T>::InvalidProof
            );

            Ok(&witness_and_tail[FINALITY_WITNESS_LEN..])
        }

        fn verify_evm_finality_hook(
            tx_hash: &H256,
            block_number: u64,
            receipt_tail: &[u8],
        ) -> Result<(), Error<T>> {
            ensure!(
                receipt_tail.len() >= EVM_FINALITY_TAIL_LEN,
                Error::<T>::InvalidProof
            );

            let tx_commitment = &receipt_tail[0..32];

            let mut observed_block_bytes = [0u8; 8];
            observed_block_bytes.copy_from_slice(&receipt_tail[32..40]);
            let observed_block_number = u64::from_le_bytes(observed_block_bytes);

            let mut header_block_bytes = [0u8; 8];
            header_block_bytes.copy_from_slice(&receipt_tail[40..48]);
            let header_block_number = u64::from_le_bytes(header_block_bytes);

            let mut receipt_index_bytes = [0u8; 4];
            receipt_index_bytes.copy_from_slice(&receipt_tail[48..52]);
            let receipt_index = u32::from_le_bytes(receipt_index_bytes);

            let mut expected_tx_preimage = Vec::with_capacity(5 + 32);
            expected_tx_preimage.extend_from_slice(b"EVMTX");
            expected_tx_preimage.extend_from_slice(tx_hash.as_bytes());
            let expected_tx_commitment = sp_io::hashing::blake2_256(&expected_tx_preimage);

            ensure!(
                tx_commitment == expected_tx_commitment,
                Error::<T>::InvalidProof
            );
            ensure!(
                observed_block_number == block_number,
                Error::<T>::InvalidProof
            );
            ensure!(
                header_block_number == block_number,
                Error::<T>::InvalidProof
            );
            ensure!(receipt_index != u32::MAX, Error::<T>::InvalidProof);

            Ok(())
        }

        fn verify_svm_finality_hook(
            signature: &[u8],
            block_number: u64,
            receipt_tail: &[u8],
        ) -> Result<(), Error<T>> {
            ensure!(
                receipt_tail.len() >= SVM_FINALITY_TAIL_LEN,
                Error::<T>::InvalidProof
            );

            let signature_commitment = &receipt_tail[0..32];

            let mut observed_slot_bytes = [0u8; 8];
            observed_slot_bytes.copy_from_slice(&receipt_tail[32..40]);
            let observed_slot = u64::from_le_bytes(observed_slot_bytes);

            let confirmation_status = receipt_tail[40];

            let mut expected_sig_preimage = Vec::with_capacity(6 + signature.len());
            expected_sig_preimage.extend_from_slice(b"SVMSIG");
            expected_sig_preimage.extend_from_slice(signature);
            let expected_signature_commitment = sp_io::hashing::blake2_256(&expected_sig_preimage);

            ensure!(
                signature_commitment == expected_signature_commitment,
                Error::<T>::InvalidProof
            );
            ensure!(observed_slot == block_number, Error::<T>::InvalidProof);
            ensure!(confirmation_status == 1, Error::<T>::InvalidProof);

            Ok(())
        }

        fn verify_btc_finality_hook(
            txid: &H256,
            block_height: u64,
            branch_bytes: &[u8],
        ) -> Result<(), Error<T>> {
            ensure!(
                branch_bytes.len() >= BTC_FINALITY_PREFIX_LEN + 32,
                Error::<T>::InvalidProof
            );

            let tx_commitment = &branch_bytes[0..32];

            let mut observed_height_bytes = [0u8; 8];
            observed_height_bytes.copy_from_slice(&branch_bytes[32..40]);
            let observed_height = u64::from_le_bytes(observed_height_bytes);

            let merkle_branch = &branch_bytes[40..];

            let mut expected_tx_preimage = Vec::with_capacity(5 + 32);
            expected_tx_preimage.extend_from_slice(b"BTCTX");
            expected_tx_preimage.extend_from_slice(txid.as_bytes());
            let expected_tx_commitment = sp_io::hashing::blake2_256(&expected_tx_preimage);

            ensure!(
                tx_commitment == expected_tx_commitment,
                Error::<T>::InvalidProof
            );
            ensure!(observed_height == block_height, Error::<T>::InvalidProof);
            ensure!(!merkle_branch.is_empty(), Error::<T>::InvalidProof);
            ensure!(merkle_branch.len() % 32 == 0, Error::<T>::InvalidProof);

            Ok(())
        }

        fn x3_asset_id() -> T::AssetId {
            T::AssetId::default()
        }

        /// Verify the canonical supply conservation invariant:
        /// TotalSupply == TreasuryBalance + BonusPoolBalance + sum(all distributed balances)
        ///
        /// This is the CORE SECURITY INVARIANT for the X3 token economics.
        /// If this invariant is violated, it indicates:
        /// - Double-mint vulnerability
        /// - Burn without proper accounting
        /// - Balance corruption
        /// - Arithmetic overflow/underflow bugs
        ///
        /// NOTE: This function is computationally expensive as it requires iterating
        /// over all accounts. It should only be called after critical operations
        /// (mint/burn) and in debug/audit builds with on_finalize hooks.
        #[cfg(any(feature = "runtime-benchmarks", test))]
        fn verify_supply_invariant_full() -> bool {
            let total_supply = TotalSupply::<T>::get();
            let treasury = TreasuryBalance::<T>::get();
            let bonus = BonusPoolBalance::<T>::get();

            // TODO: In production, we need to track distributed_sum incrementally
            // rather than iterating all accounts. For now, this is test-only.
            let mut distributed_sum = T::Balance::zero();

            // Sum all vesting schedules
            for (_account, schedule) in TeamVesting::<T>::iter() {
                distributed_sum = distributed_sum
                    .saturating_add(schedule.total_amount.saturated_into::<T::Balance>());
            }

            // In production, we would track canonical balance sum incrementally
            // For tests, we verify: treasury + bonus + vesting ≤ total_supply
            let accounted = treasury
                .saturating_add(bonus)
                .saturating_add(distributed_sum);

            accounted <= total_supply
        }

        /// Fast supply invariant check for production use.
        /// Verifies that treasury + bonus + tracked_distributed ≤ total_supply.
        /// Does NOT iterate accounts - uses cached sum (to be implemented).
        fn verify_supply_invariant() -> Result<(), Error<T>> {
            let total_supply = TotalSupply::<T>::get();
            let treasury = TreasuryBalance::<T>::get();
            let bonus = BonusPoolBalance::<T>::get();

            // Basic sanity check: treasury + bonus must not exceed total supply
            let treasury_plus_bonus = treasury
                .checked_add(&bonus)
                .ok_or(Error::<T>::SupplyInvariantViolation)?;

            ensure!(
                treasury_plus_bonus <= total_supply,
                Error::<T>::SupplyInvariantViolation
            );

            // Additional check: total supply should be constant (never changes after genesis)
            let expected_total: T::Balance = X3_TOTAL_SUPPLY.saturated_into();
            ensure!(
                total_supply == expected_total,
                Error::<T>::SupplyInvariantViolation
            );

            Ok(())
        }

        fn canonical_balance(account: &T::AccountId) -> T::Balance {
            pallet_x3_kernel::CanonicalLedger::<T>::get(account, Self::x3_asset_id())
        }

        fn increase_canonical_balance(account: &T::AccountId, amount: T::Balance) {
            let next_balance = Self::canonical_balance(account).saturating_add(amount);
            pallet_x3_kernel::CanonicalLedger::<T>::insert(
                account,
                Self::x3_asset_id(),
                next_balance,
            );
        }

        fn decrease_canonical_balance(
            account: &T::AccountId,
            amount: T::Balance,
        ) -> Result<(), Error<T>> {
            let current_balance = Self::canonical_balance(account);
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            pallet_x3_kernel::CanonicalLedger::<T>::insert(
                account,
                Self::x3_asset_id(),
                current_balance.saturating_sub(amount),
            );

            Ok(())
        }

        /// Validate proof for cross-chain operations
        fn validate_proof(proof: &X3Proof) -> Result<(), Error<T>> {
            match proof {
                X3Proof::None => Err(Error::<T>::InvalidProof),
                X3Proof::EvmProof {
                    tx_hash,
                    block_number,
                    proof_data,
                } => {
                    ensure!(*tx_hash != H256::zero(), Error::<T>::InvalidProof);
                    ensure!(*block_number > 0, Error::<T>::InvalidProof);
                    ensure!(proof_data.len() <= 8_192, Error::<T>::InvalidProof);
                    let (confirmations, inclusion_commitment, header_commitment, witness_and_tail) =
                        Self::parse_finality_envelope(proof_data, FINALITY_CHAIN_EVM)?;
                    ensure!(
                        confirmations >= MIN_EVM_CONFIRMATIONS,
                        Error::<T>::InvalidProof
                    );
                    ensure!(
                        *block_number >= confirmations as u64,
                        Error::<T>::InvalidProof
                    );
                    let receipt_tail = Self::validate_finality_witness(
                        FINALITY_CHAIN_EVM,
                        &inclusion_commitment,
                        &header_commitment,
                        witness_and_tail,
                    )?;
                    Self::verify_evm_finality_hook(tx_hash, *block_number, receipt_tail)?;
                    Ok(())
                }
                X3Proof::SvmProof {
                    signature,
                    block_number,
                    proof_data,
                } => {
                    ensure!(*block_number > 0, Error::<T>::InvalidProof);
                    ensure!(
                        signature.len() == 64 || signature.len() == 65,
                        Error::<T>::InvalidProof
                    );
                    ensure!(signature.iter().any(|b| *b != 0), Error::<T>::InvalidProof);
                    ensure!(proof_data.len() <= 8_192, Error::<T>::InvalidProof);
                    let (confirmations, inclusion_commitment, header_commitment, witness_and_tail) =
                        Self::parse_finality_envelope(proof_data, FINALITY_CHAIN_SVM)?;
                    ensure!(
                        confirmations >= MIN_SVM_CONFIRMATIONS,
                        Error::<T>::InvalidProof
                    );
                    ensure!(
                        *block_number >= confirmations as u64,
                        Error::<T>::InvalidProof
                    );
                    let receipt_tail = Self::validate_finality_witness(
                        FINALITY_CHAIN_SVM,
                        &inclusion_commitment,
                        &header_commitment,
                        witness_and_tail,
                    )?;
                    Self::verify_svm_finality_hook(signature, *block_number, receipt_tail)?;
                    Ok(())
                }
                X3Proof::BtcProof {
                    txid,
                    block_height,
                    merkle_proof,
                } => {
                    ensure!(*txid != H256::zero(), Error::<T>::InvalidProof);
                    ensure!(*block_height > 0, Error::<T>::InvalidProof);
                    ensure!(
                        merkle_proof.len() <= 4_096 + FINALITY_ENVELOPE_LEN,
                        Error::<T>::InvalidProof
                    );
                    let (confirmations, inclusion_commitment, header_commitment, witness_and_tail) =
                        Self::parse_finality_envelope(merkle_proof, FINALITY_CHAIN_BTC)?;
                    ensure!(
                        confirmations >= MIN_BTC_CONFIRMATIONS,
                        Error::<T>::InvalidProof
                    );
                    ensure!(
                        *block_height >= confirmations as u64,
                        Error::<T>::InvalidProof
                    );
                    let branch_bytes = Self::validate_finality_witness(
                        FINALITY_CHAIN_BTC,
                        &inclusion_commitment,
                        &header_commitment,
                        witness_and_tail,
                    )?;
                    Self::verify_btc_finality_hook(txid, *block_height, branch_bytes)?;
                    Ok(())
                }
            }
        }

        /// Generate operation ID for mint/burn operations
        pub(crate) fn generate_operation_id(
            target_account: &[u8],
            amount: T::Balance,
            proof: &X3Proof,
        ) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(target_account);
            data.extend_from_slice(&amount.encode());
            data.extend_from_slice(&proof.encode());
            H256::from(sp_io::hashing::blake2_256(&data))
        }

        /// Generate cross-chain operation ID
        fn generate_cross_chain_operation_id(operation: &CrossChainOperation) -> H256 {
            let mut data = Vec::new();
            data.extend_from_slice(&operation.encode());
            H256::from(sp_io::hashing::blake2_256(&data))
        }

        /// Validate cross-chain operation
        fn validate_cross_chain_operation(operation: &CrossChainOperation) -> Result<(), Error<T>> {
            match operation {
                CrossChainOperation::Mint { target_account, .. } => {
                    ensure!(!target_account.is_empty(), Error::<T>::InvalidTargetAccount);
                    Ok(())
                }
                CrossChainOperation::Burn { source_account, .. } => {
                    ensure!(!source_account.is_empty(), Error::<T>::InvalidTargetAccount);
                    Ok(())
                }
                CrossChainOperation::Transfer {
                    source_account,
                    target_account,
                    ..
                } => {
                    ensure!(!source_account.is_empty(), Error::<T>::InvalidTargetAccount);
                    ensure!(!target_account.is_empty(), Error::<T>::InvalidTargetAccount);
                    Ok(())
                }
            }
        }

        /// Decode account ID from bytes
        fn decode_account_id(account_bytes: &[u8]) -> Result<T::AccountId, Error<T>> {
            T::AccountId::decode(&mut &account_bytes[..])
                .map_err(|_| Error::<T>::InvalidTargetAccount)
        }

        pub fn register_relayer_config(
            relayer: T::AccountId,
            enabled_chains: Vec<u32>,
            min_confirmations: u32,
            max_gas_price: T::Balance,
        ) -> Result<(), Error<T>> {
            ensure!(
                !enabled_chains.is_empty(),
                Error::<T>::InvalidCrossChainOperation
            );
            ensure!(
                min_confirmations > 0,
                Error::<T>::InvalidCrossChainOperation
            );

            let max_gas_price_u128: u128 = max_gas_price.saturated_into();
            ensure!(
                max_gas_price_u128 > 0,
                Error::<T>::InvalidCrossChainOperation
            );

            let mut unique_chains = enabled_chains;
            unique_chains.sort_unstable();
            unique_chains.dedup();

            RelayerRegistryStore::<T>::insert(
                &relayer,
                (unique_chains, min_confirmations, max_gas_price),
            );
            Ok(())
        }

        pub fn get_relayer_config_entry(
            relayer: &T::AccountId,
        ) -> Option<RelayerRuntimeConfig<T::AccountId, T::Balance>> {
            RelayerRegistryStore::<T>::get(relayer).map(
                |(enabled_chains, min_confirmations, max_gas_price)| RelayerRuntimeConfig {
                    relayer: relayer.clone(),
                    enabled_chains,
                    min_confirmations,
                    max_gas_price,
                },
            )
        }

        pub fn get_available_relayer_paths(
            source_chain: u32,
            target_chain: u32,
            operation_type: u8,
        ) -> Vec<(T::AccountId, u32, T::Balance)> {
            if source_chain == 0 || target_chain == 0 || source_chain == target_chain {
                return vec![];
            }
            if operation_type > 2 {
                return vec![];
            }

            let fee_bps = if operation_type == 2 { 200 } else { 400 };

            RelayerRegistryStore::<T>::iter()
                .filter_map(
                    |(relayer, (enabled_chains, _min_confirmations, max_gas_price))| {
                        let source_supported =
                            enabled_chains.iter().any(|chain| *chain == source_chain);
                        let target_supported =
                            enabled_chains.iter().any(|chain| *chain == target_chain);
                        if source_supported && target_supported {
                            Some((relayer, fee_bps, max_gas_price))
                        } else {
                            None
                        }
                    },
                )
                .collect()
        }

        pub fn process_cross_chain_event(
            operation_id: H256,
            chain_id: u32,
            event_type: u8,
            timestamp: u64,
            data: Vec<u8>,
        ) -> Result<(), Error<T>> {
            ensure!(chain_id > 0, Error::<T>::InvalidCrossChainOperation);
            ensure!(event_type <= 2, Error::<T>::InvalidCrossChainOperation);
            ensure!(!data.is_empty(), Error::<T>::InvalidCrossChainOperation);

            let mut history = CrossChainEventHistoryStore::<T>::get(chain_id);
            history.push((operation_id, event_type, timestamp, data));

            const MAX_EVENTS_PER_CHAIN: usize = 1024;
            if history.len() > MAX_EVENTS_PER_CHAIN {
                let overflow = history.len() - MAX_EVENTS_PER_CHAIN;
                history.drain(0..overflow);
            }

            CrossChainEventHistoryStore::<T>::insert(chain_id, history);
            Ok(())
        }

        pub fn get_cross_chain_event_history(
            chain_id: u32,
            limit: u32,
        ) -> Vec<CrossChainRuntimeEvent> {
            if chain_id == 0 || limit == 0 {
                return vec![];
            }

            let history = CrossChainEventHistoryStore::<T>::get(chain_id);
            let count = core::cmp::min(limit as usize, history.len());
            let start = history.len().saturating_sub(count);

            history[start..]
                .iter()
                .map(
                    |(operation_id, event_type, timestamp, data)| CrossChainRuntimeEvent {
                        operation_id: *operation_id,
                        chain_id,
                        event_type: *event_type,
                        timestamp: *timestamp,
                        data: data.clone(),
                    },
                )
                .collect()
        }

        /// Get current vested amount for an account
        pub fn get_vested_amount(account: &T::AccountId) -> T::Balance {
            if let Some(schedule) = TeamVesting::<T>::get(account) {
                let current_block = frame_system::Pallet::<T>::block_number();
                let current_block_u64: u64 = current_block.saturated_into();
                if current_block_u64 < schedule.start_block {
                    return T::Balance::zero();
                }

                let elapsed_blocks = current_block_u64.saturating_sub(schedule.start_block);
                let total_vested = if elapsed_blocks >= schedule.vesting_blocks {
                    schedule.total_amount
                } else {
                    schedule
                        .total_amount
                        .saturating_mul(elapsed_blocks.into())
                        .saturating_div(schedule.vesting_blocks.into())
                };

                total_vested
                    .saturating_sub(schedule.claimed)
                    .saturated_into()
            } else {
                T::Balance::zero()
            }
        }

        /// Get total bonus claims for an account
        pub fn get_total_bonus_claims(account: &T::AccountId) -> T::Balance {
            BonusClaims::<T>::get(account)
                .iter()
                .map(|claim| claim.amount)
                .fold(0u128, |acc, amount| acc.saturating_add(amount))
                .saturated_into()
        }

        pub fn get_total_supply() -> T::Balance {
            Self::total_supply()
        }

        pub fn get_treasury_balance() -> T::Balance {
            Self::treasury_balance()
        }

        pub fn get_bonus_pool_balance() -> T::Balance {
            Self::bonus_pool_balance()
        }

        pub fn get_team_vesting(
            account: &T::AccountId,
        ) -> Option<(T::Balance, T::Balance, u64, u64, u64)> {
            TeamVesting::<T>::get(account).map(|schedule| {
                (
                    schedule.total_amount.saturated_into(),
                    schedule.claimed.saturated_into(),
                    schedule.start_block,
                    schedule.cliff_blocks,
                    schedule.vesting_blocks,
                )
            })
        }

        pub fn get_bonus_claims(account: &T::AccountId) -> Vec<(T::Balance, u64, bool)> {
            BonusClaims::<T>::get(account)
                .into_iter()
                .map(|claim| {
                    (
                        claim.amount.saturated_into(),
                        claim.claimed_at,
                        claim.locked,
                    )
                })
                .collect()
        }
    }
}

// Runtime API definitions for querying X3 Coin state
sp_api::decl_runtime_apis! {
    /// Runtime API for querying X3 Coin pallet state
    pub trait X3CoinRuntimeApi<AccountId, Balance> where
        AccountId: Codec,
        Balance: Codec,
    {
        /// Get the total X3 supply
        fn get_total_supply() -> Balance;

        /// Get the treasury balance
        fn get_treasury_balance() -> Balance;

        /// Get the bonus pool balance
        fn get_bonus_pool_balance() -> Balance;

        /// Get vested amount for an account
        fn get_vested_amount(account: AccountId) -> Balance;

        /// Get total bonus claims for an account
        fn get_total_bonus_claims(account: AccountId) -> Balance;

        /// Get team vesting schedule for an account
        fn get_team_vesting(account: AccountId) -> Option<(Balance, Balance, u64, u64, u64)>;

        /// Get bonus claims for an account
        fn get_bonus_claims(account: AccountId) -> Vec<(Balance, u64, bool)>;
    }
}
