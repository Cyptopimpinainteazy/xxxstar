#![cfg(test)]

use crate as pallet_x3_kernel;
use core::sync::atomic::{AtomicBool, Ordering};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{ConstBool, ConstU32, ConstU64, Get},
};
use frame_system as system;
use parity_scale_codec::Encode;
use sp_core::{H160, H256};
use sp_io::TestExternalities;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage, DispatchError,
};

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u128;
pub type AssetId = u32;
pub type AtlasId = u32;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CHARLIE: AccountId = 3;
pub const INITIAL_BALANCE: Balance = 1_000_000_000_000;

pub static EMERGENCY_HALT_TRIGGERED: AtomicBool = AtomicBool::new(false);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 250;
    pub const ExistentialDeposit: Balance = 1;
}

// Wrapper types for bridge escrow addresses
pub struct BridgeEvmEscrowValue;
impl Get<H160> for BridgeEvmEscrowValue {
    fn get() -> H160 {
        H160::zero()
    }
}

pub struct BridgeSvmEscrowValue;
impl Get<[u8; 32]> for BridgeSvmEscrowValue {
    fn get() -> [u8; 32] {
        [0; 32]
    }
}

pub struct TestEmergencyHaltController;
impl pallet_x3_kernel::EmergencyHaltController for TestEmergencyHaltController {
    fn trigger() {
        EMERGENCY_HALT_TRIGGERED.store(true, Ordering::SeqCst);
    }
}

pub type Block = system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Balances: pallet_balances,
        AtlasKernel: pallet_x3_kernel,
    }
);

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type Nonce = u64;
    type Block = Block;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<AccountId>;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type ExtensionsWeightInfo = ();
    type RuntimeTask = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MinimumPeriod: u64 = 6000;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

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
    type RuntimeHoldReason = ();
    type FreezeIdentifier = ();
    type RuntimeFreezeReason = ();
    type MaxFreezes = ConstU32<0>;
    type DoneSlashHandler = ();
}

// A minimal adapter used by tests to emit a deterministic state change so
// canonical ledger updates can be verified via submit_comit.
pub struct TestEvmAdapter;

impl pallet_x3_kernel::EvmExecutorAdapter for TestEvmAdapter {
    fn execute(_payload: &[u8], _gas_limit: u64) -> Result<crate::ExecutionReceipt, DispatchError> {
        // Use a fixed asset/balance so tests can assert on canonical ledger.
        let account: AccountId = ALICE;
        let asset_id: AssetId = 0;
        let balance: Balance = 123;

        // SCALE-encode asset_id and balance into 32-byte keys/values.
        let mut key_bytes = [0u8; 32];
        let asset_bytes = asset_id.encode();
        key_bytes[..asset_bytes.len()].copy_from_slice(&asset_bytes);

        let mut value_bytes = [0u8; 32];
        let balance_bytes = balance.encode();
        value_bytes[..balance_bytes.len()].copy_from_slice(&balance_bytes);

        Ok(crate::ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 21000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![crate::StateChange {
                address: account.encode(),
                key: H256::from(key_bytes),
                value: H256::from(value_bytes),
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(21000)
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

pub struct TestSvmAdapter;

impl pallet_x3_kernel::SvmExecutorAdapter for TestSvmAdapter {
    fn execute(
        _payload: &[u8],
        _compute_limit: u64,
    ) -> Result<crate::ExecutionReceipt, DispatchError> {
        let account: AccountId = ALICE;
        let asset_id: AssetId = 1;
        let balance: Balance = 222;

        let mut key_bytes = [0u8; 32];
        let asset_bytes = asset_id.encode();
        key_bytes[..asset_bytes.len()].copy_from_slice(&asset_bytes);

        let mut value_bytes = [0u8; 32];
        let balance_bytes = balance.encode();
        value_bytes[..balance_bytes.len()].copy_from_slice(&balance_bytes);

        Ok(crate::ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 5000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![crate::StateChange {
                address: account.encode(),
                key: H256::from(key_bytes),
                value: H256::from(value_bytes),
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }
}

pub struct TestX3Adapter;

impl pallet_x3_kernel::X3ExecutorAdapter for TestX3Adapter {
    fn execute(payload: &[u8], _gas_limit: u64) -> Result<crate::ExecutionReceipt, DispatchError> {
        // Simulate an execution failure when payload starts with 0xFF.
        if payload.first() == Some(&0xFF) {
            return Err(DispatchError::Other("X3 execution failed"));
        }

        let account: AccountId = ALICE;
        let asset_id: AssetId = 2;
        let balance: Balance = 333;

        let mut key_bytes = [0u8; 32];
        let asset_bytes = asset_id.encode();
        key_bytes[..asset_bytes.len()].copy_from_slice(&asset_bytes);

        let mut value_bytes = [0u8; 32];
        let balance_bytes = balance.encode();
        value_bytes[..balance_bytes.len()].copy_from_slice(&balance_bytes);

        Ok(crate::ExecutionReceipt {
            version: crate::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 1000,
            return_data: Vec::new(),
            logs: Vec::new(),
            state_changes: vec![crate::StateChange {
                address: account.encode(),
                key: H256::from(key_bytes),
                value: H256::from(value_bytes),
            }],
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(_payload: &[u8]) -> Result<(), DispatchError> {
        Ok(())
    }

    fn estimate_gas(_payload: &[u8]) -> Result<u64, DispatchError> {
        Ok(1000)
    }
}

impl pallet_x3_kernel::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Balance = Balance;
    type AssetId = AssetId;
    type AtlasId = AtlasId;
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
    type EvmAdapter = TestEvmAdapter;
    type SvmAdapter = TestSvmAdapter;
    type X3Adapter = TestX3Adapter;
    type GovernanceOrigin = frame_system::EnsureRoot<AccountId>;
    type CrossVmPrepareTtl = ConstU64<10>;
    type MaxPreparedCrossVmOps = ConstU32<16>;
    type MaxPreparedOpsPerBlock = ConstU32<8>;
    type MaxReplayPruneItemsPerBlock = ConstU32<64>;
    type RequireCrossVmProof = ConstBool<false>;
    type CrossChainProofVerifier = crate::NoopProofVerifier;
    type BridgeEvmEscrow = BridgeEvmEscrowValue;
    type BridgeSvmEscrow = BridgeSvmEscrowValue;
    type EmergencyHaltController = TestEmergencyHaltController;
}

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
    authorized_accounts: Vec<AccountId>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![],
            authorized_accounts: vec![],
        }
    }
}

impl ExtBuilder {
    pub fn balances(mut self, balances: Vec<(AccountId, Balance)>) -> Self {
        self.balances = balances;
        self
    }

    pub fn authorized_accounts(mut self, accounts: Vec<AccountId>) -> Self {
        self.authorized_accounts = accounts;
        self
    }

    pub fn build(self) -> TestExternalities {
        EMERGENCY_HALT_TRIGGERED.store(false, Ordering::SeqCst);
        let mut storage = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .expect("Failed to build system genesis storage");

        // Apply balances genesis
        pallet_balances::GenesisConfig::<Test> {
            balances: self.balances,
            dev_accounts: None,
        }
        .assimilate_storage(&mut storage)
        .expect("Failed to assimilate balances storage");

        let mut t = TestExternalities::new(storage);

        t.execute_with(|| {
            System::set_block_number(1);
            // Set initial timestamp
            Timestamp::set_timestamp(12000);
            // Initialize authorized accounts
            for account in self.authorized_accounts {
                pallet_x3_kernel::AuthorizedAccounts::<Test>::insert(account, ());
            }
        });
        t
    }
}

pub fn new_test_ext() -> TestExternalities {
    ExtBuilder::default()
        .balances(vec![
            (ALICE, INITIAL_BALANCE),
            (BOB, INITIAL_BALANCE),
            (CHARLIE, INITIAL_BALANCE),
        ])
        .authorized_accounts(vec![ALICE, BOB, CHARLIE])
        .build()
}

#[test]
fn migration_runs_and_sets_storage_version() {
    // Ensure migration sets storage version to declared value
    ExtBuilder::default().build().execute_with(|| {
        use frame_support::traits::StorageVersion;
        // Simulate older version
        StorageVersion::new(0).put::<pallet_x3_kernel::Pallet<Test>>();
        // Run migration
        let _w = <crate::migrations::Migration<Test> as frame_support::traits::OnRuntimeUpgrade>::on_runtime_upgrade();
        // Check it advanced
        assert!(
            StorageVersion::get::<pallet_x3_kernel::Pallet<Test>>()
                >= pallet_x3_kernel::STORAGE_VERSION
        );
    });
}

/// Mock implementation of DualVmDispatcher for testing
pub struct MockDispatcher;

impl pallet_x3_kernel::DualVmDispatcher for MockDispatcher {
    type AccountId = AccountId;
    type Balance = Balance;

    fn execute_evm_tx(
        &self,
        _tx: Vec<u8>,
    ) -> Result<pallet_x3_kernel::ExecutionReceipt, DispatchError> {
        Ok(pallet_x3_kernel::ExecutionReceipt {
            version: pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 21000,
            return_data: Default::default(),
            logs: Default::default(),
            state_changes: Default::default(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn execute_svm_tx(
        &self,
        _tx: Vec<u8>,
    ) -> Result<pallet_x3_kernel::ExecutionReceipt, DispatchError> {
        Ok(pallet_x3_kernel::ExecutionReceipt {
            version: pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            success: true,
            gas_used: 0,
            return_data: Default::default(),
            logs: Default::default(),
            state_changes: Default::default(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn execute_dual_tx(
        &self,
        evm_tx: Option<Vec<u8>>,
        svm_tx: Option<Vec<u8>>,
    ) -> Result<pallet_x3_kernel::SphereState, DispatchError> {
        let _evm_receipt = if evm_tx.is_some() {
            Some(self.execute_evm_tx(evm_tx.unwrap())?)
        } else {
            None
        };

        let _svm_receipt = if svm_tx.is_some() {
            Some(self.execute_svm_tx(svm_tx.unwrap())?)
        } else {
            None
        };

        Ok(pallet_x3_kernel::SphereState {
            state_root: H256::zero(),
            block_number: 1,
            timestamp: 12000,
        })
    }

    fn merge_receipts(
        &self,
        _evm_receipt: Option<&pallet_x3_kernel::ExecutionReceipt>,
        _svm_receipt: Option<&pallet_x3_kernel::ExecutionReceipt>,
    ) -> pallet_x3_kernel::SphereState {
        pallet_x3_kernel::SphereState {
            state_root: H256::zero(),
            block_number: 1,
            timestamp: 12000,
        }
    }

    /// Check authorization - in mock, always allow ALICE, deny others for non-empty ops
    fn auth_check(&self, caller: &Self::AccountId, operation: &[u8]) -> Result<(), DispatchError> {
        if *caller == ALICE {
            Ok(())
        } else if operation.is_empty() {
            Ok(())
        } else {
            Err(DispatchError::BadOrigin)
        }
    }

    /// Calculate fees: 1 unit per 1000 gas + 1 unit per 1000 compute units
    fn fee_accounting(
        &self,
        evm_gas_used: u64,
        svm_compute_units: u64,
        base_fee: Self::Balance,
    ) -> Result<Self::Balance, DispatchError> {
        let evm_fee = (evm_gas_used as u128) / 1000;
        let svm_fee = (svm_compute_units as u128) / 1000;
        let total = base_fee + evm_fee + svm_fee;
        Ok(total)
    }

    /// Update canonical ledger - in mock, just verify state changes are well-formed
    fn canonical_ledger_update(
        &self,
        _comit_id: H256,
        state_changes: &[pallet_x3_kernel::StateChange],
    ) -> Result<(), DispatchError> {
        // Verify all state changes have valid addresses
        for change in state_changes {
            if change.address.is_empty() {
                return Err(DispatchError::Other("Invalid state change address"));
            }
        }
        Ok(())
    }
}
