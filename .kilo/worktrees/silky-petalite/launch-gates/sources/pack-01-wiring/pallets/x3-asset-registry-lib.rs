// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-asset-registry — canonical asset registry for the X3 Universal
// Asset Kernel.
//
// Responsibilities:
//   * Store the canonical `AssetMetadata` for every asset known to the chain.
//   * Store per-route `RouteConfig` (limits, fee, expiry, proof tier).
//   * Pause / unpause assets and routes (pauses instant, unpauses timelocked
//     at the origin layer).
//   * Expose read-only traits `AssetRegistryInspect` and `RouteInspect` so the
//     supply ledger and cross-VM router can consult the registry without
//     tight pallet coupling.
//
// Explicit non-responsibilities:
//   * Does not hold balances. (That lives in the supply ledger.)
//   * Does not move funds. (That lives in the cross-VM router.)
//   * Does not verify external proofs. (That lives in the cross-chain gateway.)

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

//! X3 Asset Registry pallet.

pub use pallet::*;

#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::ConstU32, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use x3_asset_kernel_types::{
        traits::{AssetRegistryInspect, AssetRegistryMutate, RouteInspect},
        AssetId, AssetStatus, DomainId, RouteConfig, RouteKey, SupplyPolicy,
    };

    /// Max length of the human-readable asset name/symbol strings.
    pub type MaxSymbolLen = ConstU32<32>;
    pub type MaxNameLen = ConstU32<64>;
    /// Max length of the origin address bytes stored in metadata.
    pub type MaxOriginAddressLen = ConstU32<64>;

    /// Full on-chain asset metadata record.
    #[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct AssetMetadata<T: Config> {
        pub asset_id: AssetId,
        pub symbol: BoundedVec<u8, MaxSymbolLen>,
        pub name: BoundedVec<u8, MaxNameLen>,
        pub canonical_decimals: u8,
        pub origin_domain: DomainId,
        pub origin_chain_id: u64,
        pub origin_address: BoundedVec<u8, MaxOriginAddressLen>,
        pub supply_policy: SupplyPolicy,
        pub status: AssetStatus,
        pub registered_at: BlockNumberFor<T>,
        pub version: u16,
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// Canonical AssetId → full metadata.
    #[pallet::storage]
    #[pallet::getter(fn assets)]
    pub type Assets<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, AssetMetadata<T>>;

    /// (AssetId, (source, destination)) → route config.
    #[pallet::storage]
    #[pallet::getter(fn routes)]
    pub type Routes<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AssetId,
        Blake2_128Concat,
        (DomainId, DomainId),
        RouteConfig,
    >;

    /// Monotonic counter of assets registered, used to bound storage growth.
    #[pallet::storage]
    #[pallet::getter(fn total_assets)]
    pub type TotalAssets<T: Config> = StorageValue<_, u32, ValueQuery>;

    // ── Pallet ─────────────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to register assets and configure routes.
        /// Typically governance or a multisig.
        type RegistryOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Origin permitted to execute emergency pauses. MUST be instant
        /// (no timelock). Unpauses should go through `RegistryOrigin`.
        type EmergencyPauseOrigin: EnsureOrigin<Self::RuntimeOrigin>;
        /// Hard ceiling on number of assets the registry may hold.
        #[pallet::constant]
        type MaxAssets: Get<u32>;
    }

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new asset has been registered.
        AssetRegistered {
            asset_id: AssetId,
            origin_domain: DomainId,
            canonical_decimals: u8,
        },
        /// Asset status changed.
        AssetStatusChanged {
            asset_id: AssetId,
            new_status: AssetStatus,
        },
        /// A route was created or updated.
        RouteConfigured {
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            enabled: bool,
        },
        /// A route was toggled enabled/disabled without other config changes.
        RouteToggled {
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            enabled: bool,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// An asset with this id is already registered.
        AssetAlreadyExists,
        /// Unknown asset.
        UnknownAsset,
        /// Would exceed `MaxAssets`.
        TooManyAssets,
        /// Symbol/name/address exceeds bound.
        MetadataFieldTooLong,
        /// Attempted to configure a route where source == destination.
        SelfLoopRoute,
        /// Route's max amount must be non-zero if enabled.
        InvalidRouteLimits,
        /// Asset must be `Active` for route operations.
        AssetNotActive,
        /// Asset has been retired and cannot be modified further.
        AssetRetired,
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new asset. The caller supplies everything needed to
        /// derive the canonical [`AssetId`] — the pallet re-derives it from
        /// the supplied fields and rejects if they don't match.
        ///
        /// Asset is registered in `Registered` state. A separate call to
        /// `activate_asset` moves it to `Active`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(25_000, 0))]
        pub fn register_asset(
            origin: OriginFor<T>,
            symbol: Vec<u8>,
            name: Vec<u8>,
            canonical_decimals: u8,
            origin_domain: DomainId,
            origin_chain_id: u64,
            origin_address: Vec<u8>,
            supply_policy: SupplyPolicy,
        ) -> DispatchResult {
            T::RegistryOrigin::ensure_origin(origin)?;
            Self::do_register_asset(
                symbol,
                name,
                canonical_decimals,
                origin_domain,
                origin_chain_id,
                origin_address,
                supply_policy,
            )?;
            Ok(())
        }

        /// Move an asset from `Registered` → `Active` (open for routing).
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn activate_asset(origin: OriginFor<T>, asset_id: AssetId) -> DispatchResult {
            T::RegistryOrigin::ensure_origin(origin)?;
            Self::set_status(asset_id, AssetStatus::Active)
        }

        /// Pause an asset globally. Instant. No route will accept transfers
        /// while paused.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn pause_asset(origin: OriginFor<T>, asset_id: AssetId) -> DispatchResult {
            T::EmergencyPauseOrigin::ensure_origin(origin)?;
            Self::set_status(asset_id, AssetStatus::Paused)
        }

        /// Lift a pause. This path **must be behind a timelock** at the origin
        /// layer — the pallet itself does not enforce that, but governance
        /// configuration does.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn unpause_asset(origin: OriginFor<T>, asset_id: AssetId) -> DispatchResult {
            T::RegistryOrigin::ensure_origin(origin)?;
            Self::set_status(asset_id, AssetStatus::Active)
        }

        /// Retire an asset. Terminal — cannot be reactivated.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn retire_asset(origin: OriginFor<T>, asset_id: AssetId) -> DispatchResult {
            T::RegistryOrigin::ensure_origin(origin)?;
            Self::set_status(asset_id, AssetStatus::Retired)
        }

        /// Create or overwrite a route's configuration.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn configure_route(
            origin: OriginFor<T>,
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            config: RouteConfig,
        ) -> DispatchResult {
            T::RegistryOrigin::ensure_origin(origin)?;
            Self::do_configure_route(&asset_id, source, destination, config)
        }

        /// Toggle an existing route's enabled flag without changing other config.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn set_route_enabled(
            origin: OriginFor<T>,
            asset_id: AssetId,
            source: DomainId,
            destination: DomainId,
            enabled: bool,
        ) -> DispatchResult {
            // Pausing a route is an emergency action (instant); enabling a route is
            // a registry action (timelocked at origin layer).
            if enabled {
                T::RegistryOrigin::ensure_origin(origin)?;
            } else {
                T::EmergencyPauseOrigin::ensure_origin(origin)?;
            }
            Routes::<T>::try_mutate(
                asset_id,
                (source, destination),
                |maybe_cfg| -> DispatchResult {
                    let cfg = maybe_cfg.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                    cfg.enabled = enabled;
                    Self::deposit_event(Event::RouteToggled {
                        asset_id,
                        source,
                        destination,
                        enabled,
                    });
                    Ok(())
                },
            )
        }
    }

    // ── Internal helpers ───────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        fn set_status(asset_id: AssetId, new: AssetStatus) -> DispatchResult {
            Assets::<T>::try_mutate(asset_id, |maybe_meta| -> DispatchResult {
                let meta = maybe_meta.as_mut().ok_or(Error::<T>::UnknownAsset)?;
                ensure!(
                    meta.status != AssetStatus::Retired,
                    Error::<T>::AssetRetired
                );
                meta.status = new;
                Self::deposit_event(Event::AssetStatusChanged {
                    asset_id,
                    new_status: new,
                });
                Ok(())
            })
        }

        /// Origin-free registration core. Used by both the `register_asset`
        /// extrinsic (after its origin check) and by the token factory
        /// (which performs its own authorization). Returns the derived
        /// `AssetId` on success.
        pub fn do_register_asset(
            symbol: Vec<u8>,
            name: Vec<u8>,
            canonical_decimals: u8,
            origin_domain: DomainId,
            origin_chain_id: u64,
            origin_address: Vec<u8>,
            supply_policy: SupplyPolicy,
        ) -> Result<AssetId, DispatchError> {
            ensure!(
                TotalAssets::<T>::get() < T::MaxAssets::get(),
                Error::<T>::TooManyAssets
            );

            let symbol: BoundedVec<u8, MaxSymbolLen> = symbol
                .try_into()
                .map_err(|_| Error::<T>::MetadataFieldTooLong)?;
            let name: BoundedVec<u8, MaxNameLen> = name
                .try_into()
                .map_err(|_| Error::<T>::MetadataFieldTooLong)?;
            let origin_address: BoundedVec<u8, MaxOriginAddressLen> = origin_address
                .try_into()
                .map_err(|_| Error::<T>::MetadataFieldTooLong)?;

            let asset_id = x3_asset_kernel_types::derive_asset_id(
                origin_domain,
                origin_chain_id,
                &origin_address,
                &symbol,
                canonical_decimals,
            );

            ensure!(
                !Assets::<T>::contains_key(asset_id),
                Error::<T>::AssetAlreadyExists
            );

            let meta = AssetMetadata::<T> {
                asset_id,
                symbol,
                name,
                canonical_decimals,
                origin_domain,
                origin_chain_id,
                origin_address,
                supply_policy,
                status: AssetStatus::Registered,
                registered_at: <frame_system::Pallet<T>>::block_number(),
                version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            };

            Assets::<T>::insert(asset_id, meta);
            TotalAssets::<T>::mutate(|n| *n = n.saturating_add(1));

            Self::deposit_event(Event::AssetRegistered {
                asset_id,
                origin_domain,
                canonical_decimals,
            });
            Ok(asset_id)
        }

        /// Origin-free activation core.
        pub fn do_activate_asset(asset_id: &AssetId) -> DispatchResult {
            Self::set_status(*asset_id, AssetStatus::Active)
        }

        /// Origin-free route configuration core.
        pub fn do_configure_route(
            asset_id: &AssetId,
            source: DomainId,
            destination: DomainId,
            config: RouteConfig,
        ) -> DispatchResult {
            ensure!(
                Assets::<T>::contains_key(asset_id),
                Error::<T>::UnknownAsset
            );
            ensure!(source != destination, Error::<T>::SelfLoopRoute);
            if config.enabled {
                ensure!(config.limits.max_amount > 0, Error::<T>::InvalidRouteLimits);
                ensure!(
                    config.limits.daily_limit >= config.limits.max_amount,
                    Error::<T>::InvalidRouteLimits
                );
            }
            Routes::<T>::insert(*asset_id, (source, destination), config);
            Self::deposit_event(Event::RouteConfigured {
                asset_id: *asset_id,
                source,
                destination,
                enabled: config.enabled,
            });
            Ok(())
        }
    }

    // ── Trait impls (public read surface) ──────────────────────────────────

    impl<T: Config> AssetRegistryInspect for Pallet<T> {
        fn exists(asset_id: &AssetId) -> bool {
            Assets::<T>::contains_key(asset_id)
        }
        fn status(asset_id: &AssetId) -> Option<AssetStatus> {
            Assets::<T>::get(asset_id).map(|m| m.status)
        }
        fn supply_policy(asset_id: &AssetId) -> Option<SupplyPolicy> {
            Assets::<T>::get(asset_id).map(|m| m.supply_policy)
        }
        fn canonical_decimals(asset_id: &AssetId) -> Option<u8> {
            Assets::<T>::get(asset_id).map(|m| m.canonical_decimals)
        }
    }

    impl<T: Config> RouteInspect for Pallet<T> {
        fn route(
            asset_id: &AssetId,
            source: DomainId,
            destination: DomainId,
        ) -> Option<RouteConfig> {
            Routes::<T>::get(asset_id, (source, destination))
        }
    }

    impl<T: Config> AssetRegistryMutate for Pallet<T> {
        fn do_register_asset(
            symbol: Vec<u8>,
            name: Vec<u8>,
            canonical_decimals: u8,
            origin_domain: DomainId,
            origin_chain_id: u64,
            origin_address: Vec<u8>,
            supply_policy: SupplyPolicy,
        ) -> Result<AssetId, DispatchError> {
            Pallet::<T>::do_register_asset(
                symbol,
                name,
                canonical_decimals,
                origin_domain,
                origin_chain_id,
                origin_address,
                supply_policy,
            )
        }

        fn do_activate_asset(asset_id: &AssetId) -> Result<(), DispatchError> {
            Pallet::<T>::do_activate_asset(asset_id)
        }

        fn do_configure_route(
            asset_id: &AssetId,
            source: DomainId,
            destination: DomainId,
            config: RouteConfig,
        ) -> Result<(), DispatchError> {
            Pallet::<T>::do_configure_route(asset_id, source, destination, config)
        }
    }

    /// Convenience: same shape as [`RouteKey`] for external callers.
    pub use x3_asset_kernel_types::RouteKey as _RouteKey;
    /// Re-export `RouteKey` under the pallet to reduce import churn.
    pub type RouteKeyAlias = RouteKey;
}
