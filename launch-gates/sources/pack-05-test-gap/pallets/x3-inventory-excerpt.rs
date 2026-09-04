#![deny(unsafe_code)]
//! # pallet-x3-inventory
//!
//! Vault model, lane model, and inventory manager for X3 liquidity control.
//!
//! ## Ticket coverage
//! - TICKET-4.5-001: Core type definitions (`types.rs`) — implemented
//! - TICKET-4.5-002: Vault storage and band invariants — TODO
//! - TICKET-4.5-003: Lane storage and freeze mechanics — TODO
//! - TICKET-4.5-004: Inventory reserve and release — TODO
//! - TICKET-4.5-006: Global and lane unsettled notional tracking — TODO

#![cfg_attr(not(feature = "std"), no_std)]

pub mod inventory;
pub mod types;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::types::*;
    use frame_support::{pallet_prelude::*, traits::Get, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{Saturating, Zero};

    // -----------------------------------------------------------------------
    // Pallet config
    // -----------------------------------------------------------------------

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type Balance: Parameter
            + Member
            + Default
            + Copy
            + codec::MaxEncodedLen
            + scale_info::TypeInfo
            + PartialOrd
            + core::ops::Add<Output = Self::Balance>
            + core::ops::Sub<Output = Self::Balance>
            + core::fmt::Debug
            + Zero
            + Saturating;

        #[pallet::constant]
        type MaxLiquiditySources: Get<u32>;
    }

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn vaults)]
    pub type Vaults<T: Config> =
        StorageMap<_, Blake2_128Concat, VaultId, VaultState<T::Balance>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn lanes)]
    pub type Lanes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        LaneId,
        LaneState<T::Balance, T::MaxLiquiditySources>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn global_unsettled_notional)]
    pub type GlobalUnsettledNotional<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn lane_unsettled_notional)]
    pub type LaneUnsettledNotional<T: Config> =
        StorageMap<_, Blake2_128Concat, LaneId, T::Balance, ValueQuery>;

    // -----------------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------------

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        VaultCreated {
            vault_id: VaultId,
            vault_type: VaultType,
            chain_id: ChainId,
            asset_id: AssetId,
        },
        VaultStatusChanged {
            vault_id: VaultId,
            old_status: VaultStatus,
            new_status: VaultStatus,
        },
        VaultBandsUpdated {
            vault_id: VaultId,
        },
        LaneRegistered {
            lane_id: LaneId,
            lane_class: LaneClass,
        },
        LaneFrozen {
            lane_id: LaneId,
            reason: FreezeReason,
        },
        LaneUnfrozen {
            lane_id: LaneId,
            operator_id: [u8; 32],
        },
        /// Inventory successfully reserved in a vault.
        InventoryReserved {
            vault_id: VaultId,
            amount: T::Balance,
        },
        /// Inventory reservation released back to available balance.
        InventoryReleased {
            vault_id: VaultId,
            amount: T::Balance,
        },
        /// Outbound pending balance recorded (funds in flight).
        PendingOutRecorded {
            vault_id: VaultId,
            amount: T::Balance,
        },
        /// Settlement confirmed; pending_out balance cleared.
        SettlementConfirmed {
            vault_id: VaultId,
            amount: T::Balance,
        },
        /// Treasury funded a vault.
        VaultFunded {
            vault_id: VaultId,
            amount: T::Balance,
        },
        /// Lane unsettled notional hit its cap (TICKET-4.5-006 warning gate).
        ExposureCapBreached {
            lane_id: LaneId,
            current: T::Balance,
            cap: T::Balance,
        },
    }

    // -----------------------------------------------------------------------
    // Errors
    // -----------------------------------------------------------------------

    #[pallet::error]
    pub enum Error<T> {
        VaultAlreadyExists,
        VaultNotFound,
        LaneNotFound,
        LaneAlreadyExists,
        AlreadyFrozen,
        NotFrozen,
        /// Vault is frozen; reservation rejected.
        VaultFrozen,
        InvalidBandOrder,
        TooManyLiquiditySources,
        /// Vault does not have enough available balance to reserve.
        InsufficientAvailableBalance,
        /// Vault does not have enough reserved balance to release.
        InsufficientReservedBalance,
        /// Vault does not have enough pending_out balance to confirm.
        InsufficientPendingOutBalance,
        /// The requested operation would violate the balance invariant.
        BalanceInvariantViolation,
        /// Reservation would exceed the lane's unsettled notional cap (TICKET-4.5-006).
        UnsettledCapExceeded,
    }

    // -----------------------------------------------------------------------
    // Extrinsics
    // -----------------------------------------------------------------------

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // --- Vault lifecycle ---

        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn create_vault(
            origin: OriginFor<T>,
            vault_id: VaultId,
            vault_type: VaultType,
            owner_type: OwnerType,
            chain_id: ChainId,
            asset_id: AssetId,
            critical_min: T::Balance,
            min_band: T::Balance,
            target_band: T::Balance,
            max_band: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                !Vaults::<T>::contains_key(vault_id),
                Error::<T>::VaultAlreadyExists
            );
            ensure!(
                critical_min <= min_band && min_band <= target_band && target_band <= max_band,
                Error::<T>::InvalidBandOrder
            );

            let mut state = VaultState {
                vault_id,
                vault_type: vault_type.clone(),
                owner_type,
                chain_id,
                asset_id,
                available_balance: T::Balance::default(),
                reserved_balance: T::Balance::default(),
                pending_out_balance: T::Balance::default(),
                pending_in_balance: T::Balance::default(),
                critical_min,
                min_band,
                target_band,
                max_band,
                status: VaultStatus::Active, // overwritten immediately below
            };
            state.status = Self::check_band_status(&state);

            Vaults::<T>::insert(vault_id, state);
            Self::deposit_event(Event::VaultCreated {
                vault_id,
                vault_type,
                chain_id,
                asset_id,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn update_vault_bands(
            origin: OriginFor<T>,
            vault_id: VaultId,
            critical_min: T::Balance,
            min_band: T::Balance,
            target_band: T::Balance,
            max_band: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                critical_min <= min_band && min_band <= target_band && target_band <= max_band,
                Error::<T>::InvalidBandOrder
            );

            Vaults::<T>::try_mutate(vault_id, |maybe_vault| -> DispatchResult {
                let vault = maybe_vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;
                vault.critical_min = critical_min;
                vault.min_band = min_band;
                vault.target_band = target_band;
                vault.max_band = max_band;
                Self::refresh_vault_status(vault);
                Ok(())
            })?;

            Self::deposit_event(Event::VaultBandsUpdated { vault_id });
            Ok(())
        }

        // --- Lane lifecycle ---

        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn register_lane(
            origin: OriginFor<T>,
            lane_id: LaneId,
            source_chain: ChainId,
            dest_chain: ChainId,
            source_asset: AssetId,
            dest_asset: AssetId,
            lane_class: LaneClass,
            allowed_liquidity_sources: BoundedVec<LiquiditySourceType, T::MaxLiquiditySources>,
            exposure_cap: T::Balance,
            unsettled_cap: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(
                !Lanes::<T>::contains_key(lane_id),
                Error::<T>::LaneAlreadyExists
            );

            let state = LaneState {
                lane_id,
                source_chain,
                dest_chain,
                source_asset,
                dest_asset,
                lane_class: lane_class.clone(),
                allowed_liquidity_sources,
                status: LaneStatus::Active,
                exposure_cap,
                unsettled_cap,
            };

            Lanes::<T>::insert(lane_id, state);
            Self::deposit_event(Event::LaneRegistered {
                lane_id,
                lane_class,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn freeze_lane(
            origin: OriginFor<T>,
            lane_id: LaneId,
            reason: FreezeReason,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Lanes::<T>::try_mutate(lane_id, |maybe_lane| -> DispatchResult {
                let lane = maybe_lane.as_mut().ok_or(Error::<T>::LaneNotFound)?;
                ensure!(lane.status != LaneStatus::Frozen, Error::<T>::AlreadyFrozen);
                lane.status = LaneStatus::Frozen;
                Ok(())
            })?;
            Self::deposit_event(Event::LaneFrozen { lane_id, reason });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn unfreeze_lane(
            origin: OriginFor<T>,
            lane_id: LaneId,
            evidence: OperatorEvidence,
        ) -> DispatchResult {
            ensure_root(origin)?;
            Lanes::<T>::try_mutate(lane_id, |maybe_lane| -> DispatchResult {
                let lane = maybe_lane.as_mut().ok_or(Error::<T>::LaneNotFound)?;
                ensure!(lane.status == LaneStatus::Frozen, Error::<T>::NotFrozen);
                lane.status = LaneStatus::Active;
                Ok(())
            })?;
            Self::deposit_event(Event::LaneUnfrozen {
                lane_id,
                operator_id: evidence.operator_id,
            });
            Ok(())
        }

        // --- Inventory mutations (TICKET-4.5-004) ---

        /// Reserve `amount` from `available_balance` into `reserved_balance`.
        /// Rejects if vault is frozen or available balance is insufficient.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn reserve_inventory(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            super::inventory::reserve_inventory::<T>(vault_id, amount)
        }

        /// Release `amount` from `reserved_balance` back to `available_balance`.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn release_inventory(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            super::inventory::release_inventory::<T>(vault_id, amount)
        }

        /// Move `amount` from `available_balance` to `pending_out_balance` (funds in flight).
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn record_pending_out(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            super::inventory::record_pending_out::<T>(vault_id, amount)
        }

        /// Confirm settlement: reduce `pending_out_balance` by `amount`.
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn confirm_settlement(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            super::inventory::confirm_settlement::<T>(vault_id, amount)
        }

        /// Fund a vault from treasury — adds to `available_balance`.
        #[pallet::call_index(9)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn fund_vault(
            origin: OriginFor<T>,
            vault_id: VaultId,
            amount: T::Balance,
        ) -> DispatchResult {
            ensure_root(origin)?;
            super::inventory::fund_vault::<T>(vault_id, amount)
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    impl<T: Config> Pallet<T> {
        pub(crate) fn refresh_vault_status(vault: &mut VaultState<T::Balance>) {
            let new_status = Self::check_band_status(vault);
            if new_status != vault.status {
                let old_status = vault.status.clone();
                vault.status = new_status.clone();
                Self::deposit_event(Event::VaultStatusChanged {
                    vault_id: vault.vault_id,
                    old_status,
                    new_status,
                });
            }
        }

        pub fn check_band_status(vault: &VaultState<T::Balance>) -> VaultStatus {
            if vault.available_balance < vault.critical_min {
                VaultStatus::Frozen
            } else if vault.available_balance < vault.min_band {
                VaultStatus::Degraded
            } else {
                VaultStatus::Active
            }
        }

        pub fn get_vault_state(vault_id: VaultId) -> Option<VaultState<T::Balance>> {
            Vaults::<T>::get(vault_id)
        }

        pub fn get_lane_state(
            lane_id: LaneId,
        ) -> Option<LaneState<T::Balance, T::MaxLiquiditySources>> {
            Lanes::<T>::get(lane_id)
        }

        // -----------------------------------------------------------------------
        // TICKET-4.5-006: Unsettled-notional helpers (public API for x3-reservation)
        // Note: lane_unsettled_notional and global_unsettled_notional are generated
        // by #[pallet::getter] on LaneUnsettledNotional / GlobalUnsettledNotional.
        // -----------------------------------------------------------------------

        /// Increment both unsettled-notional counters.
        /// Rejects with `UnsettledCapExceeded` if the lane unsettled cap is reached.
        pub fn increment_unsettled_notional(lane_id: LaneId, amount: T::Balance) -> DispatchResult {
            if amount.is_zero() {
                return Ok(());
            }

            let lane = Lanes::<T>::get(lane_id).ok_or(Error::<T>::LaneNotFound)?;

            let current = LaneUnsettledNotional::<T>::get(lane_id);
            let new_lane_total = current.saturating_add(amount);
            ensure!(
                new_lane_total <= lane.unsettled_cap,
                Error::<T>::UnsettledCapExceeded
            );

            LaneUnsettledNotional::<T>::insert(lane_id, new_lane_total);
            GlobalUnsettledNotional::<T>::mutate(|g| {
                *g = g.saturating_add(amount);
            });

            if new_lane_total == lane.unsettled_cap {
                Self::deposit_event(Event::ExposureCapBreached {
                    lane_id,
                    current: new_lane_total,
                    cap: lane.unsettled_cap,
                });
            }

            Ok(())
        }

        /// Decrement both unsettled-notional counters (saturating; never panics).
        pub fn decrement_unsettled_notional(lane_id: LaneId, amount: T::Balance) {
            if amount.is_zero() {
                return;
            }
            LaneUnsettledNotional::<T>::mutate(lane_id, |n| {
                *n = n.saturating_sub(amount);
