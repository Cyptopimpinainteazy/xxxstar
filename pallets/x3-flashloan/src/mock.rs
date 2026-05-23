use crate as pallet_x3_flashloan;
use frame_support::{construct_runtime, derive_impl, parameter_types};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        FlashLoan: pallet_x3_flashloan,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
}

parameter_types! {
    pub const FeeBasisPoints: u32 = 9; // 0.09%
    pub const MaxLoanFraction: u32 = 50; // 50%
}

impl pallet_x3_flashloan::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type FeeBasisPoints = FeeBasisPoints;
    type MaxLoanFraction = MaxLoanFraction;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 50_000),
            (3, 200_000),
            (FlashLoan::pool_account(), 1_000_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    pallet_x3_flashloan::GenesisConfig::<Test> {
        initial_pool_balance: 1_000_000,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
