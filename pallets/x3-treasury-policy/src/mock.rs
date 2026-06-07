//! Test mock runtime for pallet-x3-treasury-policy.
//!
//! Uses:
//! - `EnsureRoot<u64>` for the `GovernanceOrigin` — tests call
//!   `RuntimeOrigin::root()`.
//! - `EnsureSigned<u64>` for the `OperatorOrigin` — tests call
//!   `RuntimeOrigin::signed(OPERATOR)`.
//! - `u128` as the inventory `Balance` type (matches `crate::Balance`).
//! - `MaxInsuranceReserve = 1_000_000_000_000` (one trillion units).

use crate as pallet_x3_treasury_policy;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        X3Inventory: pallet_x3_inventory,
        X3TreasuryPolicy: pallet_x3_treasury_policy,
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
    pub const MaxLiquiditySources: u32 = 8;
}

impl pallet_x3_inventory::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    /// `u128` satisfies all arithmetic bounds required by the inventory pallet
    /// and matches `crate::Balance` so no conversions are needed in helpers.
    type Balance = u128;
    type MaxLiquiditySources = MaxLiquiditySources;
    type WeightInfo = pallet_x3_inventory::weights::SubstrateWeight<Test>;
}

parameter_types! {
    /// One trillion units — large enough to test deposits without hitting the cap
    /// in normal test scenarios, but small enough to test cap rejection.
    pub const MaxInsuranceReserve: u128 = 1_000_000_000_000u128;
}

impl pallet_x3_treasury_policy::Config for Test {
    /// Root origin acts as governance in tests.
    type GovernanceOrigin = EnsureRoot<u64>;
    /// Any signed origin acts as the operator in tests.
    type OperatorOrigin = EnsureSigned<u64>;
    type MaxInsuranceReserve = MaxInsuranceReserve;
}

/// Convenience account ID used for operator calls in tests.
pub const OPERATOR: u64 = 1;

/// Build a fresh storage externalities with block number set to 1.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
