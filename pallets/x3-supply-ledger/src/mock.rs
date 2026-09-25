//! Test mock runtime for `pallet-x3-supply-ledger`.
//!
//! The pallet's other test modules build `SupplyLedger` values by hand and call
//! `check_invariant()` on them. That proves the arithmetic of a struct, not the behaviour of the
//! ledger: the three transitions every cross-domain operation goes through —
//! `debit_source_to_pending`, `credit_destination_from_pending`, `refund_pending_to_source` — live
//! behind `SupplyLedgerWrite`, and the S0-1 test file says in a note that they need "mock.rs with
//! runtime configuration" to run. This is that runtime, so `tests_conservation.rs` can drive the
//! real paths instead of re-deriving their arithmetic.

use crate as pallet_x3_supply_ledger;
use frame_support::derive_impl;
use sp_core::H256;
use sp_runtime::BuildStorage;
use std::cell::RefCell;
use std::collections::BTreeSet;
use x3_asset_kernel_types::traits::AssetRegistryInspect;
use x3_asset_kernel_types::{AssetId, AssetStatus, Balance, SupplyLedger, SupplyPolicy};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        SupplyLedgerPallet: pallet_x3_supply_ledger,
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
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = frame_support::traits::ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

thread_local! {
    /// Assets the mock reports as paused. A real deployment reads this from the asset registry.
    static FROZEN: RefCell<BTreeSet<AssetId>> = const { RefCell::new(BTreeSet::new()) };
}

/// The mock registry: an asset exists exactly when the ledger holds one for it, and it is active
/// unless a test paused it. That is enough to exercise both halves of the gate the transitions
/// apply — `is_active` for debit/credit, `exists` for refund.
pub struct TestRegistry;

impl AssetRegistryInspect for TestRegistry {
    fn exists(asset_id: &AssetId) -> bool {
        pallet_x3_supply_ledger::Ledgers::<Test>::contains_key(asset_id)
    }

    fn status(asset_id: &AssetId) -> Option<AssetStatus> {
        if !Self::exists(asset_id) {
            return None;
        }
        let paused = FROZEN.with(|f| f.borrow().contains(asset_id));
        Some(if paused {
            AssetStatus::Paused
        } else {
            AssetStatus::Active
        })
    }

    fn supply_policy(_asset_id: &AssetId) -> Option<SupplyPolicy> {
        Some(SupplyPolicy::NativeMintBurn)
    }

    fn canonical_decimals(_asset_id: &AssetId) -> Option<u8> {
        Some(18)
    }
}

impl pallet_x3_supply_ledger::Config for Test {
    // Test-only governance: any signed account may mint, so the account-nonce idempotency path in
    // `mint_canonical` (which only records nonces for signed origins) is exercised. Production
    // wires a governance origin here.
    type SupplyGovernance = frame_system::EnsureSigned<u64>;
    type Registry = TestRegistry;
}

/// A deterministic asset id.
pub fn asset(seed: u8) -> AssetId {
    H256::repeat_byte(seed)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    FROZEN.with(|f| f.borrow_mut().clear());
    frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("the mock genesis builds")
        .into()
}

/// Register an asset with a canonical ceiling and an initial native-domain balance.
pub fn register_asset(asset_id: AssetId, canonical: Balance, native: Balance) -> SupplyLedger {
    let ledger = SupplyLedger {
        canonical_supply: canonical,
        native_supply: native,
        evm_supply: 0,
        svm_supply: 0,
        external_locked_supply: 0,
        pending_supply: 0,
    };
    // `SupplyLedger` is `Copy`, so this stores a value and still hands one back.
    pallet_x3_supply_ledger::Ledgers::<Test>::insert(asset_id, ledger);
    ledger
}

pub fn pause(asset_id: AssetId) {
    FROZEN.with(|f| f.borrow_mut().insert(asset_id));
}

pub fn unpause(asset_id: AssetId) {
    FROZEN.with(|f| f.borrow_mut().remove(&asset_id));
}

pub fn ledger_of(asset_id: AssetId) -> SupplyLedger {
    pallet_x3_supply_ledger::Ledgers::<Test>::get(asset_id).expect("the asset is registered")
}

/// The ledger's represented total, by the ledger's own definition.
pub fn represented_of(asset_id: AssetId) -> Balance {
    ledger_of(asset_id)
        .represented()
        .expect("the mock never builds a ledger whose components overflow")
}
