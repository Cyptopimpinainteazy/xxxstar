use crate as pallet_x3_rebalance;
use frame_support::{
    derive_impl, parameter_types,
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
        X3Inventory: pallet_x3_inventory,
        X3Rebalance: pallet_x3_rebalance,
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

parameter_types! {
    pub const MaxLiquiditySources: u32 = 8;
}

impl pallet_x3_inventory::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type MaxLiquiditySources = MaxLiquiditySources;
    type WeightInfo = pallet_x3_inventory::weights::SubstrateWeight<Test>;
}

parameter_types! {
    /// 5 000 units per day — small enough to be breached by a 10 000-unit step in tests.
    pub const MaxDailyRebalanceVolume: u128 = 5_000;
    /// 10-block cooldown between steps on the same vault.
    pub const RebalanceCooldownBlocks: u64 = 10;
    /// Maximum 3 pending entries — small enough to reach the queue-full error in tests.
    pub const MaxPendingRebalances: u32 = 3;
}

impl pallet_x3_rebalance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MaxDailyRebalanceVolume = MaxDailyRebalanceVolume;
    type RebalanceCooldownBlocks = RebalanceCooldownBlocks;
    type MaxPendingRebalances = MaxPendingRebalances;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
