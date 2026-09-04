//! Mock runtime for pallet-x3-agent-registry tests.

use crate as pallet_x3_agent_registry;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Hooks},
};
use frame_system::EnsureRoot;
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
        AgentRegistry: pallet_x3_agent_registry,
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
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const RegistrationDeposit: u128 = 1000;
    pub const MinBondAmount: u128 = 1_000_000;
    pub const FinalityWindow: u64 = 100;
    pub const DefaultGasPerBlock: u128 = 1_000_000;
    pub const DefaultComputePerBlock: u128 = 500_000;
    pub const DefaultGasPerEpoch: u128 = 100_000_000;
    pub const DefaultComputePerEpoch: u128 = 50_000_000;
    pub const BlocksPerEpoch: u64 = 100;
    pub const ReputationThreshold: u64 = 50;
    pub const MaxTasksPerBlock: u32 = 10;
    pub const RateLimitMaxExtrinsicsPerEpoch: u32 = 1000;
    pub const ReputationDamageEnabled: bool = true;
    pub const SlashTreasury: u64 = 999;
}

impl pallet_x3_agent_registry::Config for Test {
    type Currency = Balances;
    type RegisterOrigin = EnsureRoot<u64>;
    type AdminOrigin = EnsureRoot<u64>;
    type MaxAgentsPerController = ConstU32<10>;
    type RegistrationDeposit = RegistrationDeposit;
    type MinBondAmount = MinBondAmount;
    type FinalityWindow = FinalityWindow;
    type DefaultGasPerBlock = DefaultGasPerBlock;
    type DefaultComputePerBlock = DefaultComputePerBlock;
    type DefaultGasPerEpoch = DefaultGasPerEpoch;
    type DefaultComputePerEpoch = DefaultComputePerEpoch;
    type BlocksPerEpoch = BlocksPerEpoch;
    type ReputationThreshold = ReputationThreshold;
    type MaxTasksPerBlock = MaxTasksPerBlock;
    type RateLimitMaxExtrinsicsPerEpoch = RateLimitMaxExtrinsicsPerEpoch;
    type ReputationDamageEnabled = ReputationDamageEnabled;
    type SlashRecipient = SlashTreasury;
    type AccountingSpine = x3_accounting_events::NoOpSpine;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const OPERATOR1: u64 = 10;
pub const OPERATOR2: u64 = 11;
pub const OPERATOR3: u64 = 12;
pub const TREASURY: u64 = 999;

/// Build test externalities.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 100_000_000),
            (BOB, 100_000_000),
            (CHARLIE, 100_000_000),
            (DAVE, 100_000_000),
            (OPERATOR1, 10_000_000),
            (OPERATOR2, 10_000_000),
            (OPERATOR3, 10_000_000),
            (TREASURY, 1_000_000_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Advance to a specific block.
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
        <AgentRegistry as Hooks<u64>>::on_initialize(System::block_number());
        <AgentRegistry as Hooks<u64>>::on_finalize(System::block_number());
    }
}
