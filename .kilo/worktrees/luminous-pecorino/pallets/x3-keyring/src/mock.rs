//! Mock runtime for x3-keyring pallet tests.

use crate as pallet_x3_keyring;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU128, ConstU32, ConstU64, Hooks},
};
use frame_system::EnsureRoot;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        X3Keyring: pallet_x3_keyring,
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
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type MaxHolds = ConstU32<0>;
    type RuntimeHoldReason = ();
}

parameter_types! {
    pub const MinAttestorStake: u128 = 1_000;
    pub const MaxProofSize: u32 = 512;
    pub const MaxKeyringSize: u32 = 256;
    pub const MaxKeyringsPerAttestor: u32 = 10;
    pub const MinConfirmations: u32 = 2;
    pub const MaxConfirmations: u32 = 100;
    pub const ProofTimeout: u64 = 100;
    pub const VerificationReward: u128 = 100;
    pub const AttestationSlashAmount: u128 = 500;
}

impl pallet_x3_keyring::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AttestorRegistrar = EnsureRoot<u64>;
    type MinAttestorStake = MinAttestorStake;
    type MaxProofSize = MaxProofSize;
    type MaxKeyringSize = MaxKeyringSize;
    type MaxKeyringsPerAttestor = MaxKeyringsPerAttestor;
    type MinConfirmations = MinConfirmations;
    type MaxConfirmations = MaxConfirmations;
    type ProofTimeout = ProofTimeout;
    type VerificationReward = VerificationReward;
    type AttestationSlashAmount = AttestationSlashAmount;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;

/// Build test externalities.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 100_000),
            (BOB, 100_000),
            (CHARLIE, 100_000),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Advance to a specific block.
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
        <X3Keyring as Hooks<u64>>::on_initialize(System::block_number());
    }
}