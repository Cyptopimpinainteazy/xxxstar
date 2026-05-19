// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-token-factory — OmniToken Factory for the X3 Universal Asset Kernel.
//
// Lets any signed account create a single canonical asset whose supply is
// simultaneously native across X3Native, X3Evm, and X3Svm via the kernel's
// supply ledger. The factory never mints to an "EVM-side token" and a separate
// "SVM-side token"; there is only ever one AssetId and one SupplyLedger, and
// the canonical_supply == native + evm + svm + external_locked + pending
// invariant is preserved at all times.
//
// Responsibilities:
//   * Accept a `TokenFactoryConfig` describing the launch (symbol, name,
//     decimals, initial_supply, token class, enabled domains, optional cap).
//   * Register the asset via `AssetRegistryMutate::do_register_asset`.
//   * Activate it via `AssetRegistryMutate::do_activate_asset`.
//   * Configure the `N * (N-1)` internal routes between enabled domains.
//   * Mint `initial_supply` to the creator's native leg via the supply ledger.
//   * Remember the mint authority and token class for post-launch operations
//     (mint for CappedMintable/GovernanceMintable, burn for Burnable).
//
// Explicit non-responsibilities:
//   * Does not move balances between accounts. (Account-level accounting lives
//     outside the kernel for now — the kernel only tracks per-domain totals.)
//   * Does not verify external proofs.
//   * Does not bypass the UAK invariant. Every state change goes through the
//     registry/ledger traits.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]
#![warn(missing_docs)]

//! X3 OmniToken Factory pallet.

pub use pallet::*;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{pallet_prelude::*, traits::ConstU32, BoundedVec};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::DecodeWithMemTracking;
    use sp_io::hashing::blake2_256;
    use sp_std::vec::Vec;
    use x3_asset_kernel_types::{
        traits::{
            AssetRegistryInspect, AssetRegistryMutate, EconomicHaltInspect, SupplyLedgerGovern,
            SupplyLedgerWrite,
        },
        AssetId, Balance, DomainId, RouteConfig, RouteLimits, SupplyPolicy, TokenClass,
    };

    /// Max symbol length accepted by the factory (kept in sync with the registry).
    pub type MaxSymbolLen = ConstU32<32>;
    /// Max name length accepted by the factory.
    pub type MaxNameLen = ConstU32<64>;
    /// Max number of domains a single launch may enable (hard bound; there are
    /// currently only 3 internal VM domains).
    pub type MaxEnabledDomains = ConstU32<8>;

    /// User-supplied launch configuration for [`create_token`].
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
    pub struct TokenFactoryConfig {
        /// Short ticker, e.g. `b"OMNI"`.
        pub symbol: BoundedVec<u8, MaxSymbolLen>,
        /// Long human-readable name.
        pub name: BoundedVec<u8, MaxNameLen>,
        /// Canonical decimal count.
        pub canonical_decimals: u8,
        /// Initial canonical supply minted into the creator's X3Native leg.
        pub initial_supply: Balance,
        /// Optional hard cap on total canonical supply. `None` means uncapped
        /// (only meaningful for `GovernanceMintable`; ignored for `FixedSupply`).
        pub max_supply: Option<Balance>,
        /// Token class selector.
        pub class: TokenClass,
        /// Which internal VM domains this token is native across.
        /// Must include `DomainId::X3Native` and at least one of X3Evm/X3Svm.
        pub enabled_domains: BoundedVec<DomainId, MaxEnabledDomains>,
    }

    /// Stored per-token record tying a factory-launched asset to its launch
    /// metadata and authority.
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
    pub struct TokenRecord<T: Config> {
        /// Creator account.
        pub creator: T::AccountId,
        /// Account currently authorised to mint (for mintable classes).
        pub mint_authority: T::AccountId,
        /// Token class chosen at launch.
        pub class: TokenClass,
        /// Cap on canonical supply, if any.
        pub max_supply: Option<Balance>,
        /// Domains this token is native across.
        pub enabled_domains: BoundedVec<DomainId, MaxEnabledDomains>,
        /// Block the token was launched at.
        pub launched_at: BlockNumberFor<T>,
    }

    // ── Pallet ─────────────────────────────────────────────────────────────

    /// Pallet wrapper type.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Config trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Origin permitted to launch a new token. Permissionless deployments
        /// use `EnsureSigned<AccountId>`; a restricted chain may require
        /// governance.
        type CreateTokenOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
        /// Handle to the asset registry (mutating side).
        type Registry: AssetRegistryMutate + AssetRegistryInspect;
        /// Handle to the supply ledger. The factory needs both the governance
        /// (mint/burn) and inspection (`ledger`) sides so it can enforce
        /// per-asset caps.
        type Ledger: SupplyLedgerGovern + SupplyLedgerWrite;
        /// Read-only economic halt gate.
        type EconomicHalt: EconomicHaltInspect;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// Factory-launched tokens indexed by `AssetId`.
    #[pallet::storage]
    #[pallet::getter(fn tokens)]
    pub type Tokens<T: Config> = StorageMap<_, Blake2_128Concat, AssetId, TokenRecord<T>>;

    /// Monotonic factory nonce used to guarantee unique origin-addresses across launches.
    #[pallet::storage]
    #[pallet::getter(fn factory_nonce)]
    pub type FactoryNonce<T: Config> = StorageValue<_, u128, ValueQuery>;

    // ── Events ─────────────────────────────────────────────────────────────

    /// Events emitted by the factory.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new OmniToken was launched.
        TokenCreated {
            /// Derived canonical asset id.
            asset_id: AssetId,
            /// Account that created the token (pays fees and receives initial supply).
            creator: T::AccountId,
            /// Token symbol.
            symbol: Vec<u8>,
            /// Initial supply minted into the X3Native leg.
            initial_supply: Balance,
            /// Token class.
            class: TokenClass,
            /// Domains enabled at launch.
            enabled_domains: Vec<DomainId>,
        },
        /// Additional supply was minted by the mint authority (capped or
        /// governance-mintable classes only).
        TokenMinted {
            /// Asset minted.
            asset_id: AssetId,
            /// Domain the supply was credited to.
            domain: DomainId,
            /// Amount minted.
            amount: Balance,
        },
        /// Supply was burned by the mint authority (burnable classes only).
        TokenBurned {
            /// Asset burned.
            asset_id: AssetId,
            /// Domain the supply was debited from.
            domain: DomainId,
            /// Amount burned.
            amount: Balance,
        },
        /// Mint authority was transferred.
        MintAuthorityChanged {
            /// Affected asset.
            asset_id: AssetId,
            /// Previous mint authority.
            old_authority: T::AccountId,
            /// New mint authority.
            new_authority: T::AccountId,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    /// Errors returned by the factory.
    #[pallet::error]
    pub enum Error<T> {
        /// Config specified zero canonical decimals above the valid range or a
        /// non-positive initial supply.
        InvalidLaunchConfig,
        /// `enabled_domains` did not include `X3Native` or contained a domain
        /// outside the supported internal set.
        InvalidEnabledDomains,
        /// TokenClass::WrappedExternal is not yet supported by the factory.
        UnsupportedTokenClass,
        /// `FixedSupply` tokens cannot be minted past launch.
        FixedSupplyCannotMint,
        /// The mint would exceed the configured `max_supply`.
        CappedMintWouldExceedMax,
        /// Caller is not the mint authority for this asset.
        NotMintAuthority,
        /// The token class does not permit burning.
        BurnNotAllowed,
        /// Asset is not known to the factory.
        UnknownToken,
        /// Nonce overflow — effectively unreachable.
        NonceOverflow,
        /// New launch/mint operations halted by economic safety policy.
        EconomicHaltActive,
    }

    // ── Extrinsics ─────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Launch a new OmniToken native across the chosen internal VM
        /// domains. Permissionless if `CreateTokenOrigin = EnsureSigned`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(60_000, 0))]
        pub fn create_token(origin: OriginFor<T>, config: TokenFactoryConfig) -> DispatchResult {
            let creator = T::CreateTokenOrigin::ensure_origin(origin)?;

            ensure!(
                !T::EconomicHalt::is_halted(),
                Error::<T>::EconomicHaltActive
            );

            // Basic config validation.
            ensure!(
                config.canonical_decimals <= 24,
                Error::<T>::InvalidLaunchConfig
            );
            ensure!(
                config.class.is_supported_at_launch(),
                Error::<T>::UnsupportedTokenClass
            );
            Self::validate_enabled_domains(&config.enabled_domains)?;
            if let Some(cap) = config.max_supply {
                ensure!(
                    cap >= config.initial_supply,
                    Error::<T>::CappedMintWouldExceedMax
                );
            }

            // Pull + bump the factory nonce so every launch has a unique origin address.
            let nonce = FactoryNonce::<T>::get();
            let next = nonce.checked_add(1).ok_or(Error::<T>::NonceOverflow)?;
            FactoryNonce::<T>::put(next);

            // Derive a deterministic origin_address from (creator, nonce). This is
            // only used as part of the AssetId derivation; it does not have to
            // match any external chain address.
            let mut preimage = Vec::with_capacity(64);
            preimage.extend_from_slice(&creator.encode());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let origin_address = blake2_256(&preimage).to_vec();

            // Register, activate, and mint initial supply via the UAK traits.
            // Factory-launched tokens always use NativeMintBurn so the kernel
            // owns supply semantics.
            let asset_id = T::Registry::do_register_asset(
                config.symbol.to_vec(),
                config.name.to_vec(),
                config.canonical_decimals,
                DomainId::X3Native,
                0u64,
                origin_address,
                SupplyPolicy::NativeMintBurn,
            )?;
            T::Registry::do_activate_asset(&asset_id)?;

            // Enable internal routes between every pair of enabled domains.
            let domains = config.enabled_domains.clone();
            for &source in domains.iter() {
                for &destination in domains.iter() {
                    if source == destination {
                        continue;
                    }
                    T::Registry::do_configure_route(
                        &asset_id,
                        source,
                        destination,
                        RouteConfig::internal(RouteLimits::DEV_PERMISSIVE, 100),
                    )?;
                }
            }

            // Mint the initial supply into the X3Native leg.
            if config.initial_supply > 0 {
                T::Ledger::do_mint_canonical(&asset_id, DomainId::X3Native, config.initial_supply)?;
            }

            let record = TokenRecord::<T> {
                creator: creator.clone(),
                mint_authority: creator.clone(),
                class: config.class,
                max_supply: config.max_supply,
                enabled_domains: config.enabled_domains.clone(),
                launched_at: frame_system::Pallet::<T>::block_number(),
            };
            Tokens::<T>::insert(asset_id, record);

            Self::deposit_event(Event::TokenCreated {
                asset_id,
                creator,
                symbol: config.symbol.to_vec(),
                initial_supply: config.initial_supply,
                class: config.class,
                enabled_domains: config.enabled_domains.to_vec(),
            });
            Ok(())
        }

        /// Mint additional supply. Only callable by the current mint authority
        /// and only for classes that permit post-launch mint.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn mint(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !T::EconomicHalt::is_halted(),
                Error::<T>::EconomicHaltActive
            );
            let record = Tokens::<T>::get(asset_id).ok_or(Error::<T>::UnknownToken)?;
            ensure!(who == record.mint_authority, Error::<T>::NotMintAuthority);
            ensure!(
                record.class.allows_post_launch_mint(),
                Error::<T>::FixedSupplyCannotMint,
            );
            if let Some(cap) = record.max_supply {
                let current = T::Ledger::ledger(&asset_id)
                    .map(|l| l.canonical_supply)
                    .unwrap_or(0);
                let after = current
                    .checked_add(amount)
                    .ok_or(Error::<T>::CappedMintWouldExceedMax)?;
                ensure!(after <= cap, Error::<T>::CappedMintWouldExceedMax);
            }
            T::Ledger::do_mint_canonical(&asset_id, domain, amount)?;
            Self::deposit_event(Event::TokenMinted {
                asset_id,
                domain,
                amount,
            });
            Ok(())
        }

        /// Burn supply. Only callable by the mint authority and only for
        /// classes that permit burning.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn burn(
            origin: OriginFor<T>,
            asset_id: AssetId,
            domain: DomainId,
            amount: Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let record = Tokens::<T>::get(asset_id).ok_or(Error::<T>::UnknownToken)?;
            ensure!(who == record.mint_authority, Error::<T>::NotMintAuthority);
            ensure!(record.class.allows_burn(), Error::<T>::BurnNotAllowed);
            T::Ledger::do_burn_canonical(&asset_id, domain, amount)?;
            Self::deposit_event(Event::TokenBurned {
                asset_id,
                domain,
                amount,
            });
            Ok(())
        }

        /// Transfer mint authority to a new account.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn transfer_mint_authority(
            origin: OriginFor<T>,
            asset_id: AssetId,
            new_authority: T::AccountId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            Tokens::<T>::try_mutate(asset_id, |maybe| -> DispatchResult {
                let record = maybe.as_mut().ok_or(Error::<T>::UnknownToken)?;
                ensure!(who == record.mint_authority, Error::<T>::NotMintAuthority);
                let old_authority = record.mint_authority.clone();
                record.mint_authority = new_authority.clone();
                Self::deposit_event(Event::MintAuthorityChanged {
                    asset_id,
                    old_authority,
                    new_authority,
                });
                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn validate_enabled_domains(
            domains: &BoundedVec<DomainId, MaxEnabledDomains>,
        ) -> Result<(), Error<T>> {
            let mut has_native = false;
            let mut has_any_remote = false;
            for &d in domains.iter() {
                match d {
                    DomainId::X3Native => has_native = true,
                    DomainId::X3Evm | DomainId::X3Svm => has_any_remote = true,
                    _ => return Err(Error::<T>::InvalidEnabledDomains),
                }
            }
            if !has_native || !has_any_remote {
                return Err(Error::<T>::InvalidEnabledDomains);
            }
            Ok(())
        }
    }
}
