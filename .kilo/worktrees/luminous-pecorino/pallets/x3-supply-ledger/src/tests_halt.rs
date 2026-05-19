use crate as pallet_x3_supply_ledger;
use frame_support::{
    assert_ok, construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, EnsureOrigin, Hooks},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use x3_asset_kernel_types::{
    traits::{AssetRegistryInspect, SupplyLedgerWrite},
    AssetId, AssetStatus, Balance, DomainId, SupplyLedger, SupplyPolicy,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Ledger: pallet_x3_supply_ledger,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

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
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

pub struct RootOnly;
impl EnsureOrigin<RuntimeOrigin> for RootOnly {
    type Success = ();
    fn try_origin(o: RuntimeOrigin) -> Result<(), RuntimeOrigin> {
        match o.clone().into() {
            Ok(system::RawOrigin::Root) => Ok(()),
            _ => Err(o),
        }
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::root())
    }
}

pub struct AlwaysActiveRegistry;
impl AssetRegistryInspect for AlwaysActiveRegistry {
    fn exists(_asset_id: &AssetId) -> bool {
        true
    }

    fn status(_asset_id: &AssetId) -> Option<AssetStatus> {
        Some(AssetStatus::Active)
    }

    fn supply_policy(_asset_id: &AssetId) -> Option<SupplyPolicy> {
        Some(SupplyPolicy::NativeMintBurn)
    }

    fn canonical_decimals(_asset_id: &AssetId) -> Option<u8> {
        Some(12)
    }
}

impl pallet_x3_supply_ledger::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SupplyGovernance = RootOnly;
    type Registry = AlwaysActiveRegistry;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn asset(id: u8) -> AssetId {
    H256::repeat_byte(id)
}

fn valid_ledger(canonical: Balance) -> SupplyLedger {
    SupplyLedger {
        canonical_supply: canonical,
        native_supply: canonical,
        evm_supply: 0,
        svm_supply: 0,
        external_locked_supply: 0,
        pending_supply: 0,
    }
}

fn invalid_ledger() -> SupplyLedger {
    SupplyLedger {
        canonical_supply: 100,
        native_supply: 90,
        evm_supply: 20,
        svm_supply: 0,
        external_locked_supply: 0,
        pending_supply: 0,
    }
}

#[test]
fn invariant_violation_emits_event() {
    new_test_ext().execute_with(|| {
        pallet_x3_supply_ledger::Ledgers::<Test>::insert(asset(1), invalid_ledger());

        <Ledger as Hooks<u64>>::on_finalize(1);

        let emitted = System::events().iter().any(|e| {
            matches!(
                e.event,
                RuntimeEvent::Ledger(
                    pallet_x3_supply_ledger::Event::SupplyInvariantViolation { .. }
                )
            )
        });
        assert!(emitted);
    });
}

#[test]
fn invariant_violation_sets_halt_flag() {
    new_test_ext().execute_with(|| {
        assert!(!pallet_x3_supply_ledger::TransferHalted::<Test>::get());

        pallet_x3_supply_ledger::Ledgers::<Test>::insert(asset(1), invalid_ledger());
        <Ledger as Hooks<u64>>::on_finalize(1);

        assert!(pallet_x3_supply_ledger::TransferHalted::<Test>::get());
    });
}

#[test]
fn halted_router_rejects_new_transfers() {
    new_test_ext().execute_with(|| {
        let id = asset(1);
        pallet_x3_supply_ledger::Ledgers::<Test>::insert(id, valid_ledger(1_000));
        pallet_x3_supply_ledger::TransferHalted::<Test>::put(true);

        let debit_res =
            <Ledger as SupplyLedgerWrite>::debit_source_to_pending(&id, DomainId::X3Native, 100);
        assert_eq!(
            debit_res,
            Err(pallet_x3_supply_ledger::Error::<Test>::TransfersHalted.into())
        );

        let credit_res = <Ledger as SupplyLedgerWrite>::credit_destination_from_pending(
            &id,
            DomainId::X3Evm,
            100,
        );
        assert_eq!(
            credit_res,
            Err(pallet_x3_supply_ledger::Error::<Test>::TransfersHalted.into())
        );
    });
}

#[test]
fn refunds_allowed_while_halted() {
    new_test_ext().execute_with(|| {
        let id = asset(1);
        let mut l = valid_ledger(1_000);
        l.native_supply = 900;
        l.pending_supply = 100;
        pallet_x3_supply_ledger::Ledgers::<Test>::insert(id, l);
        pallet_x3_supply_ledger::TransferHalted::<Test>::put(true);

        assert_ok!(<Ledger as SupplyLedgerWrite>::refund_pending_to_source(
            &id,
            DomainId::X3Native,
            100,
        ));

        let out = pallet_x3_supply_ledger::Ledgers::<Test>::get(id).unwrap();
        assert_eq!(out.pending_supply, 0);
        assert_eq!(out.native_supply, 1_000);
    });
}

#[test]
fn governance_can_resume_after_recovery() {
    new_test_ext().execute_with(|| {
        let id = asset(1);
        pallet_x3_supply_ledger::Ledgers::<Test>::insert(id, valid_ledger(1_000));

        pallet_x3_supply_ledger::TransferHalted::<Test>::put(true);
        assert_ok!(Ledger::resume_transfers(RuntimeOrigin::root()));
        assert!(!pallet_x3_supply_ledger::TransferHalted::<Test>::get());

        assert_ok!(<Ledger as SupplyLedgerWrite>::debit_source_to_pending(
            &id,
            DomainId::X3Native,
            10,
        ));
    });
}
