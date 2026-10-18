use crate as pallet_x3_kernel;
use crate::ExecutionReceipt;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
pub static EMERGENCY_HALT_TRIGGER_COUNT: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// The test chain's private-submission posture, flipped by the tests that need both answers.
    ///
    /// A constant would make the refusal untestable in the direction that matters: a check that can
    /// only ever be `false` is indistinguishable from an adapter that cannot run X3BC at all, so the
    /// tests have to be able to say "the chain *can* offer a private channel" and watch the same bytes
    /// run.
    ///
    /// Per-thread rather than a process-global `static`, because `cargo test` runs these tests on
    /// several threads at once: a global posture let one test flip the chain for another, and the
    /// refusal tests failed with `Ok(())` — the check had read a *different* test's chain. Each
    /// thread gets the honest default (`false`), and a test that flips it restores it.
    static PRIVATE_SUBMISSION_CHANNEL: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// `Config::PrivateSubmissionChannel` for the mock chain.
pub struct TestPrivateSubmissionChannel;
impl Get<bool> for TestPrivateSubmissionChannel {
    fn get() -> bool {
        PRIVATE_SUBMISSION_CHANNEL.with(|enabled| enabled.get())
    }
}

impl TestPrivateSubmissionChannel {
    /// Set this thread's chain posture. Only the thread that set it sees the value.
    pub fn set(enabled: bool) {
        PRIVATE_SUBMISSION_CHANNEL.with(|flag| flag.set(enabled));
    }
}

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
        EMERGENCY_HALT_TRIGGER_COUNT.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct TestProofVerifier;
impl pallet_x3_kernel::CrossChainProofVerifier<AccountId> for TestProofVerifier {
    fn verify_proof(
        _origin: &AccountId,
        _operation: &x3_cross_vm_bridge::CrossVmOperation,
        proof: &pallet_x3_kernel::CrossChainProof,
    ) -> Result<(), DispatchError> {
        match proof {
            pallet_x3_kernel::CrossChainProof::None => Ok(()),
            pallet_x3_kernel::CrossChainProof::LockProof(bytes)
            | pallet_x3_kernel::CrossChainProof::MerkleReceipt(bytes)
                if !bytes.is_empty() =>
            {
                Ok(())
            }
            _ => Err(DispatchError::Other("invalid test proof")),
        }
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
thread_local! { static REAL_EVM_ADAPTER: core::cell::Cell<bool> = const { core::cell::Cell::new(false) }; }
/// Run the actual production EVM adapter through this test chain's signed dispatch.
pub fn with_real_evm_adapter<R>(run: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            REAL_EVM_ADAPTER.with(|flag| flag.set(self.0));
        }
    }
    let previous = REAL_EVM_ADAPTER.with(|flag| flag.replace(true));
    let _restore = Restore(previous);
    run()
}

pub struct TestEvmAdapter;

impl pallet_x3_kernel::EvmExecutorAdapter for TestEvmAdapter {
    fn execute_call(
        caller: H160,
        _target: H160,
        input: &[u8],
        _value: u128,
        gas_limit: u64,
    ) -> Result<ExecutionReceipt, DispatchError> {
        Self::execute_from(input, gas_limit, caller)
    }

    fn execute_from(
        payload: &[u8],
        gas_limit: u64,
        _caller: sp_core::H160,
    ) -> Result<ExecutionReceipt, DispatchError> {
        if REAL_EVM_ADAPTER.with(|flag| flag.get()) {
            return <crate::wasm_adapters::ContextualWasmEvmAdapter<Test> as pallet_x3_kernel::EvmExecutorAdapter>::execute_from(payload, gas_limit, _caller);
        }
        Self::execute(payload, gas_limit)
    }

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
            storage_writes: Vec::new(),
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

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        // X3-LANG-004: the payload is the artifact the adapter executes. The chain's EVM adapter
        // refuses a SCALE-encoded `Packet` by name, so this mock has to as well — otherwise a test
        // could pass a payload every live chain refuses and still come out green.
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "EVM payload is a SCALE-encoded Packet, not bytecode: this adapter cannot execute it",
            ));
        }
        // Mirror the validator the live EVM adapter runs (`mini_evm::validate_evm`), so a fixture
        // this mock accepts is a fixture the chain accepts too. A permissive mock here is exactly
        // how the payload convention went unnoticed: every kernel test passed a packet that no
        // live chain will execute.
        x3_evm_integration::mini_evm::validate_evm(payload)
            .map_err(|_| DispatchError::Other("EVM validation failed"))
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
            storage_writes: Vec::new(),
            protocol_version: 1,
            migration_history: Vec::new(),
            compatibility_flags: 0,
            from: Vec::new(),
            to: Vec::new(),
            value: 0,
        })
    }

    fn validate(payload: &[u8]) -> Result<(), DispatchError> {
        // Same rule as `TestEvmAdapter::validate` above, for the SVM domain (X3-LANG-004).
        if crate::packet_adapters::payload_is_packet(payload) {
            return Err(DispatchError::Other(
                "SVM payload is a SCALE-encoded Packet, not a program: this adapter cannot execute it",
            ));
        }
        // Same rule as `TestEvmAdapter::validate` above: mirror the live SVM program validator.
        x3_svm_integration::interp_validate_program(payload)
            .map_err(|_| DispatchError::Other("SVM validation failed"))
    }
}

pub struct TestX3Adapter;

impl pallet_x3_kernel::X3ExecutorAdapter for TestX3Adapter {
    fn execute(payload: &[u8], _gas_limit: u64) -> Result<crate::ExecutionReceipt, DispatchError> {
        Self::execute_with_slots(payload, _gas_limit, &[])
    }

    /// The test adapter fabricates a receipt and has no chain storage to read: the slots it returns
    /// are the ones this mock synthesizes, which is exactly why the production adapter's
    /// `execute_with_slots` is exercised in `tests/x3_adapter_route.rs` instead of here.
    fn execute_with_slots(
        payload: &[u8],
        _gas_limit: u64,
        _slots: &[(sp_core::H256, [u8; 32])],
    ) -> Result<crate::ExecutionReceipt, DispatchError> {
        // Simulate an execution failure when the SCALE-encoded
        // `Packet::X3Vm(X3VmPacket::Transfer)`'s recipient field's
        // first byte is 0xFF. With Phase-1.4 strict-packet validation
        // the executor receives the SCALE-encoded packet, not the raw
        // intent bytes; the recipient's first byte sits at offset 25
        // for the Transfer variant
        // (1 Packet discriminant + 1 X3VmPacket variant + 1 from_domain
        //  + 1 to_domain + 4 asset_id + 16 amount + 1 compact-int
        //  length, for recipients < 64 bytes).
        if payload.get(25) == Some(&0xFF) {
            return Err(DispatchError::Other("X3 execution failed"));
        }

        // Two more markers in that same byte, for the failure paths a *well-behaved* adapter never
        // produces and which therefore have no other way to be reached:
        //
        //   0xFE -> a successful receipt whose ledger channel carries one entry more than
        //           `MAX_STATE_CHANGES`
        //   0xFD -> a successful receipt whose slot channel carries one write more than
        //           `MAX_STATE_CHANGES`
        //
        // Both are refused by the pallet *after* it has already withdrawn the fee and written the
        // canonical ledger for the other VMs. That is the point: the helper-level refusals are
        // covered elsewhere, but only a receipt built this way drives the *extrinsic* into a
        // refusal with real writes behind it, which is what proves those writes are rolled back.
        if let Some(&marker) = payload.get(25) {
            if marker == 0xFE || marker == 0xFD {
                /// One more than the pallet's `MAX_STATE_CHANGES` (1000), which is private to the
                /// pallet and deliberately not widened for a test.
                const ONE_PAST_THE_BOUND: usize = 1_001;

                let account: AccountId = ALICE;
                let asset_id: AssetId = 2;
                let balance: Balance = 333;

                let mut key_bytes = [0u8; 32];
                let asset_bytes = asset_id.encode();
                key_bytes[..asset_bytes.len()].copy_from_slice(&asset_bytes);

                let mut value_bytes = [0u8; 32];
                let balance_bytes = balance.encode();
                value_bytes[..balance_bytes.len()].copy_from_slice(&balance_bytes);

                let ledger_entry = crate::StateChange {
                    address: account.encode(),
                    key: H256::from(key_bytes),
                    value: H256::from(value_bytes),
                };

                let oversized_ledger = marker == 0xFE;
                return Ok(crate::ExecutionReceipt {
                    version: crate::EXECUTION_RECEIPT_VERSION,
                    success: true,
                    gas_used: 1000,
                    return_data: Vec::new(),
                    logs: Vec::new(),
                    state_changes: if oversized_ledger {
                        vec![ledger_entry.clone(); ONE_PAST_THE_BOUND]
                    } else {
                        vec![ledger_entry]
                    },
                    storage_writes: if oversized_ledger {
                        Vec::new()
                    } else {
                        (0..ONE_PAST_THE_BOUND)
                            .map(|i| crate::StorageWrite {
                                key: H256::from_low_u64_be(i as u64 + 1),
                                old_value: None,
                                new_value: Some([0x7Au8; 32]),
                            })
                            .collect()
                    },
                    protocol_version: 1,
                    migration_history: Vec::new(),
                    compatibility_flags: 0,
                    from: Vec::new(),
                    to: Vec::new(),
                    value: 0,
                });
            }
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
            storage_writes: Vec::new(),
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
    // Small on purpose: a test that fills the slot map past this gets the refusal by name, which is
    // the bound `submit_comit_v2` prices.
    type MaxX3StorageSlots = ConstU32<256>;
    type GovernanceOrigin = frame_system::EnsureRoot<AccountId>;
    type CrossVmPrepareTtl = ConstU64<10>;
    type MaxPreparedCrossVmOps = ConstU32<16>;
    type MaxPreparedOpsPerBlock = ConstU32<8>;
    type MaxReplayPruneItemsPerBlock = ConstU32<64>;
    type RequireCrossVmProof = ConstBool<false>;
    type PrivateSubmissionChannel = TestPrivateSubmissionChannel;
    type X3ChainId = frame_support::traits::ConstU64<650_000>;
    type CrossChainProofVerifier = TestProofVerifier;
    type BridgeEvmEscrow = BridgeEvmEscrowValue;
    type BridgeSvmEscrow = BridgeSvmEscrowValue;
    type EmergencyHaltController = TestEmergencyHaltController;
    type TradingHost = TestTradingHost;
}

pub struct ExtBuilder {
    balances: Vec<(AccountId, Balance)>,
    authorized_accounts: Vec<AccountId>,
    evm_escrow: H160,
    svm_escrow: [u8; 32],
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: vec![],
            authorized_accounts: vec![],
            evm_escrow: H160::repeat_byte(0xE3),
            svm_escrow: [0x53; 32],
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

    pub fn bridge_escrows(mut self, evm_escrow: H160, svm_escrow: [u8; 32]) -> Self {
        self.evm_escrow = evm_escrow;
        self.svm_escrow = svm_escrow;
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

        pallet_x3_kernel::GenesisConfig::<Test> {
            assets: Vec::new(),
            authorities: self.authorized_accounts.clone(),
            authorized_accounts: self.authorized_accounts.clone(),
            evm_escrow: self.evm_escrow,
            svm_escrow: self.svm_escrow,
        }
        .assimilate_storage(&mut storage)
        .expect("Failed to assimilate x3-kernel storage");

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
#[allow(dead_code)]
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
            storage_writes: Vec::new(),
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
            storage_writes: Vec::new(),
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
        let _evm_receipt = match evm_tx {
            Some(tx) => Some(self.execute_evm_tx(tx)?),
            None => None,
        };

        let _svm_receipt = match svm_tx {
            Some(tx) => Some(self.execute_svm_tx(tx)?),
            None => None,
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
        if *caller == ALICE || operation.is_empty() {
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

// ---------------------------------------------------------------------------------------------
// Trading host (test-only): venues and flash pools whose reserves are `CanonicalLedger` balances
// of their custody accounts, like `pallet_x3_dex` and `pallet_x3_flashloan` in the runtime.
// ---------------------------------------------------------------------------------------------

thread_local! {
    static TRADING_VENUES: core::cell::RefCell<
        std::collections::BTreeMap<String, (x3bc_core::state::VenueState, AccountId)>,
    > = Default::default();
    static TRADING_FLASH: core::cell::RefCell<
        std::collections::BTreeMap<u32, (u16, AccountId)>,
    > = Default::default();
    static TRADING_PRICES: core::cell::RefCell<std::collections::BTreeMap<u32, (u128, u64)>> =
        Default::default();
    static TRADING_HALTED: core::cell::Cell<bool> = const { core::cell::Cell::new(false) };
}

/// Venue reserves live in storage, not in the thread-local table, so a failed extrinsic's
/// storage rollback restores them exactly as it would a real dex pool's.
fn venue_reserves_key(name: &str) -> Vec<u8> {
    let mut key = b"x3-kernel-mock/venue-reserves/".to_vec();
    key.extend_from_slice(name.as_bytes());
    key
}

/// The test trading host. Reset per test with [`TestTradingHost::reset`]. Venue reserves are kept
/// in storage, so they roll back with the extrinsic.
pub struct TestTradingHost;

impl TestTradingHost {
    pub fn reset() {
        TRADING_VENUES.with(|v| v.borrow_mut().clear());
        TRADING_FLASH.with(|v| v.borrow_mut().clear());
        TRADING_PRICES.with(|v| v.borrow_mut().clear());
        TRADING_HALTED.with(|h| h.set(false));
    }

    pub fn set_halted(halted: bool) {
        TRADING_HALTED.with(|h| h.set(halted));
    }

    pub fn add_venue(venue: x3bc_core::state::VenueState, custody: AccountId) {
        frame_support::storage::unhashed::put(
            &venue_reserves_key(&venue.name),
            &(venue.reserve_a, venue.reserve_b),
        );
        TRADING_VENUES.with(|v| v.borrow_mut().insert(venue.name.clone(), (venue, custody)));
    }

    pub fn add_flash_pool(asset: u32, fee_bps: u16, custody: AccountId) {
        TRADING_FLASH.with(|v| v.borrow_mut().insert(asset, (fee_bps, custody)));
    }

    pub fn set_price(asset: u32, usd: u128, block: u64) {
        TRADING_PRICES.with(|v| v.borrow_mut().insert(asset, (usd, block)));
    }

    pub fn venue_state(name: &str) -> Option<x3bc_core::state::VenueState> {
        <Self as pallet_x3_kernel::KernelTradingHost<AccountId>>::venue(name).map(|(v, _)| v)
    }
}

impl pallet_x3_kernel::KernelTradingHost<AccountId> for TestTradingHost {
    fn venue(name: &str) -> Option<(x3bc_core::state::VenueState, AccountId)> {
        let (mut venue, custody) = TRADING_VENUES.with(|v| v.borrow().get(name).cloned())?;
        let (reserve_a, reserve_b): (u128, u128) =
            frame_support::storage::unhashed::get(&venue_reserves_key(name))?;
        venue.reserve_a = reserve_a;
        venue.reserve_b = reserve_b;
        Some((venue, custody))
    }

    fn set_venue_reserves(
        name: &str,
        reserve_a: u128,
        reserve_b: u128,
    ) -> frame_support::dispatch::DispatchResult {
        let (venue, custody) = TRADING_VENUES
            .with(|v| v.borrow().get(name).cloned())
            .ok_or(sp_runtime::DispatchError::Other("unknown venue"))?;
        let held_a = AtlasKernel::ledger_balance_of(venue.asset_a, &custody);
        let held_b = AtlasKernel::ledger_balance_of(venue.asset_b, &custody);
        if reserve_a > held_a || reserve_b > held_b {
            return Err(sp_runtime::DispatchError::Other("reserves exceed custody"));
        }
        frame_support::storage::unhashed::put(&venue_reserves_key(name), &(reserve_a, reserve_b));
        Ok(())
    }

    fn flash_pool(
        provider: &str,
        asset: u32,
    ) -> Option<(x3bc_core::state::FlashPoolState, AccountId)> {
        if provider != x3bc_core::trading::FLASH_PROVIDER {
            return None;
        }
        TRADING_FLASH.with(|v| {
            v.borrow().get(&asset).map(|(fee_bps, custody)| {
                (
                    x3bc_core::state::FlashPoolState {
                        provider: provider.into(),
                        version: x3bc_core::trading::CAPABILITY_VERSION,
                        asset,
                        liquidity: AtlasKernel::ledger_balance_of(asset, custody),
                        fee_bps: *fee_bps,
                    },
                    *custody,
                )
            })
        })
    }

    fn usd_price(asset: u32) -> Option<(u128, u64)> {
        TRADING_PRICES.with(|v| v.borrow().get(&asset).copied())
    }

    fn transfers_halted() -> bool {
        TRADING_HALTED.with(|h| h.get())
    }
}
