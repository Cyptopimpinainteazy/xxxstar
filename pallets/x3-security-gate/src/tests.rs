// SPDX-License-Identifier: Apache-2.0
// Mock runtime + tests for pallet-x3-security-gate. The pallet depends only on
// frame_system and on the app registry's VM vocabulary, so the mock wires no
// other pallet. Guardian authority is Root, exactly as it must be in
// production: whoever writes the policy decides what "certified" means.

use crate::pallet::{
    ActiveRuleset, CategoryPolicies, Error, NextRulesetVersion, Rulesets, SeverityExceptions,
};
use crate::{
    CategoryId, CategoryPolicy, GuardianVm, RiskProfile, RulesetVersion, Severity, SeveritySet,
    TestClass, TestClassSet,
};
use frame_support::{
    assert_noop, assert_ok, construct_runtime, derive_impl,
    traits::{ConstU16, ConstU32, ConstU64},
};
use frame_system::EnsureSigned;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

use crate as pallet_x3_security_gate;

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

/// The categories used by the tests. Real category ids are assigned by the
/// Guardian standard; these are the §34 examples the tests exercise.
const CATEGORY_TOKEN: CategoryId = 1;
const CATEGORY_DEX: CategoryId = 2;
const CATEGORY_UNKNOWN: CategoryId = 999;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        SecurityGate: pallet_x3_security_gate,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

/// The account the mock treats as the Guardian authority.
const GUARDIAN: AccountId = 1;

/// Test stand-in for the production governance origin.
///
/// The pallet requires an origin that yields an account because a severity
/// exception must name its approver (spec §30). Production wires a council
/// origin, which also yields an account; the mock pins the authority to
/// `GUARDIAN`, so a non-authority signed call is refused exactly as `BadOrigin`.
pub struct EnsureGuardian;

impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for EnsureGuardian {
    type Success = AccountId;

    fn try_origin(origin: RuntimeOrigin) -> Result<AccountId, RuntimeOrigin> {
        let who = EnsureSigned::<AccountId>::try_origin(origin)?;
        if who == GUARDIAN {
            Ok(who)
        } else {
            Err(RuntimeOrigin::signed(who))
        }
    }
}

impl crate::pallet::Config for Test {
    type GuardianOrigin = EnsureGuardian;
    type MaxJustificationLen = ConstU32<128>;
    type WeightInfo = ();
}

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

fn guardian() -> RuntimeOrigin {
    RuntimeOrigin::signed(GUARDIAN)
}

fn signed(who: AccountId) -> RuntimeOrigin {
    RuntimeOrigin::signed(who)
}

fn classes(list: &[TestClass]) -> TestClassSet {
    TestClassSet::from_classes(list)
}

fn profile(upgradeability: u8) -> RiskProfile {
    RiskProfile {
        upgradeability,
        ..RiskProfile::default()
    }
}

fn category_policy(required: &[TestClass]) -> CategoryPolicy {
    CategoryPolicy::new(classes(required), profile(2))
}

/// Create and activate a ruleset carrying one category policy and one VM
/// policy, returning its version.
fn activated_ruleset() -> RulesetVersion {
    assert_ok!(SecurityGate::create_ruleset(guardian()));
    let version = NextRulesetVersion::<Test>::get() - 1;
    assert_ok!(SecurityGate::set_category_policy(
        guardian(),
        version,
        CATEGORY_TOKEN,
        category_policy(&[TestClass::Unit, TestClass::Adversarial]),
    ));
    assert_ok!(SecurityGate::set_vm_policy(
        guardian(),
        version,
        GuardianVm::Evm,
        classes(&[TestClass::StaticAnalysis]),
    ));
    assert_ok!(SecurityGate::activate_ruleset(guardian(), version));
    version
}

// ── Authority ─────────────────────────────────────────────────────────────

#[test]
fn only_guardian_can_author_rulesets() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            SecurityGate::create_ruleset(signed(7)),
            sp_runtime::DispatchError::BadOrigin
        );
        // Root is not accepted either: the authority must be attributable to an
        // account, because it signs off on recorded severity exceptions.
        assert_noop!(
            SecurityGate::create_ruleset(RuntimeOrigin::root()),
            sp_runtime::DispatchError::BadOrigin
        );

        let version = activated_ruleset();
        assert_noop!(
            SecurityGate::activate_ruleset(signed(7), version),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            SecurityGate::set_category_policy(
                signed(7),
                version,
                CATEGORY_DEX,
                category_policy(&[TestClass::Unit]),
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            SecurityGate::set_vm_policy(
                signed(7),
                version,
                GuardianVm::Svm,
                classes(&[TestClass::Unit]),
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            SecurityGate::deprecate_ruleset(signed(7), version, None),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            SecurityGate::record_severity_exception(
                signed(7),
                version,
                CATEGORY_TOKEN,
                Severity::High,
                b"owner asked".to_vec(),
                100,
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ── Ruleset lifecycle (spec §28) ──────────────────────────────────────────

#[test]
fn activation_without_any_category_policy_is_refused() {
    new_test_ext().execute_with(|| {
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let version = NextRulesetVersion::<Test>::get() - 1;
        assert_noop!(
            SecurityGate::activate_ruleset(guardian(), version),
            Error::<Test>::NoCategoryPolicies
        );
        assert_eq!(ActiveRuleset::<Test>::get(), None);
    });
}

#[test]
fn unknown_ruleset_cannot_be_activated_or_deprecated() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            SecurityGate::activate_ruleset(guardian(), 42),
            Error::<Test>::UnknownRuleset
        );
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), 42, None),
            Error::<Test>::UnknownRuleset
        );
    });
}

#[test]
fn a_ruleset_cannot_be_activated_twice() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();
        assert_noop!(
            SecurityGate::activate_ruleset(guardian(), version),
            Error::<Test>::RulesetAlreadyActivated
        );
    });
}

#[test]
fn a_published_ruleset_can_never_be_reinterpreted() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();
        let before = CategoryPolicies::<Test>::get(version, CATEGORY_TOKEN).unwrap();

        // Editing an activated ruleset is refused: existing certifications were
        // judged against exactly these requirements (spec §28).
        assert_noop!(
            SecurityGate::set_category_policy(
                guardian(),
                version,
                CATEGORY_TOKEN,
                category_policy(&[TestClass::Unit]),
            ),
            Error::<Test>::RulesetImmutable
        );
        assert_noop!(
            SecurityGate::set_vm_policy(
                guardian(),
                version,
                GuardianVm::Evm,
                classes(&[TestClass::FormalVerification]),
            ),
            Error::<Test>::RulesetImmutable
        );
        assert_eq!(
            CategoryPolicies::<Test>::get(version, CATEGORY_TOKEN).unwrap(),
            before
        );
    });
}

#[test]
fn deprecation_records_replacement_and_refuses_the_active_ruleset() {
    new_test_ext().execute_with(|| {
        let first = activated_ruleset();

        // The active ruleset cannot be deprecated before a replacement exists:
        // there must never be a window with no policy in force.
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, None),
            Error::<Test>::CannotDeprecateActiveRuleset
        );
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, Some(77)),
            Error::<Test>::CannotDeprecateActiveRuleset
        );

        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let second = NextRulesetVersion::<Test>::get() - 1;
        assert_ok!(SecurityGate::set_category_policy(
            guardian(),
            second,
            CATEGORY_TOKEN,
            category_policy(&[TestClass::Unit]),
        ));
        assert_ok!(SecurityGate::activate_ruleset(guardian(), second));

        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, Some(77)),
            Error::<Test>::UnknownReplacement
        );
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, Some(first)),
            Error::<Test>::InvalidReplacement
        );

        // A draft ruleset is not an acceptable replacement: it must itself be
        // activated first, so the chain never points at an inactive policy.
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let draft = NextRulesetVersion::<Test>::get() - 1;
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, Some(draft)),
            Error::<Test>::ReplacementNotActivated
        );

        assert_ok!(SecurityGate::deprecate_ruleset(
            guardian(),
            first,
            Some(second)
        ));
        assert_eq!(
            Rulesets::<Test>::get(first).unwrap().replacement,
            Some(second)
        );
        assert_eq!(ActiveRuleset::<Test>::get(), Some(second));
        assert_noop!(
            SecurityGate::deprecate_ruleset(guardian(), first, Some(second)),
            Error::<Test>::RulesetAlreadyDeprecated
        );
    });
}

// ── Requirements (spec §34) ───────────────────────────────────────────────

#[test]
fn required_tests_is_the_union_of_category_and_vm_profiles() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();
        let required =
            SecurityGate::required_tests(version, GuardianVm::Evm, CATEGORY_TOKEN).unwrap();

        assert!(required.contains(TestClass::Unit));
        assert!(required.contains(TestClass::Adversarial));
        // Contributed by the VM profile, not the category profile.
        assert!(required.contains(TestClass::StaticAnalysis));
        assert!(!required.contains(TestClass::FormalVerification));
        assert_eq!(required.len(), 3);
    });
}

#[test]
fn required_tests_fails_closed() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();

        // Unknown category: an error, never "no requirements".
        assert!(matches!(
            SecurityGate::required_tests(version, GuardianVm::Evm, CATEGORY_UNKNOWN),
            Err(Error::<Test>::UnknownCategoryPolicy)
        ));
        // Known category, unknown VM.
        assert!(matches!(
            SecurityGate::required_tests(version, GuardianVm::Svm, CATEGORY_TOKEN),
            Err(Error::<Test>::UnknownVmPolicy)
        ));
        // Unknown ruleset.
        assert!(matches!(
            SecurityGate::required_tests(9, GuardianVm::Evm, CATEGORY_TOKEN),
            Err(Error::<Test>::UnknownRuleset)
        ));

        // A draft ruleset carries no authorities even if it has policies.
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let draft = NextRulesetVersion::<Test>::get() - 1;
        assert_ok!(SecurityGate::set_category_policy(
            guardian(),
            draft,
            CATEGORY_TOKEN,
            category_policy(&[TestClass::Unit]),
        ));
        assert!(matches!(
            SecurityGate::required_tests(draft, GuardianVm::Evm, CATEGORY_TOKEN),
            Err(Error::<Test>::RulesetNotActivated)
        ));
    });
}

#[test]
fn an_empty_requirement_set_is_refused() {
    new_test_ext().execute_with(|| {
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let version = NextRulesetVersion::<Test>::get() - 1;
        assert_noop!(
            SecurityGate::set_category_policy(
                guardian(),
                version,
                CATEGORY_TOKEN,
                category_policy(&[]),
            ),
            Error::<Test>::EmptyRequiredTests
        );
        assert_noop!(
            SecurityGate::set_vm_policy(guardian(), version, GuardianVm::Evm, TestClassSet::EMPTY,),
            Error::<Test>::EmptyRequiredTests
        );
    });
}

#[test]
fn category_policy_upsert_does_not_inflate_the_counter() {
    new_test_ext().execute_with(|| {
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let version = NextRulesetVersion::<Test>::get() - 1;
        for _ in 0..3 {
            assert_ok!(SecurityGate::set_category_policy(
                guardian(),
                version,
                CATEGORY_TOKEN,
                category_policy(&[TestClass::Unit]),
            ));
        }
        assert_eq!(Rulesets::<Test>::get(version).unwrap().category_policies, 1);
        assert_ok!(SecurityGate::set_category_policy(
            guardian(),
            version,
            CATEGORY_DEX,
            category_policy(&[TestClass::Unit, TestClass::LiveNode]),
        ));
        assert_eq!(Rulesets::<Test>::get(version).unwrap().category_policies, 2);
    });
}

#[test]
fn an_out_of_range_risk_dimension_is_refused() {
    new_test_ext().execute_with(|| {
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let version = NextRulesetVersion::<Test>::get() - 1;

        let too_risky = CategoryPolicy::new(classes(&[TestClass::Unit]), profile(11));
        assert!(!too_risky.risk.is_valid());
        assert_noop!(
            SecurityGate::set_category_policy(guardian(), version, CATEGORY_TOKEN, too_risky),
            Error::<Test>::RiskDimensionOutOfRange
        );

        // The boundary value is accepted.
        let at_limit = CategoryPolicy::new(classes(&[TestClass::Unit]), profile(10));
        assert_ok!(SecurityGate::set_category_policy(
            guardian(),
            version,
            CATEGORY_TOKEN,
            at_limit,
        ));
    });
}

// ── Severity thresholds and exceptions (spec §30) ─────────────────────────

#[test]
fn high_and_critical_block_certification_by_default() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();
        let now = System::block_number();

        assert!(!SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::Info,
            now
        )
        .unwrap());
        assert!(!SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::Medium,
            now
        )
        .unwrap());
        assert!(SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::High,
            now
        )
        .unwrap());
        assert!(SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::Critical,
            now
        )
        .unwrap());

        // Threshold decisions fail closed for unknown inputs.
        assert!(matches!(
            SecurityGate::severity_blocks_certification(
                version,
                CATEGORY_UNKNOWN,
                Severity::High,
                now
            ),
            Err(Error::<Test>::UnknownCategoryPolicy)
        ));
        assert!(matches!(
            SecurityGate::severity_blocks_certification(9, CATEGORY_TOKEN, Severity::High, now),
            Err(Error::<Test>::UnknownRuleset)
        ));
    });
}

#[test]
fn a_recorded_exception_suppresses_blocking_until_it_expires() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();
        assert_ok!(SecurityGate::record_severity_exception(
            guardian(),
            version,
            CATEGORY_TOKEN,
            Severity::High,
            b"documented remediation plan, ticket GUARD-1".to_vec(),
            50,
        ));

        assert!(!SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::High,
            10
        )
        .unwrap());
        // Critical was not excepted, so it still blocks.
        assert!(SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::Critical,
            10
        )
        .unwrap());

        // The exception expires; blocking returns.
        System::set_block_number(50);
        assert!(SecurityGate::severity_blocks_certification(
            version,
            CATEGORY_TOKEN,
            Severity::High,
            System::block_number()
        )
        .unwrap());
        assert!(
            SecurityGate::live_exception(version, CATEGORY_TOKEN, Severity::High, 50).is_none()
        );

        // Withdrawing the exception also restores blocking.
        assert_ok!(SecurityGate::revoke_severity_exception(
            guardian(),
            version,
            CATEGORY_TOKEN,
            Severity::High,
        ));
        assert!(
            SeverityExceptions::<Test>::get((version, CATEGORY_TOKEN), Severity::High).is_none()
        );
        assert_noop!(
            SecurityGate::revoke_severity_exception(
                guardian(),
                version,
                CATEGORY_TOKEN,
                Severity::High
            ),
            Error::<Test>::UnknownSeverityException
        );
    });
}

#[test]
fn an_exception_must_be_meaningful_and_attributable() {
    new_test_ext().execute_with(|| {
        let version = activated_ruleset();

        // Exceptions are only for severities that actually block.
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                version,
                CATEGORY_TOKEN,
                Severity::Info,
                b"nothing to except".to_vec(),
                50,
            ),
            Error::<Test>::SeverityNotBlocking
        );
        // No expiry in the past.
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                version,
                CATEGORY_TOKEN,
                Severity::High,
                b"expired already".to_vec(),
                System::block_number(),
            ),
            Error::<Test>::ExpiryNotInFuture
        );
        // No silent exception: the justification is mandatory.
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                version,
                CATEGORY_TOKEN,
                Severity::High,
                Vec::new(),
                50,
            ),
            Error::<Test>::EmptyJustification
        );
        // Bounded justification.
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                version,
                CATEGORY_TOKEN,
                Severity::High,
                vec![b'x'; 129],
                50,
            ),
            Error::<Test>::JustificationTooLong
        );
        // Unknown category cannot be excepted.
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                version,
                CATEGORY_UNKNOWN,
                Severity::High,
                b"documented".to_vec(),
                50,
            ),
            Error::<Test>::UnknownCategoryPolicy
        );
    });
}

#[test]
fn a_draft_ruleset_cannot_issue_exceptions() {
    new_test_ext().execute_with(|| {
        assert_ok!(SecurityGate::create_ruleset(guardian()));
        let draft = NextRulesetVersion::<Test>::get() - 1;
        assert_ok!(SecurityGate::set_category_policy(
            guardian(),
            draft,
            CATEGORY_TOKEN,
            category_policy(&[TestClass::Unit]),
        ));
        assert_noop!(
            SecurityGate::record_severity_exception(
                guardian(),
                draft,
                CATEGORY_TOKEN,
                Severity::Critical,
                b"documented".to_vec(),
                50,
            ),
            Error::<Test>::RulesetNotActivated
        );
    });
}

// ── Set algebra (the vocabulary that policies are written in) ─────────────

#[test]
fn test_class_sets_are_deterministic_and_closed() {
    new_test_ext().execute_with(|| {
        let mut set = TestClassSet::EMPTY;
        assert!(set.is_empty());
        assert!(set.insert(TestClass::Unit));
        assert!(!set.insert(TestClass::Unit));
        assert!(set.contains(TestClass::Unit));
        assert_eq!(set.len(), 1);

        let other = classes(&[TestClass::Fuzz, TestClass::Adversarial]);
        let union = set.union(other);
        assert_eq!(union.len(), 3);
        assert_eq!(
            classes(&[TestClass::Unit, TestClass::Adversarial])
                .bits()
                .count_ones(),
            2
        );

        let severities = SeveritySet::from_severities(&[Severity::Low, Severity::Critical]);
        assert!(severities.contains(Severity::Critical));
        assert!(!severities.contains(Severity::High));
        assert!(SeveritySet::EMPTY.is_empty());
        assert!(SeveritySet::HIGH_AND_CRITICAL.contains(Severity::High));
        assert!(Severity::Critical > Severity::High);
    });
}
