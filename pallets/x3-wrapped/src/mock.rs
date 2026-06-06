//! Test mock runtime for `pallet-x3-wrapped`.

use crate as pallet_x3_wrapped;
use frame_support::{
    derive_impl,
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
        X3Wrapped: pallet_x3_wrapped,
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

frame_support::parameter_types! {
    pub const MaxChainsPerAsset: u32 = 64;
    pub const MaxWrappedAssets: u32 = 256;
}

impl pallet_x3_wrapped::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type BridgeAuthority = frame_system::EnsureRoot<u64>;
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type MaxChainsPerAsset = MaxChainsPerAsset;
    type MaxWrappedAssets = MaxWrappedAssets;
}

// ── Common test fixtures ──────────────────────────────────────────────────────

/// A deterministic 32-byte asset ID for the canonical "X3" wrapped asset.
pub const X3_ASSET_ID: [u8; 32] = [0x01; 32];

/// A second asset ID used in multi-asset tests.
pub const USDC_ASSET_ID: [u8; 32] = [0x02; 32];

/// EVM Ethereum main-net chain ID.
pub const ETH_CHAIN: u32 = 1;

/// Arbitrum One chain ID.
pub const ARB_CHAIN: u32 = 42_161;

/// Default max wrapped supply: 1 billion units (no decimal scaling in tests).
pub const MAX_SUPPLY: u128 = 1_000_000_000;

/// Build clean test externalities starting at block 1.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

/// Convenience: return a root `RuntimeOrigin`.
pub fn root() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

/// Convenience: return a signed `RuntimeOrigin` for `id`.
pub fn signed(id: u64) -> RuntimeOrigin {
    RuntimeOrigin::signed(id)
}

/// Register the canonical X3 wrapped asset with default config.
pub fn register_x3_asset(max_supply: u128, weight_bps: u32) {
    use pallet_x3_wrapped::{WrappedAssetConfig, WrappedAssetStatus};
    frame_support::assert_ok!(pallet_x3_wrapped::Pallet::<Test>::register_wrapped_asset(
        root(),
        X3_ASSET_ID,
        WrappedAssetConfig {
            native_asset_id: [0xAA; 32],
            max_wrapped_supply: max_supply,
            governance_weight_bps: weight_bps,
            bridge_fee_bps: 30,
            status: WrappedAssetStatus::Active,
        },
    ));
}
