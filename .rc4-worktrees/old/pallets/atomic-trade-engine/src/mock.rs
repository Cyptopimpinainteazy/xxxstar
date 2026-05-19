//! Mock runtime for AtomicTradeEngine pallet tests
//!
//! Provides a minimal test runtime with mock VM adapters for unit testing.

use crate as pallet_atomic_trade_engine;
use frame_support::{
    derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use frame_system as system;
use pallet_x3_kernel::{adapters::*, ExecutionReceipt};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use sp_std::vec::Vec;

pub struct MockEmergencyHaltController;
impl pallet_x3_kernel::EmergencyHaltController for MockEmergencyHaltController {
    fn trigger() {}
}

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime for testing
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Timestamp: pallet_timestamp,
        AtlasKernel: pallet_x3_kernel,
        AtomicTradeEngine: pallet_atomic_trade_engine,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
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
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u128 = 1;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

/// Mock EVM adapter that returns 98% output (2% fee simulation)
/// Uses the EvmExecutorAdapter trait from x3-kernel
pub struct TradeEngineEvmAdapter;

impl EvmExecutorAdapter for TradeEngineEvmAdapter {
    fn execute(
        payload: &[u8],
        gas_limit: u64,
    ) -> Result<ExecutionReceipt, sp_runtime::DispatchError> {
        // Simulate swap: parse amount from payload and return 98%
        let amount_in = if payload.len() >= 36 {
            // Parse amount from ABI-encoded payload (bytes 4-36)
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&payload[20..36]);
            u128::from_be_bytes(bytes)
        } else {
            1_000_000_000_000_000_000u128 // Default 1 token
        };

        let amount_out = amount_in.saturating_mul(98) / 100;

        // Return data: ABI-encoded uint256 output amount
        let mut return_data = vec![0u8; 32];
        return_data[16..32].copy_from_slice(&amount_out.to_be_bytes());

        Ok(ExecutionReceipt {
            version: pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: gas_limit.min(150_000),
            return_data,
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, sp_runtime::DispatchError> {
        Ok(150_000)
    }

    fn validate(payload: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        if payload.is_empty() {
            return Err(sp_runtime::DispatchError::Other("Empty payload"));
        }
        Ok(())
    }
}

/// Mock SVM adapter that returns 97% output (3% fee simulation)
pub struct TradeEngineSvmAdapter;

impl SvmExecutorAdapter for TradeEngineSvmAdapter {
    fn execute(
        payload: &[u8],
        compute_limit: u64,
    ) -> Result<ExecutionReceipt, sp_runtime::DispatchError> {
        // Simulate swap: parse amount from ABI-encoded payload (same format as EVM)
        // Payload structure: selector (4 bytes) + amount_in (32 bytes, big-endian u256)
        let amount_in = if payload.len() >= 36 {
            // Parse amount from ABI-encoded payload (bytes 4-36, last 16 bytes of u256)
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&payload[20..36]);
            u128::from_be_bytes(bytes)
        } else if payload.len() >= 16 {
            // Fallback: Parse u64 amount from Solana instruction data
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&payload[8..16]);
            u64::from_le_bytes(bytes) as u128
        } else {
            1_000_000_000u128 // Default 1 SOL
        };

        let amount_out = amount_in.saturating_mul(97) / 100;

        // Return data: u64 little-endian (SVM-style return)
        let return_data = (amount_out as u64).to_le_bytes().to_vec();

        Ok(ExecutionReceipt {
            version: pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: compute_limit.min(200_000),
            return_data,
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(payload: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        if payload.is_empty() {
            return Err(sp_runtime::DispatchError::Other("Empty payload"));
        }
        Ok(())
    }
}

/// Mock X3 adapter that returns 99% output (1% fee simulation)
pub struct TradeEngineX3Adapter;

impl X3ExecutorAdapter for TradeEngineX3Adapter {
    fn execute(
        payload: &[u8],
        gas_limit: u64,
    ) -> Result<ExecutionReceipt, sp_runtime::DispatchError> {
        // Simulate swap: parse amount from payload and return 99%
        let amount_in = if payload.len() >= 36 {
            // Parse amount from ABI-encoded payload (bytes 4-36)
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&payload[20..36]);
            u128::from_be_bytes(bytes)
        } else {
            1_000_000_000_000_000_000u128 // Default 1 token
        };

        let amount_out = amount_in.saturating_mul(99) / 100;

        // Return data: ABI-encoded uint256 output amount
        let mut return_data = vec![0u8; 32];
        return_data[16..32].copy_from_slice(&amount_out.to_be_bytes());

        Ok(ExecutionReceipt {
            version: pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: gas_limit.min(120_000),
            return_data,
            logs: Vec::new(),
            state_changes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(payload: &[u8]) -> Result<(), sp_runtime::DispatchError> {
        if payload.is_empty() {
            return Err(sp_runtime::DispatchError::Other("Empty payload"));
        }
        Ok(())
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, sp_runtime::DispatchError> {
        Ok(120_000)
    }
}

parameter_types! {
    pub const MaxAssetsPerAccount: u32 = 100;
    pub const MaxAssetSymbolLength: u32 = 32;
    pub const MaxEvmPayloadLength: u32 = 16_384;
    pub const MaxSvmPayloadLength: u32 = 16_384;
    pub const MaxX3PayloadLength: u32 = 16_384;
    pub const MaxCombinedPayloadLength: u32 = 32_768;
    pub const MaxCombinedPayloadLengthV2: u32 = 49_152;
    pub const MaxAuthorities: u32 = 100;
    pub const MinAuthorities: u32 = 1;
    pub const DefaultEvmGasLimit: u64 = 500_000;
    pub const DefaultSvmComputeLimit: u64 = 500_000;
    pub const DefaultX3GasLimit: u64 = 500_000;
    pub const MaxReplayPruneItemsPerBlock: u32 = 64;
}

pub struct BridgeEvmEscrowValue;
impl frame_support::traits::Get<sp_core::H160> for BridgeEvmEscrowValue {
    fn get() -> sp_core::H160 {
        sp_core::H160::zero()
    }
}

pub struct BridgeSvmEscrowValue;
impl frame_support::traits::Get<[u8; 32]> for BridgeSvmEscrowValue {
    fn get() -> [u8; 32] {
        [0; 32]
    }
}

impl pallet_x3_kernel::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = u128;
    type AssetId = u32;
    type AtlasId = u64;
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
    type WeightInfo = ();
    type EvmAdapter = TradeEngineEvmAdapter;
    type SvmAdapter = TradeEngineSvmAdapter;
    type X3Adapter = TradeEngineX3Adapter;
    type GovernanceOrigin = frame_system::EnsureRoot<u64>;
    type CrossVmPrepareTtl = ConstU64<10>;
    type MaxPreparedCrossVmOps = ConstU32<16>;
    type MaxPreparedOpsPerBlock = ConstU32<8>;
    type MaxReplayPruneItemsPerBlock = MaxReplayPruneItemsPerBlock;
    type RequireCrossVmProof = frame_support::traits::ConstBool<false>;
    type CrossChainProofVerifier = pallet_x3_kernel::NoopProofVerifier;
    type BridgeEvmEscrow = BridgeEvmEscrowValue;
    type BridgeSvmEscrow = BridgeSvmEscrowValue;
    type EmergencyHaltController = MockEmergencyHaltController;
}

parameter_types! {
    pub const MaxTradeLegs: u32 = 16;
    pub const MaxCheckpoints: u32 = 8;
    pub const MaxPendingBatchesPerAccount: u32 = 64;
    pub const DefaultTradeEvmGasLimit: u64 = 500_000;
    pub const DefaultTradeSvmComputeLimit: u64 = 500_000;
    pub const DefaultTradeX3GasLimit: u64 = 500_000;
}

impl pallet_atomic_trade_engine::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EvmAdapter = TradeEngineEvmAdapter;
    type SvmAdapter = TradeEngineSvmAdapter;
    type X3Adapter = TradeEngineX3Adapter;
    type MaxTradeLegs = MaxTradeLegs;
    type MaxCheckpoints = MaxCheckpoints;
    type MaxPendingBatchesPerAccount = MaxPendingBatchesPerAccount;
    type DefaultTradeEvmGasLimit = DefaultTradeEvmGasLimit;
    type DefaultTradeSvmComputeLimit = DefaultTradeSvmComputeLimit;
    type DefaultTradeX3GasLimit = DefaultTradeX3GasLimit;
    type AmmRegistrarOrigin = frame_system::EnsureRoot<u64>;
    type WeightInfo = ();
    type Settlement = pallet_atomic_trade_engine::NoOpSettlementBridge;
    type SecurityHook = x3_security_events::NoOpHook;
}

/// Build genesis storage for testing
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1_000_000_000_000_000_000u128),
            (2, 1_000_000_000_000_000_000u128),
            (3, 1_000_000_000_000_000_000u128),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Helper to advance to a specific block
pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}

/// Helper to create test account
pub fn account(id: u64) -> u64 {
    id
}
