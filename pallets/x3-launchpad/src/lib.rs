// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-launchpad — Token launch/presale pallet with on-chain graduation.
//
// Every amount a launch handles is a `CanonicalLedger` balance held by the launch's own custody
// account (`launch_account`):
//   * `contribute` pays the quote asset into custody; `claim_refund` pays it back from a failed
//     launch.
//   * `graduate_launch` mints the token into custody (the contributors' allocation plus the pool's
//     side), creates a DEX pool and seeds it from custody with `LiquidityBps` of the raised quote
//     asset and the matching tokens at the launch price. Custody owns the LP position, so nobody
//     can remove that liquidity until `claim_lp` releases it to the creator after
//     `lp_lock_duration_blocks`.
//   * `claim_allocation` pays a contributor's tokens from custody; `withdraw_raised_funds` pays the
//     creator the raised quote asset the pool did not take.
// Before this, every one of those calls only updated counters: nothing moved.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use alloc::format;
    use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
    use frame_support::{pallet_prelude::*, traits::Get, transactional};
    use frame_system::pallet_prelude::*;
    use scale_info::TypeInfo;
    use sp_runtime::traits::Saturating;
    use sp_std::vec::Vec;
    use x3_asset_kernel_types::traits::CanonicalLedgerTransfer;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Types ───────────────────────────────────────────────────────────────

    pub type LaunchId = u32;

    #[derive(
        Clone,
        Eq,
        PartialEq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
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

    /// The DEX a graduating launch seeds: pools of ledger assets, liquidity paid from the
    /// provider's ledger balances, and LP positions owned by an account.
    pub trait DexPoolCreate<AccountId> {
        /// Create a pool of ledger assets `token_a` / `token_b`; returns its id.
        fn create_pool(
            creator: &AccountId,
            token_a: u32,
            token_b: u32,
        ) -> Result<u64, DispatchError>;
        /// Deposit `amount_a` / `amount_b` from `provider` into `pool_id`; returns the LP
        /// position, owned by `provider`.
        fn add_liquidity(
            provider: &AccountId,
            pool_id: u64,
            amount_a: u128,
            amount_b: u128,
        ) -> Result<u64, DispatchError>;
        /// Hand LP position `position_id` from `from` to `to`.
        fn transfer_position(position_id: u64, from: &AccountId, to: &AccountId) -> DispatchResult;
    }

    /// A graduated launch's pool, its locked LP position, and what the graduation set aside.
    #[derive(Clone, Eq, PartialEq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct Graduation<BlockNumber> {
        /// The launched token's ledger asset id.
        pub token_asset: u32,
        pub pool_id: u64,
        /// The LP position custody holds until `unlock_at`.
        pub position_id: u64,
        pub unlock_at: BlockNumber,
        pub lp_claimed: bool,
        /// Quote asset the pool took from the raised funds.
        pub pool_quote: u128,
        /// Tokens set aside for contributors, claimed pro rata to their contributions.
        pub contributor_tokens: u128,
    }

    // ── Pallet ──────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin that can create and cancel launches.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

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

        /// The ledger launches hold their funds on (the kernel's `CanonicalLedger`).
        type Ledger: x3_asset_kernel_types::traits::CanonicalLedgerTransfer<Self::AccountId>;

        /// Derives each launch's custody account.
        #[pallet::constant]
        type PalletId: Get<frame_support::PalletId>;

        /// Share of the raised quote asset, in basis points, that seeds the DEX pool at
        /// graduation; the creator withdraws the rest.
        #[pallet::constant]
        type LiquidityBps: Get<u16>;

        /// Ledger asset id of the quote token (e.g. X3 native, USDC).
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

    /// Each graduated launch's pool, locked LP position and set-asides.
    #[pallet::storage]
    pub type Graduations<T: Config> =
        StorageMap<_, Blake2_128Concat, LaunchId, Graduation<BlockNumberFor<T>>, OptionQuery>;

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
        /// The locked LP position was released to the creator.
        LpClaimed {
            launch_id: LaunchId,
            creator: T::AccountId,
            position_id: u64,
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
        /// The launch has not graduated yet.
        NotGraduated,
        /// The LP position is still locked.
        LpStillLocked,
        /// The raised funds are too small to seed a pool at this price.
        RaiseTooSmallForPool,
        /// An amount overflowed.
        ArithmeticOverflow,
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
            ensure!(
                soft_cap > 0 && hard_cap >= soft_cap,
                Error::<T>::SoftCapExceeded
            );
            ensure!(price_per_token > 0, Error::<T>::BadDuration);
            // A launch that reaches its soft cap must be able to graduate: the pool's share of the
            // smallest successful raise has to buy at least one token at the launch price.
            // Otherwise contributors' funds would sit in custody with no path out.
            let smallest_pool_quote = soft_cap
                .checked_mul(u128::from(T::LiquidityBps::get()))
                .ok_or(Error::<T>::ArithmeticOverflow)?
                / 10_000;
            ensure!(
                smallest_pool_quote >= price_per_token,
                Error::<T>::RaiseTooSmallForPool
            );

            let duration = end_block.saturating_sub(start_block);
            ensure!(
                duration >= T::MinLaunchDurationBlocks::get(),
                Error::<T>::BadDuration
            );
            ensure!(
                duration <= T::MaxLaunchDurationBlocks::get(),
                Error::<T>::BadDuration
            );

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
                ensure!(
                    state.status == LaunchStatus::Active,
                    Error::<T>::LaunchNotActive
                );
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
                let new_total = state
                    .total_raised
                    .checked_add(amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                ensure!(new_total <= state.hard_cap, Error::<T>::HardCapExceeded);
                let contributed = prev
                    .checked_add(amount)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;

                // The contribution is paid into the launch's custody, not just counted.
                T::Ledger::ledger_transfer(
                    T::QuoteAssetId::get(),
                    &contributor,
                    &Self::launch_account(launch_id),
                    amount,
                )?;
                Contributions::<T>::insert(launch_id, &contributor, contributed);
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
                ensure!(
                    state.status == LaunchStatus::Active,
                    Error::<T>::LaunchNotActive
                );
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

        /// Graduate a successful launch: mint its token into custody, create a DEX pool and seed
        /// it from custody with `LiquidityBps` of the raised funds plus the matching tokens at the
        /// launch price. Custody owns the LP position until `claim_lp`.
        ///
        /// Any signed account, on a `Successful` launch: the outcome is fixed by the raise, so a
        /// creator who walks away cannot hold contributors' funds hostage. After it, contributors
        /// `claim_allocation` and the creator `withdraw_raised_funds` (the share the pool did not
        /// take).
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::graduate_launch())]
        #[transactional]
        pub fn graduate_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            ensure_signed(origin)?;

            let state = Launches::<T>::get(launch_id).ok_or(Error::<T>::LaunchNotFound)?;
            ensure!(
                state.status == LaunchStatus::Successful,
                Error::<T>::LaunchNotSuccessful
            );
            ensure!(
                !GraduatedLaunches::<T>::contains_key(launch_id),
                Error::<T>::AlreadyGraduated
            );

            // The split, at the launch price: contributors' tokens for everything raised, and the
            // pool's side for `LiquidityBps` of it.
            let raised = state.total_raised;
            let price = state.price_per_token;
            let contributor_tokens = raised / price;
            let pool_quote = raised
                .checked_mul(u128::from(T::LiquidityBps::get()))
                .ok_or(Error::<T>::ArithmeticOverflow)?
                / 10_000;
            let pool_tokens = pool_quote / price;
            ensure!(
                contributor_tokens > 0 && pool_quote > 0 && pool_tokens > 0,
                Error::<T>::RaiseTooSmallForPool
            );
            let minted = contributor_tokens
                .checked_add(pool_tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?;

            // Step 1: the token, minted into the launch's custody.
            let custody = Self::launch_account(launch_id);
            let symbol = format!("TKN{}", launch_id).into_bytes();
            let name = format!("Launch Token {}", launch_id).into_bytes();
            let token_asset = T::TokenFactory::create_token(&custody, symbol, name, 18u8, minted)
                .map_err(|_| Error::<T>::TokenFactoryFailed)?;

            // Step 2: the pool, seeded from custody; custody owns the LP position.
            let quote_asset = T::QuoteAssetId::get();
            let pool_id = T::Dex::create_pool(&custody, quote_asset, token_asset)
                .map_err(|_| Error::<T>::DexPoolCreationFailed)?;
            let position_id = T::Dex::add_liquidity(&custody, pool_id, pool_quote, pool_tokens)
                .map_err(|_| Error::<T>::DexPoolCreationFailed)?;

            // Step 3: the lock. Custody has no key, so the position cannot be withdrawn by anyone
            // until `claim_lp` hands it to the creator.
            let unlock_at = frame_system::Pallet::<T>::block_number()
                .saturating_add(state.lp_lock_duration_blocks);
            Graduations::<T>::insert(
                launch_id,
                Graduation {
                    token_asset,
                    pool_id,
                    position_id,
                    unlock_at,
                    lp_claimed: false,
                    pool_quote,
                    contributor_tokens,
                },
            );
            GraduatedLaunches::<T>::insert(launch_id, (token_asset, pool_id));

            Self::deposit_event(Event::LaunchGraduated {
                launch_id,
                token_asset_id: token_asset,
                pool_id,
                total_lp_locked: pool_tokens,
            });

            Ok(())
        }

        /// Release a graduated launch's LP position to its creator once the lock has expired.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::graduate_launch())]
        pub fn claim_lp(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let state = Launches::<T>::get(launch_id).ok_or(Error::<T>::LaunchNotFound)?;
            ensure!(state.creator == caller, Error::<T>::NotCreator);
            Graduations::<T>::try_mutate(launch_id, |maybe| -> DispatchResult {
                let graduation = maybe.as_mut().ok_or(Error::<T>::NotGraduated)?;
                ensure!(!graduation.lp_claimed, Error::<T>::AlreadyClaimed);
                ensure!(
                    frame_system::Pallet::<T>::block_number() >= graduation.unlock_at,
                    Error::<T>::LpStillLocked
                );
                T::Dex::transfer_position(
                    graduation.position_id,
                    &Self::launch_account(launch_id),
                    &caller,
                )?;
                graduation.lp_claimed = true;
                Self::deposit_event(Event::LpClaimed {
                    launch_id,
                    creator: caller.clone(),
                    position_id: graduation.position_id,
                });
                Ok(())
            })
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
            ensure!(
                !RefundClaimed::<T>::get(launch_id, &contributor),
                Error::<T>::AlreadyClaimed
            );
            RefundClaimed::<T>::insert(launch_id, &contributor, true);
            T::Ledger::ledger_transfer(
                T::QuoteAssetId::get(),
                &Self::launch_account(launch_id),
                &contributor,
                amount,
            )?;
            Self::deposit_event(Event::RefundClaimed {
                launch_id,
                contributor,
                amount,
            });
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
            ensure!(
                !AllocationClaimed::<T>::get(launch_id, &contributor),
                Error::<T>::AlreadyClaimed
            );
            // Tokens exist once the launch graduates; a contributor's share is pro rata to what
            // they paid in (floor), so the shares never exceed the contributors' set-aside.
            let graduation = Graduations::<T>::get(launch_id).ok_or(Error::<T>::NotGraduated)?;
            let tokens = contribution
                .checked_mul(graduation.contributor_tokens)
                .ok_or(Error::<T>::ArithmeticOverflow)?
                / state.total_raised;
            AllocationClaimed::<T>::insert(launch_id, &contributor, true);
            T::Ledger::ledger_transfer(
                graduation.token_asset,
                &Self::launch_account(launch_id),
                &contributor,
                tokens,
            )?;
            Self::deposit_event(Event::AllocationClaimed {
                launch_id,
                contributor,
                tokens,
            });
            Ok(())
        }

        /// Cancel a launch: an `Active` one, or a `Successful` one that has not graduated (when
        /// graduation cannot complete, e.g. the token factory or DEX refuse it). Either way the
        /// launch becomes `Failed` and every contributor can `claim_refund` from custody.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::cancel_launch())]
        pub fn cancel_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                match state.status {
                    LaunchStatus::Active => {
                        ExpiryQueue::<T>::remove(state.end_block, launch_id);
                        ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                    }
                    LaunchStatus::Successful
                        if !GraduatedLaunches::<T>::contains_key(launch_id) => {}
                    _ => return Err(Error::<T>::LaunchNotActive.into()),
                }
                state.status = LaunchStatus::Failed;
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
                ensure!(
                    state.status == LaunchStatus::Successful,
                    Error::<T>::LaunchNotSuccessful
                );
                ensure!(state.creator == caller, Error::<T>::NotCreator);
                // The pool's share is set aside at graduation, so the creator withdraws after it.
                let graduation =
                    Graduations::<T>::get(launch_id).ok_or(Error::<T>::NotGraduated)?;
                let amount = state
                    .total_raised
                    .checked_sub(graduation.pool_quote)
                    .ok_or(Error::<T>::ArithmeticOverflow)?;
                T::Ledger::ledger_transfer(
                    T::QuoteAssetId::get(),
                    &Self::launch_account(launch_id),
                    &caller,
                    amount,
                )?;
                state.status = LaunchStatus::Completed;
                Self::deposit_event(Event::FundsWithdrawn {
                    launch_id,
                    creator: caller,
                    amount,
                });
                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        /// The account that holds launch `launch_id`'s funds, tokens and locked LP position.
        pub fn launch_account(launch_id: LaunchId) -> T::AccountId {
            use sp_runtime::traits::AccountIdConversion;
            T::PalletId::get().into_sub_account_truncating(launch_id)
        }
    }

    // ── Hooks ────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Auto-finalize any launches whose `end_block` has been reached.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut weight = T::WeightInfo::create_launch(); // base weight

            for (launch_id, _) in ExpiryQueue::<T>::drain_prefix(now) {
                let mut consumed = Weight::from_parts(5_000, 0);
                if let Some(mut state) = Launches::<T>::get(launch_id) {
                    let new_status = if state.total_raised >= state.soft_cap {
                        LaunchStatus::Successful
                    } else {
                        LaunchStatus::Failed
                    };
                    state.status = new_status.clone();
                    let total_raised = state.total_raised;
                    ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                    Launches::<T>::insert(launch_id, state);
                    Self::deposit_event(Event::LaunchFinalized {
                        launch_id,
                        status: new_status,
                        total_raised,
                    });
                    consumed = T::WeightInfo::finalize_launch();
                }
                weight = weight.saturating_add(consumed);
            }

            weight
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
        fn create_launch() -> Weight {
            Weight::from_parts(25_000, 0)
        }
        fn contribute() -> Weight {
            Weight::from_parts(30_000, 0)
        }
        fn finalize_launch() -> Weight {
            Weight::from_parts(15_000, 0)
        }
        fn claim_refund() -> Weight {
            Weight::from_parts(15_000, 0)
        }
        fn claim_allocation() -> Weight {
            Weight::from_parts(15_000, 0)
        }
        fn cancel_launch() -> Weight {
            Weight::from_parts(15_000, 0)
        }
        fn withdraw_raised_funds() -> Weight {
            Weight::from_parts(15_000, 0)
        }
        fn graduate_launch() -> Weight {
            Weight::from_parts(40_000, 0)
        }
    }
}
