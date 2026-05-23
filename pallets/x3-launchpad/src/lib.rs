#![deny(unsafe_code)]
//! # X3 Launchpad Pallet
//!
//! Phase 7 scaffold — governance-gated token presale (launchpad) pallet.
//! Projects submit a launch with soft/hard caps; contributors lock funds during
//! the fundraising window; the pallet auto-finalizes on expiry and opens either
//! proportional token allocation claims or contributor refunds.
//!
//! ## Invariants
//!
//! - LAUNCH-001: ActiveLaunchCount never exceeds MaxActiveLaunches.
//! - LAUNCH-002: Contributions never push total_raised above hard_cap.
//! - LAUNCH-003: Refunds are only claimable when status is Failed or Refunding.
//! - LAUNCH-004: Allocations are only claimable when status is Successful or Completed.
//! - LAUNCH-005: on_initialize auto-finalizes all launches whose end_block <= now.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

// ── Public types ───────────────────────────────────────────────────────────────

pub type LaunchId = u64;

#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum LaunchStatus {
    Pending,
    Active,
    Successful,
    Failed,
    Refunding,
    Completed,
}

#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct LaunchState<AccountId, BlockNumber> {
    pub launch_id: LaunchId,
    pub creator: AccountId,
    pub token_asset_id: u32,
    pub soft_cap: u128,
    pub hard_cap: u128,
    pub total_raised: u128,
    pub contributor_count: u32,
    pub start_block: BlockNumber,
    pub end_block: BlockNumber,
    pub status: LaunchStatus,
    pub price_per_token: u128,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Saturating;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ── Config ─────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
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

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ────────────────────────────────────────────────────────────────

    /// All launch states keyed by LaunchId.
    #[pallet::storage]
    #[pallet::getter(fn launches)]
    pub type Launches<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        LaunchId,
        LaunchState<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Contribution amounts per (launch, contributor).
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

    /// Monotonically-increasing launch identifier.
    #[pallet::storage]
    #[pallet::getter(fn next_launch_id)]
    pub type NextLaunchId<T: Config> = StorageValue<_, LaunchId, ValueQuery>;

    /// Running count of Active launches.
    #[pallet::storage]
    #[pallet::getter(fn active_launch_count)]
    pub type ActiveLaunchCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Expiry index: (end_block, launch_id) → () for O(1) per-block processing.
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

    /// Tracks accounts that have already claimed their allocation.
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

    /// Tracks accounts that have already claimed their refund.
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

    // ── Events ─────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new launch was created by governance.
        LaunchCreated {
            launch_id: LaunchId,
            creator: T::AccountId,
            token_asset_id: u32,
            soft_cap: u128,
            hard_cap: u128,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        },
        /// A contribution was made to a launch.
        ContributionMade {
            launch_id: LaunchId,
            contributor: T::AccountId,
            amount: u128,
        },
        /// A launch was finalized (either Successful or Failed).
        LaunchFinalized {
            launch_id: LaunchId,
            status: LaunchStatus,
            total_raised: u128,
        },
        /// A contributor claimed a refund.
        RefundClaimed {
            launch_id: LaunchId,
            contributor: T::AccountId,
            amount: u128,
        },
        /// A contributor claimed their token allocation.
        AllocationClaimed {
            launch_id: LaunchId,
            contributor: T::AccountId,
            tokens: u128,
        },
        /// A launch was cancelled by governance.
        LaunchCancelled { launch_id: LaunchId },
        /// The creator withdrew raised funds from a successful launch.
        FundsWithdrawn {
            launch_id: LaunchId,
            creator: T::AccountId,
            amount: u128,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No launch exists for the given id.
        LaunchNotFound,
        /// Launch is not in Active status.
        LaunchNotActive,
        /// Launch has not yet ended.
        LaunchNotEnded,
        /// Contribution would exceed the hard cap.
        HardCapExceeded,
        /// Total raised did not reach the soft cap.
        SoftCapNotMet,
        /// Caller has no contribution recorded.
        NotContributor,
        /// Caller has already claimed their refund or allocation.
        AlreadyClaimed,
        /// Duration is outside [MinLaunchDurationBlocks, MaxLaunchDurationBlocks].
        DurationOutOfBounds,
        /// Active launch count at cap.
        MaxLaunchesReached,
        /// Launch is not in a Failed/Refunding state.
        LaunchNotFailed,
        /// Launch is not in a Successful/Completed state.
        LaunchNotSuccessful,
        /// Caller is not the launch creator.
        NotCreator,
        /// Caps are invalid (soft_cap == 0, hard_cap < soft_cap, or price_per_token == 0).
        InvalidLaunchParams,
    }

    // ── Hooks ──────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Auto-finalize all launches that expire at `now`.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut reads: u64 = 1;
            let mut writes: u64 = 0;

            let expired: sp_std::vec::Vec<LaunchId> = ExpiryQueue::<T>::iter_prefix(now)
                .map(|(id, _)| id)
                .collect();

            for launch_id in &expired {
                reads += 1;
                if let Some(mut state) = Launches::<T>::get(launch_id) {
                    if state.status == LaunchStatus::Active {
                        let new_status = if state.total_raised >= state.soft_cap {
                            LaunchStatus::Successful
                        } else {
                            LaunchStatus::Failed
                        };
                        let total_raised = state.total_raised;
                        state.status = new_status.clone();
                        Launches::<T>::insert(launch_id, &state);
                        ActiveLaunchCount::<T>::mutate(|c| {
                            *c = c.saturating_sub(1);
                        });
                        writes += 2;
                        Self::deposit_event(Event::LaunchFinalized {
                            launch_id: *launch_id,
                            status: new_status,
                            total_raised,
                        });
                    }
                }
                ExpiryQueue::<T>::remove(now, launch_id);
                writes += 1;
            }

            T::DbWeight::get()
                .reads(reads)
                .saturating_add(T::DbWeight::get().writes(writes))
        }
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new token launch.
        ///
        /// Only GovernanceOrigin may call this. `creator` is the AccountId that will
        /// receive raised funds via `withdraw_raised_funds`. `start_block` and
        /// `end_block` must satisfy the configured duration bounds.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_launch())]
        pub fn create_launch(
            origin: OriginFor<T>,
            creator: T::AccountId,
            token_asset_id: u32,
            soft_cap: u128,
            hard_cap: u128,
            price_per_token: u128,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            ensure!(
                soft_cap > 0 && hard_cap >= soft_cap && price_per_token > 0,
                Error::<T>::InvalidLaunchParams
            );

            let duration = end_block.saturating_sub(start_block);
            ensure!(
                duration >= T::MinLaunchDurationBlocks::get()
                    && duration <= T::MaxLaunchDurationBlocks::get(),
                Error::<T>::DurationOutOfBounds
            );
            ensure!(
                ActiveLaunchCount::<T>::get() < T::MaxActiveLaunches::get(),
                Error::<T>::MaxLaunchesReached
            );

            let launch_id = NextLaunchId::<T>::get();

            let state = LaunchState {
                launch_id,
                creator: creator.clone(),
                token_asset_id,
                soft_cap,
                hard_cap,
                total_raised: 0,
                contributor_count: 0,
                start_block,
                end_block,
                status: LaunchStatus::Active,
                price_per_token,
            };

            Launches::<T>::insert(launch_id, &state);
            ExpiryQueue::<T>::insert(end_block, launch_id, ());
            ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_add(1));
            NextLaunchId::<T>::put(launch_id.saturating_add(1));

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

        /// Contribute to an active launch.
        ///
        /// Any signed origin may call this during the fundraising window.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::contribute())]
        pub fn contribute(
            origin: OriginFor<T>,
            launch_id: LaunchId,
            amount: u128,
        ) -> DispatchResult {
            let contributor = ensure_signed(origin)?;

            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(
                    state.status == LaunchStatus::Active,
                    Error::<T>::LaunchNotActive
                );

                let new_total = state
                    .total_raised
                    .checked_add(amount)
                    .ok_or(Error::<T>::HardCapExceeded)?;
                ensure!(new_total <= state.hard_cap, Error::<T>::HardCapExceeded);

                let prev = Contributions::<T>::get(launch_id, &contributor);
                if prev == 0 {
                    state.contributor_count = state.contributor_count.saturating_add(1);
                }
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
        ///
        /// Any origin may call this. Status transitions to Successful or Failed.
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

        /// Claim a refund on a failed launch.
        ///
        /// Only contributors may call this when the launch is Failed or Refunding.
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

            Self::deposit_event(Event::RefundClaimed {
                launch_id,
                contributor,
                amount,
            });
            Ok(())
        }

        /// Claim proportional token allocation from a successful launch.
        ///
        /// Tokens = (contribution / total_raised) * (total_raised / price_per_token).
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

            // Proportional allocation: contributor_share / total_raised * total_tokens.
            // total_tokens = total_raised / price_per_token.
            let total_tokens = state.total_raised / state.price_per_token;
            let tokens = if state.total_raised > 0 {
                contribution.saturating_mul(total_tokens) / state.total_raised
            } else {
                0
            };

            AllocationClaimed::<T>::insert(launch_id, &contributor, true);

            Self::deposit_event(Event::AllocationClaimed {
                launch_id,
                contributor,
                tokens,
            });
            Ok(())
        }

        /// Cancel a launch, transitioning it to Failed (opens refunds).
        ///
        /// Only GovernanceOrigin may call this.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::cancel_launch())]
        pub fn cancel_launch(origin: OriginFor<T>, launch_id: LaunchId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Launches::<T>::try_mutate(launch_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::LaunchNotFound)?;
                ensure!(
                    state.status == LaunchStatus::Active,
                    Error::<T>::LaunchNotActive
                );

                ExpiryQueue::<T>::remove(state.end_block, launch_id);
                state.status = LaunchStatus::Failed;
                ActiveLaunchCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                Self::deposit_event(Event::LaunchCancelled { launch_id });
                Ok(())
            })
        }

        /// Withdraw raised funds after a successful launch.
        ///
        /// Only the launch creator may call this.
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

                let amount = state.total_raised;
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
}
