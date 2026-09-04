// SPDX-License-Identifier: Apache-2.0
// Mock runtime + tests for pallet-x3-sentinel. The Sentinel is a standalone
// freeze/review layer: it depends only on frame_system + its own storage, so
// the mock wires no UAK pallets.

use crate as pallet_x3_sentinel;
use frame_support::{
    assert_ok, construct_runtime, derive_impl,
    traits::{ConstU16, ConstU64, EnsureOrigin},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use x3_asset_kernel_types::{
    traits::{SentinelDenial, SentinelGuard},
    AssetId,
};

use pallet_x3_sentinel::pallet::{
    FrozenAccounts, FrozenAssets, Pallet as SentinelPallet, ReviewEnrolled,
};

type Block = frame_system::mocking::MockBlock<Test>;

type AccountId = u64;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Sentinel: pallet_x3_sentinel,
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
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

/// Sentinel freeze actions are Root-gated in this mock (as they are in prod).
pub struct FreezeRoot;
impl EnsureOrigin<RuntimeOrigin> for FreezeRoot {
    type Success = ();
    fn try_origin(o: RuntimeOrigin) -> Result<(), RuntimeOrigin> {
        system::ensure_root(o).map_err(|_| RuntimeOrigin::none())
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::root())
    }
}

impl pallet_x3_sentinel::Config for Test {
    type FreezeOrigin = FreezeRoot;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    t.into()
}

fn root() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

fn signed(who: u64) -> RuntimeOrigin {
    RuntimeOrigin::signed(who)
}

fn asset(n: u8) -> AssetId {
    H256::repeat_byte(n)
}

fn reason(
    txt: &str,
) -> frame_support::BoundedVec<u8, pallet_x3_sentinel::pallet::MaxFreezeReasonLen> {
    txt.as_bytes().to_vec().try_into().unwrap()
}

#[test]
fn sentinel_checks_freeze_authority() {
    new_test_ext().execute_with(|| {
        let a = asset(1);
        // Fresh: nothing frozen.
        assert!(!FrozenAssets::<Test>::contains_key(a));
        assert!(!FrozenAccounts::<Test>::contains_key(a, 7));

        // An unprivileged signer cannot freeze (security: not permissionless).
        assert!(Sentinel::freeze_authority(signed(7), a, 7, reason("nope")).is_err());

        // Root freezes authority 7 on asset a.
        assert_ok!(Sentinel::freeze_authority(root(), a, 7, reason("rogue minter")));
        assert!(FrozenAccounts::<Test>::contains_key(a, 7));

        // Root-level enforce now rejects 7 on a (fail-closed), but not account 8.
        assert_eq!(
            SentinelPallet::<Test>::enforce(&a, &7),
            Err(SentinelDenial::AuthorityFrozen)
        );
        assert!(SentinelPallet::<Test>::enforce(&a, &8).is_ok());

        // Unfreeze restores.
        assert_ok!(Sentinel::unfreeze_authority(root(), a, 7));
        assert!(!FrozenAccounts::<Test>::contains_key(a, 7));
        assert!(SentinelPallet::<Test>::enforce(&a, &7).is_ok());

        // Unfreezing a not-frozen authority is an error (no silent idempotence).
        assert!(Sentinel::unfreeze_authority(root(), a, 7).is_err());
    });
}

// ── Registry-required: mint-authority check ─────────────────────────────────

#[test]
fn sentinel_checks_mint_authority() {
    new_test_ext().execute_with(|| {
        let a = asset(2);
        // Root-enroll asset a for guardian review.
        assert_ok!(Sentinel::enroll_for_review(root(), a));
        assert!(ReviewEnrolled::<Test>::contains_key(a));

        // Mint authority cannot act until a guardian approval is on file.
        // No approval yet -> fail-closed, ReviewRequired.
        assert_eq!(
            SentinelPallet::<Test>::enforce(&a, &42),
            Err(SentinelDenial::ReviewRequired)
        );

        // Grant approval -> now allowed.
        assert_ok!(Sentinel::grant_guardian_approval(root(), a));
        assert!(SentinelPallet::<Test>::enforce(&a, &42).is_ok());

        // Guardian approval only applies to the enrolled asset.
        let b = asset(3);
        assert!(!ReviewEnrolled::<Test>::contains_key(b));
        assert!(SentinelPallet::<Test>::enforce(&b, &42).is_ok());

        // Enrolling twice fails; unenrolling resets approvals.
        assert!(Sentinel::enroll_for_review(root(), a).is_err());
        assert_ok!(Sentinel::unenroll_from_review(root(), a));
        assert!(!ReviewEnrolled::<Test>::contains_key(a));
        // After unenroll, back to unrestricted (no review requirement).
        assert!(SentinelPallet::<Test>::enforce(&a, &42).is_ok());
    });
}

// ── Whole-asset freeze beats per-authority checks ───────────────────────────

#[test]
fn asset_freeze_blocks_even_a_cleared_authority() {
    new_test_ext().execute_with(|| {
        let a = asset(4);
        // Freeze whole asset.
        assert_ok!(Sentinel::freeze_asset(root(), a, reason("compromise")));
        assert!(FrozenAssets::<Test>::contains_key(a));
        // Strongest denial wins regardless of authority/approvals.
        assert_eq!(
            SentinelPallet::<Test>::enforce(&a, &1),
            Err(SentinelDenial::AssetFrozen)
        );
        // Unfreeze -> back to normal.
        assert_ok!(Sentinel::unfreeze_asset(root(), a));
        assert!(SentinelPallet::<Test>::enforce(&a, &1).is_ok());
        // Double-freeze idempotence rejected.
        assert_ok!(Sentinel::freeze_asset(root(), a, reason("x")));
        assert!(Sentinel::freeze_asset(root(), a, reason("x")).is_err());
    });
}

// ── The kernel-types SentinelGuard trait impl routes to the same logic ─────

#[test]
fn kernel_sentinelguard_trait_impl_matches_enforce() {
    new_test_ext().execute_with(|| {
        let a = asset(5);
        // Trait-level consult on an unrestricted asset is allowed.
        assert!(<SentinelPallet<Test> as SentinelGuard<u64>>::can_authorize(&a, &9).is_ok());
        // Once frozen, denied at the trait level too.
        assert_ok!(Sentinel::freeze_authority(root(), a, 9, reason("bad")));
        assert_eq!(
            <SentinelPallet<Test> as SentinelGuard<u64>>::can_authorize(&a, &9),
            Err(SentinelDenial::AuthorityFrozen)
        );
    });
}

// ── Reasons are bounded; over-long reasons are impossible to store ─────────

#[test]
fn oversize_reason_cannot_be_constructed() {
    new_test_ext().execute_with(|| {
        let a = asset(6);
        let too_long: Vec<u8> = vec![b'x'; 400]; // > 256 bound
        let bound: Result<
            frame_support::BoundedVec<u8, pallet_x3_sentinel::pallet::MaxFreezeReasonLen>,
            _,
        > = too_long.clone().try_into();
        assert!(bound.is_err());
        // And nothing was frozen by the (failed) attempt.
        assert!(!FrozenAssets::<Test>::contains_key(a));
        // A valid bounded reason works.
        assert_ok!(Sentinel::freeze_asset(root(), a, reason("fine")));
        assert!(FrozenAssets::<Test>::contains_key(a));
    });
}
