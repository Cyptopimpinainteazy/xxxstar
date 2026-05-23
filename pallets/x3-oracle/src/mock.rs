// Tests for pallet-x3-oracle

use super::*;
use crate as pallet_x3_oracle;
use frame_support::{
    parameter_types,
    traits::{ConstU32, Everything},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage},
        Oracle: pallet_x3_oracle::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type ExtensionsWeightInfo = ();
    type RuntimeTask = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MaxSubmissionsPerBlock: u32 = 10;
    pub const MaxAssets: u32 = 100;
    pub const MaxSubmissionsPerAsset: u32 = 50;
    pub const MinSubmissionsForMedian: u32 = 3;
    pub const MaxSubmissionAge: u64 = 3600; // 1 hour
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxSubmissionsPerBlock = MaxSubmissionsPerBlock;
    type MaxAssets = MaxAssets;
    type MaxSubmissionsPerAsset = MaxSubmissionsPerAsset;
    type MinSubmissionsForMedian = MinSubmissionsForMedian;
    type MaxSubmissionAge = MaxSubmissionAge;
    type UpdateOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| {
        System::set_block_number(1);
        Timestamp::set_timestamp(12_000);
    });
    ext
}
