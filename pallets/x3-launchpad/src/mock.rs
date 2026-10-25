//! Mock runtime for pallet-x3-launchpad unit tests.

use crate as pallet_x3_launchpad;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError, DispatchResult,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Launchpad: pallet_x3_launchpad,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
    pub const MinLockDuration: u64 = 5;
    pub const MaxLockDuration: u64 = 1_000;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
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
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

thread_local! {
    /// The test ledger: (asset, account) -> balance.
    static LEDGER: core::cell::RefCell<std::collections::BTreeMap<(u32, u64), u128>> =
        Default::default();
    /// LP position owners.
    static POSITIONS: core::cell::RefCell<std::collections::BTreeMap<u64, u64>> =
        Default::default();
}

/// The quote asset (`QuoteAssetId`).
pub const QUOTE: u32 = 0;
/// The ledger asset the test token factory issues.
pub const TOKEN: u32 = 42;
/// The test DEX's only pool, its custody account and the position it opens.
pub const POOL: u64 = 7;
pub const POOL_ACCOUNT: u64 = 7_000;
pub const POSITION: u64 = 3;
/// Every test account starts with this much of the quote asset.
pub const STARTING_QUOTE: u128 = 1_000_000;

/// A test-only canonical ledger with the kernel's rules: checked, never saturating.
pub struct TestLedger;

impl TestLedger {
    pub fn balance(asset: u32, who: u64) -> u128 {
        LEDGER.with(|l| l.borrow().get(&(asset, who)).copied().unwrap_or(0))
    }
    pub fn fund(asset: u32, who: u64, amount: u128) {
        LEDGER.with(|l| *l.borrow_mut().entry((asset, who)).or_insert(0) += amount);
    }
    pub fn supply(asset: u32) -> u128 {
        LEDGER.with(|l| {
            l.borrow()
                .iter()
                .filter(|((a, _), _)| *a == asset)
                .map(|(_, v)| *v)
                .sum()
        })
    }
}

impl x3_asset_kernel_types::traits::CanonicalLedgerTransfer<u64> for TestLedger {
    fn ledger_transfer(asset: u32, from: &u64, to: &u64, amount: u128) -> DispatchResult {
        LEDGER.with(|l| {
            let mut l = l.borrow_mut();
            let from_balance = l.get(&(asset, *from)).copied().unwrap_or(0);
            let remaining = from_balance
                .checked_sub(amount)
                .ok_or(DispatchError::Other("insufficient ledger balance"))?;
            l.insert((asset, *from), remaining);
            let to_balance = l.get(&(asset, *to)).copied().unwrap_or(0);
            l.insert((asset, *to), to_balance + amount);
            Ok(())
        })
    }
    fn ledger_balance(asset: u32, who: &u64) -> u128 {
        Self::balance(asset, *who)
    }
}

/// Mints the whole initial supply to the creator on the ledger, as the token factory does.
pub struct TestTokenFactory;
impl pallet_x3_launchpad::TokenFactoryCreate<u64> for TestTokenFactory {
    fn create_token(
        creator: &u64,
        _symbol: Vec<u8>,
        _name: Vec<u8>,
        _decimals: u8,
        initial_supply: u128,
    ) -> Result<u32, DispatchError> {
        TestLedger::fund(TOKEN, *creator, initial_supply);
        Ok(TOKEN)
    }
}

/// A DEX that really takes the provider's deposit into the pool's account and records who owns
/// the position.
pub struct TestDex;
impl pallet_x3_launchpad::DexPoolCreate<u64> for TestDex {
    fn create_pool(_creator: &u64, _token_a: u32, _token_b: u32) -> Result<u64, DispatchError> {
        Ok(POOL)
    }
    fn add_liquidity(
        provider: &u64,
        _pool_id: u64,
        amount_a: u128,
        amount_b: u128,
    ) -> Result<u64, DispatchError> {
        use x3_asset_kernel_types::traits::CanonicalLedgerTransfer;
        TestLedger::ledger_transfer(QUOTE, provider, &POOL_ACCOUNT, amount_a)?;
        TestLedger::ledger_transfer(TOKEN, provider, &POOL_ACCOUNT, amount_b)?;
        POSITIONS.with(|p| p.borrow_mut().insert(POSITION, *provider));
        Ok(POSITION)
    }
    fn transfer_position(position_id: u64, from: &u64, to: &u64) -> DispatchResult {
        POSITIONS.with(|p| {
            let mut p = p.borrow_mut();
            if p.get(&position_id) != Some(from) {
                return Err(DispatchError::Other("not the position owner"));
            }
            p.insert(position_id, *to);
            Ok(())
        })
    }
}

impl TestDex {
    pub fn owner(position_id: u64) -> Option<u64> {
        POSITIONS.with(|p| p.borrow().get(&position_id).copied())
    }
}

parameter_types! {
    pub const LaunchpadPalletId: frame_support::PalletId = frame_support::PalletId(*b"x3/lnchp");
}

impl pallet_x3_launchpad::Config for Test {
    type GovernanceOrigin = frame_system::EnsureSigned<u64>;
    type MaxActiveLaunches = ConstU32<10>;
    type MaxContributorsPerLaunch = ConstU32<100>;
    type MinLaunchDurationBlocks = ConstU64<5>;
    type MaxLaunchDurationBlocks = ConstU64<1_000>;
    type TokenFactory = TestTokenFactory;
    type Dex = TestDex;
    type Ledger = TestLedger;
    type PalletId = LaunchpadPalletId;
    type LiquidityBps = frame_support::traits::ConstU16<5_000>;
    type QuoteAssetId = ConstU32<0>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    LEDGER.with(|l| l.borrow_mut().clear());
    POSITIONS.with(|p| p.borrow_mut().clear());
    for who in 1..=10u64 {
        TestLedger::fund(QUOTE, who, STARTING_QUOTE);
    }
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
