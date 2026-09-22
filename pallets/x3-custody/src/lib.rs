#![cfg_attr(not(feature = "std"), no_std)]
#![allow(deprecated)]
#![allow(missing_docs)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::if_not_else)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::empty_line_after_outer_attr)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::multiple_bound_locations)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::module_name_repetitions)]
//! # X3 Custody Pallet — Phase 3.5
//!
//! On-chain custody map and signer separation policy.
//!
//! ## Core Invariants
//!
//! * Validator signing keys (`KeyRole::ValidatorSigning`) MUST NOT be registered
//!   under the `Operational` authorization tier (signer separation invariant).
//! * `CustodyMap` records which signers are authorized for each (chain, asset)
//!   pair and at which tier.
//! * `TierThresholds` encodes the minimum number of co-signers per tier.
//! * Key rotation due dates are tracked in `ValidatorKeyRegistry` records; there
//!   is no second schedule map to diverge from the registry.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// ── Public types (available outside the pallet macro) ────────────────────────

/// Authorization tier mirrors the off-chain `crates/custody-service` enum.
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
pub enum AuthorizationTier {
    /// Day-to-day operational movements (lowest threshold).
    Operational = 0,
    /// Strategic rebalances; requires multi-sig.
    Strategic = 1,
    /// Emergency circuit-breaker actions; highest multi-sig threshold.
    Emergency = 2,
    /// Policy-parameter changes (governance-gated).
    Policy = 3,
}

impl Default for AuthorizationTier {
    fn default() -> Self {
        Self::Operational
    }
}

/// Functional role assigned to a signing key.
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
pub enum KeyRole {
    /// Consensus / block-authoring validator key.
    ValidatorSigning,
    /// Treasury and operational payment key.
    TreasuryOperational,
    /// Cross-chain relayer signing key.
    RelayerSigning,
    /// Oracle price-feed submission key.
    OracleSubmission,
}

impl Default for KeyRole {
    fn default() -> Self {
        Self::TreasuryOperational
    }
}

/// Entry in the custody map for a single signer on a `(chain_id, asset_id)` pair.
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
pub struct SignerEntry<AccountId: codec::MaxEncodedLen> {
    /// Account authorized to sign.
    pub signer: AccountId,
    /// Authorization tier for this entry.
    pub tier: AuthorizationTier,
    /// Functional role of the key.
    pub role: KeyRole,
    /// Whether this entry is currently active.
    pub active: bool,
}

/// Validator key registration record.
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
pub struct ValidatorKeyRecord<BlockNumber: codec::MaxEncodedLen> {
    /// Block number when this key was registered.
    pub registered_at: BlockNumber,
    /// Block number when rotation becomes mandatory.
    pub rotation_due_at: BlockNumber,
    /// Functional role of this validator key.
    pub role: KeyRole,
    /// Whether this key is currently active.
    pub active: bool,
}

/// Per-tier minimum co-signer threshold.
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
pub struct ThresholdPolicy {
    /// Minimum number of active co-signers required.
    pub min_signers: u32,
    /// The tier this policy applies to.
    pub tier: AuthorizationTier,
}

/// Per-signer operational policy limits.
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
pub struct SignerPolicy {
    /// Maximum amount for a single operation.
    pub max_single_op_amount: u128,
    /// Maximum cumulative amount per day.
    pub max_daily_aggregate: u128,
    /// Bitmask of `AuthorizationTier` variants this signer is allowed for.
    /// Bit 0 = Operational, bit 1 = Strategic, bit 2 = Emergency, bit 3 = Policy.
    pub allowed_tiers: u8,
}

// ── Pallet ────────────────────────────────────────────────────────────────────

#[frame_support::pallet]
pub mod pallet {
    use super::{
        AuthorizationTier, KeyRole, SignerEntry, SignerPolicy, ThresholdPolicy, ValidatorKeyRecord,
    };
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// SCALE-encoded (AuthorizationTier, ThresholdPolicy) entries.
        pub initial_tier_thresholds: Vec<Vec<u8>>,
        /// SCALE-encoded (T::AccountId, SignerPolicy) entries.
        pub initial_signer_limits: Vec<Vec<u8>>,
        /// SCALE-encoded (T::AccountId, rotation_due_at) validator-key entries.
        #[serde(default)]
        pub initial_validator_keys: Vec<Vec<u8>>,
        pub _phantom: sp_std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            use codec::Decode;
            for blob in &self.initial_tier_thresholds {
                let (tier, policy): (AuthorizationTier, ThresholdPolicy) =
                    Decode::decode(&mut &blob[..]).expect("valid SCALE-encoded tier threshold");
                TierThresholds::<T>::insert(tier, policy);
            }
            for blob in &self.initial_signer_limits {
                let (signer, policy): (T::AccountId, SignerPolicy) =
                    Decode::decode(&mut &blob[..]).expect("valid SCALE-encoded signer limit");
                SignerLimits::<T>::insert(signer, policy);
            }
            for blob in &self.initial_validator_keys {
                let (account, rotation_due_at): (T::AccountId, BlockNumberFor<T>) =
                    Decode::decode(&mut &blob[..]).expect("valid SCALE-encoded validator key");
                ValidatorKeyRegistry::<T>::insert(
                    account,
                    ValidatorKeyRecord {
                        registered_at: Default::default(),
                        rotation_due_at,
                        role: KeyRole::ValidatorSigning,
                        active: true,
                    },
                );
            }
        }
    }

    // ── Config ────────────────────────────────────────────────────────────────

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.

        /// Origin for governance-level actions (register signers, keys, thresholds).
        type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Origin for operator-level administrative actions (limits, schedules).
        type OperatorOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Maximum number of signers per `(chain_id, asset_id)` vault.
        #[pallet::constant]
        type MaxSignersPerVault: Get<u32>;

        /// Maximum number of vaults a single signer may appear in.
        #[pallet::constant]
        type MaxVaultsPerSigner: Get<u32>;

        /// Maximum number of tier-threshold policies that may be stored.
        #[pallet::constant]
        type MaxPoliciesPerTier: Get<u32>;
    }

    // ── Storage ───────────────────────────────────────────────────────────────

    /// Maps `(chain_id, asset_id)` to the bounded list of signer entries.
    #[pallet::storage]
    #[pallet::getter(fn custody_map)]
    pub type CustodyMap<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        u32, // chain_id
        Blake2_128Concat,
        u32, // asset_id
        BoundedVec<SignerEntry<T::AccountId>, T::MaxSignersPerVault>,
        ValueQuery,
    >;

    /// Per-account validator key registration records.
    #[pallet::storage]
    #[pallet::getter(fn validator_key_registry)]
    pub type ValidatorKeyRegistry<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        ValidatorKeyRecord<BlockNumberFor<T>>,
        OptionQuery,
    >;

    /// Per-tier signing thresholds (minimum co-signers required).
    #[pallet::storage]
    #[pallet::getter(fn tier_thresholds)]
    pub type TierThresholds<T: Config> =
        StorageMap<_, Blake2_128Concat, AuthorizationTier, ThresholdPolicy, OptionQuery>;

    /// Per-signer operational limits.
    #[pallet::storage]
    #[pallet::getter(fn signer_limits)]
    pub type SignerLimits<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, SignerPolicy, OptionQuery>;

    // ── Events ────────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A signer was registered in the custody map.
        SignerRegistered {
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
            tier: AuthorizationTier,
            role: KeyRole,
        },
        /// A signer entry was deactivated.
        SignerDeactivated {
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
        },
        /// A validator key was registered.
        ValidatorKeyRegistered {
            account: T::AccountId,
            rotation_due_at: BlockNumberFor<T>,
        },
        /// A validator key was rotated to a new account key.
        KeyRotated {
            old_key: T::AccountId,
            new_key: T::AccountId,
        },
        /// A validator's key rotation due-date was renewed in place, without
        /// changing which account holds the key.
        ValidatorKeyRenewed {
            account: T::AccountId,
            rotation_due_at: BlockNumberFor<T>,
        },
        /// A tier signing threshold was set or updated.
        TierThresholdSet {
            tier: AuthorizationTier,
            min_signers: u32,
        },
        /// A per-signer operational limit was set.
        SignerLimitSet { signer: T::AccountId },
        /// A signer was found not to be authorized during a check.
        UnauthorizedSignerDetected {
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
            tier: AuthorizationTier,
        },
    }

    // ── Errors ────────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// Signer is already registered and active for this vault.
        SignerAlreadyRegistered,
        /// No matching signer entry was found.
        SignerNotFound,
        /// Not enough active co-signers to meet the tier threshold.
        TierThresholdNotMet,
        /// The signer's operational limits would be exceeded.
        SignerLimitExceeded,
        /// A validator key for this account already exists and is active.
        ValidatorKeyConflict,
        /// The key role is not permitted for the specified authorization tier.
        KeyRoleNotAllowedForTier,
        /// Adding another signer would exceed `MaxSignersPerVault`.
        MaxSignersReached,
        /// The supplied rotation due-date is not strictly after the current
        /// block — a rotation or renewal must always move the due-date
        /// forward, never backward or in place.
        RotationDueDateNotInFuture,
    }

    // ── Extrinsics ────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a signer for a `(chain_id, asset_id)` vault.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn register_signer(
            origin: OriginFor<T>,
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
            tier: AuthorizationTier,
            role: KeyRole,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            Self::ensure_role_tier_compatible(role, tier)?;

            CustodyMap::<T>::try_mutate(chain_id, asset_id, |entries| -> DispatchResult {
                ensure!(
                    !entries.iter().any(|e| e.signer == signer && e.active),
                    Error::<T>::SignerAlreadyRegistered
                );
                let entry = SignerEntry {
                    signer: signer.clone(),
                    tier,
                    role,
                    active: true,
                };
                entries
                    .try_push(entry)
                    .map_err(|_| Error::<T>::MaxSignersReached.into())
            })?;

            Self::deposit_event(Event::SignerRegistered {
                chain_id,
                asset_id,
                signer,
                tier,
                role,
            });
            Ok(())
        }

        /// Deactivate an existing signer entry in a vault.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(8_000, 0))]
        pub fn deactivate_signer(
            origin: OriginFor<T>,
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            CustodyMap::<T>::try_mutate(chain_id, asset_id, |entries| -> DispatchResult {
                let entry = entries
                    .iter_mut()
                    .find(|e| e.signer == signer && e.active)
                    .ok_or(Error::<T>::SignerNotFound)?;
                entry.active = false;
                Ok(())
            })?;

            Self::deposit_event(Event::SignerDeactivated {
                chain_id,
                asset_id,
                signer,
            });
            Ok(())
        }

        /// Register a validator key for `account`.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(8_000, 0))]
        pub fn register_validator_key(
            origin: OriginFor<T>,
            account: T::AccountId,
            rotation_due_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            if let Some(existing) = ValidatorKeyRegistry::<T>::get(&account) {
                ensure!(!existing.active, Error::<T>::ValidatorKeyConflict);
            }

            let current_block = frame_system::Pallet::<T>::block_number();
            let record = ValidatorKeyRecord {
                registered_at: current_block,
                rotation_due_at,
                role: KeyRole::ValidatorSigning,
                active: true,
            };

            ValidatorKeyRegistry::<T>::insert(&account, record);
            Self::deposit_event(Event::ValidatorKeyRegistered {
                account,
                rotation_due_at,
            });
            Ok(())
        }

        /// Rotate a validator key to a **different account**: deactivate
        /// `old_key`, activate `new_key`. This is for genuine account
        /// succession (e.g. the old account's key material is compromised
        /// and a new account takes over as that validator) — `session
        /// .setKeys` never changes which account is the validator, so it
        /// cannot drive this path. For routine same-account session-key
        /// rotation, use `renew_validator_key` instead.
        ///
        /// The caller supplies `next_due_at` rather than it being inherited
        /// from `old_key`'s record: reusing the old due-date would leave the
        /// new key immediately overdue whenever the rotation happens after
        /// the old key's due-date, which a driver that rotates what is due
        /// would do on every call, forever.
        ///
        /// `GovernanceOrigin` only. Emits `KeyRotated`.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(12_000, 0))]
        pub fn rotate_validator_key(
            origin: OriginFor<T>,
            old_key: T::AccountId,
            new_key: T::AccountId,
            next_due_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            let old_record = ValidatorKeyRegistry::<T>::get(&old_key)
                .filter(|r| r.active)
                .ok_or(Error::<T>::SignerNotFound)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                next_due_at > current_block,
                Error::<T>::RotationDueDateNotInFuture
            );

            if let Some(existing_new) = ValidatorKeyRegistry::<T>::get(&new_key) {
                ensure!(!existing_new.active, Error::<T>::ValidatorKeyConflict);
            }

            // Deactivate old key
            ValidatorKeyRegistry::<T>::mutate(&old_key, |maybe| {
                if let Some(r) = maybe.as_mut() {
                    r.active = false;
                }
            });
            // Register new key, on the caller-supplied due-date, inheriting role from old
            let new_record = ValidatorKeyRecord {
                registered_at: current_block,
                rotation_due_at: next_due_at,
                role: old_record.role,
                active: true,
            };
            ValidatorKeyRegistry::<T>::insert(&new_key, new_record);
            Self::deposit_event(Event::KeyRotated { old_key, new_key });
            Ok(())
        }

        /// Renew a validator's key rotation due-date **in place**, without
        /// changing which account holds the key. This is the routine path:
        /// `session.setKeys` re-signs the same account with new Aura/GRANDPA
        /// key material, so the registry entry for that account just needs
        /// its due-date pushed forward. `rotate_validator_key` is for
        /// genuine account succession, a different, rarer operation.
        ///
        /// Refuses if `account` has no active registered key, or if
        /// `next_due_at` is not strictly after the current block — the
        /// caller must always say what the next due-date is; this never
        /// silently picks one.
        ///
        /// `OperatorOrigin`. Emits `ValidatorKeyRenewed`.
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(10_000, 0))]
        pub fn renew_validator_key(
            origin: OriginFor<T>,
            account: T::AccountId,
            next_due_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::OperatorOrigin::ensure_origin(origin)?;

            ValidatorKeyRegistry::<T>::get(&account)
                .filter(|r| r.active)
                .ok_or(Error::<T>::SignerNotFound)?;

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(
                next_due_at > current_block,
                Error::<T>::RotationDueDateNotInFuture
            );

            ValidatorKeyRegistry::<T>::mutate(&account, |maybe| {
                if let Some(r) = maybe.as_mut() {
                    r.registered_at = current_block;
                    r.rotation_due_at = next_due_at;
                }
            });
            Self::deposit_event(Event::ValidatorKeyRenewed {
                account,
                rotation_due_at: next_due_at,
            });
            Ok(())
        }

        /// Set the minimum co-signer threshold for a tier.
        ///
        /// `GovernanceOrigin` only.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(6_000, 0))]
        pub fn set_tier_threshold(
            origin: OriginFor<T>,
            tier: AuthorizationTier,
            min_signers: u32,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            let policy = ThresholdPolicy { min_signers, tier };
            TierThresholds::<T>::insert(tier, policy);

            Self::deposit_event(Event::TierThresholdSet { tier, min_signers });
            Ok(())
        }

        /// Set the operational limits for a signer.
        ///
        /// `OperatorOrigin`.
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(6_000, 0))]
        pub fn set_signer_limit(
            origin: OriginFor<T>,
            signer: T::AccountId,
            policy: SignerPolicy,
        ) -> DispatchResult {
            T::OperatorOrigin::ensure_origin(origin)?;

            SignerLimits::<T>::insert(&signer, policy);
            Self::deposit_event(Event::SignerLimitSet { signer });
            Ok(())
        }

        /// Check whether `signer` is authorized at `tier` for `(chain_id, asset_id)`.
        ///
        /// Any signed origin. Returns `Ok(())` if authorized, `SignerNotFound`
        /// otherwise. Emits `UnauthorizedSignerDetected` before returning error.
        #[pallet::call_index(7)]
        #[pallet::weight(Weight::from_parts(4_000, 0))]
        pub fn check_signer_authorized(
            origin: OriginFor<T>,
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
            tier: AuthorizationTier,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            if Self::is_signer_authorized(chain_id, asset_id, &signer, tier) {
                return Ok(());
            }

            Self::deposit_event(Event::UnauthorizedSignerDetected {
                chain_id,
                asset_id,
                signer,
                tier,
            });
            Err(Error::<T>::SignerNotFound.into())
        }
    }

    // ── Public helpers ────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Returns `true` if `signer` has an active entry at `tier` in the
        /// `(chain_id, asset_id)` vault.
        pub fn is_signer_authorized(
            chain_id: u32,
            asset_id: u32,
            signer: &T::AccountId,
            tier: AuthorizationTier,
        ) -> bool {
            CustodyMap::<T>::get(chain_id, asset_id)
                .iter()
                .any(|e| &e.signer == signer && e.tier == tier && e.active)
        }

        /// Returns the `ThresholdPolicy` for `tier`, if one has been configured.
        pub fn get_threshold(tier: AuthorizationTier) -> Option<ThresholdPolicy> {
            TierThresholds::<T>::get(tier)
        }

        /// Count active signers in a vault at the given tier.
        pub fn count_active_signers(chain_id: u32, asset_id: u32, tier: AuthorizationTier) -> u32 {
            CustodyMap::<T>::get(chain_id, asset_id)
                .iter()
                .filter(|e| e.tier == tier && e.active)
                .count() as u32
        }

        /// Returns `true` if the vault meets the co-signer threshold for `tier`.
        /// Defaults to `true` when no threshold policy has been set.
        pub fn meets_threshold(chain_id: u32, asset_id: u32, tier: AuthorizationTier) -> bool {
            let count = Self::count_active_signers(chain_id, asset_id, tier);
            TierThresholds::<T>::get(tier).is_none_or(|p| count >= p.min_signers)
        }

        // ── Private helpers ───────────────────────────────────────────────────

        /// Enforces custody separation: `ValidatorSigning` keys MUST NOT be
        /// registered under the `Operational` tier.
        fn ensure_role_tier_compatible(role: KeyRole, tier: AuthorizationTier) -> DispatchResult {
            if matches!(&role, KeyRole::ValidatorSigning)
                && matches!(&tier, AuthorizationTier::Operational)
            {
                return Err(Error::<T>::KeyRoleNotAllowedForTier.into());
            }
            Ok(())
        }
    }
}
