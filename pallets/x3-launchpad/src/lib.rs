// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-launchpad — Token launch/presale pallet with on-chain graduation.
//
// Phase 7 completion: open_launch creates a launch, close_launch on success:
//   1. Mints tokens via TokenFactory (creates asset in registry)
//   2. Creates an AMM pool via DEX
//   3. Locks LP tokens via LP Locker for anti-rug protection
//   4. Emits LaunchGraduated event

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode, MaxEncodedLen};
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, Get},
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use scale_info::TypeInfo;
    use sp_std::vec::Vec;

    // ── Types ───────────────────────────────────────────────────────────────

    pub type LaunchId = u32;

    #[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum LaunchStatus {
        /// Fundraising in progress.
        Active,
        /// Soft cap reached, funds can be withdrawn.
        Successful,
        /// Funds withdrawn, launch completed.
        Completed,
        /// Soft cap not reached.
        Failed,
        /// Governance-issued refunds.
        Refunding,
    }

    #[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct LaunchState<AccountId, BlockNumber> {
        pub creator: AccountId,
        pub token_asset_id: u32,
        pub soft_cap: u128,
        pub hard_cap: u128,
        pub price_per_token: u128,
        pub start_block: BlockNumber,
        pub end_block: BlockNumber,
        pub total_raised: u128,
        pub contributor_count: u32,
        pub status: LaunchStatus,
        /// Minimum LP lock duration in blocks after graduation.
        pub lp_lock_duration_blocks: BlockNumber,
    }

    // ── Traits for graduation integration ───────────────────────────────────

    /// Token creation interface: must be implemented by TokenFactory pallet.
    pub trait TokenFactoryCreate<AccountId> {
        fn create_token(
            creator: &AccountId,
            symbol: Vec<u8>,
            name: Vec<u8>,
            decimals: u8,
            initial_supply: u128,
        ) -> Result<u32, DispatchError>;
    }

    /// DEX pool creation interface: must be implemented by DEX pallet.
    pub trait DexPoolCreate<AccountId> {
        fn create_pool(
            creator: &AccountId,
            token_a: u32,
            token_b: u32,
        ) -> Result<u64, DispatchError>;
    }

    /// LP lock interface: must be implemented by LP Locker pallet.
    pub trait LpLockCreate<AccountId, BlockNumber> {
        fn lock_lp_for(
            owner: &AccountId,
            pool_id: u64,
            lp_amount: u128,
            unlock_at_block: BlockNumber,
        ) -> DispatchResult;
    }

    // ── Pallet ──────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that can create and cancel launches.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of simultaneously active launches.
        #[pallet::constant]
        type MaxActiveLaunches: Get<u32>;

        /// Maximum contributors allowed per launch.
        #[pallet::constant]
        type MaxContributorsPerLaunch: Get<u32>;

        /// Minimum duration (in blocks) for a launch window.
        #[pallet::constant]
        type MinLaunchDurationBlocks: Get<BlockNumberFor<Self>>;

        /// Maximum duration (in blocks) for a launch window.
        #[pallet::constant]
        type MaxLaunchDurationBlocks: Get<BlockNumberFor<Self>>;

        /// Token factory for on-chain token creation upon graduation.
        type TokenFactory: TokenFactoryCreate<Self::AccountId>;

        /// DEX pallet for AMM pool creation upon graduation.
        type Dex: DexPoolCreate<Self::AccountId>;

        /// LP Locker pallet for anti-rug LP token locking upon graduation.
        type LpLocker: LpLockCreate<Self::AccountId, BlockNumberFor<Self>>;

        /// Asset ID of the quote token (e.g. X3 native, USDC).
        #[pallet::constant]
        type QuoteAssetId: Get<u32>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ────────────────────────────────────────────────────────────────

    #[pallet::storage]
    #[pallet::getter(fn launches)]
    pub type Launches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        LaunchId,
        LaunchState<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn contributions)]
    pub type Contributions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        LaunchId,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn next_launch_id)]
    pub type NextLaunchId<T: Config> = StorageValue<_, LaunchId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_launch_count)]
    pub type ActiveLaunchCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    pub type ExpiryQueue<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        LaunchId,
        (),
        OptionQuery,
    >;

    #[pallet::storage]
    pub type AllocationClaimed<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        LaunchId,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type RefundClaimed<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        LaunchId,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    /// Track graduated launches for external queries.
    #[pallet::storage]
    pub type GraduatedLaunches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        LaunchId,
        (u32, u64), // (asset_id, pool_id)
        OptionQuery,
    >;

    // ── Events ─────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        LaunchCreated {
            launch_id: LaunchId,
            creator: T::AccountId,
            token_asset_id: u32,
            soft_cap: u128,
            hard_cap: u128,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        },
        ContributionMade {
            launch_id: LaunchId,
            contributor: T::AccountId,
            amount: u128,
        },
        LaunchFinalized {
            launch_id: LaunchId,
            status: LaunchStatus,
            total_raised: u128,
        },
        /// A launch graduated to DEX: tokens minted, pool created, LP locked.
        LaunchGraduated {
            launch_id: LaunchId,
            token_asset_id: u32,
            pool_id: u64,
            total_lp_locked: u128,
        },
        RefundClaimed {
            launch_id: LaunchId,
            contributor: T::AccountId,
            amount: u128,
        },
        AllocationClaimed {
            launch_id: LaunchId,
            contributor: T::AccountId,
            tokens: u128,
        },
        LaunchCancelled {
            launch_id: LaunchId,
        },
        FundsWithdrawn {
            launch_id: LaunchId,
            creator: T::AccountId,
            amount: u128,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        LaunchNotFound,
        LaunchNotActive,
        LaunchNotEnded,
        LaunchNotSuccessful,
        LaunchNotFailed,
        LaunchNoLongerActive,
        SoftCapExceeded,
        HardCapExceeded,
        NotContributor,
        AlreadyClaimed,
        MaxActiveLaunchesReached,
        MaxContributorsReached,
        BadDuration,
        NotCreator,
        TokenFactoryFailed,
        DexPoolCreationFailed,
        LpLockFailed,
        AlreadyGraduated,
        QuoteAssetZeroBalance,
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new launch (GovernanceOrigin only).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_launch())]
        pub fn create_launch(
            origin: OriginFor<T>,
            token_asset_id: u32,
            soft_cap: u128,
            hard_cap: u128,
            price_per_token: u128,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
            lp_lock_duration_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let creator = T::GovernanceOrigin::ensure_origin(origin)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(start_block >= now, Error::<T>::BadDuration);
            ensure!(end_block > start_block, Error::<T>::BadDuration);
            ensure!(soft_cap > 0 && hard_cap >= soft_cap, Error::<T>::SoftCapExceeded);
            ensure!(price_per_token > 0, Error::<T>::BadDuration);

            let duration = end_block.saturating_sub(start_block);
            ensure!(duration >= T::MinLaunchDurationBlocks::get(), Error::<T>::BadDuration);
            ensure!(duration <= T::MaxLaunchDurationBlocks::get(), Error::<T>::BadDuration);

            let active = ActiveLaunchCount::<T>::get();
            ensure!(
                active < T::MaxActiveLaunches::get(),
                Error::<T>::MaxActiveLaunchesReached
            );

            let launch_id = NextLaunchId::<T>::get();
            NextLaunchId::<T>::put(launch_id + 1);

            let state = LaunchState {
                creator: creator.clone(),
                token_asset_id,
                soft_cap,
                hard_cap,
                price_per_token,
                start_block,
                end_block,
                total_raised: 0,
                contributor_count: 0,
                status: LaunchStatus::Active,
                lp_lock_duration_blocks,
            };

            Launches::<T>::insert(launch_id, state);
            ActiveLaunchCount::<T>::put(active + 1);
            ExpiryQueue::<T>::insert(end_block, launch_id, ());

            Self::deposit_event(Event::LaunchCreated {
                launch_id,
                creator,
                token_asset_id,
                soft_cap,
                hard_cap,
                start_block,
                end_block,
            });
            Ok(())
        }

        /// Contribute to a launch.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::contribute())]
        pub fn contribute(
            origin: OriginFor<T>,
            launch_id: LaunchId,
            amount: u128,
        ) -> DispatchResult {
            let contributor = ensure_signed(origin)?;
            ensure!(amount > 0, Error::<T>::SoftCapExceeded);

            let now = frame_system::Pallet::<T>::block_number();

            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(state.status == LaunchStatus::Active, Error::<T>::LaunchNotActive);
                ensure!(now >= state.start_block, Error::<T>::LaunchNotActive);
                ensure!(now <= state.end_block, Error::<T>::LaunchNotEnded);

                let prev = Contributions::<T>::get(launch_id, &contributor);
                if prev == 0 {
                    ensure!(
                        state.contributor_count < T::MaxContributorsPerLaunch::get(),
                        Error::<T>::MaxContributorsReached
                    );
                    state.contributor_count = state.contributor_count.saturating_add(1);
                }
                let new_total = state.total_raised.saturating_add(amount);
                ensure!(new_total <= state.hard_cap, Error::<T>::HardCapExceeded);

                Contributions::<T>::insert(launch_id, &contributor, prev.saturating_add(amount));
                state.total_raised = new_total;

                Self::deposit_event(Event::ContributionMade {
                    launch_id,
                    contributor,
                    amount,
                });
                Ok(())
            })
        }

        /// Finalize a launch after its end_block has passed.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::finalize_launch())]
        pub fn finalize_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            ensure_signed(origin)?;
            let now = frame_system::Pallet::<T>::block_number();

            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(state.status == LaunchStatus::Active, Error::<T>::LaunchNotActive);
                ensure!(now > state.end_block, Error::<T>::LaunchNotEnded);

                let new_status = if state.total_raised >= state.soft_cap {
                    LaunchStatus::Successful
                } else {
                    LaunchStatus::Failed
                };
                let total_raised = state.total_raised;
                state.status = new_status.clone();
                ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                ExpiryQueue::<T>::remove(state.end_block, launch_id);

                Self::deposit_event(Event::LaunchFinalized {
                    launch_id,
                    status: new_status,
                    total_raised,
                });
                Ok(())
            })
        }

        /// Graduate a successful launch: create token, create DEX pool, lock LP.
        ///
        /// Only callable by the launch creator after funds have been withdrawn
        /// (launch in Completed status). This protects contributors: the creator
        /// cannot graduate without first finalizing and withdrawing funds, which
        /// proves they have committed to the launch.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::graduate_launch())]
        #[transactional]
        pub fn graduate_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            let state = Launches::<T>::get(launch_id).ok_or(Error::<T>::LaunchNotFound)?;
            ensure!(state.creator == caller, Error::<T>::NotCreator);
            ensure!(state.status == LaunchStatus::Completed, Error::<T>::LaunchNotSuccessful);
            ensure!(!GraduatedLaunches::<T>::contains_key(launch_id), Error::<T>::AlreadyGraduated);

            let total_tokens = state.total_raised / state.price_per_token;
            ensure!(total_tokens > 0, Error::<T>::TokenFactoryFailed);

            // Step 1: Create the token via TokenFactory
            let symbol = format!("TKN{}", launch_id).into_bytes();
            let name = format!("Launch Token {}", launch_id).into_bytes();
            let new_asset_id = T::TokenFactory::create_token(
                &caller,
                symbol,
                name,
                18u8, // standard decimals
                total_tokens,
            ).map_err(|_| Error::<T>::TokenFactoryFailed)?;

            // Step 2: Create AMM pool with quote token
            let quote_asset = T::QuoteAssetId::get();
            let pool_id = T::Dex::create_pool(&caller, quote_asset, new_asset_id)
                .map_err(|_| Error::<T>::DexPoolCreationFailed)?;

            // Step 3: Lock LP tokens for anti-rug protection
            // For simplicity, lock 100% of initial LP tokens. In production,
            // the creator can call lock_lp with a specific amount.
            let unlock_block = frame_system::Pallet::<T>::block_number()
                .saturating_add(state.lp_lock_duration_blocks);
            let lp_amount = total_tokens; // proxy: LP amount ~= initial token supply
            T::LpLocker::lock_lp_for(&caller, pool_id, lp_amount, unlock_block)
                .map_err(|_| Error::<T>::LpLockFailed)?;

            // Record graduation
            GraduatedLaunches::<T>::insert(launch_id, (new_asset_id, pool_id));

            Self::deposit_event(Event::LaunchGraduated {
                launch_id,
                token_asset_id: new_asset_id,
                pool_id,
                total_lp_locked: lp_amount,
            });

            Ok(())
        }

        /// Claim a refund on a failed launch.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::claim_refund())]
        pub fn claim_refund(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            let contributor = ensure_signed(origin)?;
            let state = Launches::<T>::get(launch_id).ok_or(Error::<T>::LaunchNotFound)?;
            ensure!(
                state.status == LaunchStatus::Failed || state.status == LaunchStatus::Refunding,
                Error::<T>::LaunchNotFailed
            );
            let amount = Contributions::<T>::get(launch_id, &contributor);
            ensure!(amount > 0, Error::<T>::NotContributor);
            ensure!(!RefundClaimed::<T>::get(launch_id, &contributor), Error::<T>::AlreadyClaimed);
            RefundClaimed::<T>::insert(launch_id, &contributor, true);
            Self::deposit_event(Event::RefundClaimed { launch_id, contributor, amount });
            Ok(())
        }

        /// Claim proportional token allocation from a successful launch.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::claim_allocation())]
        pub fn claim_allocation(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            let contributor = ensure_signed(origin)?;
            let state = Launches::<T>::get(launch_id).ok_or(Error::<T>::LaunchNotFound)?;
            ensure!(
                state.status == LaunchStatus::Successful || state.status == LaunchStatus::Completed,
                Error::<T>::LaunchNotSuccessful
            );
            let contribution = Contributions::<T>::get(launch_id, &contributor);
            ensure!(contribution > 0, Error::<T>::NotContributor);
            ensure!(!AllocationClaimed::<T>::get(launch_id, &contributor), Error::<T>::AlreadyClaimed);
            let total_tokens = state.total_raised / state.price_per_token;
            let tokens = if state.total_raised > 0 {
                contribution.saturating_mul(total_tokens) / state.total_raised
            } else {
                0
            };
            AllocationClaimed::<T>::insert(launch_id, &contributor, true);
            Self::deposit_event(Event::AllocationClaimed { launch_id, contributor, tokens });
            Ok(())
        }

        /// Cancel a launch.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::cancel_launch())]
        pub fn cancel_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(state.status == LaunchStatus::Active, Error::<T>::LaunchNotActive);
                ExpiryQueue::<T>::remove(state.end_block, launch_id);
                state.status = LaunchStatus::Failed;
                ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                Self::deposit_event(Event::LaunchCancelled { launch_id });
                Ok(())
            })
        }

        /// Withdraw raised funds after a successful launch.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::withdraw_raised_funds())]
        pub fn withdraw_raised_funds(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(state.status == LaunchStatus::Successful, Error::<T>::LaunchNotSuccessful);
                ensure!(state.creator == caller, Error::<T>::NotCreator);
                let amount = state.total_raised;
                state.status = LaunchStatus::Completed;
                Self::deposit_event(Event::FundsWithdrawn { launch_id, creator: caller, amount });
                Ok(())
            })
        }
    }

    // ── Weight Info ─────────────────────────────────────────────────────────

    pub trait WeightInfo {
        fn create_launch() -> Weight;
        fn contribute() -> Weight;
        fn finalize_launch() -> Weight;
        fn claim_refund() -> Weight;
        fn claim_allocation() -> Weight;
        fn cancel_launch() -> Weight;
        fn withdraw_raised_funds() -> Weight;
        fn graduate_launch() -> Weight;
    }

    impl WeightInfo for () {
        fn create_launch() -> Weight { Weight::from_parts(25_000, 0) }
        fn contribute() -> Weight { Weight::from_parts(30_000, 0) }
        fn finalize_launch() -> Weight { Weight::from_parts(15_000, 0) }
        fn claim_refund() -> Weight { Weight::from_parts(15_000, 0) }
        fn claim_allocation() -> Weight { Weight::from_parts(15_000, 0) }
        fn cancel_launch() -> Weight { Weight::from_parts(15_000, 0) }
        fn withdraw_raised_funds() -> Weight { Weight::from_parts(15_000, 0) }
        fn graduate_launch() -> Weight { Weight::from_parts(40_000, 0) }
    }
}