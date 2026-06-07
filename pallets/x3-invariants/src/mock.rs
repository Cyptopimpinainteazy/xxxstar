//! Test mock for pallet-x3-invariants.

use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
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
        Invariants: crate,
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
    pub const DefaultMaxSupply: u128 = 1_000_000_000_000_000_000u128; // 1 billion tokens (18 dec)
    pub const DefaultMaxAgents: u32 = 10_000;
    pub const DefaultMaxProposalDepth: u32 = 100;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type UpdateOrigin = frame_system::EnsureRoot<u64>;
    type DefaultMaxSupply = DefaultMaxSupply;
    type DefaultMaxAgents = DefaultMaxAgents;
    type DefaultMaxProposalDepth = DefaultMaxProposalDepth;
    type WeightInfo = ();
    type SecurityHook = x3_security_events::NoOpHook;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    crate::GenesisConfig::<Test> {
        max_supply: 1_000_000_000_000_000_000u128,
        max_agents: 10_000,
        max_proposal_depth: 100,
        halt_on_violation: false,
        constitution_hash: [0u8; 32],
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
