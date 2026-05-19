//! Mock runtime for Private Execution pallet tests.

use crate as pallet_private_execution;
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
    BuildStorage,
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
        PrivateExecution: pallet_private_execution,
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
    pub const PrivateExecPalletId: PalletId = PalletId(*b"pv/exec!");
    pub const PrivateFeePremiumBps: u16 = 150; // 1.5%
    pub const MinConfidentialQuorum: u32 = 2;
    pub const MaxConfidentialValidators: u32 = 100;
    pub const MaxDiffsPerBlock: u32 = 500;
    pub const MaxEncryptedPayloadSize: u32 = 1_048_576;
    pub const AttestationValidityPeriod: u64 = 7_200; // ~1 day at 12s blocks
    pub const ConfidentialValidatorShareBps: u16 = 6_000; // 60%
    pub const PrivateBurnShareBps: u16 = 2_500;            // 25%
    pub const PrivateStakerShareBps: u16 = 1_500;          // 15%
}

impl pallet_private_execution::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BurnDestination = ();
    type AdminOrigin = frame_system::EnsureRoot<u64>;
    type PalletId = PrivateExecPalletId;
    type PrivateFeePremiumBps = PrivateFeePremiumBps;
    type MinConfidentialQuorum = MinConfidentialQuorum;
    type MaxConfidentialValidators = MaxConfidentialValidators;
    type MaxDiffsPerBlock = MaxDiffsPerBlock;
    type MaxEncryptedPayloadSize = MaxEncryptedPayloadSize;
    type AttestationValidityPeriod = AttestationValidityPeriod;
    type ConfidentialValidatorShareBps = ConfidentialValidatorShareBps;
    type PrivateBurnShareBps = PrivateBurnShareBps;
    type PrivateStakerShareBps = PrivateStakerShareBps;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1_000_000),   // validator 1
            (2, 1_000_000),   // validator 2
            (3, 1_000_000),   // validator 3
            (10, 10_000_000), // user
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
