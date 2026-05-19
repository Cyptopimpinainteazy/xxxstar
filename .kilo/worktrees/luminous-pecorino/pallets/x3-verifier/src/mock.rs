// Mock runtime for testing pallet-x3-verifier.

use super::*;
use crate as pallet_x3_verifier;
use frame_support::{
    parameter_types,
    traits::{ConstU32, Everything, Get},
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
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Verifier: pallet_x3_verifier::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: u128 = 1;
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
    type AccountData = pallet_balances::AccountData<u128>;
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
    pub const MaxOutputSize: u32 = 1024;
    pub const MaxKeySize: u32 = 64;
    pub const MaxValueSize: u32 = 128;
    pub const MaxStateChanges: u32 = 32;
    pub const MaxProofDepth: u32 = 16;
    pub const MinExecutorStake: u128 = 1_000_000_000;
    pub const ExecutorRewardShare: u32 = 70;
    pub const ProtocolFeeShare: u32 = 15;
    pub const SlashAmount: u128 = 100_000_000;
    pub const JobTimeout: u64 = 100;
    pub const MaxNonce: u64 = u64::MAX;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ExecutorRegistrar = frame_system::EnsureRoot<u64>;
    type MinExecutorStake = MinExecutorStake;
    type MaxOutputSize = MaxOutputSize;
    type MaxKeySize = MaxKeySize;
    type MaxValueSize = MaxValueSize;
    type MaxStateChanges = MaxStateChanges;
    type MaxProofDepth = MaxProofDepth;
    type ExecutorRewardShare = ExecutorRewardShare;
    type ProtocolFeeShare = ProtocolFeeShare;
    type SlashAmount = SlashAmount;
    type JobTimeout = JobTimeout;
    type MaxNonce = MaxNonce;
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
    });
    ext
}