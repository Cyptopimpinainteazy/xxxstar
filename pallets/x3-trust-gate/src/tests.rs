// SPDX-License-Identifier: Apache-2.0
// Mock runtime + tests for pallet-x3-trust-gate. The pallet depends only on
// frame_system and on the registry's application identifier, so the mock wires
// no other pallet. Guardian authority is pinned to one account because the
// census is an attributable finding (spec §16), never a self-attestation.

use crate::pallet::{CategoryPolicies, Declarations, Error};
use crate::{
    CategoryId, PartitionError, Privilege, PrivilegeClass, PrivilegeSet, TrustError, TrustPolicy,
    TrustVerdict,
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

use crate as pallet_x3_trust_gate;

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

/// The account the mock treats as the Guardian authority.
const GUARDIAN: AccountId = 1;

/// The §34 categories the tests exercise.
const CATEGORY_TOKEN: CategoryId = 1;
const CATEGORY_DEX: CategoryId = 2;
const CATEGORY_UNCONFIGURED: CategoryId = 999;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        TrustGate: pallet_x3_trust_gate,
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

/// Test stand-in for the production governance origin (Root, a council, or a
/// Guardian oracle), all of which yield the approving account.
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

fn set(privileges: &[Privilege]) -> PrivilegeSet {
    PrivilegeSet::from_privileges(privileges)
}

/// TOKEN profile: minting and a configurable fee are normal for a token; a
/// freeze/drain/upgrade/oracle power is allowed but must be disclosed; taking
/// user balances or blocking selling is not acceptable at all.
fn token_profile() -> (PrivilegeSet, PrivilegeSet, PrivilegeSet) {
    (
        set(&[Privilege::Mint, Privilege::Tax]),
        set(&[
            Privilege::Freeze,
            Privilege::Drain,
            Privilege::Upgrade,
            Privilege::OracleControl,
        ]),
        set(&[
            Privilege::ModifyBalances,
            Privilege::Seize,
            Privilege::SellRestriction,
            Privilege::ArbitraryCall,
            Privilege::RouteControl,
            Privilege::AdminRole,
        ]),
    )
}

/// DEX profile, used to show that changing policy re-judges existing censuses.
fn dex_profile() -> (PrivilegeSet, PrivilegeSet, PrivilegeSet) {
    (
        set(&[
            Privilege::Tax,
            Privilege::Upgrade,
            Privilege::OracleControl,
            Privilege::RouteControl,
        ]),
        set(&[
            Privilege::Mint,
            Privilege::Drain,
            Privilege::Freeze,
            Privilege::AdminRole,
        ]),
        set(&[
            Privilege::ModifyBalances,
            Privilege::Seize,
            Privilege::SellRestriction,
            Privilege::ArbitraryCall,
        ]),
    )
}

fn install_policy(category: CategoryId, profile: (PrivilegeSet, PrivilegeSet, PrivilegeSet)) {
    assert_ok!(TrustGate::set_category_policy(
        guardian(),
        category,
        profile.0,
        profile.1,
        profile.2
    ));
}

// ── Authority ─────────────────────────────────────────────────────────────

#[test]
fn only_guardian_can_set_policy_or_record_a_census() {
    new_test_ext().execute_with(|| {
        let (allowed, disclosed, forbidden) = token_profile();

        assert_noop!(
            TrustGate::set_category_policy(
                signed(7),
                CATEGORY_TOKEN,
                allowed,
                disclosed,
                forbidden
            ),
            sp_runtime::DispatchError::BadOrigin
        );
        // Root is not accepted: the census must name who recorded it.
        assert_noop!(
            TrustGate::set_category_policy(
                RuntimeOrigin::root(),
                CATEGORY_TOKEN,
                allowed,
                disclosed,
                forbidden
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        install_policy(CATEGORY_TOKEN, token_profile());
        assert_noop!(
            TrustGate::declare_privileges(signed(7), 1, CATEGORY_TOKEN, set(&[Privilege::Mint])),
            sp_runtime::DispatchError::BadOrigin
        );
        assert_noop!(
            TrustGate::clear_privileges(signed(7), 1),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

// ── Policy shape (spec §34, §49) ──────────────────────────────────────────

#[test]
fn a_policy_must_classify_every_privilege() {
    new_test_ext().execute_with(|| {
        // The TOKEN profile with `AdminRole` left unclassified: every other
        // privilege appears exactly once.
        let incomplete = TrustPolicy {
            allowed: set(&[Privilege::Mint, Privilege::Tax]),
            disclosed: set(&[
                Privilege::Freeze,
                Privilege::Drain,
                Privilege::Upgrade,
                Privilege::OracleControl,
            ]),
            forbidden: set(&[
                Privilege::ModifyBalances,
                Privilege::Seize,
                Privilege::SellRestriction,
                Privilege::ArbitraryCall,
                Privilege::RouteControl,
            ]),
        };
        assert_eq!(
            incomplete.validate(),
            Err(PartitionError::Missing(Privilege::AdminRole))
        );

        assert_noop!(
            TrustGate::set_category_policy(
                guardian(),
                CATEGORY_TOKEN,
                incomplete.allowed,
                incomplete.disclosed,
                incomplete.forbidden
            ),
            Error::<Test>::IncompletePolicyPartition
        );
    });
}

#[test]
fn a_policy_may_not_classify_a_privilege_twice() {
    new_test_ext().execute_with(|| {
        let (allowed, disclosed, mut forbidden) = token_profile();
        // `Tax` is already allowed; forbidding it as well is ambiguous, and an
        // ambiguous policy is refused rather than resolved by precedence.
        forbidden.insert(Privilege::Tax);

        let overlapping = TrustPolicy {
            allowed,
            disclosed,
            forbidden,
        };
        assert_eq!(
            overlapping.validate(),
            Err(PartitionError::Overlap(Privilege::Tax))
        );

        assert_noop!(
            TrustGate::set_category_policy(
                guardian(),
                CATEGORY_TOKEN,
                overlapping.allowed,
                overlapping.disclosed,
                overlapping.forbidden
            ),
            Error::<Test>::OverlappingPolicyPartition
        );
        assert!(!TrustGate::category_is_configured(CATEGORY_TOKEN));
    });
}

#[test]
fn an_installed_policy_is_stored_and_classifies_its_privileges() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert!(TrustGate::category_is_configured(CATEGORY_TOKEN));

        let policy = TrustGate::policy(CATEGORY_TOKEN).unwrap();
        assert_eq!(
            policy.classify(Privilege::Mint),
            Some(PrivilegeClass::Allowed)
        );
        assert_eq!(
            policy.classify(Privilege::Freeze),
            Some(PrivilegeClass::Disclosed)
        );
        assert_eq!(
            policy.classify(Privilege::Seize),
            Some(PrivilegeClass::Forbidden)
        );
        assert!(CategoryPolicies::<Test>::contains_key(CATEGORY_TOKEN));
    });
}

// ── Census + verdict (spec §2B, §16) ──────────────────────────────────────

#[test]
fn a_census_for_an_unconfigured_category_is_refused() {
    new_test_ext().execute_with(|| {
        // Fail closed: a census nobody could judge would leave the application
        // permanently un-evaluable.
        assert_noop!(
            TrustGate::declare_privileges(
                guardian(),
                1,
                CATEGORY_UNCONFIGURED,
                set(&[Privilege::Mint])
            ),
            Error::<Test>::UnknownCategoryPolicy
        );
        assert_eq!(TrustGate::evaluate(1), Err(TrustError::UnknownApplication));
    });
}

#[test]
fn a_clean_census_is_compliant() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint, Privilege::Tax])
        ));
        assert_eq!(TrustGate::evaluate(1), Ok(TrustVerdict::Compliant));
    });
}

#[test]
fn a_disclosed_privilege_is_surfaced_not_blocked() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint, Privilege::Freeze, Privilege::Upgrade])
        ));

        match TrustGate::evaluate(1) {
            Ok(TrustVerdict::CompliantWithDisclosure { disclosed }) => {
                assert!(disclosed.contains(Privilege::Freeze));
                assert!(disclosed.contains(Privilege::Upgrade));
                // Minting is allowed without disclosure for this category.
                assert!(!disclosed.contains(Privilege::Mint));
                assert_eq!(disclosed.len(), 2);
            }
            other => panic!("expected a disclosed verdict, got {other:?}"),
        }
    });
}

#[test]
fn a_forbidden_privilege_is_recorded_and_then_refused() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());

        // The census records the finding: surfacing comes before judging
        // (spec §16: risky functionality is not automatically malicious).
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint, Privilege::Seize])
        ));
        let declaration = Declarations::<Test>::get(1).unwrap();
        assert!(declaration.privileges.contains(Privilege::Seize));
        assert_eq!(declaration.category, CATEGORY_TOKEN);
        assert_eq!(declaration.declared_by, GUARDIAN);

        // The verdict is the refusal.
        assert_eq!(
            TrustGate::evaluate(1),
            Err(TrustError::ForbiddenPrivilege(Privilege::Seize))
        );
    });
}

#[test]
fn an_unknown_application_fails_closed() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_eq!(TrustGate::evaluate(99), Err(TrustError::UnknownApplication));
    });
}

#[test]
fn re_declaring_replaces_the_census_instead_of_inheriting_it() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint, Privilege::Upgrade])
        ));
        match TrustGate::evaluate(1) {
            Ok(TrustVerdict::CompliantWithDisclosure { disclosed }) => {
                assert!(disclosed.contains(Privilege::Upgrade))
            }
            other => panic!("expected a disclosed verdict, got {other:?}"),
        }

        // A narrower census fully replaces the previous one: a removed upgrade
        // path can never keep riding on the old declaration (spec §59 SP3 in
        // spirit: privileges are bound to the censused artifact, not a name).
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint])
        ));
        assert_eq!(TrustGate::evaluate(1), Ok(TrustVerdict::Compliant));
        assert_eq!(Declarations::<Test>::get(1).unwrap().privileges.len(), 1);
    });
}

#[test]
fn tightening_a_policy_re_judges_an_existing_census() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Tax])
        ));
        assert_eq!(TrustGate::evaluate(1), Ok(TrustVerdict::Compliant));

        // Policy review later treats a transfer tax as unacceptable. Every
        // privilege is still classified exactly once.
        install_policy(
            CATEGORY_TOKEN,
            (
                set(&[
                    Privilege::Mint,
                    Privilege::Upgrade,
                    Privilege::OracleControl,
                    Privilege::RouteControl,
                ]),
                set(&[Privilege::Drain, Privilege::Freeze, Privilege::AdminRole]),
                set(&[
                    Privilege::ModifyBalances,
                    Privilege::Seize,
                    Privilege::SellRestriction,
                    Privilege::ArbitraryCall,
                    Privilege::Tax,
                ]),
            ),
        );
        assert_eq!(
            TrustGate::evaluate(1),
            Err(TrustError::ForbiddenPrivilege(Privilege::Tax))
        );
    });
}

#[test]
fn clearing_a_census_removes_the_verdict() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::Mint])
        ));
        assert_eq!(TrustGate::evaluate(1), Ok(TrustVerdict::Compliant));

        assert_ok!(TrustGate::clear_privileges(guardian(), 1));
        assert_eq!(TrustGate::evaluate(1), Err(TrustError::UnknownApplication));
        assert_noop!(
            TrustGate::clear_privileges(guardian(), 1),
            Error::<Test>::UnknownDeclaration
        );
    });
}

#[test]
fn the_same_privilege_is_judged_per_category() {
    new_test_ext().execute_with(|| {
        install_policy(CATEGORY_TOKEN, token_profile());
        install_policy(CATEGORY_DEX, dex_profile());

        // Privileged route manipulation is core to a DEX's declared model, and
        // unacceptable for a plain token (spec §34: requirements are risk-based
        // per category, and the same capability is not judged equally).
        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            1,
            CATEGORY_TOKEN,
            set(&[Privilege::RouteControl])
        ));
        assert_eq!(
            TrustGate::evaluate(1),
            Err(TrustError::ForbiddenPrivilege(Privilege::RouteControl))
        );

        assert_ok!(TrustGate::declare_privileges(
            guardian(),
            2,
            CATEGORY_DEX,
            set(&[Privilege::RouteControl])
        ));
        assert_eq!(TrustGate::evaluate(2), Ok(TrustVerdict::Compliant));
    });
}

#[test]
fn the_privilege_vocabulary_is_closed() {
    new_test_ext().execute_with(|| {
        assert_eq!(Privilege::ALL.len(), 12);
        assert_eq!(PrivilegeSet::ALL.len(), 12);
        for privilege in Privilege::ALL {
            assert!(PrivilegeSet::ALL.contains(privilege));
        }

        let mut set = PrivilegeSet::EMPTY;
        assert!(set.is_empty());
        assert!(set.insert(Privilege::Mint));
        assert!(!set.insert(Privilege::Mint));
        assert_eq!(set.len(), 1);

        // Both shipped profiles are total partitions, which is what makes a
        // runtime lookup unable to fall through to "allowed".
        for profile in [token_profile(), dex_profile()] {
            let policy = TrustPolicy {
                allowed: profile.0,
                disclosed: profile.1,
                forbidden: profile.2,
            };
            assert_eq!(policy.validate(), Ok(()));
        }
    });
}
