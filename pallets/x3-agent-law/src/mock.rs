use crate as pallet_x3_agent_law;
use frame_support::{construct_runtime, parameter_types, traits::ConstU32};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type BlockNumber = u32;
type AccountId = u64;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const MaximumBlockWeight: frame_support::weights::Weight = frame_support::weights::Weight::from_parts(2_000_000_000_000, 0);
    pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
    pub const AvailableBlockRatio: sp_runtime::Perbill = sp_runtime::Perbill::from_percent(75);
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type AccountData = pallet_balances::AccountData<u128>;
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ();
    type AccountStore = system::Pallet<Test>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

parameter_types! {
    pub const ReputationThreshold: u64 = 100;
    pub const MaxTasksPerBlock: u32 = 50;
    pub const CheckpointGracePeriod: BlockNumber = 14400;
    pub const RateLimitEpochLength: BlockNumber = 14400; // 24 hours @ 6s blocks
    pub const RateLimitMaxExtrinsicsPerEpoch: u32 = 1000;
}

impl pallet_x3_agent_law::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = pallet_balances::Pallet<Self>;
    type ReputationThreshold = ReputationThreshold;
    type MaxTasksPerBlock = MaxTasksPerBlock;
    type CheckpointGracePeriod = CheckpointGracePeriod;
    type RateLimitEpochLength = RateLimitEpochLength;
    type RateLimitMaxExtrinsicsPerEpoch = RateLimitMaxExtrinsicsPerEpoch;
    type WeightInfo = pallet_x3_agent_law::weights::();
}

construct_runtime!(
    pub enum Test where
        Block = system::mocking::MockBlock<Test>,
        NodeBlock = system::mocking::MockBlock<Test>,
        UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>,
    {
        System: system,
        Balances: pallet_balances,
        AgentLaw: pallet_x3_agent_law,
    }
);

pub struct ExtBuilder;

impl ExtBuilder {
    pub fn build() -> sp_io::TestExternalities {
        let storage = system::GenesisConfig::default()
            .build_storage::<Test>()
            .unwrap();

        sp_io::TestExternalities::new(storage)
    }
}
