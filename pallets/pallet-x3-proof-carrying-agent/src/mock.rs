//! Mock runtime for testing the Proof-Carrying Agent pallet.

use crate as pallet_x3_proof_carrying_agent;
use frame_support::{
    parameter_types,
    traits::{ConstU32, ConstU64, Everything},
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
        ProofCarryingAgent: pallet_x3_proof_carrying_agent,
    }
);

impl system::Config for Test {
    type BaseCallFilter = Everything;
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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type RuntimeTask = ();
    type ExtensionsWeightInfo = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MaxActionPayloadSize: u32 = 1024;
    pub const MaxProofPayloadSize: u32 = 4096;
    pub const MaxPendingProofsPerAgent: u32 = 10;
    pub const MaxActiveChallenges: u32 = 100;
}

pub struct MockAdminOrigin;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for MockAdminOrigin {
    type Success = u64;

    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        match frame_system::ensure_signed(o.clone()) {
            Ok(account) if account == 10 || account == 1 => Ok(account),
            Ok(_) => Err(o),
            Err(_) => Err(o),
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::from(frame_system::RawOrigin::Signed(10)))
    }
}

impl pallet_x3_proof_carrying_agent::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type AdminOrigin = MockAdminOrigin;
    type MaxActionPayloadSize = MaxActionPayloadSize;
    type MaxProofPayloadSize = MaxProofPayloadSize;
    type MaxPendingProofsPerAgent = MaxPendingProofsPerAgent;
    type MaxActiveChallenges = MaxActiveChallenges;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;
pub const ADMIN: u64 = 10;
pub const TREASURY: u64 = 999;

/// Build genesis storage with initial balances.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (ALICE, 1_000_000),
            (BOB, 1_000_000),
            (CHARLIE, 1_000_000),
            (DAVE, 1_000_000),
            (ADMIN, 1_000_000),
            (TREASURY, 1_000_000),
        ],
        dev_accounts: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
        // Set default proof config
        let config = crate::types::ProofConfig {
            max_pending_blocks: 100,
            challenge_window: 50,
            min_challenge_stake: 100,
            max_proofs_per_epoch: 50,
        };
        crate::ProofConfig::<Test>::put(config);
    });
    ext
}
