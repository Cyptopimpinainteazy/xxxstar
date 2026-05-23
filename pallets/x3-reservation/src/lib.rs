#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # pallet-x3-reservation
//!
//! Reservation engine for the X3 liquidity control plane (TICKET-4.5-005).
//!
//! ## Responsibilities
//!
//! - Issue time-bounded inventory reservations that lock vault capacity for a specific route.
//! - Automatically expire reservations at `expiry_block` via `on_initialize`, restoring vault
//!   balance atomically.
//! - Allow explicit release (pre-expiry cancel) and consumption (settled submission).
//! - Every event carries a `solvency_snapshot_hash` for audit traceability.
//! - Detect frozen lane before reserving (returns `LaneFrozen`).
//! - Increment / decrement the unsettled-notional counters in `pallet-x3-inventory`.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use pallet_x3_inventory::{
        pallet::{Lanes, Pallet as InventoryPallet},
        types::{LaneId, LaneStatus, ReservationId, ReservationStatus, RouteId, VaultId},
    };
    use sp_runtime::traits::CheckedAdd;

    // -----------------------------------------------------------------------
    // ReservationState
    // -----------------------------------------------------------------------

    /// Full lifecycle record for one reservation.
    #[derive(
        Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, TypeInfo,
    )]
    pub struct ReservationState<Balance, BlockNumber> {
        /// Opaque route identifier supplied by the caller.
        pub route_id: RouteId,
        /// Vault whose `available_balance` was locked.
        pub vault_id: VaultId,
        /// Lane this reservation is bound to.
        pub lane_id: LaneId,
        /// Amount locked in the vault.
        pub amount: Balance,
        /// Block at which this reservation auto-expires.
        pub expiry_block: BlockNumber,
        /// Lifecycle stage.
        pub status: ReservationStatus,
        /// Hash of the solvency snapshot taken at reservation time.
        pub solvency_snapshot_hash: [u8; 32],
    }

    // -----------------------------------------------------------------------
    // Pallet definition
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_x3_inventory::pallet::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// How many blocks a reservation lives before auto-expiry if not consumed.
        #[pallet::constant]
        type ReservationTtlBlocks: Get<BlockNumberFor<Self>>;

        /// Maximum number of reservations that auto-expire in a single `on_initialize` call.
        /// Bounds the per-block weight of expiry processing.
        #[pallet::constant]
        type MaxExpirationsPerBlock: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// All active (and recently terminal) reservations, keyed by `ReservationId`.
    #[pallet::storage]
    #[pallet::getter(fn reservation)]
    pub type Reservations<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ReservationId,
        ReservationState<<T as pallet_x3_inventory::pallet::Config>::Balance, BlockNumberFor<T>>,
    >;

    /// Fast lookup: route_id → current reservation_id (one active reservation per route).
    #[pallet::storage]
    #[pallet::getter(fn reservation_by_route)]
    pub type ReservationsByRoute<T: Config> =
        StorageMap<_, Blake2_128Concat, RouteId, ReservationId>;

    /// Expiry queue: (expiry_block, reservation_id) → ().
    /// `on_initialize` iterates all entries whose first key equals the current block.
    #[pallet::storage]
    pub type ExpiryQueue<T: Config> =
        StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Blake2_128Concat, ReservationId, ()>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new reservation was created and inventory locked.
        ReservationCreated {
            reservation_id: ReservationId,
            route_id: RouteId,
            vault_id: VaultId,
            lane_id: LaneId,
            amount: <T as pallet_x3_inventory::pallet::Config>::Balance,
            expiry_block: BlockNumberFor<T>,
            solvency_snapshot_hash: [u8; 32],
        },
        /// A reservation was released explicitly before expiry.
        ReservationReleased {
            reservation_id: ReservationId,
            route_id: RouteId,
            solvency_snapshot_hash: [u8; 32],
        },
        /// A reservation expired and inventory was restored.
        ReservationExpired {
            reservation_id: ReservationId,
            route_id: RouteId,
            solvency_snapshot_hash: [u8; 32],
        },
        /// A reservation was consumed by a successful route submission.
        ReservationConsumed {
            reservation_id: ReservationId,
            route_id: RouteId,
            solvency_snapshot_hash: [u8; 32],
        },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        /// The reservation ID is unknown.
        ReservationNotFound,
        /// The reservation has already expired.
        ReservationExpired,
        /// The reservation was already released.
        ReservationAlreadyReleased,
        /// The reservation was already consumed.
        ReservationAlreadyConsumed,
        /// The reservation is not in `Active` status.
        ReservationNotActive,
        /// The lane is frozen; no reservations accepted.
        LaneFrozen,
        /// A route already has an active reservation.
        RouteAlreadyHasReservation,
        /// Arithmetic overflow in block number calculation.
        BlockNumberOverflow,
    }

    // -----------------------------------------------------------------------
    // Hooks
    // -----------------------------------------------------------------------

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        /// Process expired reservations for the current block.
        ///
        /// Limited to `MaxExpirationsPerBlock` entries to bound per-block weight.
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let max = T::MaxExpirationsPerBlock::get();
            let mut processed: u32 = 0;
            let mut weight = Weight::zero();

            // Drain all expiry entries at `now`.
            let expired_ids: sp_std::vec::Vec<ReservationId> = ExpiryQueue::<T>::iter_prefix(now)
                .take(max as usize)
                .map(|(id, _)| id)
                .collect();

            for res_id in expired_ids {
                ExpiryQueue::<T>::remove(now, res_id);

                Reservations::<T>::mutate(res_id, |maybe| {
                    if let Some(state) = maybe.as_mut() {
                        if state.status == ReservationStatus::Active {
                            state.status = ReservationStatus::Expired;

                            // Restore vault balance.
                            let _ = pallet_x3_inventory::inventory::release_inventory::<T>(
                                state.vault_id,
                                state.amount,
                            );

                            // Decrement unsettled-notional counters.
                            let _ = InventoryPallet::<T>::decrement_unsettled_notional(
                                state.lane_id,
                                state.amount,
                            );

                            // Remove route → reservation mapping.
                            ReservationsByRoute::<T>::remove(state.route_id);

                            Self::deposit_event(Event::ReservationExpired {
                                reservation_id: res_id,
                                route_id: state.route_id,
                                solvency_snapshot_hash: state.solvency_snapshot_hash,
                            });
                        }
                    }
                });

                processed += 1;
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
            }

            let _ = processed; // suppress unused warning if no logging
            weight
        }
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request a new reservation.
        ///
        /// - Checks the lane is not frozen.
        /// - Calls `reserve_inventory` on the target vault.
        /// - Increments unsettled-notional counters.
        /// - Stores the reservation and enqueues its expiry.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn request_reservation(
            origin: OriginFor<T>,
            reservation_id: ReservationId,
            route_id: RouteId,
            vault_id: VaultId,
            lane_id: LaneId,
            amount: <T as pallet_x3_inventory::pallet::Config>::Balance,
            solvency_snapshot_hash: [u8; 32],
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Guard: no two reservations for the same route.
            ensure!(
                !ReservationsByRoute::<T>::contains_key(route_id),
                Error::<T>::RouteAlreadyHasReservation
            );
            // Guard: reservation ID must be fresh.
            ensure!(
                !Reservations::<T>::contains_key(reservation_id),
                Error::<T>::RouteAlreadyHasReservation
            );

            // Guard: lane must not be frozen.
            if let Some(lane) = Lanes::<T>::get(lane_id) {
                ensure!(lane.status != LaneStatus::Frozen, Error::<T>::LaneFrozen);
            }

            // Lock inventory in the vault (propagates VaultNotFound / VaultFrozen /
            // InsufficientAvailableBalance from x3-inventory).
            pallet_x3_inventory::inventory::reserve_inventory::<T>(vault_id, amount)?;

            // Increment unsettled-notional (propagates UnsettledCapExceeded).
            InventoryPallet::<T>::increment_unsettled_notional(lane_id, amount)?;

            // Calculate expiry block.
            let now = <frame_system::Pallet<T>>::block_number();
            let ttl = T::ReservationTtlBlocks::get();
            let expiry_block = now
                .checked_add(&ttl)
                .ok_or(Error::<T>::BlockNumberOverflow)?;

            let state = ReservationState {
                route_id,
                vault_id,
                lane_id,
                amount,
                expiry_block,
                status: ReservationStatus::Active,
                solvency_snapshot_hash,
            };

            Reservations::<T>::insert(reservation_id, state);
            ReservationsByRoute::<T>::insert(route_id, reservation_id);
            ExpiryQueue::<T>::insert(expiry_block, reservation_id, ());

            Self::deposit_event(Event::ReservationCreated {
                reservation_id,
                route_id,
                vault_id,
                lane_id,
                amount,
                expiry_block,
                solvency_snapshot_hash,
            });
            Ok(())
        }

        /// Explicitly release a reservation before expiry (pre-expiry cancel).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn release_reservation(
            origin: OriginFor<T>,
            reservation_id: ReservationId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Self::do_release(reservation_id, ReservationStatus::Released)?;
            Ok(())
        }

        /// Consume a reservation after a successful route submission.
        ///
        /// Transitions the reserved balance to `pending_out` in the vault.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn consume_reservation(
            origin: OriginFor<T>,
            reservation_id: ReservationId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let state =
                Reservations::<T>::get(reservation_id).ok_or(Error::<T>::ReservationNotFound)?;

            match state.status {
                ReservationStatus::Active => {}
                ReservationStatus::Expired => return Err(Error::<T>::ReservationExpired.into()),
                ReservationStatus::Released => {
                    return Err(Error::<T>::ReservationAlreadyReleased.into())
                }
                ReservationStatus::Consumed => {
                    return Err(Error::<T>::ReservationAlreadyConsumed.into())
                }
            }

            // Move reserved → pending_out in the vault.
            // Semantically: release from reserve, then immediately record as pending_out.
            pallet_x3_inventory::inventory::release_inventory::<T>(state.vault_id, state.amount)?;
            pallet_x3_inventory::inventory::record_pending_out::<T>(state.vault_id, state.amount)?;

            // Decrement unsettled-notional.
            let _ = InventoryPallet::<T>::decrement_unsettled_notional(state.lane_id, state.amount);

            // Remove expiry entry (already consumed; no need to expire).
            ExpiryQueue::<T>::remove(state.expiry_block, reservation_id);

            let (route_id, snapshot_hash) = (state.route_id, state.solvency_snapshot_hash);

            Reservations::<T>::mutate(reservation_id, |s| {
                if let Some(s) = s.as_mut() {
                    s.status = ReservationStatus::Consumed;
                }
            });
            ReservationsByRoute::<T>::remove(route_id);

            Self::deposit_event(Event::ReservationConsumed {
                reservation_id,
                route_id,
                solvency_snapshot_hash: snapshot_hash,
            });
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // Public helpers (callable by other pallets / solvency gates)
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        /// Returns `true` iff the reservation exists and is in `Active` status and has not
        /// passed its expiry block.
        pub fn is_reservation_valid(reservation_id: ReservationId) -> bool {
            if let Some(state) = Reservations::<T>::get(reservation_id) {
                let now = <frame_system::Pallet<T>>::block_number();
                state.status == ReservationStatus::Active && now < state.expiry_block
            } else {
                false
            }
        }

        // --- Internal helpers ---

        /// Common implementation for releasing or expiring a reservation.
        fn do_release(
            reservation_id: ReservationId,
            terminal_status: ReservationStatus,
        ) -> DispatchResult {
            let state =
                Reservations::<T>::get(reservation_id).ok_or(Error::<T>::ReservationNotFound)?;

            match state.status {
                ReservationStatus::Active => {}
                ReservationStatus::Expired => return Err(Error::<T>::ReservationExpired.into()),
                ReservationStatus::Released => {
                    return Err(Error::<T>::ReservationAlreadyReleased.into())
                }
                ReservationStatus::Consumed => {
                    return Err(Error::<T>::ReservationAlreadyConsumed.into())
                }
            }

            // Restore vault balance.
            pallet_x3_inventory::inventory::release_inventory::<T>(state.vault_id, state.amount)?;

            // Decrement unsettled-notional.
            let _ = InventoryPallet::<T>::decrement_unsettled_notional(state.lane_id, state.amount);

            // Remove expiry entry.
            ExpiryQueue::<T>::remove(state.expiry_block, reservation_id);

            let (route_id, snapshot_hash) = (state.route_id, state.solvency_snapshot_hash);

            Reservations::<T>::mutate(reservation_id, |s| {
                if let Some(s) = s.as_mut() {
                    s.status = terminal_status;
                }
            });
            ReservationsByRoute::<T>::remove(route_id);

            Self::deposit_event(Event::ReservationReleased {
                reservation_id,
                route_id,
                solvency_snapshot_hash: snapshot_hash,
            });
            Ok(())
        }
    }
}
