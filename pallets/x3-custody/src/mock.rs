//! Test mock runtime for `pallet-x3-custody`.

use crate as pallet_x3_custody;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        X3Custody: pallet_x3_custody,
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
    /// Small vault capacity makes MaxSignersReached easy to trigger in tests.
    pub const MaxSignersPerVault: u32 = 4;
    pub const MaxVaultsPerSigner: u32 = 8;
    pub const MaxPoliciesPerTier: u32 = 4;
    /// Key rotation period used by the rotation-semantics tests.
    pub const KeyRotationPeriod: u64 = 100;
}

impl pallet_x3_custody::Config for Test {
    /// Governance: root only (simulates on-chain council supermajority).
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    /// Operator: any signed account.
    type OperatorOrigin = frame_system::EnsureSigned<u64>;
    type MaxSignersPerVault = MaxSignersPerVault;
    type MaxVaultsPerSigner = MaxVaultsPerSigner;
    type MaxPoliciesPerTier = MaxPoliciesPerTier;
    type KeyRotationPeriod = KeyRotationPeriod;
}

/// Build externalities whose genesis authorizes the given gateway accounts.
///
/// `new_test_ext` builds only the frame-system genesis, so the registry starts
/// empty — which is exactly the posture a chain that named no gateway has, and the
/// one the origin tests below start from.
pub fn new_test_ext_with_gateways(
    x3_lang_gateways: Vec<u64>,
    settlement_gateways: Vec<u64>,
) -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_x3_custody::GenesisConfig::<Test> {
        x3_lang_gateways,
        settlement_gateways,
        ..Default::default()
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Build clean test externalities with block number initialised to 1.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut ext: sp_io::TestExternalities = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap()
        .into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
