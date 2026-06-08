// SPDX-License-Identifier: Apache-2.0
//
// Test mock for pallet-x3-lp-locker.

use crate as pallet_x3_lp_locker;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        LpLocker: pallet_x3_lp_locker,
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
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU32<42>;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const MinLockDuration: u64 = 100; // 100 blocks minimum
    pub const MaxLockDuration: u64 = 100_000; // 100k blocks maximum (~2 weeks at 12s blocks)
}

impl pallet_x3_lp_locker::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MinLockDuration = MinLockDuration;
    type MaxLockDuration = MaxLockDuration;
    type WeightInfo = ();
}

/// Build genesis storage.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    t.into()
}