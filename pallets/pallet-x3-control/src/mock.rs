//! Mock runtime for the control-plane pallet's tests.

use crate as pallet_x3_control;
use frame_support::{derive_impl, parameter_types};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Control: pallet_x3_control,
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
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type PalletInfo = PalletInfo;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_x3_control::Config for Test {
    type WeightInfo = pallet_x3_control::weights::SubstrateWeight<Test>;
}

parameter_types! {
    pub const RootAccount: u64 = 1;
    pub const CommitteeMember: u64 = 2;
    pub const Stranger: u64 = 3;
}

/// Genesis: account 1 is the only account with a root-like origin (the tests
/// use `RuntimeOrigin::root()` directly), account 2 is on the emergency
/// committee, account 3 is an ordinary signed account.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("genesis storage builds");

    let mut ext = sp_io::TestExternalities::new(storage);
    ext.execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(1);
        // Seed the committee from an origin that is allowed to set it.
        pallet_x3_control::Pallet::<Test>::set_emergency_committee(
            RuntimeOrigin::root(),
            vec![CommitteeMember::get()].try_into().unwrap(),
        )
        .expect("root may set the committee");
    });
    ext
}
