#![cfg(test)]

use crate as pallet_x3_domain_registry;

use frame_support::{construct_runtime, parameter_types, traits::Everything};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

pub type AccountId = u64;

pub type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        X3DomainRegistry: pallet_x3_domain_registry,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaxDomainLen: u32 = 253;
    pub const MaxDomains: u32 = 100;
    pub const MaxRecordsPerDomain: u32 = 16;
    pub const MaxCnameLen: u32 = 253;
    pub const MaxTxtLen: u32 = 1024;
}

impl system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SystemWeightInfo = ();
}

impl pallet_x3_domain_registry::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type UpdateOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxDomainLen = MaxDomainLen;
    type MaxDomains = MaxDomains;
    type MaxRecordsPerDomain = MaxRecordsPerDomain;
    type MaxCnameLen = MaxCnameLen;
    type MaxTxtLen = MaxTxtLen;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
