use crate as pallet_x3_flashloan;
use frame_support::{construct_runtime, derive_impl, parameter_types};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        FlashLoan: pallet_x3_flashloan,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
    type AccountStore = System;
}

parameter_types! {
    pub const FeeBasisPoints: u32 = 9; // 0.09%
    pub const MaxLoanFraction: u32 = 50; // 50%
}

thread_local! {
    /// The test ledger: (asset, account) -> balance.
    static LEDGER: core::cell::RefCell<std::collections::BTreeMap<(u32, u64), u128>> =
        Default::default();
}

/// A test-only canonical ledger: checked transfers over an in-memory map.
pub struct TestLedger;

impl TestLedger {
    pub fn fund(asset: u32, who: u64, amount: u128) {
        LEDGER.with(|ledger| *ledger.borrow_mut().entry((asset, who)).or_insert(0) += amount);
    }
}

impl x3_asset_kernel_types::traits::CanonicalLedgerTransfer<u64> for TestLedger {
    fn ledger_transfer(
        asset: u32,
        from: &u64,
        to: &u64,
        amount: u128,
    ) -> Result<(), sp_runtime::DispatchError> {
        LEDGER.with(|ledger| {
            let mut ledger = ledger.borrow_mut();
            let from_balance = ledger.get(&(asset, *from)).copied().unwrap_or(0);
            let remaining =
                from_balance
                    .checked_sub(amount)
                    .ok_or(sp_runtime::DispatchError::Other(
                        "insufficient ledger balance",
                    ))?;
            let credited = ledger
                .get(&(asset, *to))
                .copied()
                .unwrap_or(0)
                .checked_add(amount)
                .ok_or(sp_runtime::DispatchError::Other("ledger overflow"))?;
            ledger.insert((asset, *from), remaining);
            ledger.insert((asset, *to), credited);
            Ok(())
        })
    }

    fn ledger_balance(asset: u32, who: &u64) -> u128 {
        LEDGER.with(|ledger| ledger.borrow().get(&(asset, *who)).copied().unwrap_or(0))
    }
}

impl pallet_x3_flashloan::Config for Test {
    type Currency = Balances;
    type FeeBasisPoints = FeeBasisPoints;
    type MaxLoanFraction = MaxLoanFraction;
    type WeightInfo = pallet_x3_flashloan::weights::SubstrateWeight<Test>;
    type Ledger = TestLedger;
    type AssetPoolOrigin = frame_system::EnsureRoot<u64>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 50_000),
            (3, 200_000),
            (FlashLoan::pool_account(), 1_000_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    pallet_x3_flashloan::GenesisConfig::<Test> {
        initial_pool_balance: 1_000_000,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    LEDGER.with(|ledger| ledger.borrow_mut().clear());
    for who in 1..=3u64 {
        TestLedger::fund(7, who, 1_000_000);
    }
    t.into()
}
