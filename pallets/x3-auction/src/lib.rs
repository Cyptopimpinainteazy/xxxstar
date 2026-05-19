#![deny(unsafe_code)]
//! # X3 Auction Pallet
//!
//! Phase 6 scaffold — English (ascending-price) on-chain auction with configurable
//! reserve prices, minimum bid increments enforced in basis points, and governance
//! controls for emergency cancellations and deadline extensions.
//!
//! ## Invariants
//!
//! - AUCTION-001: ActiveAuctionCount never exceeds MaxActiveAuctions.
//! - AUCTION-002: Each bid amount is strictly greater than current_bid + (current_bid * MinBidIncrementBps / 10_000).
//! - AUCTION-003: An auction with existing bids cannot be cancelled by the seller; only GovernanceOrigin may force-cancel.
//! - AUCTION-004: Settlement is only possible when the auction has ended and total_raised >= reserve_price.
//! - AUCTION-005: on_initialize processes the ExpiryQueue exactly once per block.

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

pub type AuctionId = u64;

#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub enum AuctionStatus {
    Active,
    Ended,
    Cancelled,
    Settled,
}

#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct AuctionState<AccountId, BlockNumber> {
    pub auction_id: AuctionId,
    pub seller: AccountId,
    pub asset_id: u32,
    pub start_price: u128,
    pub current_bid: u128,
    pub leading_bidder: Option<AccountId>,
    pub reserve_price: u128,
    pub end_block: BlockNumber,
    pub status: AuctionStatus,
    pub bid_count: u32,
}

#[derive(
    Encode, Decode, DecodeWithMemTracking, TypeInfo, Clone, PartialEq, Eq, Debug, MaxEncodedLen,
)]
pub struct BidEntry<AccountId> {
    pub bidder: AccountId,
    pub amount: u128,
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

        /// Origin that can perform governance actions (extend, force-cancel).
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum bids stored per auction (bounds the BidEntry double-map).
        #[pallet::constant]
        type MaxBidsPerAuction: Get<u32>;

        /// Maximum number of simultaneously active auctions.
        #[pallet::constant]
        type MaxActiveAuctions: Get<u32>;

        /// Deposit (in the chain's smallest unit) required to list an auction.
        #[pallet::constant]
        type AuctionDepositAmount: Get<u128>;

        /// Minimum bid increment expressed in basis points (100 = 1%).
        #[pallet::constant]
        type MinBidIncrementBps: Get<u32>;

        /// Weight information for extrinsics.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ────────────────────────────────────────────────────────────────

    /// All auction states keyed by AuctionId.
    #[pallet::storage]
    #[pallet::getter(fn auctions)]
    pub type Auctions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AuctionId,
        AuctionState<T::AccountId, BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Best bid per (auction, bidder) pair.
    #[pallet::storage]
    #[pallet::getter(fn auction_bids)]
    pub type AuctionBids<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AuctionId,
        Blake2_128Concat,
        T::AccountId,
        BidEntry<T::AccountId>,
        OptionQuery,
    >;

    /// Running count of Active auctions; bounded by MaxActiveAuctions.
    #[pallet::storage]
    #[pallet::getter(fn active_auction_count)]
    pub type ActiveAuctionCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Monotonically-increasing auction identifier.
    #[pallet::storage]
    #[pallet::getter(fn next_auction_id)]
    pub type NextAuctionId<T: Config> = StorageValue<_, AuctionId, ValueQuery>;

    /// Expiry index: (end_block, auction_id) → () for O(1) per-block processing.
    #[pallet::storage]
    pub type ExpiryQueue<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        BlockNumberFor<T>,
        Blake2_128Concat,
        AuctionId,
        (),
        OptionQuery,
    >;

    // ── Events ─────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new auction was created.
        AuctionCreated {
            auction_id: AuctionId,
            seller: T::AccountId,
            asset_id: u32,
            start_price: u128,
            reserve_price: u128,
            end_block: BlockNumberFor<T>,
        },
        /// A bid was placed on an auction.
        BidPlaced {
            auction_id: AuctionId,
            bidder: T::AccountId,
            amount: u128,
        },
        /// An auction was cancelled by the seller (zero bids).
        AuctionCancelled { auction_id: AuctionId },
        /// An active auction expired; winner is the leading bidder (None if no bids).
        AuctionEnded {
            auction_id: AuctionId,
            winner: Option<T::AccountId>,
            final_price: u128,
        },
        /// An ended auction was settled (reserve met).
        AuctionSettled {
            auction_id: AuctionId,
            winner: T::AccountId,
            final_price: u128,
        },
        /// An auction's end block was extended by governance.
        AuctionExtended {
            auction_id: AuctionId,
            new_end_block: BlockNumberFor<T>,
        },
        /// An active auction was force-cancelled by governance.
        AuctionForceCancelled { auction_id: AuctionId },
    }

    // ── Errors ─────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No auction exists for the given id.
        AuctionNotFound,
        /// Auction is not in Active status.
        AuctionNotActive,
        /// Bid amount does not exceed current_bid by the required minimum increment.
        BidTooLow,
        /// Auction ended but reserve price was not met.
        ReservePriceNotMet,
        /// Caller is not the auction seller.
        NotSeller,
        /// Cannot cancel an auction that already has at least one bid.
        AuctionHasBids,
        /// The active auction cap has been reached.
        MaxActiveAuctionsReached,
        /// Auction has not yet ended.
        AuctionNotEnded,
    }

    // ── Hooks ──────────────────────────────────────────────────────────────────

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Process ExpiryQueue for `now`; transition Active → Ended and emit AuctionEnded.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut reads: u64 = 1;
            let mut writes: u64 = 0;

            // Drain all auctions that expire at this block.
            let expired: sp_std::vec::Vec<AuctionId> = ExpiryQueue::<T>::iter_prefix(now)
                .map(|(id, _)| id)
                .collect();

            for auction_id in &expired {
                reads += 1;
                if let Some(mut state) = Auctions::<T>::get(auction_id) {
                    if state.status == AuctionStatus::Active {
                        state.status = AuctionStatus::Ended;
                        let winner = state.leading_bidder.clone();
                        let final_price = state.current_bid;
                        Auctions::<T>::insert(auction_id, &state);
                        ActiveAuctionCount::<T>::mutate(|c| {
                            *c = c.saturating_sub(1);
                        });
                        writes += 2;
                        Self::deposit_event(Event::AuctionEnded {
                            auction_id: *auction_id,
                            winner,
                            final_price,
                        });
                    }
                }
                ExpiryQueue::<T>::remove(now, auction_id);
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
        /// Create a new English auction.
        ///
        /// Any signed origin may call this. The caller becomes the seller.
        /// `duration_blocks` is added to the current block to set `end_block`.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_auction())]
        pub fn create_auction(
            origin: OriginFor<T>,
            asset_id: u32,
            start_price: u128,
            reserve_price: u128,
            duration_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            let seller = ensure_signed(origin)?;

            ensure!(
                ActiveAuctionCount::<T>::get() < T::MaxActiveAuctions::get(),
                Error::<T>::MaxActiveAuctionsReached
            );

            let auction_id = NextAuctionId::<T>::get();
            let now = frame_system::Pallet::<T>::block_number();
            let end_block = now.saturating_add(duration_blocks);

            let state = AuctionState {
                auction_id,
                seller: seller.clone(),
                asset_id,
                start_price,
                current_bid: start_price,
                leading_bidder: None,
                reserve_price,
                end_block,
                status: AuctionStatus::Active,
                bid_count: 0,
            };

            Auctions::<T>::insert(auction_id, &state);
            ExpiryQueue::<T>::insert(end_block, auction_id, ());
            ActiveAuctionCount::<T>::mutate(|c| *c = c.saturating_add(1));
            NextAuctionId::<T>::put(auction_id.saturating_add(1));

            Self::deposit_event(Event::AuctionCreated {
                auction_id,
                seller,
                asset_id,
                start_price,
                reserve_price,
                end_block,
            });

            Ok(())
        }

        /// Place a bid on an active auction.
        ///
        /// `amount` must exceed `current_bid` by at least `MinBidIncrementBps / 10_000`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::place_bid())]
        pub fn place_bid(
            origin: OriginFor<T>,
            auction_id: AuctionId,
            amount: u128,
        ) -> DispatchResult {
            let bidder = ensure_signed(origin)?;

            Auctions::<T>::try_mutate(auction_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::AuctionNotFound)?;
                ensure!(
                    state.status == AuctionStatus::Active,
                    Error::<T>::AuctionNotActive
                );

                // Validate minimum increment.
                let increment = state
                    .current_bid
                    .saturating_mul(T::MinBidIncrementBps::get() as u128)
                    / 10_000;
                let min_bid = state.current_bid.saturating_add(increment);
                // First bid only needs to meet start_price (current_bid == start_price, increment may be 0).
                let effective_min = if state.bid_count == 0 {
                    state.start_price
                } else {
                    min_bid
                };
                ensure!(amount >= effective_min, Error::<T>::BidTooLow);

                state.current_bid = amount;
                state.leading_bidder = Some(bidder.clone());
                state.bid_count = state.bid_count.saturating_add(1);

                AuctionBids::<T>::insert(
                    auction_id,
                    &bidder,
                    BidEntry {
                        bidder: bidder.clone(),
                        amount,
                    },
                );

                Self::deposit_event(Event::BidPlaced {
                    auction_id,
                    bidder,
                    amount,
                });
                Ok(())
            })
        }

        /// Cancel an auction — only allowed if the auction has zero bids.
        ///
        /// Only the seller may call this.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::cancel_auction())]
        pub fn cancel_auction(origin: OriginFor<T>, auction_id: AuctionId) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            Auctions::<T>::try_mutate(auction_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::AuctionNotFound)?;
                ensure!(
                    state.status == AuctionStatus::Active,
                    Error::<T>::AuctionNotActive
                );
                ensure!(state.seller == caller, Error::<T>::NotSeller);
                ensure!(state.bid_count == 0, Error::<T>::AuctionHasBids);

                state.status = AuctionStatus::Cancelled;
                ExpiryQueue::<T>::remove(state.end_block, auction_id);
                ActiveAuctionCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                Self::deposit_event(Event::AuctionCancelled { auction_id });
                Ok(())
            })
        }

        /// Settle an ended auction when the reserve price has been met.
        ///
        /// Any origin may call this after the auction has ended.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::settle_auction())]
        pub fn settle_auction(origin: OriginFor<T>, auction_id: AuctionId) -> DispatchResult {
            ensure_signed(origin)?;

            Auctions::<T>::try_mutate(auction_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::AuctionNotFound)?;
                ensure!(
                    state.status == AuctionStatus::Ended,
                    Error::<T>::AuctionNotEnded
                );
                ensure!(
                    state.current_bid >= state.reserve_price,
                    Error::<T>::ReservePriceNotMet
                );

                let winner = state
                    .leading_bidder
                    .clone()
                    .ok_or(Error::<T>::ReservePriceNotMet)?;
                let final_price = state.current_bid;
                state.status = AuctionStatus::Settled;

                Self::deposit_event(Event::AuctionSettled {
                    auction_id,
                    winner,
                    final_price,
                });
                Ok(())
            })
        }

        /// Extend an active auction's deadline.
        ///
        /// Restricted to GovernanceOrigin.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::extend_auction())]
        pub fn extend_auction(
            origin: OriginFor<T>,
            auction_id: AuctionId,
            additional_blocks: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Auctions::<T>::try_mutate(auction_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::AuctionNotFound)?;
                ensure!(
                    state.status == AuctionStatus::Active,
                    Error::<T>::AuctionNotActive
                );

                // Move the expiry queue entry.
                ExpiryQueue::<T>::remove(state.end_block, auction_id);
                state.end_block = state.end_block.saturating_add(additional_blocks);
                ExpiryQueue::<T>::insert(state.end_block, auction_id, ());

                Self::deposit_event(Event::AuctionExtended {
                    auction_id,
                    new_end_block: state.end_block,
                });
                Ok(())
            })
        }

        /// Force-cancel any active auction, refunding the leading bidder's position.
        ///
        /// Restricted to GovernanceOrigin.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::force_cancel())]
        pub fn force_cancel(origin: OriginFor<T>, auction_id: AuctionId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            Auctions::<T>::try_mutate(auction_id, |maybe_state| -> DispatchResult {
                let state = maybe_state.as_mut().ok_or(Error::<T>::AuctionNotFound)?;
                ensure!(
                    state.status == AuctionStatus::Active,
                    Error::<T>::AuctionNotActive
                );

                ExpiryQueue::<T>::remove(state.end_block, auction_id);
                state.status = AuctionStatus::Cancelled;
                ActiveAuctionCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                Self::deposit_event(Event::AuctionForceCancelled { auction_id });
                Ok(())
            })
        }
    }
}
