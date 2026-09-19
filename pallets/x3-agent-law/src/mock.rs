//! Mock runtime for the agent-law pallet tests.
//!
//! Rewritten against the current polkadot-sdk: the previous version still used
//! `frame_system::Config`'s pre-2022 `Index`/`BlockNumber`/`Header` associated
//! types and `sp_runtime::testing::Header`, so it could not compile at all —
//! which is why the tests that depend on it had never run.

use crate as pallet_x3_agent_law;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64},
};
use frame_system::EnsureRoot;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        AgentLaw: pallet_x3_agent_law,
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
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const ReputationThreshold: u64 = 100;
    pub const MaxTasksPerBlock: u32 = 50;
    pub const CheckpointGracePeriod: u32 = 14400;
    // 24 hours at 6s blocks
    pub const RateLimitEpochLength: u32 = 14400;
    pub const RateLimitMaxExtrinsicsPerEpoch: u32 = 1000;
}

impl pallet_x3_agent_law::Config for Test {
    type Currency = Balances;
    type GovernanceOrigin = EnsureRoot<u64>;
    type ReputationThreshold = ReputationThreshold;
    type MaxTasksPerBlock = MaxTasksPerBlock;
    type CheckpointGracePeriod = CheckpointGracePeriod;
    type RateLimitEpochLength = RateLimitEpochLength;
    type RateLimitMaxExtrinsicsPerEpoch = RateLimitMaxExtrinsicsPerEpoch;
    type WeightInfo = ();
}

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("mock genesis storage builds");

        sp_io::TestExternalities::new(storage)
    }
}
