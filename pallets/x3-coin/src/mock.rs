//! Mock runtime for testing X3 Coin pallet

use crate as pallet_x3_coin;
use frame_support::{
    construct_runtime, derive_impl, parameter_types,
    traits::{ConstBool, ConstU32},
};
use frame_system::EnsureRoot;
use pallet_x3_kernel::{MockEvmAdapter, MockSvmAdapter, MockX3Adapter, NoopProofVerifier};
use sp_core::{H160, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

// ─── Primitive aliases ───────────────────────────────────────────────────────
pub type AccountId = u64;
pub type Balance = u128;
pub type AssetId = u32;
pub type AtlasId = u32;

pub struct BridgeEvmEscrowValue;
impl frame_support::traits::Get<H160> for BridgeEvmEscrowValue {
    fn get() -> H160 {
        H160::zero()
    }
}

pub struct BridgeSvmEscrowValue;
impl frame_support::traits::Get<[u8; 32]> for BridgeSvmEscrowValue {
    fn get() -> [u8; 32] {
        [0; 32]
    }
}

// ─── Shared constants ────────────────────────────────────────────────────────
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const ExistentialDeposit: Balance = 1;
    pub const MinimumPeriod: u64 = 1;
}

// ─── construct_runtime! ──────────────────────────────────────────────────────
construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Timestamp: pallet_timestamp,
        X3Kernel: pallet_x3_kernel,
        X3Coin: pallet_x3_coin,
    }
);

pub type Block = frame_system::mocking::MockBlock<Test>;

// ─── frame_system ────────────────────────────────────────────────────────────
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
    type Lookup = IdentityLookup<AccountId>;
    type Block = Block;
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
}

// ─── pallet_balances ─────────────────────────────────────────────────────────
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
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type DoneSlashHandler = ();
}

// ─── pallet_timestamp ────────────────────────────────────────────────────────
impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

// ─── pallet_x3_kernel ────────────────────────────────────────────────────────
parameter_types! {
    pub const MaxAssetsPerAccount: u32 = 100;
    pub const MaxAssetSymbolLength: u32 = 8;
    pub const MaxEvmPayloadLength: u32 = 65_536;
    pub const MaxSvmPayloadLength: u32 = 65_536;
    pub const MaxX3PayloadLength: u32 = 65_536;
    pub const MaxCombinedPayloadLength: u32 = 131_072;
    pub const MaxCombinedPayloadLengthV2: u32 = 196_608;
    pub const MaxAuthorities: u32 = 100;
    pub const MinAuthorities: u32 = 1;
    pub const DefaultEvmGasLimit: u64 = 1_000_000;
    pub const DefaultSvmComputeLimit: u64 = 1_000_000;
    pub const DefaultX3GasLimit: u64 = 1_000_000;
    pub const CrossVmPrepareTtl: u64 = 100;
    pub const MaxPreparedCrossVmOps: u32 = 64;
    pub const MaxPreparedOpsPerBlock: u32 = 16;
    pub const MaxReplayPruneItemsPerBlock: u32 = 64;
}

impl pallet_x3_kernel::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = Balance;
    type AssetId = AssetId;
    type AtlasId = AtlasId;
    type MaxAssetsPerAccount = MaxAssetsPerAccount;
    type MaxAssetSymbolLength = MaxAssetSymbolLength;
    type MaxEvmPayloadLength = MaxEvmPayloadLength;
    type MaxSvmPayloadLength = MaxSvmPayloadLength;
    type MaxX3PayloadLength = MaxX3PayloadLength;
    type MaxCombinedPayloadLength = MaxCombinedPayloadLength;
    type MaxCombinedPayloadLengthV2 = MaxCombinedPayloadLengthV2;
    type MaxAuthorities = MaxAuthorities;
    type MinAuthorities = MinAuthorities;
    type DefaultEvmGasLimit = DefaultEvmGasLimit;
    type DefaultSvmComputeLimit = DefaultSvmComputeLimit;
    type DefaultX3GasLimit = DefaultX3GasLimit;
    type CrossVmPrepareTtl = CrossVmPrepareTtl;
    type MaxPreparedCrossVmOps = MaxPreparedCrossVmOps;
    type MaxPreparedOpsPerBlock = MaxPreparedOpsPerBlock;
    type MaxReplayPruneItemsPerBlock = MaxReplayPruneItemsPerBlock;
    type RequireCrossVmProof = ConstBool<false>;
    type WeightInfo = ();
    type EvmAdapter = MockEvmAdapter;
    type SvmAdapter = MockSvmAdapter;
    type X3Adapter = MockX3Adapter;
    type CrossChainProofVerifier = NoopProofVerifier;
    type GovernanceOrigin = EnsureRoot<AccountId>;
    type BridgeEvmEscrow = BridgeEvmEscrowValue;
    type BridgeSvmEscrow = BridgeSvmEscrowValue;
    type EmergencyHaltController = ();
}

// ─── pallet_x3_coin ──────────────────────────────────────────────────────────
parameter_types! {
    pub const TreasuryAccount: AccountId = 1;
    pub const MaxBonusClaims: u32 = 10;
    pub const TeamVestingBlocks: u64 = 15_768_000;
    pub const TeamVestingCliff: u64 = 7_884_000;
    pub const BonusClaimPeriod: u64 = 3_942_000;
}

impl pallet_x3_coin::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type UnixTime = Timestamp;
    type WeightInfo = ();
    type TreasuryAccount = TreasuryAccount;
    type MaxBonusClaims = MaxBonusClaims;
    type TeamVestingBlocks = TeamVestingBlocks;
    type TeamVestingCliff = TeamVestingCliff;
    type BonusClaimPeriod = BonusClaimPeriod;
}

// ─── Test externalities builder ──────────────────────────────────────────────
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1_000_000_000_000_u128), // Treasury — covers existential deposit
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_x3_kernel::GenesisConfig::<Test>::default()
        .assimilate_storage(&mut t)
        .unwrap();

    pallet_x3_coin::GenesisConfig::<Test> {
        team_allocations: vec![(2, 300_000_000_000_000_000_000_u128)],
        ecosystem_allocations: vec![(3, 500_000_000_000_000_000_000_u128)],
        liquidity_allocations: vec![(4, 600_000_000_000_000_000_000_u128)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
