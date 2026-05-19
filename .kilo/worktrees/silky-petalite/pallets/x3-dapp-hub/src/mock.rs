//! Mock runtime for `pallet-x3-dapp-hub` tests.

use crate as pallet_x3_dapp_hub;
use frame_support::{
    assert_ok, derive_impl, parameter_types,
    traits::ConstU32,
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        DappHub: pallet_x3_dapp_hub,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

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
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    /// Each developer may hold at most 3 dApps in tests.
    pub const MaxDAppsPerDeveloper: u32 = 3;
    /// Global cap of 100 active dApps.
    pub const MaxActiveDApps: u32 = 100;
    pub const RegistrationDepositAmount: u128 = 1_000_000;
    pub const FeaturedPlacementFeeAmount: u128 = 5_000_000;
    pub const PremiumPlacementFeeAmount: u128 = 10_000_000;
}

impl pallet_x3_dapp_hub::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type MaxDAppsPerDeveloper = MaxDAppsPerDeveloper;
    type MaxActiveDApps = MaxActiveDApps;
    type RegistrationDepositAmount = RegistrationDepositAmount;
    type FeaturedPlacementFeeAmount = FeaturedPlacementFeeAmount;
    type PremiumPlacementFeeAmount = PremiumPlacementFeeAmount;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Build a default policy (treasury 30 % + developer 70 %) and insert it.
pub fn setup_policy(policy_id: u32) {
    use x3_revenue_sharing::{RevenueDestination, RevenueSplitEntry, RevenueSplitPolicy};

    fn empty() -> RevenueSplitEntry {
        RevenueSplitEntry { destination: RevenueDestination::Treasury, share_bps: 0 }
    }

    let policy = RevenueSplitPolicy {
        policy_id,
        entries_len: 2,
        entries: [
            RevenueSplitEntry { destination: RevenueDestination::Treasury, share_bps: 3_000 },
            RevenueSplitEntry {
                destination: RevenueDestination::DeveloperAccount,
                share_bps: 7_000,
            },
            empty(),
            empty(),
            empty(),
            empty(),
            empty(),
            empty(),
        ],
    };

    assert_ok!(DappHub::set_revenue_policy(RuntimeOrigin::root(), policy_id, policy));
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}
