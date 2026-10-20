// Tests for pallet-x3-dex

use super::*;
use crate as pallet_x3_dex;
use codec::Encode;
use frame_support::{derive_impl, parameter_types, traits::Everything};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use x3_asset_kernel_types::traits::NoEconomicHalt;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        DEX: pallet_x3_dex::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
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
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MaxPools: u32 = 100;
    pub const DexPalletId: frame_support::PalletId = frame_support::PalletId(*b"x3/dexpl");
    pub const LedgerChainId: u32 = 1;
}

/// Test-only ledger stored in FRAME, like the runtime's CanonicalLedger.
/// Thread-local balances would survive with_transaction rollback and could not
/// test atomic DEX custody transfers.
pub struct TestLedger;

impl TestLedger {
    fn key(asset: u32, who: u64) -> Vec<u8> {
        (b"dex-test-ledger", asset, who).encode()
    }

    pub fn fund(asset: u32, who: u64, amount: u128) {
        let key = Self::key(asset, who);
        let balance: u128 = frame_support::storage::unhashed::get_or_default(&key);
        let funded = balance.checked_add(amount).expect("test funding fits u128");
        frame_support::storage::unhashed::put(&key, &funded);
    }
}

impl x3_asset_kernel_types::traits::CanonicalLedgerTransfer<u64> for TestLedger {
    fn ledger_transfer(
        asset: u32,
        from: &u64,
        to: &u64,
        amount: u128,
    ) -> Result<(), sp_runtime::DispatchError> {
        // Match the canonical ledger's zero/self-transfer behavior.
        if amount == 0 || from == to {
            return Ok(());
        }
        let remaining = Self::ledger_balance(asset, from)
            .checked_sub(amount)
            .ok_or(sp_runtime::DispatchError::Other(
                "insufficient ledger balance",
            ))?;
        let credited = Self::ledger_balance(asset, to)
            .checked_add(amount)
            .ok_or(sp_runtime::DispatchError::Other("ledger overflow"))?;
        frame_support::storage::unhashed::put(&Self::key(asset, *from), &remaining);
        frame_support::storage::unhashed::put(&Self::key(asset, *to), &credited);
        Ok(())
    }

    fn ledger_balance(asset: u32, who: &u64) -> u128 {
        frame_support::storage::unhashed::get_or_default(&Self::key(asset, *who))
    }
}

impl Config for Test {
    type MaxPools = MaxPools;
    type WeightInfo = ();
    type EconomicHalt = NoEconomicHalt;
    type Ledger = TestLedger;
    type PalletId = DexPalletId;
    type LedgerChainId = LedgerChainId;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = TestLedger;
}

#[cfg(feature = "runtime-benchmarks")]
impl x3_asset_kernel_types::traits::LedgerBenchmarkFunding<u64> for TestLedger {
    fn fund(asset: u32, who: &u64, amount: u128) {
        TestLedger::fund(asset, *who, amount);
    }
}

/// Every test account starts with this much of every test asset.
pub const STARTING_BALANCE: u128 = 1_000_000_000;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| {
        System::set_block_number(1);
        for asset in 0..4u32 {
            for who in 1..=5u64 {
                TestLedger::fund(asset, who, STARTING_BALANCE);
            }
        }
    });
    ext
}
