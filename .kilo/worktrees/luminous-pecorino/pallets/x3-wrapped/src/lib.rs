#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
//! # pallet-x3-wrapped
//!
//! On-chain wrapped X3 token pallet providing cross-chain mint/burn with
//! authority gates, per-chain supply tracking, and governance power mapping
//! derived from wrapped X3 holdings.
//!
//! ## Design
//!
//! - A `BridgeAuthority` origin (e.g. bridge multi-sig) calls `mint_wrapped`
//!   to credit a recipient on proof of an off-chain lock.  Nonces prevent replay.
//! - Any holder calls `burn_wrapped` to destroy their wrapped tokens, signalling
//!   the bridge to release the locked collateral on the source chain.
//! - `GovernancePowerMap` stores the weighted sum of wrapped X3 balances across
//!   all registered assets for each account.  `update_governance_power` is
//!   permissionless so any party can refresh a stale entry.
//! - `TotalWrappedSupply == sum(WrappedSupply[*])` is maintained atomically on
//!   every mint and burn.

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// ── Public shared types (outside the pallet macro so callers can import them) ──

/// Identifies a remote chain by a u32 integer (e.g. EVM chain-id, Cosmos chain
/// numeric, or an X3-internal chain registry index).
pub type ChainId = u32;

/// 32-byte asset identifier — typically a hash of `(chain_id ++ contract_address)`
/// or a well-known constant for native assets.
pub type AssetId = [u8; 32];

/// Operational status of a registered wrapped asset.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    codec::Encode,
    codec::Decode,
    codec::DecodeWithMemTracking,
    scale_info::TypeInfo,
    codec::MaxEncodedLen,
    sp_runtime::RuntimeDebug,
)]
pub enum WrappedAssetStatus {
    /// Normal operation — minting and burning are permitted.
    Active,
    /// Minting is blocked; burning is still allowed to drain supply.
    Paused,
    /// Asset is end-of-life; no new operations are permitted.
    Deprecated,
}

/// On-chain configuration record for a registered wrapped asset.
#[derive(
    Clone,
    PartialEq,
    Eq,
    codec::Encode,
    codec::Decode,
    codec::DecodeWithMemTracking,
    scale_info::TypeInfo,
    codec::MaxEncodedLen,
    sp_runtime::RuntimeDebug,
)]
pub struct WrappedAssetConfig<Balance> {
    /// The canonical X3 asset ID that this wrapped asset represents.
    pub native_asset_id: AssetId,
    /// Hard cap on the total wrapped supply across **all** chains.
    pub max_wrapped_supply: Balance,
    /// Governance weight in basis points per unit of wrapped balance.
    /// e.g. 10_000 bps = 1.0 × weight per token unit.
    pub governance_weight_bps: u32,
    /// Fee charged on bridge operations in basis points.
    pub bridge_fee_bps: u32,
    /// Current operational status.
    pub status: WrappedAssetStatus,
}

// ── FRAME pallet ──────────────────────────────────────────────────────────────

#[frame_support::pallet]
pub mod pallet {
    use super::{AssetId, ChainId, WrappedAssetConfig, WrappedAssetStatus};
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{AtLeast32BitUnsigned, Saturating, Zero};

    // ── Config ────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Numeric balance type — must be at-least-32-bit unsigned so arithmetic
        /// never wraps silently.
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaxEncodedLen
            + scale_info::TypeInfo;

        /// Origin that may call `mint_wrapped` (e.g. bridge multi-sig or sudo).
        type BridgeAuthority: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin that may call governance extrinsics (register, pause, resume,
        /// set fee).  Typically `EnsureRootOrHalfCouncil` in production.
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of chains tracked per asset.
        #[pallet::constant]
        type MaxChainsPerAsset: Get<u32>;

        /// Maximum number of wrapped assets that may be registered.
        #[pallet::constant]
        type MaxWrappedAssets: Get<u32>;
    }

    // ── Pallet declaration ────────────────────────────────────────────────────

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    // ── Storage ───────────────────────────────────────────────────────────────

    /// Wrapped supply for each `(chain_id, asset_id)` pair.
    ///
    /// `WrappedSupply[chain_id][asset_id] = amount`
    #[pallet::storage]
    #[pallet::getter(fn wrapped_supply)]
    pub type WrappedSupply<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        ChainId,
        Blake2_128Concat,
        AssetId,
        T::Balance,
        ValueQuery,
    >;

    /// Per-account wrapped balances keyed by `(chain_id, asset_id, account)`.
    ///
    /// Uses `StorageNMap` with three key segments so the full triple is the
    /// composite key without needing a bespoke struct.
    #[pallet::storage]
    #[pallet::getter(fn wrapped_balance)]
    pub type WrappedBalances<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, ChainId>,
            NMapKey<Blake2_128Concat, AssetId>,
            NMapKey<Blake2_128Concat, T::AccountId>,
        ),
        T::Balance,
        ValueQuery,
    >;

    /// Total governance power (weighted wrapped X3) per account.
    ///
    /// Refreshed lazily by `update_governance_power`.
    #[pallet::storage]
    #[pallet::getter(fn governance_power_of_account)]
    pub type GovernancePowerMap<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::Balance, ValueQuery>;

    /// Registered wrapped asset configurations keyed by `asset_id`.
    #[pallet::storage]
    #[pallet::getter(fn registered_asset)]
    pub type RegisteredWrappedAssets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        AssetId,
        WrappedAssetConfig<T::Balance>,
    >;

    /// Nonce replay-prevention map: `(chain_id, nonce) -> bool`.
    ///
    /// Once `true`, that nonce may never be reused for `mint_wrapped`.
    #[pallet::storage]
    #[pallet::getter(fn nonce_used)]
    pub type BridgeNonces<T: Config> =
        StorageDoubleMap<_, Twox64Concat, ChainId, Twox64Concat, u64, bool, ValueQuery>;

    /// Global sum of all `WrappedSupply` entries.
    ///
    /// **Invariant**: `TotalWrappedSupply == Σ WrappedSupply[c][a]` for all
    /// registered `(chain_id, asset_id)` pairs.  Updated atomically on every
    /// mint and burn.
    #[pallet::storage]
    #[pallet::getter(fn total_wrapped_supply)]
    pub type TotalWrappedSupply<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new wrapped asset has been registered.
        WrappedAssetRegistered {
            asset_id: AssetId,
            max_supply: T::Balance,
        },
        /// Wrapped tokens were minted to `recipient` on behalf of `chain_id`.
        WrappedMinted {
            chain_id: ChainId,
            asset_id: AssetId,
            recipient: T::AccountId,
            amount: T::Balance,
        },
        /// Wrapped tokens were burned by `who` on `chain_id`.
        WrappedBurned {
            chain_id: ChainId,
            asset_id: AssetId,
            who: T::AccountId,
            amount: T::Balance,
        },
        /// An account's governance power snapshot was refreshed.
        GovernancePowerUpdated {
            account: T::AccountId,
            new_power: T::Balance,
        },
        /// A wrapped asset was paused (minting blocked).
        AssetPaused { asset_id: AssetId },
        /// A wrapped asset was resumed (minting re-enabled).
        AssetResumed { asset_id: AssetId },
        /// The bridge fee for an asset was updated.
        BridgeFeeSet {
            asset_id: AssetId,
            fee_bps: u32,
        },
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No wrapped asset is registered under the given `asset_id`.
        AssetNotFound,
        /// The asset is currently paused; minting is prohibited.
        AssetPaused,
        /// Minting would exceed the configured `max_wrapped_supply` for the asset.
        SupplyCapExceeded,
        /// The provided bridge nonce has already been consumed.
        NonceAlreadyUsed,
        /// The caller does not have enough wrapped balance to burn.
        InsufficientWrappedBalance,
        /// An asset with this `asset_id` is already registered.
        AssetAlreadyRegistered,
        /// The requested amount is zero or otherwise invalid.
        InvalidAmount,
        /// The asset is deprecated and no new operations are permitted.
        AssetDeprecated,
    }

    // ── Extrinsics ────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new wrapped asset.
        ///
        /// Requires `GovernanceOrigin`.  Each `asset_id` may only be registered
        /// once; subsequent calls with the same id return `AssetAlreadyRegistered`.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn register_wrapped_asset(
            origin: OriginFor<T>,
            asset_id: AssetId,
            config: WrappedAssetConfig<T::Balance>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                !RegisteredWrappedAssets::<T>::contains_key(asset_id),
                Error::<T>::AssetAlreadyRegistered
            );
            ensure!(
                config.max_wrapped_supply > T::Balance::zero(),
                Error::<T>::InvalidAmount
            );
            let max_supply = config.max_wrapped_supply;
            RegisteredWrappedAssets::<T>::insert(asset_id, config);
            Self::deposit_event(Event::WrappedAssetRegistered {
                asset_id,
                max_supply,
            });
            Ok(())
        }

        /// Mint wrapped tokens for `recipient` on `chain_id`.
        ///
        /// Requires `BridgeAuthority`.  The `nonce` must be unique per chain to
        /// prevent replay attacks.  Enforces `max_wrapped_supply` and asset status.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(15_000, 0))]
        pub fn mint_wrapped(
            origin: OriginFor<T>,
            chain_id: ChainId,
            asset_id: AssetId,
            recipient: T::AccountId,
            amount: T::Balance,
            nonce: u64,
        ) -> DispatchResult {
            T::BridgeAuthority::ensure_origin(origin)?;
            ensure!(amount > T::Balance::zero(), Error::<T>::InvalidAmount);

            // --- nonce replay guard ---
            ensure!(
                !BridgeNonces::<T>::get(chain_id, nonce),
                Error::<T>::NonceAlreadyUsed
            );

            // --- asset validation ---
            let config = RegisteredWrappedAssets::<T>::get(asset_id)
                .ok_or(Error::<T>::AssetNotFound)?;
            ensure!(
                config.status == WrappedAssetStatus::Active,
                if config.status == WrappedAssetStatus::Paused {
                    Error::<T>::AssetPaused
                } else {
                    Error::<T>::AssetDeprecated
                }
            );

            // --- supply cap check ---
            let current_chain_supply = WrappedSupply::<T>::get(chain_id, asset_id);
            let new_chain_supply = current_chain_supply
                .checked_add(&amount)
                .ok_or(Error::<T>::SupplyCapExceeded)?;

            // Sum across all chains for this asset to enforce the global cap.
            let total_asset_supply = Self::total_asset_supply(asset_id);
            let new_total_asset_supply = total_asset_supply
                .checked_add(&amount)
                .ok_or(Error::<T>::SupplyCapExceeded)?;
            ensure!(
                new_total_asset_supply <= config.max_wrapped_supply,
                Error::<T>::SupplyCapExceeded
            );

            // --- apply state changes atomically ---
            BridgeNonces::<T>::insert(chain_id, nonce, true);

            WrappedSupply::<T>::insert(chain_id, asset_id, new_chain_supply);

            let key = (chain_id, asset_id, recipient.clone());
            let prev_bal = WrappedBalances::<T>::get(&key);
            WrappedBalances::<T>::insert(&key, prev_bal.saturating_add(amount));

            TotalWrappedSupply::<T>::mutate(|t| *t = t.saturating_add(amount));

            Self::deposit_event(Event::WrappedMinted {
                chain_id,
                asset_id,
                recipient,
                amount,
            });
            Ok(())
        }

        /// Burn the caller's wrapped tokens on `chain_id`.
        ///
        /// Any signed origin may call this.  Reduces the per-chain supply and
        /// global total atomically.  The bridge is responsible for releasing the
        /// collateral on the source chain upon observing the on-chain burn.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(12_000, 0))]
        pub fn burn_wrapped(
            origin: OriginFor<T>,
            chain_id: ChainId,
            asset_id: AssetId,
            amount: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(amount > T::Balance::zero(), Error::<T>::InvalidAmount);

            let config = RegisteredWrappedAssets::<T>::get(asset_id)
                .ok_or(Error::<T>::AssetNotFound)?;
            // Allow burning even when Paused (drain); block only Deprecated.
            ensure!(
                config.status != WrappedAssetStatus::Deprecated,
                Error::<T>::AssetDeprecated
            );

            let key = (chain_id, asset_id, who.clone());
            let current_bal = WrappedBalances::<T>::get(&key);
            ensure!(current_bal >= amount, Error::<T>::InsufficientWrappedBalance);

            // --- apply state changes atomically ---
            let new_bal = current_bal.saturating_sub(amount);
            if new_bal.is_zero() {
                WrappedBalances::<T>::remove(&key);
            } else {
                WrappedBalances::<T>::insert(&key, new_bal);
            }

            WrappedSupply::<T>::mutate(chain_id, asset_id, |s| {
                *s = s.saturating_sub(amount);
            });

            TotalWrappedSupply::<T>::mutate(|t| *t = t.saturating_sub(amount));

            Self::deposit_event(Event::WrappedBurned {
                chain_id,
                asset_id,
                who,
                amount,
            });
            Ok(())
        }

        /// Recalculate and persist the governance power for `account`.
        ///
        /// Permissionless — anyone may trigger a refresh.  Power is the sum of
        /// `balance * governance_weight_bps / 10_000` for every `(chain, asset)`
        /// entry belonging to the account.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(20_000, 0))]
        pub fn update_governance_power(
            origin: OriginFor<T>,
            account: T::AccountId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let power = Self::compute_governance_power(&account);
            GovernancePowerMap::<T>::insert(&account, power);
            Self::deposit_event(Event::GovernancePowerUpdated {
                account,
                new_power: power,
            });
            Ok(())
        }

        /// Pause minting for the given asset.
        ///
        /// Requires `GovernanceOrigin`.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(8_000, 0))]
        pub fn pause_wrapped_asset(
            origin: OriginFor<T>,
            asset_id: AssetId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            RegisteredWrappedAssets::<T>::try_mutate(asset_id, |maybe_cfg| -> DispatchResult {
                let cfg = maybe_cfg.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                cfg.status = WrappedAssetStatus::Paused;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetPaused { asset_id });
            Ok(())
        }

        /// Resume minting for a previously paused asset.
        ///
        /// Requires `GovernanceOrigin`.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(8_000, 0))]
        pub fn resume_wrapped_asset(
            origin: OriginFor<T>,
            asset_id: AssetId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            RegisteredWrappedAssets::<T>::try_mutate(asset_id, |maybe_cfg| -> DispatchResult {
                let cfg = maybe_cfg.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                ensure!(
                    cfg.status != WrappedAssetStatus::Deprecated,
                    Error::<T>::AssetDeprecated
                );
                cfg.status = WrappedAssetStatus::Active;
                Ok(())
            })?;
            Self::deposit_event(Event::AssetResumed { asset_id });
            Ok(())
        }

        /// Update the bridge fee for a registered wrapped asset.
        ///
        /// Requires `GovernanceOrigin`.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(8_000, 0))]
        pub fn set_bridge_fee(
            origin: OriginFor<T>,
            asset_id: AssetId,
            fee_bps: u32,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            RegisteredWrappedAssets::<T>::try_mutate(asset_id, |maybe_cfg| -> DispatchResult {
                let cfg = maybe_cfg.as_mut().ok_or(Error::<T>::AssetNotFound)?;
                cfg.bridge_fee_bps = fee_bps;
                Ok(())
            })?;
            Self::deposit_event(Event::BridgeFeeSet { asset_id, fee_bps });
            Ok(())
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Sum wrapped supply across all chains for one `asset_id`.
        ///
        /// This iterates `WrappedSupply` with a prefix drain — O(chains) but
        /// bounded by `MaxChainsPerAsset`.
        fn total_asset_supply(asset_id: AssetId) -> T::Balance {
            // We can't iterate StorageDoubleMap by second key directly, so we
            // rely on the maintained `TotalWrappedSupply` minus other-asset
            // portions being consistent.  For the cap check we use a dedicated
            // per-asset accumulator approach via full iteration.
            WrappedSupply::<T>::iter()
                .filter_map(|(_, aid, bal)| if aid == asset_id { Some(bal) } else { None })
                .fold(T::Balance::zero(), |acc, b| acc.saturating_add(b))
        }

        /// Compute the governance power for `account` by iterating all wrapped
        /// balances and applying the per-asset `governance_weight_bps` factor.
        fn compute_governance_power(account: &T::AccountId) -> T::Balance {
            let mut power = T::Balance::zero();
            // Iterate all (chain_id, asset_id, account_id) entries.
            // Filter to entries owned by `account`.
            for ((_, asset_id, acct), bal) in WrappedBalances::<T>::iter() {
                if acct != *account {
                    continue;
                }
                let weight_bps = RegisteredWrappedAssets::<T>::get(asset_id)
                    .map(|c| c.governance_weight_bps)
                    .unwrap_or(0);
                // power += balance * weight_bps / 10_000
                // Use u128 intermediary to avoid overflow on Balance multiply.
                // Balance is AtLeast32BitUnsigned so TryInto<u128> is safe.
                let contribution = Self::apply_bps(bal, weight_bps);
                power = power.saturating_add(contribution);
            }
            power
        }

        /// Apply `bps/10_000` scaling to `value`, returning `Balance`.
        ///
        /// The computation is: `value * bps / 10_000` clamped to `Balance::MAX`.
        fn apply_bps(value: T::Balance, bps: u32) -> T::Balance {
            // Saturating multiply then divide to avoid intermediate overflow.
            // `saturating_mul` is available on `AtLeast32BitUnsigned`.
            value
                .saturating_mul(T::Balance::from(bps))
                .checked_div(&T::Balance::from(10_000_u32))
                .unwrap_or(T::Balance::zero())
        }
    }

    // ── Public helper (accessible outside the pallet macro scope) ─────────────

    impl<T: Config> Pallet<T> {
        /// Return the cached governance power for `account`.
        ///
        /// This reads the lazily-updated `GovernancePowerMap` storage entry.
        /// Call `update_governance_power` first if a fresh calculation is needed.
        pub fn governance_power_of(account: &T::AccountId) -> T::Balance {
            GovernancePowerMap::<T>::get(account)
        }
    }
}

/// Free function alias for external callers that import only the crate root.
pub fn governance_power_of<T: pallet::Config>(account: &T::AccountId) -> T::Balance {
    pallet::Pallet::<T>::governance_power_of(account)
}
