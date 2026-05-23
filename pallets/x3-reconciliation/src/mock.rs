//! Test mock runtime for `pallet-x3-reconciliation`.

use crate as pallet_x3_reconciliation;
use frame_support::{
    derive_impl,
    traits::{ConstU32, ConstU64},
};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        X3Reconciliation: pallet_x3_reconciliation,
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

/// 24-hour cycle in blocks at 6-second block time.
pub const TEST_CYCLE_BLOCKS: u32 = 14_400;
/// 1-hour halt threshold at 6-second block time.
pub const TEST_HALT_THRESHOLD: u32 = 600;
/// Tolerance: 1 bps = 0.01 %.
pub const TEST_TOLERANCE_BPS: u32 = 1;
/// Governance alert at 100 bps = 1 % divergence.
pub const TEST_GOV_ALERT_BPS: u32 = 100;

frame_support::parameter_types! {
    pub const MaxSupportedChains: u32 = 16;
    pub const ReconciliationCycleBlocks: u32 = TEST_CYCLE_BLOCKS;
    pub const MintHaltThresholdBlocks: u32 = TEST_HALT_THRESHOLD;
    pub const ToleranceBps: u32 = TEST_TOLERANCE_BPS;
    pub const GovernanceDivergenceAlertBps: u32 = TEST_GOV_ALERT_BPS;
}

impl pallet_x3_reconciliation::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type MaxSupportedChains = MaxSupportedChains;
    type ReconciliationCycleBlocks = ReconciliationCycleBlocks;
    type MintHaltThresholdBlocks = MintHaltThresholdBlocks;
    type ToleranceBps = ToleranceBps;
    type GovernanceDivergenceAlertBps = GovernanceDivergenceAlertBps;
}

/// Build clean test externalities starting at block 1.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}
