//! Mock runtime for DePIN marketplace pallet tests.

use crate as pallet_depin_marketplace;
use frame_support::{
    parameter_types,
    traits::{ConstU128, ConstU16, ConstU32, ConstU64},
    PalletId,
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, Perbill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        DepinMarketplace: pallet_depin_marketplace,
    }
);

impl system::Config for Test {
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
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type MaxHolds = ConstU32<0>;
}

parameter_types! {
    pub const DepinMarketplacePalletId: PalletId = PalletId(*b"dp/mktpl");
    pub const ValidatorShareBps: u16 = 5_500;
    pub const BurnShareBps: u16 = 2_500;
    pub const StakerShareBps: u16 = 2_000;
    pub const MinProviderStake: u128 = 1_000;
    pub const MaxJobsPerProvider: u32 = 10;
    pub const MaxJobDuration: u64 = 100_000;
    pub const MaxPendingOrders: u32 = 1_000;
    pub const SlashFraction: Perbill = Perbill::from_percent(10);
}

impl pallet_depin_marketplace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BurnDestination = ();
    type AdminOrigin = frame_system::EnsureRoot<u64>;
    type PalletId = DepinMarketplacePalletId;
    type ValidatorShareBps = ValidatorShareBps;
    type BurnShareBps = BurnShareBps;
    type StakerShareBps = StakerShareBps;
    type MinProviderStake = MinProviderStake;
    type MaxJobsPerProvider = MaxJobsPerProvider;
    type MaxJobDuration = MaxJobDuration;
    type MaxPendingOrders = MaxPendingOrders;
    type SlashFraction = SlashFraction;
    type WeightInfo = ();
}

/// Build a test externalities with initial balances.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (3, 100_000),
            (10, 1_000_000), // rich customer
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
