use crate as pallet_meme_overlord;
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
        MemeOverlord: pallet_meme_overlord,
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
    pub const MaxTemplateNameLength: u32 = 64;
    pub const MaxMemeDataLength: u32 = 256;
    pub const MaxAchievements: u32 = 32;
    pub const MinProfitBps: u32 = 100;
    pub const ViralThreshold: u32 = 2;
}

impl pallet_meme_overlord::Config for Test {
    type WeightInfo = ();
    type MaxTemplateNameLength = MaxTemplateNameLength;
    type MaxMemeDataLength = MaxMemeDataLength;
    type MaxAchievements = MaxAchievements;
    type MinProfitBps = MinProfitBps;
    type ViralThreshold = ViralThreshold;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("test storage builds")
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
