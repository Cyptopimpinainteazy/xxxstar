// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-trust-gate — X3 Guardian trust gate.
//
// Separates "the code does what it says" from "what the code is allowed to do to
// users". Risky capabilities (mint, freeze, seize, drain, upgrade, tax, oracle
// control) are recorded as a machine-readable Privilege Map and judged against
// the declared application category. Risky functionality is surfaced, never
// assumed malicious (spec §2B, §16).
//
// The security gate answers "does this artifact pass the required tests". This
// pallet answers the orthogonal question the spec insists on keeping separate:
// "given the powers this artifact holds over its users, is that acceptable for
// the category it claims to be". An application can be perfectly implemented
// and still fail here — that is the point (§16: risky functionality does not
// imply malicious intent; it must be *surfaced* and judged by policy).
//
// Three deliberate design decisions:
//   * The privilege map is a **Guardian finding**, not an owner self-attestation
//     (§16). A contract that can mint can also claim it cannot, so the census is
//     written by the Guardian authority, like a certification.
//   * Declaring a forbidden privilege is *allowed and recorded*; the verdict is
//     what refuses it. A census that cannot record a finding cannot surface it.
//   * Unknown category, unknown application, or an unclassified privilege is an
//     error, never "assume allowed" (§49 fail-safe).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::{
    Decode, DecodeWithMemTracking, Encode, MaxEncodedLen, RuntimeDebug, TypeInfo,
};

/// Re-exported so the trust gate and the registry cannot drift apart on what an
/// application identifier is.
pub use pallet_x3_app_registry::ApplicationId;

/// Application category id, interpreted by the security gate (spec §34) and
/// used here to select the trust policy.
pub type CategoryId = u16;

/// A privileged capability an application can hold over its users (spec §16).
///
/// The vocabulary is closed: a policy must classify every variant, so a new
/// capability cannot be silently unclassified (which would read as "allowed").
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum Privilege {
    /// Minting, hidden minting, or otherwise changing supply (§16 "hidden
    /// minting", "unlimited minting", "mutable token supply").
    Mint,
    /// Writing balances directly ("arbitrary balance modification").
    ModifyBalances,
    /// Taking assets from an arbitrary account ("arbitrary account seizure").
    Seize,
    /// Withdrawing pooled value ("owner-drain", "liquidity drain privileges",
    /// "unrestricted treasury withdrawals", "emergency functions capable of
    /// stealing assets").
    Drain,
    /// Halting or restricting transfers ("blacklists", "arbitrary freezes",
    /// "arbitrary transfer restrictions").
    Freeze,
    /// Letting some accounts sell while others cannot ("sell restrictions",
    /// "asymmetric buy/sell behaviour", "honeypot conditions").
    SellRestriction,
    /// Taking a configurable cut of transfers ("stealth transfer taxes",
    /// "configurable tax abuse", "fees approaching confiscatory levels").
    Tax,
    /// Replacing behaviour after deployment ("stealth proxy upgrades",
    /// "upgrade-without-delay", "recoverable ownership", "fake ownership
    /// renouncement").
    Upgrade,
    /// Executing caller-supplied code ("dangerous delegatecall patterns",
    /// "arbitrary call execution").
    ArbitraryCall,
    /// Changing the data feed the app trusts ("owner-controlled oracle changes").
    OracleControl,
    /// Rerouting or reselecting executions ("privileged route manipulation",
    /// "privileged settlement manipulation").
    RouteControl,
    /// Holding undisclosed control ("hidden admin roles").
    AdminRole,
}

impl Privilege {
    /// Every privilege, in declaration order. A policy must classify all of
    /// them (see `TrustPolicy::validate`).
    pub const ALL: [Privilege; 12] = [
        Privilege::Mint,
        Privilege::ModifyBalances,
        Privilege::Seize,
        Privilege::Drain,
        Privilege::Freeze,
        Privilege::SellRestriction,
        Privilege::Tax,
        Privilege::Upgrade,
        Privilege::ArbitraryCall,
        Privilege::OracleControl,
        Privilege::RouteControl,
        Privilege::AdminRole,
    ];

    /// Bit used by [`PrivilegeSet`]. The list above must stay under 16 entries.
    pub const fn bit(self) -> u16 {
        1u16 << (self as u8)
    }
}

/// A set of privileges held by one application, or classified by one policy.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PrivilegeSet(u16);

impl PrivilegeSet {
    /// No privileges.
    pub const EMPTY: PrivilegeSet = PrivilegeSet(0);

    /// Every known privilege. Used to prove a policy classifies all of them.
    pub const ALL: PrivilegeSet = PrivilegeSet(0x0fff);

    /// Build a set from an explicit list (deterministic; duplicates ignored).
    pub fn from_privileges(privileges: &[Privilege]) -> Self {
        let mut bits = 0u16;
        for privilege in privileges {
            bits |= privilege.bit();
        }
        Self(bits)
    }

    /// Add a privilege. Returns `false` if it was already present.
    pub fn insert(&mut self, privilege: Privilege) -> bool {
        let bit = privilege.bit();
        let was_absent = self.0 & bit == 0;
        self.0 |= bit;
        was_absent
    }

    /// Whether `privilege` is in the set.
    pub fn contains(self, privilege: Privilege) -> bool {
        self.0 & privilege.bit() != 0
    }

    /// Whether the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Number of privileges in the set.
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Raw bit representation, for off-chain consumers.
    pub fn bits(self) -> u16 {
        self.0
    }
}

/// How a category treats one privilege.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum PrivilegeClass {
    /// Acceptable for this category without further user-facing disclosure.
    Allowed,
    /// Acceptable, but the privilege must be surfaced to users (spec §16, §29,
    /// §57: surface the behaviour, do not hide it).
    Disclosed,
    /// Not acceptable for this category.
    Forbidden,
}

/// Why a candidate policy is not a valid classification of every privilege.
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum PartitionError {
    /// A privilege was placed in more than one class.
    Overlap(Privilege),
    /// A privilege was left unclassified.
    Missing(Privilege),
}

/// A category's trust policy: a total, disjoint classification of every
/// privilege (spec §34).
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct TrustPolicy {
    /// Privileges acceptable for this category.
    pub allowed: PrivilegeSet,
    /// Privileges acceptable but requiring user-facing disclosure.
    pub disclosed: PrivilegeSet,
    /// Privileges this category may never hold.
    pub forbidden: PrivilegeSet,
}

impl TrustPolicy {
    /// How this policy classifies one privilege, or `None` if it is
    /// unclassified (which callers must treat as a failure, not as allowed).
    pub fn classify(&self, privilege: Privilege) -> Option<PrivilegeClass> {
        match (
            self.allowed.contains(privilege),
            self.disclosed.contains(privilege),
            self.forbidden.contains(privilege),
        ) {
            (true, false, false) => Some(PrivilegeClass::Allowed),
            (false, true, false) => Some(PrivilegeClass::Disclosed),
            (false, false, true) => Some(PrivilegeClass::Forbidden),
            _ => None,
        }
    }

    /// Every privilege classified exactly once. An incomplete policy is refused
    /// at install time, so no runtime lookup can fall through to "allowed".
    pub fn validate(&self) -> Result<(), PartitionError> {
        for privilege in Privilege::ALL {
            match self.classify(privilege) {
                Some(_) => {}
                None => {
                    return Err(
                        if self.allowed.contains(privilege)
                            || self.disclosed.contains(privilege)
                            || self.forbidden.contains(privilege)
                        {
                            PartitionError::Overlap(privilege)
                        } else {
                            PartitionError::Missing(privilege)
                        },
                    )
                }
            }
        }
        Ok(())
    }
}

/// The outcome of judging a recorded privilege map against its category policy.
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum TrustVerdict {
    /// Nothing forbidden, nothing needing disclosure.
    Compliant,
    /// Acceptable, but these privileges must be surfaced to users.
    CompliantWithDisclosure { disclosed: PrivilegeSet },
}

/// Why a privilege map cannot be judged compliant. Every variant is a refusal:
/// the trust gate never converts "cannot tell" into "acceptable" (spec §49).
#[derive(Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub enum TrustError {
    /// No privilege map is recorded for this application.
    UnknownApplication,
    /// The recorded category has no trust policy.
    UnknownCategoryPolicy,
    /// The application holds a privilege its category forbids.
    ForbiddenPrivilege(Privilege),
    /// The category policy does not classify this privilege.
    PrivilegeNotClassified(Privilege),
}

#[frame_support::pallet]
pub mod pallet {
    use super::{
        ApplicationId, CategoryId, Privilege, PrivilegeClass, PrivilegeSet, TrustError,
        TrustPolicy, TrustVerdict,
    };
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// The Guardian's recorded privilege census for one application (spec §16).
    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct PrivilegeDeclaration<T: Config> {
        /// Application the census belongs to.
        pub app_id: ApplicationId,
        /// Category the census was judged against. Recorded with the census, so
        /// a later category change requires a fresh declaration instead of
        /// silently inheriting the old verdict.
        pub category: CategoryId,
        /// Privileged capabilities the census found.
        pub privileges: PrivilegeSet,
        /// Block the census was recorded.
        pub declared_at: BlockNumberFor<T>,
        /// Guardian origin that recorded it (spec §16: attributable finding).
        pub declared_by: T::AccountId,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin permitted to set category trust policies and record privilege
        /// censuses. MUST be a privileged origin, never `EnsureSigned`: the
        /// census is the evidence a rug would most like to forge.
        type GuardianOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
        /// Weights for the dispatchables.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// CategoryId → how that category treats each privilege (spec §34).
    #[pallet::storage]
    #[pallet::getter(fn category_policies)]
    pub type CategoryPolicies<T: Config> = StorageMap<_, Blake2_128Concat, CategoryId, TrustPolicy>;

    /// ApplicationId → the recorded privilege census.
    #[pallet::storage]
    #[pallet::getter(fn declarations)]
    pub type Declarations<T: Config> =
        StorageMap<_, Blake2_128Concat, ApplicationId, PrivilegeDeclaration<T>>;

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A category trust policy was set (or replaced) by the Guardian.
        CategoryPolicySet {
            category: CategoryId,
            allowed: PrivilegeSet,
            disclosed: PrivilegeSet,
            forbidden: PrivilegeSet,
        },
        /// A privilege census was recorded for an application. Forbidden
        /// privileges are recorded too: a census that could not record a
        /// finding could not surface it (spec §16).
        PrivilegesDeclared {
            app_id: ApplicationId,
            category: CategoryId,
            privileges: PrivilegeSet,
        },
        /// A census was withdrawn (e.g. the application was revoked).
        PrivilegesCleared { app_id: ApplicationId },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// The policy left at least one privilege unclassified.
        IncompletePolicyPartition,
        /// The policy classified at least one privilege twice.
        OverlappingPolicyPartition,
        /// No trust policy exists for this category.
        UnknownCategoryPolicy,
        /// No privilege census is recorded for this application.
        UnknownDeclaration,
    }

    // ── Calls ──────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the trust policy for an application category (spec §34).
        ///
        /// The three sets must partition every known privilege: an unknown
        /// privilege is a refusal, because an unclassified privilege would
        /// otherwise have to be treated as allowed.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::set_category_policy())]
        pub fn set_category_policy(
            origin: OriginFor<T>,
            category: CategoryId,
            allowed: PrivilegeSet,
            disclosed: PrivilegeSet,
            forbidden: PrivilegeSet,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let policy = TrustPolicy {
                allowed,
                disclosed,
                forbidden,
            };
            match policy.validate() {
                Ok(()) => {}
                Err(super::PartitionError::Overlap(_)) => {
                    return Err(Error::<T>::OverlappingPolicyPartition.into())
                }
                Err(super::PartitionError::Missing(_)) => {
                    return Err(Error::<T>::IncompletePolicyPartition.into())
                }
            }
            CategoryPolicies::<T>::insert(category, policy);
            Self::deposit_event(Event::CategoryPolicySet {
                category,
                allowed,
                disclosed,
                forbidden,
            });
            Ok(())
        }

        /// Record the Guardian's privilege census for an application.
        ///
        /// The category must already have a trust policy: recording a census
        /// that could never be judged would leave an application in a state
        /// nobody can evaluate, so it is refused instead.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::declare_privileges())]
        pub fn declare_privileges(
            origin: OriginFor<T>,
            app_id: ApplicationId,
            category: CategoryId,
            privileges: PrivilegeSet,
        ) -> DispatchResult {
            let declared_by = T::GuardianOrigin::ensure_origin(origin)?;
            ensure!(
                CategoryPolicies::<T>::contains_key(category),
                Error::<T>::UnknownCategoryPolicy
            );
            Declarations::<T>::insert(
                app_id,
                PrivilegeDeclaration::<T> {
                    app_id,
                    category,
                    privileges,
                    declared_at: frame_system::Pallet::<T>::block_number(),
                    declared_by,
                },
            );
            Self::deposit_event(Event::PrivilegesDeclared {
                app_id,
                category,
                privileges,
            });
            Ok(())
        }

        /// Withdraw an application's privilege census.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::clear_privileges())]
        pub fn clear_privileges(origin: OriginFor<T>, app_id: ApplicationId) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            ensure!(
                Declarations::<T>::contains_key(app_id),
                Error::<T>::UnknownDeclaration
            );
            Declarations::<T>::remove(app_id);
            Self::deposit_event(Event::PrivilegesCleared { app_id });
            Ok(())
        }
    }

    // ── Read API for the registry, wallet, explorer, and Agent Law ─────────

    impl<T: Config> Pallet<T> {
        /// The recorded census for an application, if any.
        pub fn declaration(app_id: ApplicationId) -> Option<PrivilegeDeclaration<T>> {
            Declarations::<T>::get(app_id)
        }

        /// The trust policy for a category, if any.
        pub fn policy(category: CategoryId) -> Option<TrustPolicy> {
            CategoryPolicies::<T>::get(category)
        }

        /// Whether a category can be judged at all.
        pub fn category_is_configured(category: CategoryId) -> bool {
            CategoryPolicies::<T>::contains_key(category)
        }

        /// Judge an application's recorded privilege map against its recorded
        /// category policy (spec §2B, §16).
        ///
        /// Fails closed: no census, no policy, a forbidden privilege, or an
        /// unclassified privilege is a refusal. A caller must treat `Err` as
        /// "not trusted", never as "no objection raised".
        pub fn evaluate(app_id: ApplicationId) -> Result<TrustVerdict, TrustError> {
            let declaration =
                Declarations::<T>::get(app_id).ok_or(TrustError::UnknownApplication)?;
            let policy = CategoryPolicies::<T>::get(declaration.category)
                .ok_or(TrustError::UnknownCategoryPolicy)?;

            let mut disclosed = PrivilegeSet::EMPTY;
            for privilege in Privilege::ALL {
                if !declaration.privileges.contains(privilege) {
                    continue;
                }
                match policy.classify(privilege) {
                    Some(PrivilegeClass::Allowed) => {}
                    Some(PrivilegeClass::Disclosed) => {
                        disclosed.insert(privilege);
                    }
                    Some(PrivilegeClass::Forbidden) => {
                        return Err(TrustError::ForbiddenPrivilege(privilege))
                    }
                    None => return Err(TrustError::PrivilegeNotClassified(privilege)),
                }
            }

            if disclosed.is_empty() {
                Ok(TrustVerdict::Compliant)
            } else {
                Ok(TrustVerdict::CompliantWithDisclosure { disclosed })
            }
        }
    }
}

#[cfg(test)]
mod tests;
