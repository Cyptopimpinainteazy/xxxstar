// SPDX-License-Identifier: Apache-2.0
//
// Acceptance tests for the X3 OmniToken Factory.
//
// Wires the factory together with the real `pallet-x3-asset-registry`,
// `pallet-x3-supply-ledger`, and `pallet-x3-cross-vm-router`, then drives the
// 12 acceptance scenarios from the spec. The UAK invariant
//     canonical_supply == native + evm + svm + external_locked + pending
// MUST hold at every observation.

#![cfg(test)]

use crate as pallet_x3_token_factory;
use frame_support::{
    assert_noop, assert_ok, construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, EnsureOrigin},
    BoundedVec,
};
use frame_system::{self as system, EnsureSigned};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use x3_asset_kernel_types::{AccountBytes, AssetId, DomainId, TokenClass};

use crate::pallet::{
    Error, Event, MaxEnabledDomains, MaxNameLen, MaxSymbolLen, TokenFactoryConfig,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Registry: pallet_x3_asset_registry,
        Ledger: pallet_x3_supply_ledger,
        Router: pallet_x3_cross_vm_router,
        Factory: pallet_x3_token_factory,
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

pub struct RootOrAny;
impl EnsureOrigin<RuntimeOrigin> for RootOrAny {
    type Success = ();
    fn try_origin(o: RuntimeOrigin) -> Result<(), RuntimeOrigin> {
        match o.clone().into() {
            Ok(system::RawOrigin::Root) => Ok(()),
            Ok(system::RawOrigin::Signed(_)) => Ok(()),
            _ => Err(o),
        }
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::root())
    }
}

parameter_types! {
    pub const MaxAssets: u32 = 128;
}

impl pallet_x3_asset_registry::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RegistryOrigin = RootOrAny;
    type EmergencyPauseOrigin = RootOrAny;
    type MaxAssets = MaxAssets;
}

impl pallet_x3_supply_ledger::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type SupplyGovernance = RootOrAny;
    type Registry = Registry;
}

impl pallet_x3_cross_vm_router::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Registry = Registry;
    type Ledger = Ledger;
    type ExternalExecutorOrigin = RootOrAny;
    type VmAdapterOrigin = RootOrAny;
    type EconomicHalt = Ledger;
}

impl pallet_x3_token_factory::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CreateTokenOrigin = EnsureSigned<u64>;
    type Registry = Registry;
    type Ledger = Ledger;
    type EconomicHalt = Ledger;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

// ── Fixtures ──────────────────────────────────────────────────────────────

const CREATOR: u64 = 42;

fn bounded_symbol(bytes: &[u8]) -> BoundedVec<u8, MaxSymbolLen> {
    bytes.to_vec().try_into().expect("symbol fits")
}
fn bounded_name(bytes: &[u8]) -> BoundedVec<u8, MaxNameLen> {
    bytes.to_vec().try_into().expect("name fits")
}
fn bounded_domains(v: Vec<DomainId>) -> BoundedVec<DomainId, MaxEnabledDomains> {
    v.try_into().expect("domains fit")
}

fn all_three_domains() -> BoundedVec<DomainId, MaxEnabledDomains> {
    bounded_domains(vec![DomainId::X3Native, DomainId::X3Evm, DomainId::X3Svm])
}

fn fixed_config(
    symbol: &[u8],
    initial_supply: u128,
    domains: BoundedVec<DomainId, MaxEnabledDomains>,
) -> TokenFactoryConfig {
    TokenFactoryConfig {
        symbol: bounded_symbol(symbol),
        name: bounded_name(symbol),
        canonical_decimals: 12,
        initial_supply,
        max_supply: None,
        class: TokenClass::FixedSupply,
        enabled_domains: domains,
    }
}

fn capped_config(
    symbol: &[u8],
    initial_supply: u128,
    max_supply: u128,
    domains: BoundedVec<DomainId, MaxEnabledDomains>,
) -> TokenFactoryConfig {
    TokenFactoryConfig {
        symbol: bounded_symbol(symbol),
        name: bounded_name(symbol),
        canonical_decimals: 12,
        initial_supply,
        max_supply: Some(max_supply),
        class: TokenClass::CappedMintable,
        enabled_domains: domains,
    }
}

fn burnable_config(
    symbol: &[u8],
    initial_supply: u128,
    domains: BoundedVec<DomainId, MaxEnabledDomains>,
) -> TokenFactoryConfig {
    TokenFactoryConfig {
        symbol: bounded_symbol(symbol),
        name: bounded_name(symbol),
        canonical_decimals: 12,
        initial_supply,
        max_supply: None,
        class: TokenClass::Burnable,
        enabled_domains: domains,
    }
}

fn launch(config: TokenFactoryConfig) -> AssetId {
    assert_ok!(Factory::create_token(
        RuntimeOrigin::signed(CREATOR),
        config
    ));
    let events = System::events();
    let mut asset_id = None;
    for rec in events.iter().rev() {
        if let RuntimeEvent::Factory(Event::TokenCreated { asset_id: aid, .. }) = &rec.event {
            asset_id = Some(*aid);
            break;
        }
    }
    asset_id.expect("TokenCreated event emitted")
}

fn addr_for(domain: DomainId) -> AccountBytes {
    match domain {
        DomainId::X3Native => {
            let mut native = [0u8; 32];
            native[0] = CREATOR as u8;
            AccountBytes::X3Native(native)
        }
        DomainId::X3Evm => AccountBytes::Evm([2u8; 20]),
        DomainId::X3Svm => AccountBytes::Svm([3u8; 32]),
        _ => unreachable!("only internal domains in factory tests"),
    }
}

/// Submit + settle an internal xvm transfer through the router.
fn do_xvm(asset_id: AssetId, src: DomainId, dst: DomainId, amount: u128) {
    let sender = addr_for(src);
    let recipient = addr_for(dst);
    let now = System::block_number();
    let expires_at = now + 50;

    let nonce = Router::next_nonce(src, sender.clone());

    assert_ok!(Router::do_initiate_transfer(
        asset_id,
        src,
        dst,
        sender.clone(),
        recipient.clone(),
        amount,
        expires_at,
    ));

    let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
        version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
        asset_id,
        source_domain: src,
        destination_domain: dst,
        sender,
        recipient,
        amount,
        nonce,
        created_at: now,
        expires_at,
    };
    let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);
    assert_ok!(Router::complete_xvm_transfer(
        RuntimeOrigin::signed(CREATOR),
        message_id
    ));
}

fn submit_xvm_expect_err(asset_id: AssetId, src: DomainId, dst: DomainId, amount: u128) {
    let sender = addr_for(src);
    let recipient = addr_for(dst);
    let now = System::block_number();
    let expires_at = now + 50;

    assert!(
        Router::do_initiate_transfer(
            asset_id,
            src,
            dst,
            sender,
            recipient,
            amount,
            expires_at,
        )
        .is_err(),
        "expected xvm_transfer to fail on disabled domain / route",
    );
}

fn leg_balance(l: &x3_asset_kernel_types::SupplyLedger, d: DomainId) -> u128 {
    match d {
        DomainId::X3Native => l.native_supply,
        DomainId::X3Evm => l.evm_supply,
        DomainId::X3Svm => l.svm_supply,
        _ => 0,
    }
}

// ── Acceptance tests ──────────────────────────────────────────────────────

/// 1. Create a fixed-supply 1M token enabling Native/EVM/SVM; TokenCreated fires.
#[test]
fn test_01_create_fixed_supply_token_emits_event() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"OMNI", 1_000_000, all_three_domains()));

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 1_000_000);
        assert_eq!(l.native_supply, 1_000_000);
        assert_eq!(l.evm_supply, 0);
        assert_eq!(l.svm_supply, 0);
        assert_eq!(l.pending_supply, 0);
        l.check_invariant().unwrap();

        assert!(Registry::assets(asset_id).is_some());
    });
}

/// 2. Native → EVM transfer preserves the invariant.
#[test]
fn test_02_native_to_evm_preserves_invariant() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"OMNI", 1_000_000, all_three_domains()));
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 250_000);

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.native_supply, 750_000);
        assert_eq!(l.evm_supply, 250_000);
        assert_eq!(l.svm_supply, 0);
        assert_eq!(l.pending_supply, 0);
        assert_eq!(l.canonical_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 3. EVM → SVM transfer preserves the invariant.
#[test]
fn test_03_evm_to_svm_preserves_invariant() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"OMNI", 1_000_000, all_three_domains()));
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 400_000);
        do_xvm(asset_id, DomainId::X3Evm, DomainId::X3Svm, 150_000);

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.native_supply, 600_000);
        assert_eq!(l.evm_supply, 250_000);
        assert_eq!(l.svm_supply, 150_000);
        assert_eq!(l.pending_supply, 0);
        assert_eq!(l.canonical_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 4. SVM → Native round-trip preserves the invariant.
#[test]
fn test_04_roundtrip_native_evm_svm_native() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"OMNI", 1_000_000, all_three_domains()));
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 300_000);
        do_xvm(asset_id, DomainId::X3Evm, DomainId::X3Svm, 200_000);
        do_xvm(asset_id, DomainId::X3Svm, DomainId::X3Native, 200_000);

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.native_supply, 900_000);
        assert_eq!(l.evm_supply, 100_000);
        assert_eq!(l.svm_supply, 0);
        assert_eq!(l.pending_supply, 0);
        assert_eq!(l.canonical_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 5. FixedSupply tokens reject post-launch mint.
#[test]
fn test_05_fixed_supply_cannot_mint() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"FIX", 1_000_000, all_three_domains()));
        assert_noop!(
            Factory::mint(
                RuntimeOrigin::signed(CREATOR),
                asset_id,
                DomainId::X3Native,
                1,
            ),
            Error::<Test>::FixedSupplyCannotMint
        );
    });
}

/// 6. CappedMintable can mint additional supply up to the cap.
#[test]
fn test_06_capped_mintable_up_to_ceiling() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(capped_config(
            b"CAP",
            500_000,
            1_000_000,
            all_three_domains(),
        ));
        assert_ok!(Factory::mint(
            RuntimeOrigin::signed(CREATOR),
            asset_id,
            DomainId::X3Native,
            500_000,
        ));
        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 1_000_000);
        assert_eq!(l.native_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 7. CappedMintable cannot cross the cap.
#[test]
fn test_07_capped_mintable_over_ceiling_fails() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(capped_config(
            b"CAP",
            500_000,
            1_000_000,
            all_three_domains(),
        ));
        assert_ok!(Factory::mint(
            RuntimeOrigin::signed(CREATOR),
            asset_id,
            DomainId::X3Native,
            500_000,
        ));
        assert_noop!(
            Factory::mint(
                RuntimeOrigin::signed(CREATOR),
                asset_id,
                DomainId::X3Native,
                1,
            ),
            Error::<Test>::CappedMintWouldExceedMax
        );
        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 8. Burnable reduces canonical_supply.
#[test]
fn test_08_burnable_reduces_canonical_supply() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(burnable_config(b"BRN", 1_000_000, all_three_domains()));
        assert_ok!(Factory::burn(
            RuntimeOrigin::signed(CREATOR),
            asset_id,
            DomainId::X3Native,
            250_000,
        ));
        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 750_000);
        assert_eq!(l.native_supply, 750_000);
        assert_eq!(l.pending_supply, 0);
        l.check_invariant().unwrap();
    });
}

/// 9. Transfer to a domain that was not enabled at launch fails.
#[test]
fn test_09_disabled_domain_transfer_fails() {
    new_test_ext().execute_with(|| {
        let domains = bounded_domains(vec![DomainId::X3Native, DomainId::X3Evm]);
        let asset_id = launch(fixed_config(b"NOSVM", 1_000_000, domains));
        submit_xvm_expect_err(asset_id, DomainId::X3Native, DomainId::X3Svm, 100);

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 1_000_000);
        assert_eq!(l.svm_supply, 0);
        l.check_invariant().unwrap();
    });
}

/// 10. Pausing the asset closes every route.
#[test]
fn test_10_disabled_route_transfer_fails() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"OMNI", 1_000_000, all_three_domains()));
        assert_ok!(Registry::pause_asset(RuntimeOrigin::root(), asset_id));
        submit_xvm_expect_err(asset_id, DomainId::X3Native, DomainId::X3Evm, 100);

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 1_000_000);
        l.check_invariant().unwrap();
    });
}

/// 11. TokenCreated carries asset_id, creator, symbol, supply, class, domains.
#[test]
fn test_11_token_created_event_fields() {
    new_test_ext().execute_with(|| {
        let asset_id = launch(fixed_config(b"EVT", 777_000, all_three_domains()));

        let mut matched = false;
        for rec in System::events() {
            if let RuntimeEvent::Factory(Event::TokenCreated {
                asset_id: aid,
                creator,
                symbol,
                initial_supply,
                class,
                enabled_domains,
            }) = rec.event
            {
                assert_eq!(aid, asset_id);
                assert_eq!(creator, CREATOR);
                assert_eq!(symbol, b"EVT".to_vec());
                assert_eq!(initial_supply, 777_000);
                assert_eq!(class, TokenClass::FixedSupply);
                assert_eq!(
                    enabled_domains,
                    vec![DomainId::X3Native, DomainId::X3Evm, DomainId::X3Svm]
                );
                matched = true;
                break;
            }
        }
        assert!(matched, "TokenCreated event with expected fields must fire");
    });
}

// ── Fuzz (test 12) ────────────────────────────────────────────────────────

fn rng_next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn random_transfer_step(asset_id: AssetId, rng: &mut u64) -> bool {
    let domains = [DomainId::X3Native, DomainId::X3Evm, DomainId::X3Svm];
    let src = domains[(rng_next(rng) as usize) % 3];
    let dst = domains[(rng_next(rng) as usize) % 3];
    if src == dst {
        return false;
    }
    let l = Ledger::ledgers(asset_id).unwrap();
    let avail = leg_balance(&l, src);
    if avail == 0 {
        return false;
    }
    let amount = 1 + (rng_next(rng) as u128 % avail);
    do_xvm(asset_id, src, dst, amount);
    true
}

/// 12. Fuzz: many launches × many random transfers — invariant never breaks.
#[test]
fn test_12_fuzz_launches_and_transfers_preserve_invariant() {
    for seed in 0u64..8 {
        new_test_ext().execute_with(|| {
            let initial = 1_000_000u128 + seed as u128 * 17;
            let asset_id = launch(fixed_config(b"FUZZ", initial, all_three_domains()));
            let mut rng = seed.wrapping_mul(0xABCDEF0123456789).wrapping_add(1);

            for _ in 0..24 {
                let _ = random_transfer_step(asset_id, &mut rng);
                let l = Ledger::ledgers(asset_id).unwrap();
                l.check_invariant()
                    .expect("invariant holds across factory launch + transfers");
                assert_eq!(
                    l.canonical_supply, initial,
                    "canonical supply immutable under transfers (seed={})",
                    seed
                );
                assert_eq!(
                    l.pending_supply, 0,
                    "pending must drain after synchronous completion"
                );
                assert_eq!(
                    l.represented().expect("represented ok"),
                    l.canonical_supply,
                    "represented == canonical (seed={})",
                    seed,
                );
            }
        });
    }
}
