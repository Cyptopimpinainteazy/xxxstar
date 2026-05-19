//! Mock runtime for pallet-x3-atomic-kernel tests.
//!
//! This mock provides a complete FRAME runtime for testing the full bundle
//! lifecycle: submit → assign → finalize → rollback.

#![cfg(test)]

use crate as pallet_x3_atomic_kernel;
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstU32, ConstU64},
};
use frame_system as system;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    testing::UintAuthorityId,
    traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
    BuildStorage, MultiSignature, Perbill,
};
use x3_asset_kernel_types::traits::NoEconomicHalt;

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;
pub const MIN_BOND: Balance = 10_000_000;

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const ExistentialDeposit: Balance = 1;
    pub const MinBond: u128 = MIN_BOND;
    pub const MaxLegsPerBundle: u32 = 16;
    pub const BundleDeadlineBlocks: BlockNumber = 100;
}

// ── Construct Runtime ─────────────────────────────────────────────────────

construct_runtime!(
    pub enum Test
    where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system,
        Balances: pallet_balances,
        AtomicKernel: pallet_x3_atomic_kernel,
    }
);

pub type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
pub type Block = system::mocking::MockBlock<Test>;

// ── System Config ─────────────────────────────────────────────────────────

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
}

// ── Balances Config ───────────────────────────────────────────────────────

impl pallet_balances::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type MaxHolds = ConstU32<0>;
    type MaxFreezes = ConstU32<0>;
    type RuntimeFreezeReason = ();
}

// ── CreateTransactionBase + CreateBare for unsigned transactions ─────────

impl<LocalCall> frame_system::offchain::CreateTransactionBase<LocalCall> for Test
where
    RuntimeCall: From<LocalCall>,
{
    type RuntimeCall = RuntimeCall;
    type Extrinsic = UncheckedExtrinsic;
}

impl<LocalCall> frame_system::offchain::CreateBare<LocalCall> for Test
where
    RuntimeCall: From<LocalCall>,
{
    fn create_bare(call: RuntimeCall) -> UncheckedExtrinsic {
        UncheckedExtrinsic::new_bare(call)
    }
}

// ── Atomic Kernel Config ──────────────────────────────────────────────────

impl pallet_x3_atomic_kernel::Config for Test {
    type Currency = Balances;
    type WeightInfo = ();
    type MinBond = MinBond;
    type MaxLegsPerBundle = MaxLegsPerBundle;
    type BundleDeadlineBlocks = BundleDeadlineBlocks;
    type EconomicHalt = NoEconomicHalt;
}

// ── Test Externalities Builder ────────────────────────────────────────────

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![
                (ALICE, INITIAL_BALANCE),
                (BOB, INITIAL_BALANCE),
                (CHARLIE, INITIAL_BALANCE),
            ],
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Failed to build system genesis storage");

        pallet_balances::GenesisConfig::<Test> {
            balances: self.balances,
        }
        .assimilate_storage(&mut storage)
        .expect("Failed to assimilate balances storage");

        let mut ext = TestExternalities::new(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

/// Convenience function to create a test environment with default balances.
pub fn new_test_ext() -> TestExternalities {
    ExtBuilder::default().build()
}

/// Advance the current block number and trigger on_initialize/on_finalize hooks.
pub fn run_to_block(n: BlockNumber) {
    while System::block_number() < n {
        AtomicKernel::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        AtomicKernel::on_initialize(System::block_number());
    }
}

/// Helper to create a test leg with default values.
pub fn test_leg(vm_type: crate::proof::VmType) -> crate::proof::BundleLeg {
    crate::proof::BundleLeg {
        vm_type,
        token_in: H256::repeat_byte(0x01),
        token_out: H256::repeat_byte(0x02),
        amount_in: 1_000_000,
        min_amount_out: 900_000,
        deadline: 10_000,
        access: crate::proof::DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    }
}
