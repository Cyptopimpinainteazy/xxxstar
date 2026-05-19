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
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        TypeInfo,
        MaxEncodedLen,
        RuntimeDebug,
    )]
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

#[cfg(test)]
mod tests {
    use super::pallet::*;
    use crate as pallet_x3_asset_registry;
    use frame_support::{
        assert_noop, assert_ok, construct_runtime, parameter_types,
        traits::{ConstU32, EnsureOrigin},
    };
    use frame_system as system;
    use sp_core::H256;
    use sp_io::TestExternalities;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage, DispatchError,
    };
    use x3_asset_kernel_types::{
        traits::{AssetRegistryInspect, RouteInspect},
        AssetStatus, DomainId, RouteConfig, RouteLimits, SupplyPolicy,
    };

    // ── Mock runtime ──────────────────────────────────────────────────────

    type AccountId = u64;
    type Block = system::mocking::MockBlock<Test>;

    pub const ALICE: AccountId = 1;

    construct_runtime!(
        pub enum Test {
            System: frame_system,
            AssetRegistry: pallet_x3_asset_registry,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaxAssets: u32 = 10;
    }

    impl system::Config for Test {
        type BaseCallFilter = frame_support::traits::Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u64;
        type Block = Block;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<AccountId>;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type ExtensionsWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = ConstU32<16>;
        type RuntimeTask = ();
        type SingleBlockMigrations = ();
        type MultiBlockMigrator = ();
        type PreInherents = ();
        type PostInherents = ();
        type PostTransactions = ();
    }

    // Root-only origin for RegistryOrigin and emergency.
    pub struct RootOrigin;
    impl EnsureOrigin<RuntimeOrigin> for RootOrigin {
        type Success = ();
        fn try_origin(o: RuntimeOrigin) -> Result<(), RuntimeOrigin> {
            system::ensure_root(o).map_err(|_| RuntimeOrigin::none())
        }
        #[cfg(feature = "runtime-benchmarks")]
        fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
            Ok(RuntimeOrigin::root())
        }
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type RegistryOrigin = RootOrigin;
        type EmergencyPauseOrigin = RootOrigin;
        type MaxAssets = MaxAssets;
    }

    fn new_test_ext() -> TestExternalities {
        system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .into()
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    fn usdc_eth() -> (Vec<u8>, Vec<u8>, u8, DomainId, u64, Vec<u8>, SupplyPolicy) {
        (
            b"USDC".to_vec(),
            b"USD Coin".to_vec(),
            6,
            DomainId::Ethereum,
            1,
            vec![0xA0; 20],
            SupplyPolicy::LockMint,
        )
    }

    fn register_usdc() -> Result<(), DispatchError> {
        let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
        Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy)
            .map(|_| ())
    }

    fn internal_route_cfg() -> RouteConfig {
        RouteConfig::internal(RouteLimits::DEV_PERMISSIVE, 100)
    }

    // ── register_asset tests ───────────────────────────────────────────────

    #[test]
    fn register_asset_stores_metadata() {
        new_test_ext().execute_with(|| {
            assert_ok!(register_usdc());
            assert_eq!(AssetRegistry::total_assets(), 1);
        });
    }

    #[test]
    fn register_asset_status_starts_registered() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let asset_id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy)
                    .unwrap();
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::status(&asset_id),
                Some(AssetStatus::Registered)
            );
            // Not active yet — is_active must return false.
            assert!(!<Pallet<Test> as AssetRegistryInspect>::is_active(&asset_id));
        });
    }

    #[test]
    fn register_asset_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let asset_id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy)
                    .unwrap();
            System::assert_last_event(
                Event::AssetRegistered {
                    asset_id,
                    origin_domain: DomainId::Ethereum,
                    canonical_decimals: 6,
                }
                .into(),
            );
        });
    }

    #[test]
    fn register_asset_fails_on_duplicate() {
        new_test_ext().execute_with(|| {
            assert_ok!(register_usdc());
            // Same derivation inputs — same AssetId — must fail.
            assert_noop!(register_usdc(), Error::<Test>::AssetAlreadyExists);
            assert_eq!(AssetRegistry::total_assets(), 1);
        });
    }

    #[test]
    fn register_asset_fails_when_at_max_capacity() {
        new_test_ext().execute_with(|| {
            // MaxAssets = 10. Fill it up.
            for i in 0u64..10 {
                let chain_id = 1 + i;
                assert_ok!(Pallet::<Test>::do_register_asset(
                    b"TKN".to_vec(),
                    b"Token".to_vec(),
                    18,
                    DomainId::Ethereum,
                    chain_id,
                    vec![i as u8; 20],
                    SupplyPolicy::LockMint,
                ));
            }
            assert_eq!(AssetRegistry::total_assets(), 10);
            assert_noop!(
                Pallet::<Test>::do_register_asset(
                    b"OVER".to_vec(),
                    b"Over".to_vec(),
                    18,
                    DomainId::Ethereum,
                    999,
                    vec![0xFF; 20],
                    SupplyPolicy::LockMint,
                ),
                Error::<Test>::TooManyAssets
            );
        });
    }

    #[test]
    fn register_asset_fails_if_symbol_too_long() {
        new_test_ext().execute_with(|| {
            // MaxSymbolLen = 32; send 33 bytes.
            let long_sym = vec![b'X'; 33];
            assert_noop!(
                Pallet::<Test>::do_register_asset(
                    long_sym,
                    b"Name".to_vec(),
                    18,
                    DomainId::Ethereum,
                    1,
                    vec![0u8; 20],
                    SupplyPolicy::LockMint,
                ),
                Error::<Test>::MetadataFieldTooLong
            );
            assert_eq!(AssetRegistry::total_assets(), 0);
        });
    }

    #[test]
    fn register_asset_extrinsic_requires_registry_origin() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            // Signed origin should be rejected.
            assert!(AssetRegistry::register_asset(
                RuntimeOrigin::signed(ALICE),
                sym,
                name,
                dec,
                dom,
                chain,
                addr,
                policy,
            )
            .is_err());
        });
    }

    // ── activate / pause / unpause / retire tests ─────────────────────────

    #[test]
    fn activate_makes_asset_active() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(Pallet::<Test>::do_activate_asset(&id));
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::status(&id),
                Some(AssetStatus::Active)
            );
            assert!(<Pallet<Test> as AssetRegistryInspect>::is_active(&id));
        });
    }

    #[test]
    fn activate_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(Pallet::<Test>::do_activate_asset(&id));
            System::assert_last_event(
                Event::AssetStatusChanged {
                    asset_id: id,
                    new_status: AssetStatus::Active,
                }
                .into(),
            );
        });
    }

    #[test]
    fn activate_fails_for_unknown_asset() {
        new_test_ext().execute_with(|| {
            let fake_id = x3_asset_kernel_types::derive_asset_id(
                DomainId::Ethereum, 1, b"\x00", b"FAKE", 18,
            );
            assert_noop!(
                Pallet::<Test>::do_activate_asset(&fake_id),
                Error::<Test>::UnknownAsset
            );
        });
    }

    #[test]
    fn pause_asset_blocks_active_state() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(Pallet::<Test>::do_activate_asset(&id));
            // Pause via extrinsic (requires root).
            assert_ok!(AssetRegistry::pause_asset(RuntimeOrigin::root(), id));
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::status(&id),
                Some(AssetStatus::Paused)
            );
            assert!(!<Pallet<Test> as AssetRegistryInspect>::is_active(&id));
        });
    }

    #[test]
    fn unpause_restores_active() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(Pallet::<Test>::do_activate_asset(&id));
            assert_ok!(AssetRegistry::pause_asset(RuntimeOrigin::root(), id));
            assert_ok!(AssetRegistry::unpause_asset(RuntimeOrigin::root(), id));
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::status(&id),
                Some(AssetStatus::Active)
            );
        });
    }

    #[test]
    fn retire_asset_makes_it_terminal() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(AssetRegistry::retire_asset(RuntimeOrigin::root(), id));
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::status(&id),
                Some(AssetStatus::Retired)
            );
        });
    }

    #[test]
    fn retired_asset_cannot_be_modified() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(AssetRegistry::retire_asset(RuntimeOrigin::root(), id));
            // Activate after retire must fail.
            assert_noop!(
                Pallet::<Test>::do_activate_asset(&id),
                Error::<Test>::AssetRetired
            );
            // Pause after retire must fail.
            assert_noop!(
                AssetRegistry::pause_asset(RuntimeOrigin::root(), id),
                Error::<Test>::AssetRetired
            );
        });
    }

    // ── configure_route tests ─────────────────────────────────────────────

    #[test]
    fn configure_route_succeeds_for_active_asset() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            let cfg = internal_route_cfg();
            assert_ok!(Pallet::<Test>::do_configure_route(
                &id,
                DomainId::X3Native,
                DomainId::X3Evm,
                cfg,
            ));
            let stored = <Pallet<Test> as RouteInspect>::route(
                &id,
                DomainId::X3Native,
                DomainId::X3Evm,
            );
            assert!(stored.is_some());
            assert!(stored.unwrap().enabled);
        });
    }

    #[test]
    fn configure_route_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            let cfg = internal_route_cfg();
            assert_ok!(Pallet::<Test>::do_configure_route(
                &id,
                DomainId::X3Native,
                DomainId::X3Evm,
                cfg,
            ));
            System::assert_last_event(
                Event::RouteConfigured {
                    asset_id: id,
                    source: DomainId::X3Native,
                    destination: DomainId::X3Evm,
                    enabled: true,
                }
                .into(),
            );
        });
    }

    #[test]
    fn configure_route_fails_for_unknown_asset() {
        new_test_ext().execute_with(|| {
            let fake_id = x3_asset_kernel_types::derive_asset_id(
                DomainId::Ethereum, 1, b"\x00", b"NONE", 18,
            );
            assert_noop!(
                Pallet::<Test>::do_configure_route(
                    &fake_id,
                    DomainId::X3Native,
                    DomainId::X3Evm,
                    internal_route_cfg(),
                ),
                Error::<Test>::UnknownAsset
            );
        });
    }

    #[test]
    fn configure_route_fails_for_self_loop() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_noop!(
                Pallet::<Test>::do_configure_route(
                    &id,
                    DomainId::X3Native,
                    DomainId::X3Native, // same → self-loop
                    internal_route_cfg(),
                ),
                Error::<Test>::SelfLoopRoute
            );
        });
    }

    #[test]
    fn configure_route_fails_for_invalid_limits_when_enabled() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            // enabled = true but max_amount = 0 → invalid.
            let bad_cfg = RouteConfig {
                enabled: true,
                limits: RouteLimits {
                    min_amount: 0,
                    max_amount: 0, // invalid
                    daily_limit: 0,
                    per_wallet_daily_limit: 0,
                    pending_limit: 0,
                },
                fee_bps: 0,
                expiry_blocks: 100,
                proof_tier: x3_asset_kernel_types::ProofTier::TrustedInternal,
            };
            assert_noop!(
                Pallet::<Test>::do_configure_route(
                    &id,
                    DomainId::X3Native,
                    DomainId::X3Evm,
                    bad_cfg,
                ),
                Error::<Test>::InvalidRouteLimits
            );
        });
    }

    #[test]
    fn configure_route_fails_when_daily_limit_less_than_max() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            let bad_cfg = RouteConfig {
                enabled: true,
                limits: RouteLimits {
                    min_amount: 0,
                    max_amount: 1_000,
                    daily_limit: 500, // less than max_amount → invalid
                    per_wallet_daily_limit: 500,
                    pending_limit: 10,
                },
                fee_bps: 0,
                expiry_blocks: 100,
                proof_tier: x3_asset_kernel_types::ProofTier::TrustedInternal,
            };
            assert_noop!(
                Pallet::<Test>::do_configure_route(
                    &id,
                    DomainId::X3Native,
                    DomainId::X3Evm,
                    bad_cfg,
                ),
                Error::<Test>::InvalidRouteLimits
            );
        });
    }

    // ── set_route_enabled tests ────────────────────────────────────────────

    #[test]
    fn set_route_enabled_toggles_route() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_ok!(Pallet::<Test>::do_configure_route(
                &id,
                DomainId::X3Native,
                DomainId::X3Evm,
                internal_route_cfg(),
            ));
            // Disable via emergency pause origin (root).
            assert_ok!(AssetRegistry::set_route_enabled(
                RuntimeOrigin::root(),
                id,
                DomainId::X3Native,
                DomainId::X3Evm,
                false,
            ));
            let route = <Pallet<Test> as RouteInspect>::route(&id, DomainId::X3Native, DomainId::X3Evm).unwrap();
            assert!(!route.enabled);
            assert!(!<Pallet<Test> as RouteInspect>::is_route_open(&id, DomainId::X3Native, DomainId::X3Evm));

            // Re-enable.
            assert_ok!(AssetRegistry::set_route_enabled(
                RuntimeOrigin::root(),
                id,
                DomainId::X3Native,
                DomainId::X3Evm,
                true,
            ));
            assert!(<Pallet<Test> as RouteInspect>::is_route_open(&id, DomainId::X3Native, DomainId::X3Evm));
        });
    }

    #[test]
    fn set_route_enabled_fails_for_nonexistent_route() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert_noop!(
                AssetRegistry::set_route_enabled(
                    RuntimeOrigin::root(),
                    id,
                    DomainId::X3Native,
                    DomainId::X3Evm,
                    false,
                ),
                Error::<Test>::UnknownAsset
            );
        });
    }

    // ── AssetRegistryInspect trait tests ──────────────────────────────────

    #[test]
    fn exists_returns_false_for_unknown() {
        new_test_ext().execute_with(|| {
            let fake_id = x3_asset_kernel_types::derive_asset_id(
                DomainId::Ethereum, 1, b"\x00", b"FAKE", 6,
            );
            assert!(!<Pallet<Test> as AssetRegistryInspect>::exists(&fake_id));
        });
    }

    #[test]
    fn exists_returns_true_after_registration() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, dec, dom, chain, addr, policy).unwrap();
            assert!(<Pallet<Test> as AssetRegistryInspect>::exists(&id));
        });
    }

    #[test]
    fn supply_policy_is_stored_and_retrievable() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, _) = usdc_eth();
            let id = Pallet::<Test>::do_register_asset(
                sym, name, dec, dom, chain, addr, SupplyPolicy::NativeMintBurn,
            ).unwrap();
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::supply_policy(&id),
                Some(SupplyPolicy::NativeMintBurn)
            );
        });
    }

    #[test]
    fn canonical_decimals_are_stored_and_retrievable() {
        new_test_ext().execute_with(|| {
            let (sym, name, _, dom, chain, addr, policy) = usdc_eth();
            let id =
                Pallet::<Test>::do_register_asset(sym, name, 8, dom, chain, addr, policy).unwrap();
            assert_eq!(
                <Pallet<Test> as AssetRegistryInspect>::canonical_decimals(&id),
                Some(8)
            );
        });
    }

    // ── AssetId determinism ────────────────────────────────────────────────

    #[test]
    fn same_inputs_produce_same_asset_id() {
        new_test_ext().execute_with(|| {
            let (sym, name, dec, dom, chain, addr, policy) = usdc_eth();
            let id1 = Pallet::<Test>::do_register_asset(
                sym.clone(), name.clone(), dec, dom, chain, addr.clone(), policy,
            ).unwrap();
            // Duplicate registration fails, but the id is deterministic.
            let computed =
                x3_asset_kernel_types::derive_asset_id(dom, chain, &addr, &sym, dec);
            assert_eq!(id1, computed);
        });
    }

    #[test]
    fn different_chains_produce_different_asset_ids() {
        new_test_ext().execute_with(|| {
            let id_eth = Pallet::<Test>::do_register_asset(
                b"USDC".to_vec(), b"USD Coin".to_vec(), 6,
                DomainId::Ethereum, 1, vec![0xA0; 20], SupplyPolicy::LockMint,
            ).unwrap();
            let id_base = Pallet::<Test>::do_register_asset(
                b"USDC".to_vec(), b"USD Coin".to_vec(), 6,
                DomainId::Base, 8453, vec![0x83; 20], SupplyPolicy::LockMint,
            ).unwrap();
            assert_ne!(id_eth, id_base);
        });
    }
}
