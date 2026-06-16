// Tests for pallet-x3-automation

use super::*;
use crate as pallet_x3_automation;
use frame_support::{derive_impl, parameter_types};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use std::collections::HashMap;

/// Test oracle that returns a static price map.
/// Use `set_price` to configure expected values before each test.
pub struct TestOracle {
    prices: HashMap<u32, u128>,
}

impl TestOracle {
    pub fn new() -> Self {
        Self {
            prices: HashMap::new(),
        }
    }
    pub fn set_price(&mut self, asset_id: u32, price: u128) {
        self.prices.insert(asset_id, price);
    }
}

impl pallet::OracleProvider for TestOracle {
    fn get_price(asset_id: &[u8]) -> Option<u128> {
        // Decode 4 LE bytes → u32 asset id
        let id = u32::from_le_bytes(asset_id.try_into().ok()?);
        // Lookup is done via a thread-local — see `with_test_oracle` below.
        TEST_ORACLE.with(|cell| cell.borrow().prices.get(&id).copied())
    }
}

/// Thread-local so that `OracleProvider::get_price` (a static call) can
/// reach test-configured prices.
std::thread_local! {
    static TEST_ORACLE: std::cell::RefCell<TestOracle> =
        std::cell::RefCell::new(TestOracle::new());
}

/// Run a closure with the given oracle price map.
pub fn with_test_oracle<R>(prices: &[(u32, u128)], f: impl FnOnce() -> R) -> R {
    let mut oracle = TestOracle::new();
    for (id, price) in prices {
        oracle.set_price(*id, *price);
    }
    TEST_ORACLE.with(|cell| {
        *cell.borrow_mut() = oracle;
    });
    f()
}

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Automation: pallet_x3_automation,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxTasksPerAccount: u32 = 10;
    pub const BaseRegistrationFee: u64 = 100;
    pub const ExecutionFee: u64 = 50;
    pub const MaxTaskExpiryBlocks: u32 = 1000;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
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
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Balance = u64;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

impl Config for Test {
    type Currency = Balances;
    type MaxTasksPerAccount = MaxTasksPerAccount;
    type BaseRegistrationFee = BaseRegistrationFee;
    type ExecutionFee = ExecutionFee;
    type MaxTaskExpiryBlocks = MaxTaskExpiryBlocks;
    type WeightInfo = ();
    type Oracle = TestOracle;
    type CustomEvaluator = NoopCustomEvaluator;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 1000000), (2, 1000000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
