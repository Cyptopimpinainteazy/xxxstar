// Tests for pallet-x3-automation

use super::*;
use crate as pallet_x3_automation;
use frame_support::{
    derive_impl, parameter_types,
    traits::{Currency, ExistenceRequirement},
    weights::Weight,
};
use sp_runtime::DispatchError;
use x3_automation::{Action, ActionOutcome};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use std::collections::HashMap;

/// Test oracle that returns a static price map.
/// Use `set_price` to configure expected values before each test.
#[derive(Default)]
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

// Thread-local so that `OracleProvider::get_price` (a static call) can
// reach test-configured prices.
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

/// What the mock dispatcher does with an `Action::Custom` payload.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DispatchMode {
    /// Report the action applied.
    Apply,
    /// Refuse the action, as an unwired runtime would.
    Refuse,
}

std::thread_local! {
    static DISPATCH_MODE: std::cell::Cell<DispatchMode> =
        const { std::cell::Cell::new(DispatchMode::Apply) };
}

/// Run a closure with the mock dispatcher in `mode`.
pub fn with_dispatch_mode<R>(mode: DispatchMode, f: impl FnOnce() -> R) -> R {
    let previous = DISPATCH_MODE.with(|cell| cell.replace(mode));
    let out = f();
    DISPATCH_MODE.with(|cell| cell.set(previous));
    out
}

/// Test dispatcher.
///
/// `Noop` and `TransferNative` are real: the latter moves balances. `Custom`
/// follows the thread-local mode so a test can assert both the applied and the
/// refused path without a second runtime.
pub struct TestDispatcher;

impl pallet::ActionDispatcher<u64> for TestDispatcher {
    fn dispatch(
        owner: &u64,
        action: &Action,
    ) -> Result<(ActionOutcome, Weight, [u8; 64]), DispatchError> {
        match action {
            Action::Noop => Ok((ActionOutcome::Noop, Weight::from_parts(1_000, 0), [0u8; 64])),
            Action::TransferNative { recipient, amount } => {
                let mut key = [0u8; 8];
                key.copy_from_slice(&recipient[..8]);
                let to = u64::from_le_bytes(key);
                let value = u64::try_from(*amount)
                    .map_err(|_| DispatchError::Other("mock: amount out of range"))?;
                <Balances as Currency<u64>>::transfer(
                    owner,
                    &to,
                    value,
                    frame_support::traits::ExistenceRequirement::KeepAlive,
                )
                .map_err(|_| DispatchError::Other("mock: transfer refused"))?;
                let mut out = [0u8; 64];
                out[..8].copy_from_slice(&value.to_le_bytes());
                Ok((ActionOutcome::Applied, Weight::from_parts(10_000, 100), out))
            }
            Action::TransferLedgerAsset { .. } => {
                Err(DispatchError::Other("mock: no ledger asset backend"))
            }
            Action::Custom(_) => match DISPATCH_MODE.with(|cell| cell.get()) {
                DispatchMode::Apply => {
                    Ok((ActionOutcome::Applied, Weight::from_parts(5_000, 50), [0u8; 64]))
                }
                DispatchMode::Refuse => Err(DispatchError::Other("mock: custom refused")),
            },
        }
    }
}

/// Fixed wall clock for the mock: 1_700_000_000_000 ms.
pub const MOCK_NOW_MS: u64 = 1_700_000_000_000;

impl Config for Test {
    type Currency = Balances;
    type MaxTasksPerAccount = MaxTasksPerAccount;
    type BaseRegistrationFee = BaseRegistrationFee;
    type ExecutionFee = ExecutionFee;
    type MaxTaskExpiryBlocks = MaxTaskExpiryBlocks;
    type MaxAttempts = MaxAttempts;
    type Now = MockNow;
    type WeightInfo = ();
    type Oracle = TestOracle;
    type CustomEvaluator = NoopCustomEvaluator;
    type Dispatcher = TestDispatcher;
}

parameter_types! {
    pub const MaxAttempts: u32 = 3;
    pub const MockNow: u64 = MOCK_NOW_MS;
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
