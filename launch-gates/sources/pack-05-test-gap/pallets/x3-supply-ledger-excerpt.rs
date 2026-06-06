// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-supply-ledger — per-asset supply accounting.
//
// Single source of truth for how much of each asset exists in each
// representation (X3Native, X3Evm, X3Svm, external_locked) and in flight
// (pending). Every mutation is guarded by the king invariant:
//
//     represented_total ≤ canonical_supply
//
//     where represented_total = native + evm + svm + external_locked + pending
//
// "No operation may increase represented supply unless there is:
//   1. a native mint,
//   2. a source-side burn,
//   3. a collateral lock,
//   4. or a verified external proof."

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! X3 Supply Ledger pallet.

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use x3_asset_kernel_types::{
        traits::{AssetRegistryInspect, SupplyLedgerGovern, SupplyLedgerWrite},
        AssetId, Balance, DomainId, SupplyLedger,
    };

    /// AssetId → per-asset supply ledger.
    #[pallet::storage]
    #[pallet::getter(fn ledgers)]
    pub type Ledgers<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, SupplyLedger>;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin allowed to mint or burn canonical supply (governance).
        type SupplyGovernance: EnsureOrigin<Self::RuntimeOrigin>;
        /// Read-only access to the asset registry. Wire the registry pallet here.
        type Registry: AssetRegistryInspect;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CanonicalMinted {
            asset_id: AssetId,
            amount: Balance,
            domain: DomainId,
        },
        CanonicalBurned {
            asset_id: AssetId,
            amount: Balance,
            domain: DomainId,
        },
        LegDebited {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
        LegCredited {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
        Refunded {
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        UnknownAsset,
        AssetNotActive,
        Underflow,
        Overflow,
        /// King invariant would be violated — hard stop.
        InvariantViolation,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Governance-only: mint canonical supply into a specific domain leg.
        /// The only path by which represented supply may legitimately grow.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn mint_canonical(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            T::SupplyGovernance::ensure_origin(origin)?;
            Self::do_mint_canonical(&asset_id, domain, amount)
        }

        /// Governance-only: burn canonical supply from a specific domain leg.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn burn_canonical(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            T::SupplyGovernance::ensure_origin(origin)?;
            Self::do_burn_canonical(&asset_id, domain, amount)
        }
    }

    impl<T: Config> Pallet<T> {
        /// Origin-free mint core. Used by both the governance `mint_canonical`
        /// extrinsic and by the token factory.
        pub fn do_mint_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            ensure!(T::Registry::exists(asset_id), Error::<T>::UnknownAsset);
            Ledgers::<T>::try_mutate(*asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.get_or_insert_with(SupplyLedger::default);
                ledger.canonical_supply = ledger
                    .canonical_supply
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                Self::add_to_domain(ledger, domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::CanonicalMinted {
                asset_id: *asset_id,
                amount,
                domain,
            });
            Ok(())
        }

        /// Origin-free burn core. Used by both the governance `burn_canonical`
        /// extrinsic and by the token factory (for `Burnable` token class).
        pub fn do_burn_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            Ledgers::<T>::try_mutate(*asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                Self::sub_from_domain(ledger, domain, amount)?;
                ledger.canonical_supply = ledger
                    .canonical_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::CanonicalBurned {
                asset_id: *asset_id,
                amount,
                domain,
            });
            Ok(())
        }

        fn add_to_domain(
            ledger: &mut SupplyLedger,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), Error<T>> {
            let slot = Self::domain_slot_mut(ledger, domain);
            *slot = slot.checked_add(amount).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }

        fn sub_from_domain(
            ledger: &mut SupplyLedger,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), Error<T>> {
            let slot = Self::domain_slot_mut(ledger, domain);
            *slot = slot.checked_sub(amount).ok_or(Error::<T>::Underflow)?;
            Ok(())
        }

        /// Map `DomainId` → ledger field it controls.
        /// External domains share `external_locked_supply` (unused in MVP).
        fn domain_slot_mut(ledger: &mut SupplyLedger, domain: DomainId) -> &mut Balance {
            match domain {
                DomainId::X3Native => &mut ledger.native_supply,
                DomainId::X3Evm => &mut ledger.evm_supply,
                DomainId::X3Svm => &mut ledger.svm_supply,
                _ => &mut ledger.external_locked_supply,
            }
        }
    }

    impl<T: Config> SupplyLedgerWrite for Pallet<T> {
        fn debit_source_to_pending(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            ensure!(T::Registry::is_active(asset_id), Error::<T>::AssetNotActive);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                Self::sub_from_domain(ledger, source_domain, amount)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_add(amount)
                    .ok_or(Error::<T>::Overflow)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::LegDebited {
                asset_id: *asset_id,
                domain: source_domain,
                amount,
            });
            Ok(())
        }

        fn credit_destination_from_pending(
            asset_id: &AssetId,
            destination_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            ensure!(T::Registry::is_active(asset_id), Error::<T>::AssetNotActive);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                Self::add_to_domain(ledger, destination_domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::LegCredited {
                asset_id: *asset_id,
                domain: destination_domain,
                amount,
            });
            Ok(())
        }

        fn refund_pending_to_source(
            asset_id: &AssetId,
            source_domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            // Refunds allowed even while paused — pausing must not strand funds.
            ensure!(T::Registry::exists(asset_id), Error::<T>::UnknownAsset);
            Ledgers::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let ledger = maybe.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                ledger.pending_supply = ledger
                    .pending_supply
                    .checked_sub(amount)
                    .ok_or(Error::<T>::Underflow)?;
                Self::add_to_domain(ledger, source_domain, amount)?;
                ledger
                    .check_invariant()
                    .map_err(|_| Error::<T>::InvariantViolation)?;
                Ok(())
            })?;
            Self::deposit_event(Event::Refunded {
                asset_id: *asset_id,
                domain: source_domain,
                amount,
            });
            Ok(())
        }

        fn ledger(asset_id: &AssetId) -> Option<SupplyLedger> {
            Ledgers::<T>::get(asset_id)
        }
    }

    impl<T: Config> SupplyLedgerGovern for Pallet<T> {
        fn do_mint_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            Pallet::<T>::do_mint_canonical(asset_id, domain, amount)
        }

        fn do_burn_canonical(
            asset_id: &AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> Result<(), DispatchError> {
            Pallet::<T>::do_burn_canonical(asset_id, domain, amount)
        }
    }
}
