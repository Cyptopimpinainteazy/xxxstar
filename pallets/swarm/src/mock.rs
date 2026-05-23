//! Mock runtime for pallet-swarm tests.

use crate as pallet_swarm;
use frame_support::{derive_impl, parameter_types, traits::ConstU32};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Swarm: pallet_swarm,
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
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
}

parameter_types! {
    pub const MinContributorStake: u128 = 1_000;
    pub const HeartbeatInterval: u64 = 100;
    pub const UnstakeCooldown: u64 = 50;
    pub const DefaultTaskTimeout: u64 = 200;
    pub const CommitPhaseDuration: u64 = 10;
    pub const RevealPhaseDuration: u64 = 10;
    pub const ContributorRewardPct: u8 = 80;
    pub const ProtocolFeePct: u8 = 10;
    pub const SlashAmount: u128 = 500;
    pub const MaxTasksPerContributor: u32 = 5;
    pub const MaxJuryVoters: u32 = 10;
}

impl pallet_swarm::Config for Test {
    type Currency = Balances;
    type AdminOrigin = frame_system::EnsureRoot<u64>;
    type SlashOrigin = frame_system::EnsureRoot<u64>;
    type MinContributorStake = MinContributorStake;
    type HeartbeatInterval = HeartbeatInterval;
    type UnstakeCooldown = UnstakeCooldown;
    type DefaultTaskTimeout = DefaultTaskTimeout;
    type CommitPhaseDuration = CommitPhaseDuration;
    type RevealPhaseDuration = RevealPhaseDuration;
    type ContributorRewardPct = ContributorRewardPct;
    type ProtocolFeePct = ProtocolFeePct;
    type SlashAmount = SlashAmount;
    type MaxTasksPerContributor = MaxTasksPerContributor;
    type MaxJuryVoters = MaxJuryVoters;
    type WeightInfo = crate::weights::SubstrateWeight<Test>;
}

/// Build genesis storage for tests.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    // Give test accounts some balance
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (3, 100_000),
            (4, 100_000),
            (5, 100_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

/// Advance blocks
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}

/// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const EVE: u64 = 5;
