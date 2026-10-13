//! Mock runtime for pallet-x3-atomic-kernel tests.
//!
//! This mock provides a complete FRAME runtime for testing the full bundle
//! lifecycle: submit → assign → finalize → rollback.

use crate as pallet_x3_atomic_kernel;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, EnsureOrigin},
};
use frame_system as system;
use sp_core::offchain::{
    testing::{TestOffchainExt, TestTransactionPoolExt},
    OffchainDbExt, TransactionPoolExt,
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use std::cell::Cell;
use x3_asset_kernel_types::traits::EconomicHaltInspect;

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;

#[allow(dead_code)]
pub const ALICE: AccountId = 1;
#[allow(dead_code)]
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
#[allow(dead_code)]
pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;
pub const MIN_BOND: Balance = 10_000_000;

// ── Switchable economic halt ───────────────────────────────────────────────
//
// `NoEconomicHalt` can never halt, which left the halt guard inside
// `submit_atomic_bundle` with no way to observe it. This provider lets a test flip the flag.
//
// The flag is **thread-local**, and it used to be a process-wide `AtomicBool` behind a mutex. The
// mutex serialised the halt tests against each other, but every other test in the crate read the
// same global: `cargo test -p pallet-x3-atomic-kernel` failed about one run in three with
// `EconomicHaltActive` raised by `submit_atomic_bundle` inside a test that had nothing to do with
// halting — a test that never takes this guard can observe a halt test's `true` mid-flight.
// `cargo test` runs each test on its own thread, so a thread-local gives every test its own economy
// and removes the race instead of scheduling around it. (Measured: 2 of 6 whole-suite runs failed
// before this change, 0 of 10 after — see
// `.ai/reports/unsigned-finalization-removed-20260923.md`.)

pub struct SwitchableEconomicHalt;

thread_local! {
    static ECONOMIC_HALTED: Cell<bool> = const { Cell::new(false) };
}

impl EconomicHaltInspect for SwitchableEconomicHalt {
    fn is_halted() -> bool {
        ECONOMIC_HALTED.with(Cell::get)
    }
}

/// Opens the economy on this thread's mock; flip it with [`EconomicHaltGuard::halt`].
///
/// The flag is cleared on drop even if the test panics, so a failing halt test cannot leave the
/// next test on the same thread halted.
#[allow(dead_code)]
pub fn economy_open() -> EconomicHaltGuard {
    ECONOMIC_HALTED.with(|halted| halted.set(false));
    EconomicHaltGuard
}

#[allow(dead_code)]
pub struct EconomicHaltGuard;

#[allow(dead_code)]
impl EconomicHaltGuard {
    /// Halt new economic operations, as governance would.
    pub fn halt(&self) {
        ECONOMIC_HALTED.with(|halted| halted.set(true));
    }

    /// Lift the halt.
    pub fn resume(&self) {
        ECONOMIC_HALTED.with(|halted| halted.set(false));
    }
}

impl Drop for EconomicHaltGuard {
    fn drop(&mut self) {
        ECONOMIC_HALTED.with(|halted| halted.set(false));
    }
}

// ── Switchable VM reverter ────────────────────────────────────────────────
//
// `NoopVmReverter` always succeeds, so `do_revert_bundle_legs`'s failure path
// (its count, the `IncompleteVmRevert` event, and the rollback log) had no way
// to be observed. This reverter behaves exactly like the no-op unless a test
// flips the thread-local flag; the guard clears it on drop, even on panic, so a
// failing test cannot leak the flag into the next test on the same thread.

thread_local! {
    static VM_REVERT_FAILS: Cell<bool> = const { Cell::new(false) };
}

pub struct SwitchableVmReverter;

impl pallet_x3_atomic_kernel::vm_revert::VmReverter for SwitchableVmReverter {
    fn revert_leg(
        _vm_type: pallet_x3_atomic_kernel::proof::VmType,
        _state_diff: &pallet_x3_atomic_kernel::vm_revert::StateDiff,
    ) -> Result<
        pallet_x3_atomic_kernel::vm_revert::RevertOutcome,
        pallet_x3_atomic_kernel::vm_revert::RevertError,
    > {
        if VM_REVERT_FAILS.with(Cell::get) {
            Err(pallet_x3_atomic_kernel::vm_revert::RevertError::InvalidStateDiff)
        } else {
            Ok(pallet_x3_atomic_kernel::vm_revert::RevertOutcome::Reverted)
        }
    }
}

/// Make every subsequent revert fail on this thread; cleared on drop.
#[allow(dead_code)]
pub fn vm_revert_failing() -> VmRevertGuard {
    VM_REVERT_FAILS.with(|fails| fails.set(true));
    VmRevertGuard
}

/// Make every subsequent revert succeed on this thread; cleared on drop.
#[allow(dead_code)]
pub fn vm_revert_ok() -> VmRevertGuard {
    VM_REVERT_FAILS.with(|fails| fails.set(false));
    VmRevertGuard
}

#[allow(dead_code)]
pub struct VmRevertGuard;

impl Drop for VmRevertGuard {
    fn drop(&mut self) {
        VM_REVERT_FAILS.with(|fails| fails.set(false));
    }
}

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

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
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
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
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

pub struct RootOrSignedAccount;
impl EnsureOrigin<RuntimeOrigin> for RootOrSignedAccount {
    type Success = AccountId;

    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        match o.clone().into() {
            Ok(system::RawOrigin::Root) => Ok(0),
            Ok(system::RawOrigin::Signed(who)) => Ok(who),
            _ => Err(o),
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::signed(ALICE))
    }
}

/// Settlement-only origin for testing that `finalize_with_settlement`
/// is properly gated. Only CHARLIE may call settlement paths.
pub struct SettlementOnlyOrigin;
impl EnsureOrigin<RuntimeOrigin> for SettlementOnlyOrigin {
    type Success = AccountId;

    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        match o.clone().into() {
            Ok(system::RawOrigin::Signed(who)) if who == CHARLIE => Ok(who),
            _ => Err(o),
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::signed(CHARLIE))
    }
}

// ── Atomic Kernel Config ──────────────────────────────────────────────────

impl pallet_x3_atomic_kernel::Config for Test {
    type Currency = Balances;
    type WeightInfo = ();
    type MinBond = MinBond;
    type MaxLegsPerBundle = MaxLegsPerBundle;
    type BundleDeadlineBlocks = BundleDeadlineBlocks;
    type EconomicHalt = SwitchableEconomicHalt;
    type X3LangOrigin = RootOrSignedAccount;
    type SettlementOrigin = SettlementOnlyOrigin;
    type VmReverter = SwitchableVmReverter;
}

// ── Test Externalities Builder ────────────────────────────────────────────

#[allow(dead_code)]
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

#[allow(dead_code)]
impl ExtBuilder {
    pub fn build(self) -> TestExternalities {
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Failed to build system genesis storage");

        pallet_balances::GenesisConfig::<Test> {
            balances: self.balances,
            dev_accounts: None,
        }
        .assimilate_storage(&mut storage)
        .expect("Failed to assimilate balances storage");

        let mut ext = TestExternalities::new(storage);
        ext.execute_with(|| System::set_block_number(1));
        ext
    }
}

/// Convenience function to create a test environment with default balances.
#[allow(dead_code)]
pub fn new_test_ext() -> TestExternalities {
    ExtBuilder::default().build()
}

/// Test environment with the offchain extensions registered.
///
/// `offchain_worker` reads and clears `sp_io::offchain::local_storage_*`, and
/// submits unsigned transactions through the host transaction pool. Neither
/// works under the plain [`new_test_ext`]: the offchain DB calls panic without
/// `OffchainDbExt` and the submits panic without `TransactionPoolExt`. This
/// builder registers both and returns an accessor for the submitted extrinsics.
#[allow(dead_code)]
pub fn new_test_ext_offchain() -> (TestExternalities, impl Fn() -> Vec<Vec<u8>>) {
    let (offchain, _offchain_state) = TestOffchainExt::new();
    let (pool, pool_state) = TestTransactionPoolExt::new();

    let mut ext = ExtBuilder::default().build();
    ext.register_extension(OffchainDbExt::new(offchain));
    ext.register_extension(TransactionPoolExt::new(pool));
    (ext, move || pool_state.read().transactions.clone())
}
