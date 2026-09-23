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
//! * Key rotation due dates are tracked in `KeyRotationSchedule`.

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

/// Which privileged gate an account is authorized to pass.
///
/// The atomic kernel, the cross-VM router and the settlement engine each take an
/// `EnsureOrigin<Success = AccountId>` from the runtime. Those origins were
/// `EnsureSignedBy<X3LangGatewayAccount, _>` — an account whose seed is the
/// well-known development phrase `//x3-atomic-gateway`, so the privilege belonged
/// to anyone who read this repository. They are this registry instead: filled from
/// genesis and edited only by governance, so a chain's privileged accounts are the
/// ones its own genesis names, and no compiled-in account is privileged anywhere.
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
pub enum GatewayRole {
    /// May assign, finalize and roll back atomic bundles, and drive the cross-VM
    /// router's X3-language entry points.
    X3Lang = 0,
    /// May call the atomic kernel's settlement finalization path.
    Settlement = 1,
}

/// Marker: the [`GatewayRole::X3Lang`] gate.
pub enum X3LangGatewayRole {}

/// Marker: the [`GatewayRole::Settlement`] gate.
pub enum SettlementGatewayRole {}

/// A role a [`EnsureAuthorizedGateway`] checks membership for.
pub trait GatewayRoleMarker {
    /// The role this marker stands for.
    const ROLE: GatewayRole;
}

impl GatewayRoleMarker for X3LangGatewayRole {
    const ROLE: GatewayRole = GatewayRole::X3Lang;
}

impl GatewayRoleMarker for SettlementGatewayRole {
    const ROLE: GatewayRole = GatewayRole::Settlement;
}

/// `EnsureOrigin` accepting a signed origin whose account this pallet's
/// `AuthorizedGateways` authorizes for `R`'s role.
///
/// Membership lives in storage rather than in a `parameter_types!` constant
/// because a constant cannot be set per chain: it is compiled into the WASM every
/// chain shares. Nothing is authorized until a chain's genesis says so, which is
/// what makes "the dev account is the gateway" impossible by construction rather
/// than by remembering to change a constant at release time.
pub struct EnsureAuthorizedGateway<T, R>(sp_std::marker::PhantomData<(T, R)>);

impl<T, R> frame_support::traits::EnsureOrigin<T::RuntimeOrigin> for EnsureAuthorizedGateway<T, R>
where
    T: Config,
    R: GatewayRoleMarker,
{
    type Success = T::AccountId;

    fn try_origin(o: T::RuntimeOrigin) -> Result<Self::Success, T::RuntimeOrigin> {
        o.into().and_then(|o| match o {
            frame_system::RawOrigin::Signed(who) => {
                if AuthorizedGateways::<T>::contains_key(R::ROLE, &who) {
                    Ok(who)
                } else {
                    Err(frame_system::RawOrigin::Signed(who).into())
                }
            }
            other => Err(other.into()),
        })
    }

    /// No origin is *always* authorized: membership is per chain, per account, and
    /// changeable by governance, so there is no origin this could return that is
    /// still valid one block later. A pallet that wants to act as a gateway has to
    /// route through a call the chain authorized, not through a synthesized origin —
    /// and a benchmark that needs a gateway origin has to authorize one first.
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<T::RuntimeOrigin, ()> {
        Err(())
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::{
        AuthorizationTier, GatewayRole, KeyRole, SignerEntry, SignerPolicy, ThresholdPolicy,
        ValidatorKeyRecord,
    };
    use frame_support::{pallet_prelude::*, BoundedVec};
    use frame_system::pallet_prelude::*;
    use sp_runtime::Saturating;
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
        /// Accounts authorized for `GatewayRole::X3Lang`.
        ///
        /// A chain with none of these cannot run an atomic bundle at all. That is
        /// the intended failure mode: the alternative was a compiled-in account
        /// whose seed is a public development phrase.
        ///
        /// `serde(default)` because this field did not exist when the specs in
        /// `chain-specs/` and `deployment/chain-specs/` were generated: a plain spec
        /// that names this pallet but not this field fails to load without it
        /// (`missing field 'x3LangGateways'` — measured, not assumed). An old spec
        /// loads and simply authorizes nobody, which is the same fail-closed posture
        /// as a chain that never named a gateway.
        #[serde(default)]
        pub x3_lang_gateways: Vec<T::AccountId>,
        /// Accounts authorized for `GatewayRole::Settlement`. `serde(default)` for
        /// the same reason as `x3_lang_gateways`.
        #[serde(default)]
        pub settlement_gateways: Vec<T::AccountId>,
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
            for account in &self.x3_lang_gateways {
                AuthorizedGateways::<T>::insert(GatewayRole::X3Lang, account, ());
            }
            for account in &self.settlement_gateways {
                AuthorizedGateways::<T>::insert(GatewayRole::Settlement, account, ());
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

        /// Block interval after which a rotated validator key becomes due for
        /// its next rotation.
        ///
        /// A successful rotation resets the clock: the new key's
        /// `rotation_due_at` is `current_block + KeyRotationPeriod`, never the
        /// old key's due block. This is what makes a *late* rotation (one
        /// performed after the old key's due block has already passed) land the
        /// new key with a fresh, future due date instead of one that is already
        /// overdue — the thrash bug the old inheritance semantics produced.
        #[pallet::constant]
        type KeyRotationPeriod: Get<BlockNumberFor<Self>>;
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

    /// Block number at which each signer's key rotation becomes mandatory.
    #[pallet::storage]
    #[pallet::getter(fn key_rotation_schedule)]
    pub type KeyRotationSchedule<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberFor<T>, OptionQuery>;

    /// Accounts authorized for a privileged [`GatewayRole`]: filled from genesis,
    /// edited by `GovernanceOrigin` only, and read by [`EnsureAuthorizedGateway`].
    ///
    /// The runtime wires this to the atomic kernel's `X3LangOrigin` and
    /// `SettlementOrigin` and to the cross-VM router's `X3LangOrigin`.
    #[pallet::storage]
    #[pallet::getter(fn authorized_gateways)]
    pub type AuthorizedGateways<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        GatewayRole,
        Blake2_128Concat,
        T::AccountId,
        (),
        OptionQuery,
    >;

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
        /// A tier signing threshold was set or updated.
        TierThresholdSet {
            tier: AuthorizationTier,
            min_signers: u32,
        },
        /// A per-signer operational limit was set.
        SignerLimitSet { signer: T::AccountId },
        /// A key rotation due-block was scheduled for a signer.
        KeyRotationScheduled {
            signer: T::AccountId,
            rotation_block: BlockNumberFor<T>,
        },
        /// A signer was found not to be authorized during a check.
        UnauthorizedSignerDetected {
            chain_id: u32,
            asset_id: u32,
            signer: T::AccountId,
            tier: AuthorizationTier,
        },
        /// An account was authorized for a privileged gateway role.
        GatewayAuthorized {
            role: GatewayRole,
            account: T::AccountId,
        },
        /// An account's authorization for a privileged gateway role was revoked.
        GatewayRevoked {
            role: GatewayRole,
            account: T::AccountId,
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
        /// The account is already authorized for this gateway role.
        GatewayAlreadyAuthorized,
        /// The account is not authorized for this gateway role.
        GatewayNotAuthorized,
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
            KeyRotationSchedule::<T>::insert(&account, rotation_due_at);

            Self::deposit_event(Event::ValidatorKeyRegistered {
                account,
                rotation_due_at,
            });
            Ok(())
        }

        /// Rotate a validator key: deactivate `old_key`, activate `new_key`.
        ///
        /// `GovernanceOrigin` only. Emits `KeyRotated`.
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(12_000, 0))]
        pub fn rotate_validator_key(
            origin: OriginFor<T>,
            old_key: T::AccountId,
            new_key: T::AccountId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;

            let old_record = ValidatorKeyRegistry::<T>::get(&old_key)
                .filter(|r| r.active)
                .ok_or(Error::<T>::SignerNotFound)?;

            if let Some(existing_new) = ValidatorKeyRegistry::<T>::get(&new_key) {
                ensure!(!existing_new.active, Error::<T>::ValidatorKeyConflict);
            }

            let current_block = frame_system::Pallet::<T>::block_number();

            // Deactivate old key
            ValidatorKeyRegistry::<T>::mutate(&old_key, |maybe| {
                if let Some(r) = maybe.as_mut() {
                    r.active = false;
                }
            });
            KeyRotationSchedule::<T>::remove(&old_key);

            // Register the new key with a fresh, future due block. The previous
            // implementation copied `old_record.rotation_due_at` verbatim, so a
            // key rotated after its due block was born already overdue and had
            // to rotate again immediately. A rotation now always grants a full
            // `KeyRotationPeriod` measured from the current block.
            let rotation_due_at = current_block.saturating_add(T::KeyRotationPeriod::get());
            let new_record = ValidatorKeyRecord {
                registered_at: current_block,
                rotation_due_at,
                role: old_record.role,
                active: true,
            };
            ValidatorKeyRegistry::<T>::insert(&new_key, new_record);
            KeyRotationSchedule::<T>::insert(&new_key, rotation_due_at);

            Self::deposit_event(Event::KeyRotated { old_key, new_key });
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

        /// Schedule a key rotation due-block for a signer.
        ///
        /// `OperatorOrigin`.
        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(5_000, 0))]
        pub fn set_key_rotation_schedule(
            origin: OriginFor<T>,
            signer: T::AccountId,
            rotation_block: BlockNumberFor<T>,
        ) -> DispatchResult {
            T::OperatorOrigin::ensure_origin(origin)?;

            KeyRotationSchedule::<T>::insert(&signer, rotation_block);
            Self::deposit_event(Event::KeyRotationScheduled {
                signer,
                rotation_block,
            });
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

        /// Authorize `account` for a privileged gateway `role`.
        ///
        /// `GovernanceOrigin` only. This is the call a live chain uses to name the
        /// operator's gateway service, instead of the dev account that used to be
        /// compiled into the runtime as the origin itself.
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(6_000, 0))]
        pub fn authorize_gateway(
            origin: OriginFor<T>,
            role: GatewayRole,
            account: T::AccountId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                !AuthorizedGateways::<T>::contains_key(role, &account),
                Error::<T>::GatewayAlreadyAuthorized
            );
            AuthorizedGateways::<T>::insert(role, &account, ());
            Self::deposit_event(Event::GatewayAuthorized { role, account });
            Ok(())
        }

        /// Revoke `account`'s authorization for a privileged gateway `role`.
        ///
        /// `GovernanceOrigin` only. Revoking the last `X3Lang` gateway leaves the
        /// chain unable to finalize a bundle, which is the same posture as never
        /// having named one: fail closed, visibly, rather than fall back.
        #[pallet::call_index(9)]
        #[pallet::weight(Weight::from_parts(6_000, 0))]
        pub fn revoke_gateway(
            origin: OriginFor<T>,
            role: GatewayRole,
            account: T::AccountId,
        ) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(
                AuthorizedGateways::<T>::contains_key(role, &account),
                Error::<T>::GatewayNotAuthorized
            );
            AuthorizedGateways::<T>::remove(role, &account);
            Self::deposit_event(Event::GatewayRevoked { role, account });
            Ok(())
        }
    }

    // ── Public helpers ────────────────────────────────────────────────────────

    impl<T: Config> Pallet<T> {
        /// Returns `true` if `account` is authorized for `role`.
        pub fn is_gateway_authorized(role: GatewayRole, account: &T::AccountId) -> bool {
            AuthorizedGateways::<T>::contains_key(role, account)
        }

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
