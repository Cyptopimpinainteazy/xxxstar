//! Mock runtime for pallet-x3-launchpad unit tests.

use crate as pallet_x3_launchpad;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64, Hooks},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError, DispatchResult,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Launchpad: pallet_x3_launchpad,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const MinLockDuration: u64 = 5;
    pub const MaxLockDuration: u64 = 1_000;
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

pub struct DummyTokenFactory;
impl pallet_x3_launchpad::TokenFactoryCreate<u64> for DummyTokenFactory {
    fn create_token(
        _creator: &u64,
        _symbol: Vec<u8>,
        _name: Vec<u8>,
        _decimals: u8,
        _initial_supply: u128,
    ) -> Result<u32, DispatchError> {
        Ok(42)
    }
}

pub struct DummyDex;
impl pallet_x3_launchpad::DexPoolCreate<u64> for DummyDex {
    fn create_pool(_creator: &u64, _token_a: u32, _token_b: u32) -> Result<u64, DispatchError> {
        Ok(7)
    }
}

pub struct DummyLpLocker;
impl pallet_x3_launchpad::LpLockCreate<u64, u64> for DummyLpLocker {
    fn lock_lp_for(
        _owner: &u64,
        _pool_id: u64,
        _lp_amount: u128,
        _unlock_at_block: u64,
    ) -> DispatchResult {
        Ok(())
    }
}

impl pallet_x3_launchpad::Config for Test {
    type GovernanceOrigin = frame_system::EnsureSigned<u64>;
    type MaxActiveLaunches = ConstU32<10>;
    type MaxContributorsPerLaunch = ConstU32<100>;
    type MinLaunchDurationBlocks = ConstU64<5>;
    type MaxLaunchDurationBlocks = ConstU64<1_000>;
    type TokenFactory = DummyTokenFactory;
    type Dex = DummyDex;
    type LpLocker = DummyLpLocker;
    type QuoteAssetId = ConstU32<0>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
