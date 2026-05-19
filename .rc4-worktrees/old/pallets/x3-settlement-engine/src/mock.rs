//! Mock runtime for x3-settlement-engine pallet tests.

use crate as pallet_x3_settlement_engine;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstBool, ConstU32, ConstU64},
};
use frame_system::EnsureRoot;
use sp_core::{H160, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

pub struct TestEmergencyHaltController;
impl pallet_x3_kernel::EmergencyHaltController for TestEmergencyHaltController {
    fn trigger() {}
}

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        AtlasKernel: pallet_x3_kernel,
        Timestamp: pallet_timestamp,
        X3SettlementEngine: pallet_x3_settlement_engine,
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
    type BlockHashCount = frame_support::traits::ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = frame_support::traits::ConstU32<50>;
    type MaxReserves = frame_support::traits::ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = frame_support::traits::ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = frame_support::traits::ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = frame_support::traits::ConstU64<1>;
    type WeightInfo = ();
}

parameter_types! {
    pub const SomeDeposit: u128 = 1000;
    pub const MockBridgeEvmEscrow: H160 = H160([0x00; 20]);
    pub const MockBridgeSvmEscrow: [u8; 32] = [0x00; 32];
}

impl pallet_x3_kernel::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = u128;
    type AssetId = u32;
    type AtlasId = u32;
    type MaxAssetsPerAccount = ConstU32<16>;
    type MaxAssetSymbolLength = ConstU32<16>;
    type MaxEvmPayloadLength = ConstU32<4096>;
    type MaxSvmPayloadLength = ConstU32<4096>;
    type MaxX3PayloadLength = ConstU32<4096>;
    type MaxCombinedPayloadLength = ConstU32<8192>;
    type MaxCombinedPayloadLengthV2 = ConstU32<12_288>;
    type MaxAuthorities = ConstU32<100>;
    type MinAuthorities = ConstU32<1>;
    type DefaultEvmGasLimit = ConstU64<10_000_000>;
    type DefaultSvmComputeLimit = ConstU64<200_000>;
    type DefaultX3GasLimit = ConstU64<5_000_000>;
    type WeightInfo = ();
    type EvmAdapter = ();
    type SvmAdapter = ();
    type X3Adapter = ();
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type CrossVmPrepareTtl = ConstU64<10>;
    type MaxPreparedCrossVmOps = ConstU32<16>;
    type MaxPreparedOpsPerBlock = ConstU32<8>;
    type RequireCrossVmProof = ConstBool<false>;
    type CrossChainProofVerifier = pallet_x3_kernel::NoopProofVerifier;
    type BridgeEvmEscrow = MockBridgeEvmEscrow;
    type BridgeSvmEscrow = MockBridgeSvmEscrow;
    type MaxReplayPruneItemsPerBlock = ConstU32<64>;
    type EmergencyHaltController = TestEmergencyHaltController;
}

impl pallet_x3_settlement_engine::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SettlementWeightInfo = ();
    type Currency = Balances;
    type UnixTime = pallet_timestamp::Pallet<Test>;
    type MaxSettlementLegs = frame_support::traits::ConstU32<4>;
    type MaxPendingIntents = frame_support::traits::ConstU32<10>;
    type DefaultSettlementTimeout = frame_support::traits::ConstU64<60>;
    type MinBtcConfirmations = frame_support::traits::ConstU32<1>;
    type ChallengePeriod = frame_support::traits::ConstU64<10>;
    type SettlementTimeoutBlocks = frame_support::traits::ConstU64<28800>; // ~24 hours at 3s blocks
    type CrossChainValidator =
        pallet_x3_settlement_engine::bridge_integration::NoOpCrossChainValidator; // Phase 4: Use no-op for tests
}

// Test accounts
pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;

/// Build test externalities.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE, 1_000_000), (BOB, 1_000_000)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
