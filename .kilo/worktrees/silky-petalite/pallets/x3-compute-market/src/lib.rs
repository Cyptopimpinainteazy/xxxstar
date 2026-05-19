#![deny(unsafe_code)]
//! # X3 Compute Market Pallet
//!
//! Phase 10 — Productize AI swarm as service: bot rental, premium execution,
//! and compute marketplace with session-based billing.
//!
//! ## Invariants
//!
//! - COMPUTE-001: A provider must hold `MinStakeForProvider` in `ProviderStake` before creating a listing.
//! - COMPUTE-002: `active_sessions` on a listing never exceeds `capacity_units`.
//! - COMPUTE-003: A session transitions from Active only once (Completed / Expired / Disputed).
//! - COMPUTE-004: `on_initialize` drains ExpiryQueue at `now` and transitions Active sessions to Expired.
//! - COMPUTE-005: Only the listing owner may pause or resume their own listing.

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

// ── Public types ──────────────────────────────────────────────────────────────

pub type ListingId = u64;
pub type SessionId = u64;

/// Compute tier offered by a provider listing.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum ComputeTier {
    /// Single-bot rental session.
    BotRental = 0,
    /// Premium execution with priority scheduling.
    PremiumExecution = 1,
    /// Node in an AI swarm cluster.
    SwarmNode = 2,
    /// Batch inference workload.
    BatchInference = 3,
}

impl Default for ComputeTier {
    fn default() -> Self {
        ComputeTier::BotRental
    }
}

/// Status of a compute listing.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum ListingStatus {
    /// Accepting new sessions.
    Active = 0,
    /// Temporarily not accepting sessions; existing sessions continue.
    Paused = 1,
    /// Permanently removed.
    Delisted = 2,
}

impl Default for ListingStatus {
    fn default() -> Self {
        ListingStatus::Active
    }
}

/// Lifecycle status of a compute session.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum SessionStatus {
    /// Session is running.
    Active = 0,
    /// Provider marked the session done and payment calculated.
    Completed = 1,
    /// Session reached `expiry_block` without being completed.
    Expired = 2,
    /// Renter raised a dispute; awaiting governance resolution.
    Disputed = 3,
}

impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::Active
    }
}

/// On-chain record for a compute listing.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct ComputeListing<AccountId, BlockNumber> {
    pub listing_id: ListingId,
    pub provider: AccountId,
    pub tier: ComputeTier,
    /// Price paid per block by the renter.
    pub price_per_block: u128,
    /// Maximum number of concurrent active sessions.
    pub capacity_units: u32,
    /// Current number of active sessions.
    pub active_sessions: u32,
    pub status: ListingStatus,
    pub created_at: BlockNumber,
    /// Cumulative revenue earned (informational; actual transfers are off-chain in Phase 10).
    pub total_earned: u128,
}

/// On-chain record for a compute session.
#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct ComputeSession<AccountId, BlockNumber> {
    pub session_id: SessionId,
    pub listing_id: ListingId,
    pub renter: AccountId,
    pub provider: AccountId,
    pub tier: ComputeTier,
    pub price_per_block: u128,
    pub start_block: BlockNumber,
    pub expiry_block: BlockNumber,
    pub status: SessionStatus,
    /// Running total paid (set on completion/expiry).
    pub total_paid: u128,
}

// ── Pallet ────────────────────────────────────────────────────────────────────

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

    // ── Config ────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Origin that can perform governance actions (resolve disputes, etc.).
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of listings that may be Active or Paused at once.
        #[pallet::constant]
        type MaxActiveListings: Get<u32>;

        /// Maximum number of sessions tracked per provider.
        #[pallet::constant]
        type MaxSessionsPerProvider: Get<u32>;

        /// Default session expiry window in blocks when renting.
        #[pallet::constant]
        type SessionExpiryBlocks: Get<BlockNumberFor<Self>>;

        /// Minimum stake (in smallest denomination) a provider must hold to create listings.
        #[pallet::constant]
        type MinStakeForProvider: Get<u128>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ───────────────────────────────────────────────────────────────

    /// All compute listings keyed by ListingId.
    #[pallet::storage]
    #[pallet::getter(fn compute_listings)]
    pub type ComputeListings<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ListingId,
        ComputeListing<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// All compute sessions keyed by SessionId.
    #[pallet::storage]
    #[pallet::getter(fn active_sessions)]
    pub type ActiveSessions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        SessionId,
        ComputeSession<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Sessions associated with each provider (bounded vec).
    #[pallet::storage]
    #[pallet::getter(fn provider_sessions)]
    pub type ProviderSessions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<SessionId, T::MaxSessionsPerProvider>,
        ValueQuery,
    >;

    /// Staked balance per provider.
    #[pallet::storage]
    #[pallet::getter(fn provider_stake)]
    pub type ProviderStake<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    /// Running total of compute revenue across all sessions.
    #[pallet::storage]
    #[pallet::getter(fn total_compute_revenue)]
    pub type TotalComputeRevenue<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Monotonically-increasing listing identifier.
    #[pallet::storage]
    #[pallet::getter(fn next_listing_id)]
    pub type NextListingId<T: Config> = StorageValue<_, ListingId, ValueQuery>;

    /// Monotonically-increasing session identifier.
    #[pallet::storage]
    #[pallet::getter(fn next_session_id)]
    pub type NextSessionId<T: Config> = StorageValue<_, SessionId, ValueQuery>;

    /// Expiry index: (expiry_block, session_id) → () for O(n) per-block processing.
    #[pallet::storage]
    pub type ExpiryQueue<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        SessionId,
        (),
        OptionQuery,
    >;

    /// Running count of Active or Paused listings; bounded by MaxActiveListings.
    #[pallet::storage]
    #[pallet::getter(fn active_listing_count)]
    pub type ActiveListingCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A provider staked tokens to qualify for listing.
        ProviderStaked { provider: T::AccountId, amount: u128 },
        /// A new compute listing was created.
        ListingCreated {
            listing_id: ListingId,
            provider: T::AccountId,
            tier: ComputeTier,
            price_per_block: u128,
            capacity_units: u32,
        },
        /// A listing was paused by its provider.
        ListingPaused { listing_id: ListingId, provider: T::AccountId },
        /// A listing was resumed by its provider.
        ListingResumed { listing_id: ListingId, provider: T::AccountId },
        /// A compute session was started by a renter.
        SessionStarted {
            session_id: SessionId,
            listing_id: ListingId,
            renter: T::AccountId,
            expiry_block: BlockNumberFor<T>,
        },
        /// A session was completed and payment calculated.
        SessionCompleted { session_id: SessionId, provider: T::AccountId, total_paid: u128 },
        /// A session expired via on_initialize without being completed.
        SessionExpired { session_id: SessionId },
        /// A renter opened a dispute on their session.
        SessionDisputed { session_id: SessionId, renter: T::AccountId },
        /// A governance origin resolved a dispute.
        DisputeResolved { session_id: SessionId, renter_wins: bool },
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No listing found for the given id.
        ListingNotFound,
        /// No session found for the given id.
        SessionNotFound,
        /// Provider does not hold enough stake to create a listing.
        InsufficientStake,
        /// All capacity_units on the listing are occupied.
        CapacityFull,
        /// Caller is not the owner of the listing.
        NotListingOwner,
        /// Session is not in Active status.
        SessionNotActive,
        /// Listing is not in Active status.
        ListingNotActive,
        /// Provider has reached MaxSessionsPerProvider.
        MaxSessionsReached,
        /// Global listing cap (MaxActiveListings) has been reached.
        MaxListingsReached,
    }

    // ── Hooks ─────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Drain ExpiryQueue at `now`; transition Active sessions to Expired.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut reads: u64 = 1;
            let mut writes: u64 = 0;

            let expired_ids: sp_std::vec::Vec<SessionId> =
                ExpiryQueue::<T>::iter_prefix(now).map(|(id, _)| id).collect();

            for session_id in &expired_ids {
                reads += 1;
                if let Some(mut session) = ActiveSessions::<T>::get(session_id) {
                    if session.status == SessionStatus::Active {
                        session.status = SessionStatus::Expired;
                        // Decrement listing active_sessions count.
                        if let Some(mut listing) =
                            ComputeListings::<T>::get(session.listing_id)
                        {
                            listing.active_sessions =
                                listing.active_sessions.saturating_sub(1);
                            ComputeListings::<T>::insert(session.listing_id, &listing);
                            writes += 1;
                        }
                        ActiveSessions::<T>::insert(session_id, &session);
                        writes += 1;
                        Self::deposit_event(Event::SessionExpired {
                            session_id: *session_id,
                        });
                    }
                }
                ExpiryQueue::<T>::remove(now, session_id);
                writes += 1;
            }

            T::DbWeight::get()
                .reads(reads)
                .saturating_add(T::DbWeight::get().writes(writes))
        }
    }

    // ── Extrinsics ────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Stake tokens as a compute provider.
        ///
        /// Any signed origin may call. The `amount` is added to the caller's `ProviderStake`
        /// record (bookkeeping only in Phase 10; actual token lock is scheduled for Phase 11).
        /// The provider must have a cumulative stake >= `MinStakeForProvider` before listing.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::stake_as_provider())]
        pub fn stake_as_provider(origin: OriginFor<T>, amount: u128) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ProviderStake::<T>::mutate(&who, |stake| {
                *stake = stake.saturating_add(amount);
            });
            Self::deposit_event(Event::ProviderStaked { provider: who, amount });
            Ok(())
        }

        /// Create a new compute listing.
        ///
        /// The caller must have a `ProviderStake` >= `MinStakeForProvider`.
        /// `capacity_units` controls the maximum concurrent sessions.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::create_listing())]
        pub fn create_listing(
            origin: OriginFor<T>,
            tier: ComputeTier,
            price_per_block: u128,
            capacity_units: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                ProviderStake::<T>::get(&who) >= T::MinStakeForProvider::get(),
                Error::<T>::InsufficientStake
            );

            ActiveListingCount::<T>::try_mutate(|count| -> DispatchResult {
                ensure!(
                    *count < T::MaxActiveListings::get(),
                    Error::<T>::MaxListingsReached
                );
                *count = count.saturating_add(1);
                Ok(())
            })?;

            let listing_id = NextListingId::<T>::get();
            NextListingId::<T>::put(listing_id.saturating_add(1));

            let now = frame_system::Pallet::<T>::block_number();
            let listing = ComputeListing {
                listing_id,
                provider: who.clone(),
                tier: tier.clone(),
                price_per_block,
                capacity_units,
                active_sessions: 0,
                status: ListingStatus::Active,
                created_at: now,
                total_earned: 0,
            };
            ComputeListings::<T>::insert(listing_id, &listing);

            Self::deposit_event(Event::ListingCreated {
                listing_id,
                provider: who,
                tier,
                price_per_block,
                capacity_units,
            });
            Ok(())
        }

        /// Pause a listing — the caller must be the listing's provider.
        ///
        /// No new sessions will be accepted while Paused; existing sessions continue.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::pause_listing())]
        pub fn pause_listing(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ComputeListings::<T>::try_mutate(listing_id, |maybe_listing| -> DispatchResult {
                let listing = maybe_listing.as_mut().ok_or(Error::<T>::ListingNotFound)?;
                ensure!(listing.provider == who, Error::<T>::NotListingOwner);
                ensure!(
                    listing.status == ListingStatus::Active,
                    Error::<T>::ListingNotActive
                );
                listing.status = ListingStatus::Paused;
                Ok(())
            })?;

            Self::deposit_event(Event::ListingPaused { listing_id, provider: who });
            Ok(())
        }

        /// Resume a paused listing — the caller must be the listing's provider.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::resume_listing())]
        pub fn resume_listing(origin: OriginFor<T>, listing_id: ListingId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ComputeListings::<T>::try_mutate(listing_id, |maybe_listing| -> DispatchResult {
                let listing = maybe_listing.as_mut().ok_or(Error::<T>::ListingNotFound)?;
                ensure!(listing.provider == who, Error::<T>::NotListingOwner);
                ensure!(
                    listing.status == ListingStatus::Paused,
                    Error::<T>::ListingNotActive
                );
                listing.status = ListingStatus::Active;
                Ok(())
            })?;

            Self::deposit_event(Event::ListingResumed { listing_id, provider: who });
            Ok(())
        }

        /// Rent compute capacity from a listing.
        ///
        /// Validates that the listing is Active and has free capacity. Creates a `ComputeSession`
        /// expiring at `current_block + duration_blocks`, adds it to ExpiryQueue, and increments
        /// `active_sessions` on the listing.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::rent_compute())]
        pub fn rent_compute(
            origin: OriginFor<T>,
            listing_id: ListingId,
            duration_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let (provider, tier, price_per_block, expiry_block) =
                ComputeListings::<T>::try_mutate(
                    listing_id,
                    |maybe_listing| -> Result<
                        (T::AccountId, ComputeTier, u128, BlockNumberFor<T>),
                        DispatchError,
                    > {
                        let listing =
                            maybe_listing.as_mut().ok_or(Error::<T>::ListingNotFound)?;
                        ensure!(
                            listing.status == ListingStatus::Active,
                            Error::<T>::ListingNotActive
                        );
                        ensure!(
                            listing.active_sessions < listing.capacity_units,
                            Error::<T>::CapacityFull
                        );
                        listing.active_sessions =
                            listing.active_sessions.saturating_add(1);
                        let now = frame_system::Pallet::<T>::block_number();
                        let expiry = now.saturating_add(duration_blocks);
                        Ok((
                            listing.provider.clone(),
                            listing.tier.clone(),
                            listing.price_per_block,
                            expiry,
                        ))
                    },
                )?;

            let session_id = NextSessionId::<T>::get();
            NextSessionId::<T>::put(session_id.saturating_add(1));

            let now = frame_system::Pallet::<T>::block_number();
            let session = ComputeSession {
                session_id,
                listing_id,
                renter: who.clone(),
                provider: provider.clone(),
                tier,
                price_per_block,
                start_block: now,
                expiry_block,
                status: SessionStatus::Active,
                total_paid: 0,
            };
            ActiveSessions::<T>::insert(session_id, &session);

            // Register session under provider (bounded).
            ProviderSessions::<T>::try_mutate(&provider, |sessions| -> DispatchResult {
                sessions
                    .try_push(session_id)
                    .map_err(|_| Error::<T>::MaxSessionsReached)?;
                Ok(())
            })?;

            // Queue expiry.
            ExpiryQueue::<T>::insert(expiry_block, session_id, ());

            Self::deposit_event(Event::SessionStarted {
                session_id,
                listing_id,
                renter: who,
                expiry_block,
            });
            Ok(())
        }

        /// Complete a session and record payment.
        ///
        /// Only the listing's provider or a GovernanceOrigin may call this.
        /// Payment = `price_per_block * blocks_used` is recorded on-chain; actual transfers
        /// are handled by the settlement layer in Phase 11.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::complete_session())]
        pub fn complete_session(origin: OriginFor<T>, session_id: SessionId) -> DispatchResult {
            // Accept either the provider (signed) or governance.
            let caller = ensure_signed_or_governance::<T>(origin)?;

            let total_paid = ActiveSessions::<T>::try_mutate(
                session_id,
                |maybe_session| -> Result<u128, DispatchError> {
                    let session =
                        maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                    ensure!(
                        session.status == SessionStatus::Active,
                        Error::<T>::SessionNotActive
                    );

                    // Signed caller must be the session provider.
                    if let Some(ref who) = caller {
                        ensure!(*who == session.provider, Error::<T>::NotListingOwner);
                    }

                    let now = frame_system::Pallet::<T>::block_number();
                    let blocks_used = now.saturating_sub(session.start_block);
                    // BlockNumber is a u64 in the mock; saturated cast to u128.
                    let blocks_u128 = TryInto::<u128>::try_into(blocks_used)
                        .unwrap_or(u128::MAX);
                    let paid = session
                        .price_per_block
                        .saturating_mul(blocks_u128);
                    session.total_paid = paid;
                    session.status = SessionStatus::Completed;
                    Ok(paid)
                },
            )?;

            // Read provider and listing_id before second storage mutation.
            let (provider, listing_id) = ActiveSessions::<T>::get(session_id)
                .map(|s| (s.provider, s.listing_id))
                .ok_or(Error::<T>::SessionNotFound)?;

            // Update listing earnings and active_sessions count.
            ComputeListings::<T>::try_mutate(listing_id, |maybe_listing| -> DispatchResult {
                if let Some(listing) = maybe_listing.as_mut() {
                    listing.total_earned =
                        listing.total_earned.saturating_add(total_paid);
                    listing.active_sessions =
                        listing.active_sessions.saturating_sub(1);
                }
                Ok(())
            })?;

            TotalComputeRevenue::<T>::mutate(|rev| {
                *rev = rev.saturating_add(total_paid);
            });

            Self::deposit_event(Event::SessionCompleted {
                session_id,
                provider,
                total_paid,
            });
            Ok(())
        }

        /// Open a dispute on an active session (renter only).
        ///
        /// The GovernanceOrigin resolves disputes via `resolve_dispute`.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::dispute_session())]
        pub fn dispute_session(origin: OriginFor<T>, session_id: SessionId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ActiveSessions::<T>::try_mutate(
                session_id,
                |maybe_session| -> DispatchResult {
                    let session =
                        maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                    ensure!(
                        session.status == SessionStatus::Active,
                        Error::<T>::SessionNotActive
                    );
                    ensure!(session.renter == who, Error::<T>::NotListingOwner);
                    session.status = SessionStatus::Disputed;
                    Ok(())
                },
            )?;

            Self::deposit_event(Event::SessionDisputed { session_id, renter: who });
            Ok(())
        }

        /// Resolve a disputed session.
        ///
        /// Only callable by GovernanceOrigin. If `renter_wins` the session is marked Expired
        /// (no payment); otherwise it is marked Completed with full computed payment.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::resolve_dispute())]
        pub fn resolve_dispute(
            origin: OriginFor<T>,
            session_id: SessionId,
            renter_wins: bool,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            let total_paid = ActiveSessions::<T>::try_mutate(
                session_id,
                |maybe_session| -> Result<u128, DispatchError> {
                    let session =
                        maybe_session.as_mut().ok_or(Error::<T>::SessionNotFound)?;
                    ensure!(
                        session.status == SessionStatus::Disputed,
                        Error::<T>::SessionNotActive
                    );

                    if renter_wins {
                        session.status = SessionStatus::Expired;
                        session.total_paid = 0;
                        Ok(0)
                    } else {
                        // Provider wins: compute full payment.
                        let now = frame_system::Pallet::<T>::block_number();
                        let blocks_used = now.saturating_sub(session.start_block);
                        let blocks_u128 = TryInto::<u128>::try_into(blocks_used)
                            .unwrap_or(u128::MAX);
                        let paid = session.price_per_block.saturating_mul(blocks_u128);
                        session.total_paid = paid;
                        session.status = SessionStatus::Completed;
                        Ok(paid)
                    }
                },
            )?;

            // If provider wins, update listing earnings.
            if !renter_wins && total_paid > 0 {
                if let Some(session) = ActiveSessions::<T>::get(session_id) {
                    ComputeListings::<T>::try_mutate(
                        session.listing_id,
                        |maybe_listing| -> DispatchResult {
                            if let Some(listing) = maybe_listing.as_mut() {
                                listing.total_earned =
                                    listing.total_earned.saturating_add(total_paid);
                                listing.active_sessions =
                                    listing.active_sessions.saturating_sub(1);
                            }
                            Ok(())
                        },
                    )?;
                    TotalComputeRevenue::<T>::mutate(|rev| {
                        *rev = rev.saturating_add(total_paid);
                    });
                }
            } else if renter_wins {
                // Decrement active_sessions even when renter wins.
                if let Some(session) = ActiveSessions::<T>::get(session_id) {
                    ComputeListings::<T>::try_mutate(
                        session.listing_id,
                        |maybe_listing| -> DispatchResult {
                            if let Some(listing) = maybe_listing.as_mut() {
                                listing.active_sessions =
                                    listing.active_sessions.saturating_sub(1);
                            }
                            Ok(())
                        },
                    )?;
                }
            }

            Self::deposit_event(Event::DisputeResolved { session_id, renter_wins });
            Ok(())
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Returns `Ok(Some(AccountId))` for a signed origin, `Ok(None)` for a governance origin,
    /// and `Err` for anything else.
    fn ensure_signed_or_governance<T: Config>(
        origin: OriginFor<T>,
    ) -> Result<Option<T::AccountId>, DispatchError> {
        match ensure_signed(origin.clone()) {
            Ok(who) => Ok(Some(who)),
            Err(_) => {
                T::GovernanceOrigin::ensure_origin(origin)?;
                Ok(None)
            }
        }
    }
}
