// SPDX-License-Identifier: Apache-2.0
// Mock runtime + tests for pallet-x3-app-registry. The registry depends only on
// frame_system, so the mock wires no other pallet. Guardian authority is Root,
// exactly as it must be in production (certification is a security power).

use crate::pallet::{
    AddressOwners, AppAddresses, Applications, ManifestHashes, Manifests, NextApplicationId,
    RevokedArtifacts, Versions,
};
use crate::{
    ApplicationRegistryInspect, ArtifactHashes, CertificationTier, EvidenceRef, GuardianVm,
    RestrictionReason, SecurityManifest, StandardRefs,
};
use frame_support::{
    assert_noop, assert_ok, construct_runtime, derive_impl,
    traits::{ConstU16, ConstU32, ConstU64},
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

use crate as pallet_x3_app_registry;

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        AppRegistry: pallet_x3_app_registry,
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

impl crate::pallet::Config for Test {
    type OwnerOrigin = EnsureSigned<AccountId>;
    type GuardianOrigin = EnsureRoot<AccountId>;
    type MaxApplications = ConstU32<1000>;
    type MaxAddressesPerApp = ConstU32<4>;
    type MaxVersionsPerApp = ConstU32<4>;
    type WeightInfo = ();
}

fn new_test_ext() -> sp_io::TestExternalities {
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into()
}

fn signed(who: u64) -> RuntimeOrigin {
    RuntimeOrigin::signed(who)
}

fn root() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

fn h(n: u8) -> H256 {
    H256::repeat_byte(n)
}

fn hashes(b: u8, s: u8, m: u8) -> ArtifactHashes {
    ArtifactHashes {
        bytecode: h(b),
        source: h(s),
        manifest: h(m),
    }
}

fn stds() -> StandardRefs {
    StandardRefs {
        guardian_standard: 1,
        trust_standard: 1,
        exploit_corpus: 1,
    }
}

fn register(owner: u64, name: &[u8], vm: GuardianVm, hs: ArtifactHashes) -> crate::ApplicationId {
    assert_ok!(AppRegistry::register_application(
        signed(owner),
        name.to_vec(),
        vm,
        1,
        hs,
        stds(),
    ));
    NextApplicationId::<Test>::get() - 1
}

#[test]
fn register_creates_experimental_app() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_eq!(id, 0);

        let app = Applications::<Test>::get(id).unwrap();
        assert_eq!(app.owner, 7);
        assert_eq!(app.id, id);
        assert_eq!(app.vm, GuardianVm::Evm);
        assert_eq!(app.current_version, 0);
        assert_eq!(app.tier, CertificationTier::Experimental);
        assert!(!app.revoked);

        let ver = Versions::<Test>::get(id, 0).unwrap();
        assert_eq!(ver.hashes, hashes(1, 2, 3));
        assert_eq!(ver.tier, CertificationTier::Experimental);
        assert_eq!(ver.certified_at, None);

        // Not certified => no privileges.
        assert!(!AppRegistry::has_guardian_privileges(id));
    });
}

#[test]
fn register_assigns_sequential_ids() {
    new_test_ext().execute_with(|| {
        let a = register(1, b"a", GuardianVm::Evm, hashes(1, 1, 1));
        let b = register(1, b"b", GuardianVm::X3Vm, hashes(2, 2, 2));
        let c = register(2, b"c", GuardianVm::Svm, hashes(3, 3, 3));
        assert_eq!(a, 0);
        assert_eq!(b, 1);
        assert_eq!(c, 2);
        assert_eq!(NextApplicationId::<Test>::get(), 3);
    });
}

#[test]
fn register_rejects_overlong_name() {
    new_test_ext().execute_with(|| {
        let long = vec![b'x'; 65];
        assert_noop!(
            AppRegistry::register_application(
                signed(1),
                long,
                GuardianVm::Evm,
                1,
                hashes(1, 1, 1),
                stds()
            ),
            crate::Error::<Test>::NameTooLong
        );
    });
}

#[test]
fn add_version_requires_ownership() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_noop!(
            AppRegistry::add_version(signed(8), id, hashes(4, 5, 6), stds()),
            crate::Error::<Test>::NotApplicationOwner
        );
    });
}

#[test]
fn add_version_resets_to_under_review() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        // Certify v0 first.
        assert_ok!(AppRegistry::certify_application(
            root(),
            id,
            0,
            hashes(1, 2, 3),
            Some(100)
        ));
        assert!(AppRegistry::has_guardian_privileges(id));

        // New bytecode must NOT inherit the certification.
        assert_ok!(AppRegistry::add_version(
            signed(7),
            id,
            hashes(4, 5, 6),
            stds()
        ));
        let app = Applications::<Test>::get(id).unwrap();
        assert_eq!(app.current_version, 1);
        assert_eq!(app.tier, CertificationTier::UnderReview);
        assert_eq!(app.upgrade_count, 1);
        assert!(!AppRegistry::has_guardian_privileges(id));
        // Old version history is retained.
        assert!(Versions::<Test>::contains_key(id, 0));
        assert!(Versions::<Test>::contains_key(id, 1));
    });
}

#[test]
fn submit_for_review_only_from_experimental() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_ok!(AppRegistry::submit_for_review(signed(7), id));
        assert_eq!(
            Applications::<Test>::get(id).unwrap().tier,
            CertificationTier::UnderReview
        );
        // A second submission from UNDER_REVIEW is not a valid transition.
        assert_noop!(
            AppRegistry::submit_for_review(signed(7), id),
            crate::Error::<Test>::InvalidStatusTransition
        );
    });
}

#[test]
fn certify_binds_to_exact_hashes() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));

        // Wrong bytecode hash => refused (certification cannot bind to it).
        assert_noop!(
            AppRegistry::certify_application(root(), id, 0, hashes(9, 2, 3), None),
            crate::Error::<Test>::ArtifactHashMismatch
        );
        // Wrong source hash => refused.
        assert_noop!(
            AppRegistry::certify_application(root(), id, 0, hashes(1, 9, 3), None),
            crate::Error::<Test>::ArtifactHashMismatch
        );

        assert_ok!(AppRegistry::certify_application(
            root(),
            id,
            0,
            hashes(1, 2, 3),
            Some(500)
        ));
        assert_eq!(
            Applications::<Test>::get(id).unwrap().tier,
            CertificationTier::X3Verified
        );
        let ver = Versions::<Test>::get(id, 0).unwrap();
        assert_eq!(ver.tier, CertificationTier::X3Verified);
        assert_eq!(ver.next_review, Some(500));

        assert!(AppRegistry::has_guardian_privileges(id));
        assert!(AppRegistry::is_certified_artifact(id, h(1)));
        // A different bytecode hash is never "the certified artifact".
        assert!(!AppRegistry::is_certified_artifact(id, h(9)));
    });
}

#[test]
fn certify_only_accepts_current_version() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_ok!(AppRegistry::add_version(
            signed(7),
            id,
            hashes(4, 5, 6),
            stds()
        ));
        // v0 is no longer current: certifying it cannot resurrect old bytecode.
        assert_noop!(
            AppRegistry::certify_application(root(), id, 0, hashes(1, 2, 3), None),
            crate::Error::<Test>::VersionNotCurrent
        );
    });
}

#[test]
fn only_guardian_can_certify_or_restrict() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        // A signed (non-guardian) origin cannot certify.
        assert!(AppRegistry::certify_application(signed(7), id, 0, hashes(1, 2, 3), None).is_err());
        // Nor restrict.
        assert!(
            AppRegistry::restrict_application(signed(7), id, RestrictionReason::OwnerRequest)
                .is_err()
        );
    });
}

#[test]
fn restrict_removes_privileges_and_keeps_history() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_ok!(AppRegistry::certify_application(
            root(),
            id,
            0,
            hashes(1, 2, 3),
            None
        ));
        assert!(AppRegistry::has_guardian_privileges(id));

        assert_ok!(AppRegistry::restrict_application(
            root(),
            id,
            RestrictionReason::SecurityFinding
        ));
        let app = Applications::<Test>::get(id).unwrap();
        assert_eq!(app.tier, CertificationTier::Restricted);
        assert_eq!(app.restriction, Some(RestrictionReason::SecurityFinding));
        assert!(!app.revoked); // restricted, not revoked
        assert!(!AppRegistry::has_guardian_privileges(id));
        assert_eq!(AppRegistry::is_restricted(id), Some(true));
        // History is retained, not deleted (§47).
        assert!(Versions::<Test>::contains_key(id, 0));
    });
}

#[test]
fn revoke_records_artifacts_and_blocks_recertification() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_ok!(AppRegistry::certify_application(
            root(),
            id,
            0,
            hashes(1, 2, 3),
            None
        ));

        assert_ok!(AppRegistry::revoke_application(
            root(),
            id,
            RestrictionReason::Incident
        ));
        assert!(Applications::<Test>::get(id).unwrap().revoked);
        assert!(!AppRegistry::has_guardian_privileges(id));
        assert!(AppRegistry::is_artifact_revoked(h(1)));
        assert!(AppRegistry::is_artifact_revoked(h(2)));
        assert!(AppRegistry::is_artifact_revoked(h(3)));
        assert!(!AppRegistry::is_artifact_revoked(h(9)));

        // Recertification and new versions are refused after revocation.
        assert_noop!(
            AppRegistry::certify_application(root(), id, 0, hashes(1, 2, 3), None),
            crate::Error::<Test>::ApplicationRevoked
        );
        assert_noop!(
            AppRegistry::add_version(signed(7), id, hashes(7, 8, 9), stds()),
            crate::Error::<Test>::ApplicationRevoked
        );
    });
}

#[test]
fn address_binding_is_exclusive_and_releasable() {
    new_test_ext().execute_with(|| {
        let a = register(7, b"a", GuardianVm::Evm, hashes(1, 2, 3));
        let b = register(8, b"b", GuardianVm::Evm, hashes(4, 5, 6));
        let addr = [7u8; 32];

        assert_ok!(AppRegistry::bind_address(
            signed(7),
            a,
            GuardianVm::Evm,
            addr
        ));
        assert_eq!(AddressOwners::<Test>::get((GuardianVm::Evm, addr)), Some(a));
        assert_eq!(AppAddresses::<Test>::get(a).unwrap().len(), 1);

        // A second app cannot claim the same VM address.
        assert_noop!(
            AppRegistry::bind_address(signed(8), b, GuardianVm::Evm, addr),
            crate::Error::<Test>::AddressAlreadyBound
        );
        // The same numeric address on a *different* VM is a different identity.
        assert_ok!(AppRegistry::bind_address(
            signed(8),
            b,
            GuardianVm::X3Vm,
            addr
        ));

        assert_ok!(AppRegistry::unbind_address(
            signed(7),
            a,
            GuardianVm::Evm,
            addr
        ));
        assert!(!AddressOwners::<Test>::contains_key((
            GuardianVm::Evm,
            addr
        )));
        assert!(AppAddresses::<Test>::get(a).unwrap().is_empty());

        // Now the freed EVM address can be claimed.
        assert_ok!(AppRegistry::bind_address(
            signed(8),
            b,
            GuardianVm::Evm,
            addr
        ));

        // Unbinding something not bound fails.
        assert_noop!(
            AppRegistry::unbind_address(signed(7), a, GuardianVm::Evm, addr),
            crate::Error::<Test>::AddressNotBound
        );
    });
}

#[test]
fn version_cap_enforced() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        // MaxVersionsPerApp = 4 => versions 0..=3 exist; the 5th add is refused.
        assert_ok!(AppRegistry::add_version(
            signed(7),
            id,
            hashes(4, 4, 4),
            stds()
        ));
        assert_ok!(AppRegistry::add_version(
            signed(7),
            id,
            hashes(5, 5, 5),
            stds()
        ));
        assert_ok!(AppRegistry::add_version(
            signed(7),
            id,
            hashes(6, 6, 6),
            stds()
        ));
        assert_noop!(
            AppRegistry::add_version(signed(7), id, hashes(7, 7, 7), stds()),
            crate::Error::<Test>::TooManyVersions
        );
    });
}

#[test]
fn unknown_application_is_rejected() {
    new_test_ext().execute_with(|| {
        let hs = hashes(1, 1, 1);
        assert_noop!(
            AppRegistry::add_version(signed(7), 42, hs, stds()),
            crate::Error::<Test>::UnknownApplication
        );
        assert_noop!(
            AppRegistry::certify_application(root(), 42, 0, hs, None),
            crate::Error::<Test>::UnknownApplication
        );
        assert_eq!(AppRegistry::tier(42), None);
        // Unknown is `None`, not `false`: a caller that only asks "is it
        // restricted?" must not read an absent application as unrestricted.
        assert_eq!(AppRegistry::is_restricted(42), None);
        assert!(!AppRegistry::has_guardian_privileges(42));
    });
}

#[test]
fn is_restricted_distinguishes_unknown_from_unrestricted() {
    new_test_ext().execute_with(|| {
        // Unknown application: the typed read has no answer, and the fail-closed
        // privilege helper denies it.
        assert_eq!(AppRegistry::is_restricted(99), None);
        assert!(!AppRegistry::has_guardian_privileges(99));

        // A registered, unendorsed application is known and not restricted; it
        // still holds no Guardian privileges (certification is required).
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_eq!(AppRegistry::is_restricted(id), Some(false));
        assert!(!AppRegistry::has_guardian_privileges(id));

        // Certified then restricted: known and restricted.
        assert_ok!(AppRegistry::certify_application(
            root(),
            id,
            0,
            hashes(1, 2, 3),
            None
        ));
        assert_eq!(AppRegistry::is_restricted(id), Some(false));
        assert!(AppRegistry::has_guardian_privileges(id));
        assert_ok!(AppRegistry::restrict_application(
            root(),
            id,
            RestrictionReason::SecurityFinding
        ));
        assert_eq!(AppRegistry::is_restricted(id), Some(true));
    });
}

#[test]
fn revoked_bytecode_is_tracked_fleet_wide() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert!(!RevokedArtifacts::<Test>::contains_key(h(1)));
        assert_ok!(AppRegistry::revoke_application(
            root(),
            id,
            RestrictionReason::PolicyViolation
        ));
        assert!(RevokedArtifacts::<Test>::contains_key(h(1)));
    });
}

fn manifest(name: &[u8], b: u8, s: u8, m: u8) -> SecurityManifest {
    SecurityManifest {
        name: name.to_vec().try_into().unwrap(),
        version: b"1.0.0".to_vec().try_into().unwrap(),
        vm: GuardianVm::Evm,
        category: 1,
        hashes: hashes(b, s, m),
        standards: stds(),
        declared_capabilities: vec![1u32, 2].try_into().unwrap(),
        evidence: vec![EvidenceRef {
            kind: 0,
            uri_hash: h(9),
        }]
        .try_into()
        .unwrap(),
    }
}

#[test]
fn manifest_hash_is_deterministic_and_field_sensitive() {
    new_test_ext().execute_with(|| {
        let a = manifest(b"dex", 1, 2, 3);
        let b = manifest(b"dex", 1, 2, 3);
        // Same content => same hash (deterministic).
        assert_eq!(a.canonical_hash(), b.canonical_hash());

        // Any field change => different hash (SP5: no silent reuse).
        let mut c = manifest(b"dex", 1, 2, 3);
        c.category = 2;
        assert_ne!(a.canonical_hash(), c.canonical_hash());

        let mut d = manifest(b"dex", 1, 2, 3);
        d.name = b"dexx".to_vec().try_into().unwrap();
        assert_ne!(a.canonical_hash(), d.canonical_hash());

        let e = manifest(b"dex", 4, 2, 3);
        assert_ne!(a.canonical_hash(), e.canonical_hash());
    });
}

#[test]
fn register_manifest_requires_matching_artifacts() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        // A manifest describing different bytecode is refused.
        assert_noop!(
            AppRegistry::register_manifest(signed(7), id, manifest(b"dex", 9, 2, 3)),
            crate::Error::<Test>::ArtifactHashMismatch
        );
        assert!(!Manifests::<Test>::contains_key(id));

        assert_ok!(AppRegistry::register_manifest(
            signed(7),
            id,
            manifest(b"dex", 1, 2, 3)
        ));
        assert!(Manifests::<Test>::contains_key(id));
    });
}

#[test]
fn register_manifest_records_a_traceable_hash() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        let m = manifest(b"dex", 1, 2, 3);
        let expected = m.canonical_hash();
        assert_ok!(AppRegistry::register_manifest(signed(7), id, m));

        assert_eq!(ManifestHashes::<Test>::get(id), Some(expected));
        assert_eq!(AppRegistry::manifest_hash(id), Some(expected));
        // The stored manifest re-hashes to the recorded hash.
        assert_eq!(
            Manifests::<Test>::get(id).unwrap().canonical_hash(),
            expected
        );
    });
}

#[test]
fn register_manifest_requires_ownership() {
    new_test_ext().execute_with(|| {
        let id = register(7, b"dex", GuardianVm::Evm, hashes(1, 2, 3));
        assert_noop!(
            AppRegistry::register_manifest(signed(8), id, manifest(b"dex", 1, 2, 3)),
            crate::Error::<Test>::NotApplicationOwner
        );
    });
}
