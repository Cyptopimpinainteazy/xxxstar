//! Test mock runtime.

use crate as pallet_x3_slash;
use frame_support::{derive_impl, parameter_types, traits::Everything};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    AccountId32, BuildStorage,
};

type AccountId = AccountId32;
type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Slash: pallet_x3_slash,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = Everything;
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
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 500;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type MaxReserves = frame_support::traits::ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = frame_support::traits::ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MinBondAmount: u128 = 1_000_000;
    pub const FinalityWindow: u64 = 100;
    pub const ReputationDamageEnabled: bool = true;
    pub SlashTreasury: AccountId = AccountId32::from([9u8; 32]);
}

impl pallet_x3_slash::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SlashWeightInfo = pallet_x3_slash::weights::SubstrateWeight<Test>;
    type Currency = Balances;
    type MinBondAmount = MinBondAmount;
    type FinalityWindow = FinalityWindow;
    type ReputationDamageEnabled = ReputationDamageEnabled;
    type SlashRecipient = SlashTreasury;
    type AccountingSpine = x3_accounting_events::NoOpSpine;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (AccountId32::from([0u8; 32]), 1_000_000_000u128),
            (AccountId32::from([1u8; 32]), 1_000_000_000u128),
            (AccountId32::from([9u8; 32]), 500u128),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
