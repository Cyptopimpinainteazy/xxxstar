// SPDX-License-Identifier: Apache-2.0
//
// pallet-x3-security-gate — versioned Guardian security policy.
//
// Holds the rules an application must satisfy: required test classes, severity
// thresholds, and per-category / per-VM requirements. Policies are versioned
// and activated, never silently reinterpreted (spec §28).
//
// Explicit non-responsibilities:
//   * Does not run analyses, fuzzing, or the exploit corpus (that is the
//     Guardian runner, spec §5/§6).
//   * Does not store application identity or certification tier (that is
//     pallet-x3-app-registry).
//   * Does not decide whether an application is *trusted* (that is
//     pallet-x3-trust-gate, spec §2B).
//
// For a candidate artifact this pallet answers exactly two questions:
//   1. which test classes does its (VM, category) pair require — `required_tests`
//   2. does an unresolved finding at a given severity block certification —
//      `severity_blocks_certification`
// Both fail closed: an unknown ruleset, an unactivated ruleset, an unknown
// category, or an unknown VM is an error, never a default of "nothing
// required" / "not blocking" (spec §49).

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

pub use pallet::*;

pub mod weights;
pub use weights::WeightInfo;

use frame_support::pallet_prelude::{
    Decode, DecodeWithMemTracking, Encode, MaxEncodedLen, RuntimeDebug, TypeInfo,
};

/// Re-exported so a policy author cannot accidentally invent a second VM
/// vocabulary. The registry owns the execution-domain enum; this pallet only
/// attaches requirements to it.
pub use pallet_x3_app_registry::GuardianVm;

/// A version of the Guardian security ruleset (spec §28). Versions are
/// allocated sequentially and never reused.
pub type RulesetVersion = u32;

/// Application category id.
///
/// `pallet-x3-app-registry` stores this on the application record but
/// deliberately does not interpret it. Turning a category id into requirements
/// is this pallet's job (spec §34).
pub type CategoryId = u16;

/// A category must require at least one test class: a policy that requires
/// nothing would certify on an empty evaluation.
pub const MIN_REQUIRED_TEST_CLASSES: usize = 1;

/// Largest accepted value of a single §35 risk dimension.
pub const MAX_RISK_DIMENSION: u8 = 10;

/// Deterministic finding severity (spec §30).
///
/// Ordered from least to most severe, so `Severity::Critical > Severity::Low`
/// and a policy can compare thresholds without re-implementing the ordering.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Every severity, least severe first.
    pub const ALL: [Severity; 5] = [
        Severity::Info,
        Severity::Low,
        Severity::Medium,
        Severity::High,
        Severity::Critical,
    ];

    /// Bit used by [`SeveritySet`].
    pub const fn bit(self) -> u8 {
        1u8 << (self as u8)
    }

    /// Severities that block production certification by default (spec §30:
    /// unresolved Critical, or unresolved High unless an explicit recorded
    /// exception exists).
    pub const fn blocking_by_default(self) -> bool {
        matches!(self, Severity::High | Severity::Critical)
    }
}

/// A set of severities that block certification.
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
pub struct SeveritySet(u8);

impl SeveritySet {
    /// The empty set: nothing blocks.
    pub const EMPTY: SeveritySet = SeveritySet(0);

    /// The spec §30 default: High and Critical block unless excepted.
    pub const HIGH_AND_CRITICAL: SeveritySet =
        SeveritySet(Severity::High.bit() | Severity::Critical.bit());

    /// Build a set from an explicit list (deterministic; duplicates ignored).
    pub fn from_severities(severities: &[Severity]) -> Self {
        let mut bits = 0u8;
        for severity in severities {
            bits |= severity.bit();
        }
        Self(bits)
    }

    /// Whether `severity` is in the set.
    pub fn contains(self, severity: Severity) -> bool {
        self.0 & severity.bit() != 0
    }

    /// Whether the set is empty.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Raw bit representation, for off-chain consumers.
    pub fn bits(self) -> u8 {
        self.0
    }
}

/// A class of evidence the Guardian runner can produce (spec §6, §21, §40).
///
/// The taxonomy is closed on purpose: policies reference a fixed vocabulary so
/// a ruleset cannot require a test class nobody can run.
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
pub enum TestClass {
    /// Fast in-process unit tests.
    Unit,
    /// Component tests across a crate boundary.
    Component,
    /// Integration tests against a real subsystem.
    Integration,
    /// Tests against a live node, not a harness.
    LiveNode,
    /// Cross-VM tests (EVM/SVM/X3VM in one operation).
    CrossVm,
    /// Adversarial/abuse-case tests (spec §21).
    Adversarial,
    /// Restart and recovery tests (spec §22).
    RestartRecovery,
    /// Long-duration soak tests.
    LongDuration,
    /// Static analysis of source or bytecode (spec §7).
    StaticAnalysis,
    /// Stateless fuzzing (spec §8).
    Fuzz,
    /// Stateful fuzzing (spec §9).
    StatefulFuzz,
    /// Property and invariant testing (spec §10).
    PropertyInvariant,
    /// Mutation testing (spec §13).
    Mutation,
    /// Differential testing against a reference (spec §14).
    Differential,
    /// Formal verification of stated properties (spec §11).
    FormalVerification,
    /// Economic attack simulation (spec §15).
    EconomicSimulation,
}

impl TestClass {
    /// Every test class, in declaration order.
    pub const ALL: [TestClass; 16] = [
        TestClass::Unit,
        TestClass::Component,
        TestClass::Integration,
        TestClass::LiveNode,
        TestClass::CrossVm,
        TestClass::Adversarial,
        TestClass::RestartRecovery,
        TestClass::LongDuration,
        TestClass::StaticAnalysis,
        TestClass::Fuzz,
        TestClass::StatefulFuzz,
        TestClass::PropertyInvariant,
        TestClass::Mutation,
        TestClass::Differential,
        TestClass::FormalVerification,
        TestClass::EconomicSimulation,
    ];

    /// Bit used by [`TestClassSet`]. The list above must stay under 32 entries.
    pub const fn bit(self) -> u32 {
        1u32 << (self as u8)
    }
}

/// A set of required test classes.
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
pub struct TestClassSet(u32);

impl TestClassSet {
    /// No required tests.
    pub const EMPTY: TestClassSet = TestClassSet(0);

    /// Build a set from an explicit list (deterministic; duplicates ignored).
    pub fn from_classes(classes: &[TestClass]) -> Self {
        let mut bits = 0u32;
        for class in classes {
            bits |= class.bit();
        }
        Self(bits)
    }

    /// Add a class. Returns `false` if it was already present.
    pub fn insert(&mut self, class: TestClass) -> bool {
        let bit = class.bit();
        let was_absent = self.0 & bit == 0;
        self.0 |= bit;
        was_absent
    }

    /// Whether `class` is required.
    pub fn contains(self, class: TestClass) -> bool {
        self.0 & class.bit() != 0
    }

    /// Whether nothing is required.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Number of required classes.
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }

    /// Union of two requirement sets: a (VM, category) pair must satisfy both
    /// the category profile and the VM profile (spec §5, §34).
    pub fn union(self, other: TestClassSet) -> TestClassSet {
        TestClassSet(self.0 | other.0)
    }

    /// Raw bit representation, for off-chain consumers.
    pub fn bits(self) -> u32 {
        self.0
    }
}

/// Risk dimensions (spec §35).
///
/// This is a **scheduling** input, never a safety claim: the spec explicitly
/// forbids presenting a simplified score as proof of safety, so no aggregate
/// score or "safe/unsafe" verdict is derived here.
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
pub struct RiskProfile {
    /// 0..=MAX_RISK_DIMENSION; how replaceable the deployed code is.
    pub upgradeability: u8,
    /// 0..=MAX_RISK_DIMENSION; breadth of privileged control.
    pub privileged_control: u8,
    /// 0..=MAX_RISK_DIMENSION; reliance on external contracts/programs.
    pub external_dependency: u8,
    /// 0..=MAX_RISK_DIMENSION; value held on behalf of users.
    pub custody: u8,
    /// 0..=MAX_RISK_DIMENSION; economic mechanism complexity.
    pub economic_complexity: u8,
    /// 0..=MAX_RISK_DIMENSION; cross-VM complexity.
    pub cross_vm_complexity: u8,
    /// 0..=MAX_RISK_DIMENSION; whether supply can change.
    pub mutable_supply: u8,
    /// 0..=MAX_RISK_DIMENSION; oracle dependence.
    pub oracle_dependence: u8,
}

impl RiskProfile {
    /// Every dimension as an array, for off-chain scheduling.
    pub fn as_array(&self) -> [u8; 8] {
        [
            self.upgradeability,
            self.privileged_control,
            self.external_dependency,
            self.custody,
            self.economic_complexity,
            self.cross_vm_complexity,
            self.mutable_supply,
            self.oracle_dependence,
        ]
    }

    /// Whether every dimension is inside `0..=MAX_RISK_DIMENSION`.
    pub fn is_valid(&self) -> bool {
        self.as_array().iter().all(|d| *d <= MAX_RISK_DIMENSION)
    }
}

/// Requirements attached to one application category inside one ruleset
/// (spec §34). Categories are risk-based: a static NFT profile is not the same
/// as a cross-chain lending profile.
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
pub struct CategoryPolicy {
    /// Test classes this category must pass.
    pub required_tests: TestClassSet,
    /// Severities that block certification unless a live, recorded exception
    /// exists (spec §30).
    pub blocking_severities: SeveritySet,
    /// Scheduling hint only (spec §35).
    pub risk: RiskProfile,
}

impl CategoryPolicy {
    /// The spec §30 default profile for a category requiring `required_tests`.
    pub fn new(required_tests: TestClassSet, risk: RiskProfile) -> Self {
        Self {
            required_tests,
            blocking_severities: SeveritySet::HIGH_AND_CRITICAL,
            risk,
        }
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::{
        CategoryId, CategoryPolicy, RiskProfile, RulesetVersion, Severity, SeveritySet,
        TestClassSet,
    };
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_x3_app_registry::GuardianVm;
    use sp_std::vec::Vec;

    /// A ruleset header (spec §28): identity, lifecycle, and the counts used to
    /// refuse an activation that would activate an empty policy.
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
    pub struct RulesetRecord<T: Config> {
        /// Ruleset version.
        pub version: RulesetVersion,
        /// Block the version was created as a draft.
        pub created_at: BlockNumberFor<T>,
        /// Block the version was activated, if it ever was.
        pub activated_at: Option<BlockNumberFor<T>>,
        /// Block the version was deprecated, if it ever was.
        pub deprecated_at: Option<BlockNumberFor<T>>,
        /// Replacement ruleset recorded at deprecation (spec §28).
        pub replacement: Option<RulesetVersion>,
        /// Number of category policies attached.
        pub category_policies: u32,
        /// Number of VM policies attached.
        pub vm_policies: u32,
    }

    /// A publicly recorded severity exception (spec §30). Never silent: the
    /// approver, the justification, and the expiration are all on chain.
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
    pub struct SeverityExceptionRecord<T: Config> {
        /// Origin that approved the exception.
        pub approver: T::AccountId,
        /// Public justification (bounded; the chain stores the text, the
        /// evidence itself stays off chain).
        pub justification: BoundedVec<u8, T::MaxJustificationLen>,
        /// Block the exception was recorded.
        pub recorded_at: BlockNumberFor<T>,
        /// Block after which the exception no longer applies.
        pub expires_at: BlockNumberFor<T>,
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Origin permitted to create policies, activate rulesets, terminate
        /// rulesets, and approve severity exceptions. MUST be a privileged
        /// origin (Root or a governance council), never `EnsureSigned` — a
        /// security policy is the thing being certified against.
        type GuardianOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;
        /// Hard ceiling on a recorded exception justification.
        #[pallet::constant]
        type MaxJustificationLen: Get<u32>;
        /// Weights for the dispatchables.
        type WeightInfo: WeightInfo;
    }

    // ── Storage ────────────────────────────────────────────────────────────

    /// The ruleset every certification is currently judged against.
    #[pallet::storage]
    #[pallet::getter(fn active_ruleset)]
    pub type ActiveRuleset<T> = StorageValue<_, RulesetVersion, OptionQuery>;

    /// Next ruleset version to allocate. Versions are never reused.
    #[pallet::storage]
    #[pallet::getter(fn next_ruleset_version)]
    pub type NextRulesetVersion<T> = StorageValue<_, RulesetVersion, ValueQuery>;

    /// RulesetVersion → header. Never pruned (spec §28: activation history is
    /// part of the evidence a certification is bound to).
    #[pallet::storage]
    #[pallet::getter(fn rulesets)]
    pub type Rulesets<T: Config> =
        StorageMap<_, Blake2_128Concat, RulesetVersion, RulesetRecord<T>>;

    /// (RulesetVersion, CategoryId) → category requirements.
    #[pallet::storage]
    #[pallet::getter(fn category_policies)]
    pub type CategoryPolicies<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        RulesetVersion,
        Blake2_128Concat,
        CategoryId,
        CategoryPolicy,
    >;

    /// (RulesetVersion, VM) → VM-specific required test classes.
    #[pallet::storage]
    #[pallet::getter(fn vm_policies)]
    pub type VmPolicies<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        RulesetVersion,
        Blake2_128Concat,
        GuardianVm,
        TestClassSet,
    >;

    /// (RulesetVersion, CategoryId, Severity) → recorded exception (spec §30).
    #[pallet::storage]
    #[pallet::getter(fn severity_exceptions)]
    pub type SeverityExceptions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        (RulesetVersion, CategoryId),
        Blake2_128Concat,
        Severity,
        SeverityExceptionRecord<T>,
    >;

    // ── Events ─────────────────────────────────────────────────────────────

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A draft ruleset version was created.
        RulesetCreated { version: RulesetVersion },
        /// A category policy was set on a draft ruleset.
        CategoryPolicySet {
            version: RulesetVersion,
            category: CategoryId,
        },
        /// A VM policy was set on a draft ruleset.
        VmPolicySet {
            version: RulesetVersion,
            vm: GuardianVm,
        },
        /// A ruleset became the active policy. The activation block is recorded
        /// so a certification can be bound to the exact ruleset that judged it
        /// (spec §28).
        RulesetActivated {
            version: RulesetVersion,
            activation_block: BlockNumberFor<T>,
        },
        /// A ruleset was deprecated, optionally naming its replacement.
        RulesetDeprecated {
            version: RulesetVersion,
            replacement: Option<RulesetVersion>,
        },
        /// A public severity exception was recorded.
        SeverityExceptionRecorded {
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
            expires_at: BlockNumberFor<T>,
        },
        /// A severity exception was withdrawn before it expired.
        SeverityExceptionRevoked {
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
        },
    }

    // ── Errors ─────────────────────────────────────────────────────────────

    #[pallet::error]
    pub enum Error<T> {
        /// No such ruleset version.
        UnknownRuleset,
        /// The ruleset is activated or deprecated; published policy is
        /// immutable, so an existing certification can never be reinterpreted
        /// (spec §28).
        RulesetImmutable,
        /// The ruleset exists but was never activated.
        RulesetNotActivated,
        /// The ruleset was already activated.
        RulesetAlreadyActivated,
        /// The ruleset was already deprecated.
        RulesetAlreadyDeprecated,
        /// The active ruleset cannot be deprecated before a replacement is
        /// activated.
        CannotDeprecateActiveRuleset,
        /// The named replacement ruleset does not exist.
        UnknownReplacement,
        /// The replacement is the ruleset being deprecated.
        InvalidReplacement,
        /// The named replacement has not been activated.
        ReplacementNotActivated,
        /// Activation was refused because no category policy was attached: an
        /// empty ruleset would require nothing of anyone.
        NoCategoryPolicies,
        /// The ruleset has no policy for this category.
        UnknownCategoryPolicy,
        /// The ruleset has no policy for this VM.
        UnknownVmPolicy,
        /// A category policy with no required test class was refused.
        EmptyRequiredTests,
        /// A risk dimension was outside `0..=MAX_RISK_DIMENSION`.
        RiskDimensionOutOfRange,
        /// An exception must expire in the future.
        ExpiryNotInFuture,
        /// The severity being excepted is not blocking under this policy, so an
        /// exception would be meaningless (and silently permissive).
        SeverityNotBlocking,
        /// The recorded justification exceeded `MaxJustificationLen`.
        JustificationTooLong,
        /// The recorded justification was empty.
        EmptyJustification,
        /// No such exception.
        UnknownSeverityException,
    }

    // ── Calls ──────────────────────────────────────────────────────────────

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new draft ruleset version. Drafts are mutable; activation
        /// freezes them (spec §28).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_ruleset())]
        pub fn create_ruleset(origin: OriginFor<T>) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let version = NextRulesetVersion::<T>::get();
            NextRulesetVersion::<T>::put(version.saturating_add(1));
            Rulesets::<T>::insert(
                version,
                RulesetRecord::<T> {
                    version,
                    created_at: frame_system::Pallet::<T>::block_number(),
                    activated_at: None,
                    deprecated_at: None,
                    replacement: None,
                    category_policies: 0,
                    vm_policies: 0,
                },
            );
            Self::deposit_event(Event::RulesetCreated { version });
            Ok(())
        }

        /// Attach or replace a category policy on a draft ruleset (spec §34).
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_category_policy())]
        pub fn set_category_policy(
            origin: OriginFor<T>,
            version: RulesetVersion,
            category: CategoryId,
            policy: CategoryPolicy,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            ensure!(
                record.activated_at.is_none() && record.deprecated_at.is_none(),
                Error::<T>::RulesetImmutable
            );
            ensure!(
                !policy.required_tests.is_empty(),
                Error::<T>::EmptyRequiredTests
            );
            ensure!(policy.risk.is_valid(), Error::<T>::RiskDimensionOutOfRange);

            if !CategoryPolicies::<T>::contains_key(version, category) {
                record.category_policies = record.category_policies.saturating_add(1);
                Rulesets::<T>::insert(version, record);
            }
            CategoryPolicies::<T>::insert(version, category, policy);
            Self::deposit_event(Event::CategoryPolicySet { version, category });
            Ok(())
        }

        /// Attach or replace the VM-specific required tests on a draft ruleset.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::set_vm_policy())]
        pub fn set_vm_policy(
            origin: OriginFor<T>,
            version: RulesetVersion,
            vm: GuardianVm,
            required_tests: TestClassSet,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            ensure!(
                record.activated_at.is_none() && record.deprecated_at.is_none(),
                Error::<T>::RulesetImmutable
            );
            ensure!(!required_tests.is_empty(), Error::<T>::EmptyRequiredTests);

            if !VmPolicies::<T>::contains_key(version, vm) {
                record.vm_policies = record.vm_policies.saturating_add(1);
                Rulesets::<T>::insert(version, record);
            }
            VmPolicies::<T>::insert(version, vm, required_tests);
            Self::deposit_event(Event::VmPolicySet { version, vm });
            Ok(())
        }

        /// Activate a ruleset. Activation is the only way a policy starts
        /// judging certifications, and it freezes the version permanently.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::activate_ruleset())]
        pub fn activate_ruleset(origin: OriginFor<T>, version: RulesetVersion) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            ensure!(record.deprecated_at.is_none(), Error::<T>::RulesetImmutable);
            ensure!(
                record.activated_at.is_none(),
                Error::<T>::RulesetAlreadyActivated
            );
            ensure!(record.category_policies > 0, Error::<T>::NoCategoryPolicies);

            let activation_block = frame_system::Pallet::<T>::block_number();
            record.activated_at = Some(activation_block);
            Rulesets::<T>::insert(version, record);
            ActiveRuleset::<T>::put(version);
            Self::deposit_event(Event::RulesetActivated {
                version,
                activation_block,
            });
            Ok(())
        }

        /// Deprecate an activated ruleset, recording its replacement
        /// (spec §28). The active ruleset cannot be deprecated: activate the
        /// replacement first, so there is never a window with no policy.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::deprecate_ruleset())]
        pub fn deprecate_ruleset(
            origin: OriginFor<T>,
            version: RulesetVersion,
            replacement: Option<RulesetVersion>,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            let mut record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            ensure!(
                record.activated_at.is_some(),
                Error::<T>::RulesetNotActivated
            );
            ensure!(
                record.deprecated_at.is_none(),
                Error::<T>::RulesetAlreadyDeprecated
            );
            ensure!(
                ActiveRuleset::<T>::get() != Some(version),
                Error::<T>::CannotDeprecateActiveRuleset
            );

            if let Some(replacement_version) = replacement {
                ensure!(
                    replacement_version != version,
                    Error::<T>::InvalidReplacement
                );
                let replacement_record = Rulesets::<T>::get(replacement_version)
                    .ok_or(Error::<T>::UnknownReplacement)?;
                ensure!(
                    replacement_record.activated_at.is_some(),
                    Error::<T>::ReplacementNotActivated
                );
            }

            record.deprecated_at = Some(frame_system::Pallet::<T>::block_number());
            record.replacement = replacement;
            Rulesets::<T>::insert(version, record);
            Self::deposit_event(Event::RulesetDeprecated {
                version,
                replacement,
            });
            Ok(())
        }

        /// Record a public, expiring, attributable exception to a blocking
        /// severity (spec §30: exceptions must never be silent).
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::record_severity_exception())]
        pub fn record_severity_exception(
            origin: OriginFor<T>,
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
            justification: Vec<u8>,
            expires_at: BlockNumberFor<T>,
        ) -> DispatchResult {
            let approver = T::GuardianOrigin::ensure_origin(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            ensure!(expires_at > now, Error::<T>::ExpiryNotInFuture);

            let record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            ensure!(
                record.activated_at.is_some(),
                Error::<T>::RulesetNotActivated
            );
            let policy = CategoryPolicies::<T>::get(version, category)
                .ok_or(Error::<T>::UnknownCategoryPolicy)?;
            ensure!(
                policy.blocking_severities.contains(severity),
                Error::<T>::SeverityNotBlocking
            );

            let justification = BoundedVec::<u8, T::MaxJustificationLen>::try_from(justification)
                .map_err(|_| Error::<T>::JustificationTooLong)?;
            ensure!(!justification.is_empty(), Error::<T>::EmptyJustification);

            SeverityExceptions::<T>::insert(
                (version, category),
                severity,
                SeverityExceptionRecord::<T> {
                    approver,
                    justification,
                    recorded_at: now,
                    expires_at,
                },
            );
            Self::deposit_event(Event::SeverityExceptionRecorded {
                version,
                category,
                severity,
                expires_at,
            });
            Ok(())
        }

        /// Withdraw a recorded exception before it expires.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::revoke_severity_exception())]
        pub fn revoke_severity_exception(
            origin: OriginFor<T>,
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
        ) -> DispatchResult {
            T::GuardianOrigin::ensure_origin(origin)?;
            ensure!(
                SeverityExceptions::<T>::contains_key((version, category), severity),
                Error::<T>::UnknownSeverityException
            );
            SeverityExceptions::<T>::remove((version, category), severity);
            Self::deposit_event(Event::SeverityExceptionRevoked {
                version,
                category,
                severity,
            });
            Ok(())
        }
    }

    // ── Read API for the runner, registry, wallet, and explorer ────────────

    impl<T: Config> Pallet<T> {
        /// Whether `version` is the currently active ruleset.
        pub fn is_active_ruleset(version: RulesetVersion) -> bool {
            ActiveRuleset::<T>::get() == Some(version)
        }

        /// The exception that applies right now, if any. An expired exception is
        /// not returned (spec §30: exceptions expire).
        pub fn live_exception(
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
            now: BlockNumberFor<T>,
        ) -> Option<SeverityExceptionRecord<T>> {
            SeverityExceptions::<T>::get((version, category), severity)
                .filter(|exception| exception.expires_at > now)
        }

        /// Required test classes for a (VM, category) pair: the union of the
        /// category profile and the VM profile. Fails closed — an unknown or
        /// not-yet-activated ruleset, an unknown category, or an unknown VM is
        /// an error, never an empty requirement set.
        pub fn required_tests(
            version: RulesetVersion,
            vm: GuardianVm,
            category: CategoryId,
        ) -> Result<TestClassSet, Error<T>> {
            let record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            if record.activated_at.is_none() {
                return Err(Error::<T>::RulesetNotActivated);
            }
            let category_policy = CategoryPolicies::<T>::get(version, category)
                .ok_or(Error::<T>::UnknownCategoryPolicy)?;
            let vm_tests = VmPolicies::<T>::get(version, vm).ok_or(Error::<T>::UnknownVmPolicy)?;
            Ok(category_policy.required_tests.union(vm_tests))
        }

        /// Whether an unresolved finding at `severity` blocks production
        /// certification for this category.
        ///
        /// Fails closed: an unknown/unactivated ruleset or an unknown category
        /// is an error, and a caller that turns that error into "not blocking"
        /// would be converting uncertainty into success (spec §49).
        pub fn severity_blocks_certification(
            version: RulesetVersion,
            category: CategoryId,
            severity: Severity,
            now: BlockNumberFor<T>,
        ) -> Result<bool, Error<T>> {
            let record = Rulesets::<T>::get(version).ok_or(Error::<T>::UnknownRuleset)?;
            if record.activated_at.is_none() {
                return Err(Error::<T>::RulesetNotActivated);
            }
            let policy = CategoryPolicies::<T>::get(version, category)
                .ok_or(Error::<T>::UnknownCategoryPolicy)?;
            if !policy.blocking_severities.contains(severity) {
                return Ok(false);
            }
            Ok(Self::live_exception(version, category, severity, now).is_none())
        }

        /// Whether a privilege-free, test-complete finding ledger would be
        /// accepted: convenience for policy authors that need the §30 default.
        pub fn default_blocking_severities() -> SeveritySet {
            SeveritySet::HIGH_AND_CRITICAL
        }

        /// Validates a candidate risk profile without writing it (spec §35).
        pub fn validate_risk_profile(risk: &RiskProfile) -> bool {
            risk.is_valid()
        }
    }
}

#[cfg(test)]
mod tests;
