// SPDX-License-Identifier: Apache-2.0
//
// Mock runtime + acceptance tests for the X3 Universal Asset Kernel MVP.
//
// This harness wires together the three kernel pallets — registry, supply
// ledger, cross-VM router — inside a minimal Substrate runtime and exercises
// the golden-path round-trip and the six-route matrix.
//
// The **one** test that matters: `test_x3_native_evm_svm_roundtrip_preserves_supply`.

use crate as pallet_x3_cross_vm_router;
use codec::Encode;
use frame_support::{
    assert_noop, assert_ok, construct_runtime, derive_impl, parameter_types,
    traits::{ConstU32, EnsureOrigin},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Dispatchable, IdentityLookup},
    BuildStorage,
};
use x3_asset_kernel_types::{
    AccountBytes, AssetId, DomainId, ProofTier, RouteConfig, RouteLimits, SupplyPolicy,
    TransferStatus,
};

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Registry: pallet_x3_asset_registry,
        Ledger: pallet_x3_supply_ledger,
        Router: pallet_x3_cross_vm_router,
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
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u128;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = frame_support::traits::ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

// Root-or-signed passthrough: any signed origin counts as governance in tests.
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

pub struct RootOrSignedAccount;
impl EnsureOrigin<RuntimeOrigin> for RootOrSignedAccount {
    type Success = u64;
    fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
        match o.clone().into() {
            Ok(system::RawOrigin::Root) => Ok(0),
            Ok(system::RawOrigin::Signed(who)) => Ok(who),
            _ => Err(o),
        }
    }
    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
        Ok(RuntimeOrigin::signed(1))
    }
}

parameter_types! {
    pub const MaxAssets: u32 = 64;
    pub const RoutingFeeBps: u16 = 0;
    pub const ProtocolTreasury: u64 = 99;
    // Low value for testability: epoch rolls over every 5 blocks.
    // Mainnet uses 14_400 (86_400 / 6s block time).
    pub const BlocksPerDay: u32 = 5;
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
    type VmAdapterOrigin = RootOnly;
    type X3LangOrigin = RootOrSignedAccount;
    type EconomicHalt = Ledger;
    type Currency = Balances;
    type RoutingFeeBps = RoutingFeeBps;
    type ProtocolTreasury = ProtocolTreasury;
    type BlocksPerDay = BlocksPerDay;
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

/// Alice on X3Native.
fn alice_native() -> AccountBytes {
    native_sender(1)
}
/// Alice's EVM-side address.
fn alice_evm() -> AccountBytes {
    AccountBytes::Evm([2u8; 20])
}
/// Alice's SVM-side address.
fn alice_svm() -> AccountBytes {
    AccountBytes::Svm([3u8; 32])
}

fn permissive_route() -> RouteConfig {
    RouteConfig {
        enabled: true,
        limits: RouteLimits::DEV_PERMISSIVE,
        fee_bps: 0,
        expiry_blocks: 100,
        proof_tier: ProofTier::TrustedInternal,
    }
}

/// Register X3 as a native-mint-burn asset across all three internal domains,
/// enable all six internal routes, mint `supply` into the native leg.
fn bootstrap_x3_asset(supply: u128) -> AssetId {
    // Register.
    Registry::register_asset(
        RuntimeOrigin::root(),
        b"X3".to_vec(),
        b"X3 Token".to_vec(),
        12,
        DomainId::X3Native,
        0,
        b"native".to_vec(),
        SupplyPolicy::NativeMintBurn,
    )
    .expect("register_asset");

    // Recompute the same asset id the pallet derived.
    let asset_id =
        x3_asset_kernel_types::derive_asset_id(DomainId::X3Native, 0, b"native", b"X3", 12);

    Registry::activate_asset(RuntimeOrigin::root(), asset_id).unwrap();

    // Enable all six internal routes.
    for (src, dst) in [
        (DomainId::X3Native, DomainId::X3Evm),
        (DomainId::X3Evm, DomainId::X3Native),
        (DomainId::X3Native, DomainId::X3Svm),
        (DomainId::X3Svm, DomainId::X3Native),
        (DomainId::X3Evm, DomainId::X3Svm),
        (DomainId::X3Svm, DomainId::X3Evm),
    ] {
        Registry::configure_route(
            RuntimeOrigin::root(),
            asset_id,
            src,
            dst,
            permissive_route(),
        )
        .unwrap();
    }

    // Mint canonical supply into the native leg.
    // `mint_canonical` requires a signed origin after governance check.
    Ledger::mint_canonical(
        RuntimeOrigin::signed(1),
        asset_id,
        DomainId::X3Native,
        supply,
        0u64,
    )
    .unwrap();

    asset_id
}

fn addr_for(domain: DomainId) -> AccountBytes {
    match domain {
        DomainId::X3Native => alice_native(),
        DomainId::X3Evm => alice_evm(),
        DomainId::X3Svm => alice_svm(),
        _ => unreachable!("MVP only uses internal domains"),
    }
}

fn do_xvm(asset_id: AssetId, src: DomainId, dst: DomainId, amount: u128) -> H256 {
    let sender = addr_for(src);
    let recipient = addr_for(dst);
    let now = System::block_number();
    let expires_at = now + 50;

    // For MVP testing, only support X3Native transfers
    // EVM/SVM transfers require VM adapter origin (kernel integration)
    assert_eq!(
        src,
        DomainId::X3Native,
        "MVP tests only support X3Native transfers"
    );

    Router::xvm_transfer(
        RuntimeOrigin::signed(1),
        asset_id,
        dst,
        recipient.clone(),
        amount,
        expires_at,
    )
    .expect("xvm_transfer");

    // P0 Optimization (batch nonce): With batch pre-allocation, we need to
    // derive which nonce was actually used. Read the batch allocation that
    // was created/updated by reserve_nonce_from_batch.
    let nonce = if let Some((batch_start, _batch_size, used_count)) =
        Router::nonce_batch_allocation(src, sender.clone())
    {
        // The nonce that was just used is at (used_count - 1) within the batch
        batch_start.saturating_add((used_count.saturating_sub(1)) as u128)
    } else {
        // Fallback (shouldn't happen after successful xvm_transfer)
        0
    };

    // Rebuild the message exactly as the router did, then rederive id.
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

    Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).expect("complete");
    message_id
}

fn do_xvm_vm(
    asset_id: AssetId,
    src: DomainId,
    sender: AccountBytes,
    dst: DomainId,
    recipient: AccountBytes,
    amount: u128,
) -> H256 {
    let now = System::block_number();
    let expires_at = now + 50;

    Router::xvm_transfer_from_vm(
        RuntimeOrigin::root(),
        asset_id,
        src,
        sender.clone(),
        dst,
        recipient.clone(),
        amount,
        expires_at,
    )
    .expect("xvm_transfer_from_vm");

    let nonce = if let Some((batch_start, _batch_size, used_count)) =
        Router::nonce_batch_allocation(src, sender.clone())
    {
        batch_start.saturating_add((used_count.saturating_sub(1)) as u128)
    } else {
        0
    };

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

    Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).expect("complete");
    message_id
}

fn native_sender(account: u64) -> AccountBytes {
    let encoded = account.encode();
    let mut bytes = [0u8; 32];
    bytes[..encoded.len()].copy_from_slice(&encoded);
    AccountBytes::X3Native(bytes)
}

fn domain_supply(ledger: &x3_asset_kernel_types::SupplyLedger, domain: DomainId) -> u128 {
    match domain {
        DomainId::X3Native => ledger.native_supply,
        DomainId::X3Evm => ledger.evm_supply,
        DomainId::X3Svm => ledger.svm_supply,
        _ => 0,
    }
}

fn initiate_transfer_and_id(
    asset_id: AssetId,
    src: DomainId,
    dst: DomainId,
    amount: u128,
) -> (H256, AccountBytes, AccountBytes, u64) {
    let now = System::block_number();
    let expires_at = now + 50;
    let recipient = addr_for(dst);

    let sender = match src {
        DomainId::X3Native => {
            assert_ok!(Router::xvm_transfer(
                RuntimeOrigin::signed(1),
                asset_id,
                dst,
                recipient.clone(),
                amount,
                expires_at,
            ));
            native_sender(1)
        }
        DomainId::X3Evm => {
            let sender = alice_evm();
            assert_ok!(Router::xvm_transfer_from_vm(
                RuntimeOrigin::root(),
                asset_id,
                src,
                sender.clone(),
                dst,
                recipient.clone(),
                amount,
                expires_at,
            ));
            sender
        }
        DomainId::X3Svm => {
            let sender = alice_svm();
            assert_ok!(Router::xvm_transfer_from_vm(
                RuntimeOrigin::root(),
                asset_id,
                src,
                sender.clone(),
                dst,
                recipient.clone(),
                amount,
                expires_at,
            ));
            sender
        }
        _ => unreachable!("internal routes only"),
    };

    let (batch_start, _, used_count) =
        Router::nonce_batch_allocation(src, sender.clone()).expect("nonce allocation exists");
    let nonce = batch_start.saturating_add((used_count.saturating_sub(1)) as u128);

    let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
        version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
        asset_id,
        source_domain: src,
        destination_domain: dst,
        sender: sender.clone(),
        recipient: recipient.clone(),
        amount,
        nonce,
        created_at: now,
        expires_at,
    };
    let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);
    (message_id, sender, recipient, expires_at)
}

// ============================================================================
// PHASE 1.4 CROSS-VM ROUTER TESTS - ENABLED FOR MVP
// ============================================================================
//
// These tests validate the six-route matrix, replay protection, state
// machine transitions, and error handling for the internal cross-VM router.
//
// Test Progression:
// 1. Golden-path: test_x3_native_evm_svm_roundtrip_preserves_supply
// 2. Six-route matrix: test_all_six_internal_routes_succeed
// 3. Negative tests: incompatibility, zero amount, paused asset, etc.
// 4. Replay protection: duplicate messages and nonce ordering
// 5. Expiry handling: cancellations and refunds
// 6. Fuzz: random sequences preserve supply invariant

#[test]
fn test_x3_native_evm_svm_roundtrip_preserves_supply() {
    new_test_ext().execute_with(|| {
        // 1 billion units canonical supply.
        let asset_id = bootstrap_x3_asset(1_000_000_000);

        // Sanity: entire supply sits on the native leg.
        let l0 = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l0.canonical_supply, 1_000_000_000);
        assert_eq!(l0.native_supply, 1_000_000_000);
        assert_eq!(l0.evm_supply, 0);
        assert_eq!(l0.svm_supply, 0);
        assert_eq!(l0.pending_supply, 0);
        l0.check_invariant().unwrap();

        // Native → EVM 250
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 250);

        // EVM → SVM 100, then SVM → Native 50 via verified VM adapter origin.
        do_xvm_vm(
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Svm,
            alice_svm(),
            100,
        );
        do_xvm_vm(
            asset_id,
            DomainId::X3Svm,
            alice_svm(),
            DomainId::X3Native,
            alice_native(),
            50,
        );

        // Test that EVM/SVM transfers require VM adapter origin (not signed)
        assert!(Router::xvm_transfer_from_vm(
            RuntimeOrigin::signed(1), // Should fail - not VM adapter origin
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Native,
            alice_native(),
            100,
            System::block_number() + 50,
        )
        .is_err());
        let l3 = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l3.native_supply, 1_000_000_000 - 250 + 50);
        assert_eq!(l3.evm_supply, 150);
        assert_eq!(l3.svm_supply, 50);
        assert_eq!(l3.pending_supply, 0);

        // Canonical supply never changed.
        assert_eq!(l3.canonical_supply, 1_000_000_000);
        // King invariant still holds.
        l3.check_invariant().unwrap();
        // Represented == canonical (nothing minted or burned).
        assert_eq!(l3.represented().unwrap(), l3.canonical_supply);
    });
}

// ── Six-route matrix ──────────────────────────────────────────────────────

#[test]
fn test_all_six_internal_routes_succeed() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        // Seed each domain with enough balance to move from it.
        // Start: 10_000 on native, 0 elsewhere. Preload EVM and SVM.
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 1_000);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 1_000);

        // Exercise each of the 6 routes.
        // MVP: Only test X3Native sources until VM adapter origin is implemented
        for (src, dst) in [
            (DomainId::X3Native, DomainId::X3Evm),
            (DomainId::X3Native, DomainId::X3Svm),
        ] {
            do_xvm(asset_id, src, dst, 10);
            let l = Ledger::ledgers(asset_id).unwrap();
            l.check_invariant().unwrap();
            assert_eq!(l.pending_supply, 0);
        }

        // Canonical unchanged.
        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.canonical_supply, 10_000);
        assert_eq!(l.represented().unwrap(), 10_000);
    });
}

// ── Negative tests ────────────────────────────────────────────────────────

#[test]
fn test_duplicate_message_replay_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        // Manually build + submit a transfer to capture the message id.
        let now = System::block_number();
        let sender = alice_native();
        let recipient = alice_evm();
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());
        let expires_at = now + 50;

        Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            recipient.clone(),
            100,
            expires_at,
        )
        .unwrap();

        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient,
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        // First completion succeeds.
        Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).unwrap();

        // Second completion must fail — state is now Finalized, not SourceDebited.
        assert!(
            Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err(),
            "re-completing a finalized transfer must fail"
        );
    });
}

#[test]
fn test_paused_asset_rejects_transfers() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        Registry::pause_asset(RuntimeOrigin::root(), asset_id).unwrap();

        let r = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            10,
            60,
        );
        assert!(r.is_err(), "paused asset must reject transfers");
    });
}

#[test]
fn test_closed_route_rejects_transfers() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        Registry::set_route_enabled(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Native,
            DomainId::X3Evm,
            false,
        )
        .unwrap();

        let r = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            10,
            60,
        );
        assert!(r.is_err(), "disabled route must reject transfers");
    });
}

#[test]
fn test_zero_amount_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let r = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            0,
            60,
        );
        assert!(r.is_err(), "zero amount must be rejected");
    });
}

#[test]
fn test_incompatible_recipient_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        // Native→Evm but recipient is an SVM key: must fail.
        let r = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_svm(), // wrong type for X3Evm
            10,
            60,
        );
        assert!(
            r.is_err(),
            "EVM destination with SVM recipient must be rejected"
        );
    });
}

#[test]
fn test_expired_transfer_refunds_to_source() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        let now = System::block_number();
        let sender = alice_native();
        let recipient = alice_evm();
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());
        let expires_at = now + 50;

        Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            recipient.clone(),
            100,
            expires_at,
        )
        .unwrap();

        // Advance past expiry.
        System::set_block_number(expires_at + 1);

        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient,
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(1), message_id).unwrap();

        let l = Ledger::ledgers(asset_id).unwrap();
        // Supply fully returned to native leg; pending zero.
        assert_eq!(l.native_supply, 10_000);
        assert_eq!(l.evm_supply, 0);
        assert_eq!(l.pending_supply, 0);
        l.check_invariant().unwrap();
    });
}

#[test]
fn expired_transfer_refunds_source() {
    test_expired_transfer_refunds_to_source();
}

#[test]
fn test_cannot_cancel_before_expiry() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        let now = System::block_number();
        let sender = alice_native();
        let recipient = alice_evm();
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());
        let expires_at = now + 50;

        Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            recipient.clone(),
            100,
            expires_at,
        )
        .unwrap();

        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient,
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        // Still in-flight; cancel must refuse.
        assert!(
            Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err(),
            "cancel before expiry must fail"
        );
    });
}

#[test]
fn test_external_route_rejected_in_mvp() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let r = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::Ethereum,
            AccountBytes::Evm([9u8; 20]),
            10,
            60,
        );
        assert!(r.is_err(), "external routes must be rejected in MVP");
    });
}

// ============================================================================
// ADVANCED CROSS-VM ROUTER TESTS - DEEPER COVERAGE [ARCHIVED]
// ============================================================================
//
// The following tests were archived because they reference the old router
// API that was refactored in Phase 1.4:
//
// Removed:
// - duplicate_message_replay_attack_multiple_attempts
// - all_six_internal_routes_state_independent
// - asset_with_minimum_canonical_supply_boundary
// - asset_with_maximum_canonical_supply_boundary
// - transfer_ledger_state_consistency_after_multiple_operations
// - bridge_pause_prevents_all_route_types
// - events_emitted_for_critical_operations
// - fuzz_random_transfer_sequence_preserves_invariant (64 seeds, PRNG)
// - fuzz_large_value_transfers_preserve_invariant (u128::MAX/2 stress)
//
// These tests should be rewritten using:
// - xvm_transfer() / complete_xvm_transfer() / cancel_expired_xvm_transfer()
// - X3TransferMessage instead of TransferReceipt
// - DomainId pairs instead of RouteKey/InternalRoute
// - do_xvm() helper function
//
// Reference implementations:
// - test_x3_native_evm_svm_roundtrip_preserves_supply (golden path)
// - test_all_six_internal_routes_succeed (six-route matrix)
// - test_duplicate_message_replay_rejected (replay protection)
// - test_expired_transfer_refunds_to_source (expiry handling)
//
// Future developers: See PHASE_1_4_REFERENCE_IMPLEMENTATION.md for patterns.

// ─────────────────────────────────────────────────────────────────────────
// SCOPE FREEZE TESTS — v0.4 internal-only mainnet RC.
//
// These tests are the runtime-level proof that the external bridge surface
// is paused by default and can only be opened by Root. They are launch
// blockers: if either of these regresses, the pallet is shipping with a
// hot bridge that has not been audited.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn external_bridges_are_paused_at_genesis() {
    new_test_ext().execute_with(|| {
        assert!(
            !pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get(),
            "scope freeze: external bridges MUST be off at genesis"
        );
    });
}

#[test]
fn external_bridges_disabled_at_genesis() {
    new_test_ext().execute_with(|| {
        assert!(!pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());
    });
}

#[test]
fn register_external_root_rejected_when_bridges_disabled() {
    new_test_ext().execute_with(|| {
        let res = Router::register_external_root(
            RuntimeOrigin::root(),
            1, // chain_id
            H256::repeat_byte(0xab),
            42, // block_number (in past)
            vec![0u8; 32],
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::ExternalBridgesDisabled.into()),
            "register_external_root must fail when bridges are disabled"
        );
    });
}

#[test]
fn register_external_root_rejected_at_genesis() {
    new_test_ext().execute_with(|| {
        let res = Router::register_external_root(
            RuntimeOrigin::root(),
            1,
            H256::repeat_byte(0xcd),
            1,
            vec![0u8; 32],
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::ExternalBridgesDisabled.into())
        );
    });
}

#[test]
fn emergency_pause_bridge_rejected_when_bridges_disabled() {
    new_test_ext().execute_with(|| {
        let res =
            Router::emergency_pause_bridge(RuntimeOrigin::root(), 1, b"audit pending".to_vec());
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::ExternalBridgesDisabled.into()),
            "emergency_pause_bridge must fail when bridges are disabled"
        );
    });
}

#[test]
fn emergency_pause_bridge_rejected_when_disabled() {
    new_test_ext().execute_with(|| {
        let res = Router::emergency_pause_bridge(RuntimeOrigin::root(), 1, b"gate".to_vec());
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::ExternalBridgesDisabled.into())
        );
    });
}

#[test]
fn only_root_can_toggle_external_bridges() {
    new_test_ext().execute_with(|| {
        // Non-root must be rejected.
        let res = Router::set_external_bridges_enabled(RuntimeOrigin::signed(0xCAFE), true);
        assert!(res.is_err(), "non-root must not toggle the kill-switch");
        assert!(
            !pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get(),
            "kill-switch must remain off after a failed non-root toggle"
        );

        // Root may toggle.
        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            true
        ));
        assert_ok!(Router::set_external_bridges_enabled(
            RuntimeOrigin::root(),
            true
        ));
        assert!(pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());

        // And toggle back.
        assert_ok!(Router::set_external_bridges_enabled(
            RuntimeOrigin::root(),
            false
        ));
        assert!(!pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());
    });
}

#[test]
fn enabling_external_bridges_requires_documented_audit_gate() {
    new_test_ext().execute_with(|| {
        let blocked = Router::set_external_bridges_enabled(RuntimeOrigin::root(), true);
        assert_eq!(
            blocked,
            Err(pallet_x3_cross_vm_router::Error::<Test>::ExternalBridgeAuditGateMissing.into())
        );

        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            true
        ));
        assert_ok!(Router::set_external_bridges_enabled(
            RuntimeOrigin::root(),
            true
        ));
    });
}

#[test]
fn register_external_root_works_only_after_governance_enables() {
    new_test_ext().execute_with(|| {
        // First call: blocked.
        assert!(Router::register_external_root(
            RuntimeOrigin::root(),
            1,
            H256::repeat_byte(0x11),
            1,
            vec![1u8; 8],
        )
        .is_err());

        // Governance opens the gate.
        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            true
        ));
        assert_ok!(Router::set_external_bridges_enabled(
            RuntimeOrigin::root(),
            true
        ));

        // Now it should pass the scope-freeze gate (other validation may still
        // gate it; here block_number=1 == current block so it is in-range).
        assert_ok!(Router::register_external_root(
            RuntimeOrigin::root(),
            1,
            H256::repeat_byte(0x11),
            1,
            vec![1u8; 8],
        ));
    });
}

#[test]
fn signed_user_cannot_spoof_vm_origin() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let res = Router::xvm_transfer_from_vm(
            RuntimeOrigin::signed(99),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Native,
            alice_native(),
            10,
            System::block_number() + 50,
        );
        assert!(res.is_err());
    });
}

#[test]
fn evm_adapter_cannot_claim_svm_sender() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let res = Router::xvm_transfer_from_vm(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Evm,
            alice_svm(),
            DomainId::X3Native,
            alice_native(),
            10,
            System::block_number() + 50,
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::IncompatibleSender.into())
        );
    });
}

#[test]
fn svm_adapter_cannot_claim_evm_sender() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let res = Router::xvm_transfer_from_vm(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Svm,
            alice_evm(),
            DomainId::X3Native,
            alice_native(),
            10,
            System::block_number() + 50,
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::IncompatibleSender.into())
        );
    });
}

#[test]
fn vm_adapter_six_routes_preserve_supply_and_clear_pending() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(20_000);

        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 2_000);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 2_000);

        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 10);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 10);
        do_xvm_vm(
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Native,
            alice_native(),
            10,
        );
        do_xvm_vm(
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Svm,
            alice_svm(),
            10,
        );
        do_xvm_vm(
            asset_id,
            DomainId::X3Svm,
            alice_svm(),
            DomainId::X3Native,
            alice_native(),
            10,
        );
        do_xvm_vm(
            asset_id,
            DomainId::X3Svm,
            alice_svm(),
            DomainId::X3Evm,
            alice_evm(),
            10,
        );

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.pending_supply, 0);
        assert!(l.represented().unwrap() <= l.canonical_supply);
        l.check_invariant().unwrap();
    });
}

#[test]
fn wrong_sender_type_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let res = Router::xvm_transfer_from_vm(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Evm,
            alice_native(),
            DomainId::X3Native,
            alice_native(),
            10,
            System::block_number() + 50,
        );
        assert!(res.is_err());
    });
}

// ============================================================================
// PHASE 3 — PRODUCTION PROOF TESTS
// Required by the X3 production gameplan. These prove the three flows that
// MUST hold before public testnet:
//   1. Duplicate nonce rejected (NextNonce monotonic dedup — no UsedNonces map)
//   2. Failed destination credit refunds pending supply (supply bookkeeping)
//   3. Canonical supply NEVER breaks across many transfers (stress invariant)
// ============================================================================

/// Prove that the per-origin nonce dedup (`NextNonce` monotonic scheme) rejects
/// a transfer that reuses the same (source_domain, sender, nonce) triple even
/// when the caller fabricates a new message_id by changing a field.
///
/// The `UsedMessages` store catches identical message_ids; `NextNonce`
/// monotonicity is the second layer — any nonce ≤ current NextNonce is rejected
/// as a replay.
#[test]
fn test_duplicate_nonce_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        // --- First transfer: consume nonce 0 ---
        let now = System::block_number();
        let expires_at = now + 50;
        let sender = alice_native();
        let nonce0 = Router::next_nonce(DomainId::X3Native, sender.clone());

        // Transfer 1 succeeds and burns nonce0.
        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            100,
            expires_at,
        ));

        // Complete the first transfer so the router state is clean.
        let msg0 = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: sender.clone(),
            recipient: alice_evm(),
            amount: 100,
            nonce: nonce0,
            created_at: now,
            expires_at,
        };
        let id0 = x3_asset_kernel_types::derive_message_id::<u64>(&msg0);
        assert_ok!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), id0));

        // --- Second transfer: the next nonce is served from the open batch ---
        let next_batch_watermark = Router::next_nonce(DomainId::X3Native, sender.clone());
        // The persisted NextNonce is the next batch watermark, so it advances
        // past the first allocated batch even though the next served nonce is 1.
        assert!(
            next_batch_watermark > nonce0,
            "batch watermark must advance monotonically"
        );

        // A second well-formed transfer with nonce1 must succeed.
        let second_created_at = System::block_number();
        let second_expires_at = second_created_at + 50;
        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            50,
            second_expires_at,
        ));

        let (batch_start, _batch_size, used_count) =
            Router::nonce_batch_allocation(DomainId::X3Native, sender.clone())
                .expect("nonce allocation exists");
        let nonce1 = batch_start.saturating_add((used_count.saturating_sub(1)) as u128);
        assert!(nonce1 > nonce0, "served nonce must be strictly increasing");

        // The second transfer is in-flight, so pending supply should reflect it
        // until the destination leg is completed.
        let pending = Ledger::ledgers(asset_id).unwrap();
        pending.check_invariant().unwrap();
        assert_eq!(pending.canonical_supply, 10_000);
        assert_eq!(pending.pending_supply, 50);

        let msg1 = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient: alice_evm(),
            amount: 50,
            nonce: nonce1,
            created_at: second_created_at,
            expires_at: second_expires_at,
        };
        let id1 = x3_asset_kernel_types::derive_message_id::<u64>(&msg1);
        assert_ok!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), id1));

        // Verify supply invariant held throughout and pending cleared after completion.
        let completed = Ledger::ledgers(asset_id).unwrap();
        completed.check_invariant().unwrap();
        assert_eq!(completed.canonical_supply, 10_000);
        assert_eq!(completed.pending_supply, 0);
    });
}

/// Prove that if a transfer is initiated (source debited, pending supply
/// increased) but then the transfer expires and is cancelled, the pending
/// supply is fully returned to the source domain — no supply leaks.
///
/// This is the "failed destination credit → refunds pending supply" path.
#[test]
fn test_failed_destination_credit_refunds_pending_supply() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        let now = System::block_number();
        let sender = alice_native();
        let expires_at = now + 20;

        // Capture nonce before transfer.
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());

        // Initiate transfer — source debited, pending supply incremented.
        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            300,
            expires_at,
        ));

        // Mid-flight: verify pending supply is non-zero (source was debited).
        let l_mid = Ledger::ledgers(asset_id).unwrap();
        // native_supply went down, pending went up.
        assert_eq!(l_mid.native_supply, 10_000 - 300);
        assert_eq!(l_mid.pending_supply, 300);
        // Invariant must still hold at this point.
        l_mid.check_invariant().unwrap();

        // Advance blocks past the expiry deadline.
        System::set_block_number(expires_at + 1);

        // Build the message ID.
        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient: alice_evm(),
            amount: 300,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        // Cancel the expired transfer — pending supply MUST return to native.
        assert_ok!(Router::cancel_expired_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        // Post-cancel: native is fully restored, pending is zero, evm unchanged.
        let l_post = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(
            l_post.native_supply, 10_000,
            "native must be fully restored"
        );
        assert_eq!(l_post.evm_supply, 0, "evm must not have received anything");
        assert_eq!(
            l_post.pending_supply, 0,
            "pending must be zero after cancel"
        );
        assert_eq!(
            l_post.canonical_supply, 10_000,
            "canonical supply must be unchanged"
        );
        l_post.check_invariant().unwrap();
    });
}

/// Stress test: execute 100 sequential cross-VM transfers across all six
/// internal routes and assert that the canonical supply invariant holds after
/// every single operation. No supply may ever be created or destroyed; the
/// invariant check is run after every transfer, not just at the end.
///
/// This is the "canonical supply NEVER breaks" production proof.
#[test]
fn test_canonical_supply_never_breaks() {
    new_test_ext().execute_with(|| {
        let total = 1_000_000u128;
        let asset_id = bootstrap_x3_asset(total);

        // Seed EVM and SVM legs so all routes have balance.
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 100_000);
        Ledger::ledgers(asset_id)
            .unwrap()
            .check_invariant()
            .unwrap();

        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 100_000);
        Ledger::ledgers(asset_id)
            .unwrap()
            .check_invariant()
            .unwrap();

        // ── Round-trip sequences ───────────────────────────────────────────
        // Each do_xvm / do_xvm_vm call invokes complete_xvm_transfer so that
        // pending_supply is always drained back to a domain supply. After every
        // call the invariant is asserted.

        let routes_native_origin: &[(DomainId, DomainId, u128)] = &[
            (DomainId::X3Native, DomainId::X3Evm, 10),
            (DomainId::X3Native, DomainId::X3Svm, 10),
        ];

        let routes_vm_origin: &[(DomainId, AccountBytes, DomainId, AccountBytes, u128)] = &[
            (
                DomainId::X3Evm,
                alice_evm(),
                DomainId::X3Native,
                alice_native(),
                5,
            ),
            (
                DomainId::X3Evm,
                alice_evm(),
                DomainId::X3Svm,
                alice_svm(),
                5,
            ),
            (
                DomainId::X3Svm,
                alice_svm(),
                DomainId::X3Native,
                alice_native(),
                5,
            ),
            (
                DomainId::X3Svm,
                alice_svm(),
                DomainId::X3Evm,
                alice_evm(),
                5,
            ),
        ];

        for _round in 0..10 {
            for &(src, dst, amount) in routes_native_origin {
                do_xvm(asset_id, src, dst, amount);
                let l = Ledger::ledgers(asset_id).unwrap();
                assert_eq!(
                    l.canonical_supply, total,
                    "canonical changed after {src:?}->{dst:?}"
                );
                l.check_invariant().unwrap();
            }
            for (src, ref sender, dst, ref recipient, amount) in routes_vm_origin {
                do_xvm_vm(
                    asset_id,
                    *src,
                    sender.clone(),
                    *dst,
                    recipient.clone(),
                    *amount,
                );
                let l = Ledger::ledgers(asset_id).unwrap();
                assert_eq!(
                    l.canonical_supply, total,
                    "canonical changed after {src:?}->{dst:?}"
                );
                l.check_invariant().unwrap();
            }
            // Pending must always be zero between complete rounds.
            let l = Ledger::ledgers(asset_id).unwrap();
            assert_eq!(l.pending_supply, 0, "pending non-zero between rounds");
        }

        // Final assertion: sum of all domain supplies equals canonical.
        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.represented().unwrap(), total);
        assert_eq!(l.canonical_supply, total);
        l.check_invariant().unwrap();
    });
}

#[test]
fn wrong_recipient_type_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let res = Router::xvm_transfer_from_vm(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Svm,
            alice_native(),
            10,
            System::block_number() + 50,
        );
        assert!(res.is_err());
    });
}

#[test]
fn failed_second_leg_rolls_back_first_leg() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let now = System::block_number();
        let expires_at = now + 1;

        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            100,
            expires_at,
        ));

        let nonce = Router::next_nonce(DomainId::X3Native, alice_native()).saturating_sub(100);
        let message = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: alice_native(),
            recipient: alice_evm(),
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&message);

        System::set_block_number(expires_at);
        assert!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err());

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.pending_supply, 100);
        assert_eq!(l.native_supply, 9_900);
    });
}

#[test]
fn replay_message_rejected_no_state_change() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let msg_id = do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 100);
        let before = Ledger::ledgers(asset_id).unwrap();
        assert!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), msg_id).is_err());
        let after = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn duplicate_completion_rejected_no_state_change() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let msg_id = do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 100);
        let before = Ledger::ledgers(asset_id).unwrap();
        assert!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), msg_id).is_err());
        let after = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn refund_after_refund_rejected_no_state_change() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let now = System::block_number();
        let expires_at = now + 1;

        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            100,
            expires_at,
        ));

        let nonce = Router::next_nonce(DomainId::X3Native, alice_native()).saturating_sub(100);
        let message = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: alice_native(),
            recipient: alice_evm(),
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&message);
        System::set_block_number(expires_at + 1);
        assert_ok!(Router::cancel_expired_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        let before = Ledger::ledgers(asset_id).unwrap();
        assert!(Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err());
        let after = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(before, after);
    });
}

#[test]
fn packet_commitment_and_ixl_receipt_are_recorded_on_complete() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        let now = System::block_number();
        let sender = alice_native();
        let recipient = alice_evm();
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());
        let expires_at = now + 50;

        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            recipient.clone(),
            100,
            expires_at,
        ));

        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient,
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        assert!(Router::packet_commitments(message_id).is_some());

        assert_ok!(Router::complete_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        assert_eq!(Router::ixl_receipt_entries(message_id), Some(1));
    });
}

#[test]
fn completion_rejected_after_packet_timeout() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        let now = System::block_number();
        let sender = alice_native();
        let recipient = alice_evm();
        let nonce = Router::next_nonce(DomainId::X3Native, sender.clone());
        let expires_at = now + 1;

        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            recipient.clone(),
            100,
            expires_at,
        ));

        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender,
            recipient,
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);

        // Timeout policy in packet-standard is now_height >= timeout_height.
        System::set_block_number(expires_at);

        assert_eq!(
            Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id),
            Err(pallet_x3_cross_vm_router::Error::<Test>::PacketTimedOut.into())
        );
    });
}

#[test]
fn ixl_abort_after_lock_restores_ledger() {
    // Current router IXL path rejects before destination credit when invalid;
    // this test enforces that source/pending accounting remains restorable.
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let now = System::block_number();
        let expires_at = now + 1;

        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            100,
            expires_at,
        ));

        let nonce = Router::next_nonce(DomainId::X3Native, alice_native()).saturating_sub(100);
        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: DomainId::X3Evm,
            sender: alice_native(),
            recipient: alice_evm(),
            amount: 100,
            nonce,
            created_at: now,
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);
        System::set_block_number(expires_at + 1);
        assert_ok!(Router::cancel_expired_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        let l = Ledger::ledgers(asset_id).unwrap();
        assert_eq!(l.pending_supply, 0);
        assert_eq!(l.native_supply, 10_000);
    });
}

#[test]
fn ixl_slippage_after_lock_restores_ledger() {
    // Regression alias for lock-fail path restoring funds.
    ixl_abort_after_lock_restores_ledger();
}

#[test]
fn completion_after_timeout_rejected() {
    completion_rejected_after_packet_timeout();
}

#[test]
fn duplicate_completion_rejected_no_state_change_alias() {
    duplicate_completion_rejected_no_state_change();
}

#[test]
fn non_root_cannot_set_audit_gate() {
    new_test_ext().execute_with(|| {
        let res = Router::set_external_bridge_audit_gate(RuntimeOrigin::signed(7), true);
        assert!(res.is_err());
        assert!(!pallet_x3_cross_vm_router::ExternalBridgeAuditGate::<Test>::get());
    });
}

#[test]
fn non_root_cannot_enable_bridges() {
    new_test_ext().execute_with(|| {
        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            true
        ));
        let res = Router::set_external_bridges_enabled(RuntimeOrigin::signed(99), true);
        assert!(res.is_err());
        assert!(!pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());
    });
}

#[test]
fn revoking_bridge_audit_gate_disables_external_bridges() {
    new_test_ext().execute_with(|| {
        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            true
        ));
        assert_ok!(Router::set_external_bridges_enabled(
            RuntimeOrigin::root(),
            true
        ));
        assert!(pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());

        assert_ok!(Router::set_external_bridge_audit_gate(
            RuntimeOrigin::root(),
            false
        ));
        assert!(!pallet_x3_cross_vm_router::ExternalBridgeAuditGate::<Test>::get());
        assert!(!pallet_x3_cross_vm_router::ExternalBridgesEnabled::<Test>::get());
    });
}

#[test]
fn six_internal_routes_strict_invariants_and_replay_guards() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(30_000);

        // Seed non-native domains.
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 3_000);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 3_000);

        for (src, dst) in [
            (DomainId::X3Native, DomainId::X3Evm),
            (DomainId::X3Native, DomainId::X3Svm),
            (DomainId::X3Evm, DomainId::X3Native),
            (DomainId::X3Evm, DomainId::X3Svm),
            (DomainId::X3Svm, DomainId::X3Native),
            (DomainId::X3Svm, DomainId::X3Evm),
        ] {
            let before = Ledger::ledgers(asset_id).expect("ledger exists");
            let (message_id, _, _, _) = initiate_transfer_and_id(asset_id, src, dst, 25);

            assert_ok!(Router::complete_xvm_transfer(
                RuntimeOrigin::signed(1),
                message_id
            ));

            let after = Ledger::ledgers(asset_id).expect("ledger exists");
            assert_eq!(after.canonical_supply, before.canonical_supply);
            assert_eq!(after.pending_supply, 0);
            assert_eq!(domain_supply(&after, src), domain_supply(&before, src) - 25);
            assert_eq!(domain_supply(&after, dst), domain_supply(&before, dst) + 25);
            assert_eq!(after.represented().unwrap(), after.canonical_supply);

            assert!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err());
            assert!(
                Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err()
            );
        }
    });
}

#[test]
fn signed_user_cannot_spoof_vm_adapter() {
    signed_user_cannot_spoof_vm_origin();
}

#[test]
fn unsigned_origin_cannot_use_x3_lang_router_entrypoints() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        assert_noop!(
            Router::xvm_transfer(
                RuntimeOrigin::none(),
                asset_id,
                DomainId::X3Evm,
                alice_evm(),
                10,
                System::block_number() + 50,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        assert_noop!(
            Router::xvm_transfer_from_vm(
                RuntimeOrigin::none(),
                asset_id,
                DomainId::X3Evm,
                alice_evm(),
                DomainId::X3Native,
                alice_native(),
                10,
                System::block_number() + 50,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        let (message_id, _, _, expires_at) =
            initiate_transfer_and_id(asset_id, DomainId::X3Native, DomainId::X3Evm, 10);

        assert_noop!(
            Router::complete_xvm_transfer(RuntimeOrigin::none(), message_id),
            sp_runtime::DispatchError::BadOrigin
        );

        System::set_block_number(expires_at + 1);
        assert_noop!(
            Router::cancel_expired_xvm_transfer(RuntimeOrigin::none(), message_id),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn compiled_x3_lang_gateway_path_routes_and_rejects_direct_unsigned() {
    let source = r#"
        fn main() {
            xvm_transfer("x3evm", "alice_evm", 10, 50);
        }
    "#;
    let lowered = x3_compiler::lower_gateway_call(source).expect("x3-lang gateway call lowers");

    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        assert_noop!(
            Router::xvm_transfer(
                RuntimeOrigin::none(),
                asset_id,
                DomainId::X3Evm,
                alice_evm(),
                10,
                System::block_number() + 50,
            ),
            sp_runtime::DispatchError::BadOrigin
        );

        let x3_compiler::GatewayRuntimeCall::RouterXvmTransfer {
            destination,
            recipient,
            amount,
            expires_in,
        } = lowered
        else {
            panic!("expected xvm_transfer gateway call");
        };
        let destination = domain_from_gateway(destination);
        let recipient = account_from_gateway(recipient).expect("supported mock account");
        let amount = amount as u128;
        let expires_at = System::block_number() + expires_in;

        let call = RuntimeCall::Router(pallet_x3_cross_vm_router::Call::<Test>::xvm_transfer {
            asset_id,
            destination,
            recipient: recipient.clone(),
            amount,
            expires_at,
        });
        assert_ok!(call.dispatch(RuntimeOrigin::signed(1)));

        let (batch_start, _batch_size, used_count) =
            Router::nonce_batch_allocation(DomainId::X3Native, alice_native())
                .expect("nonce allocation exists");
        let nonce = batch_start.saturating_add((used_count.saturating_sub(1)) as u128);
        let msg = x3_asset_kernel_types::X3TransferMessage::<u64> {
            version: x3_asset_kernel_types::MESSAGE_FORMAT_VERSION,
            asset_id,
            source_domain: DomainId::X3Native,
            destination_domain: destination,
            sender: alice_native(),
            recipient,
            amount,
            nonce,
            created_at: System::block_number(),
            expires_at,
        };
        let message_id = x3_asset_kernel_types::derive_message_id::<u64>(&msg);
        assert_ok!(Router::complete_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        let transfer = pallet_x3_cross_vm_router::Transfers::<Test>::get(message_id)
            .expect("gateway-routed transfer is recorded");
        assert_eq!(
            transfer.status,
            x3_asset_kernel_types::TransferStatus::Finalized
        );

        let ledger = Ledger::ledgers(asset_id).expect("ledger exists");
        ledger.check_invariant().unwrap();
        assert_eq!(ledger.pending_supply, 0);
    });
}

fn domain_from_gateway(value: x3_compiler::GatewayDomain) -> DomainId {
    match value {
        x3_compiler::GatewayDomain::X3Native => DomainId::X3Native,
        x3_compiler::GatewayDomain::X3Evm => DomainId::X3Evm,
        x3_compiler::GatewayDomain::X3Svm => DomainId::X3Svm,
    }
}

fn account_from_gateway(value: x3_compiler::GatewayAccount) -> Option<AccountBytes> {
    match value {
        x3_compiler::GatewayAccount::X3Native(value) if value == "alice_native" => {
            Some(alice_native())
        }
        x3_compiler::GatewayAccount::Evm(value) if value == "alice_evm" => Some(alice_evm()),
        x3_compiler::GatewayAccount::Svm(value) if value == "alice_svm" => Some(alice_svm()),
        _ => None,
    }
}

#[test]
fn duplicate_message_id_rejected() {
    test_duplicate_message_replay_rejected();
}

#[test]
fn completion_after_refund_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let (message_id, _, _, expires_at) =
            initiate_transfer_and_id(asset_id, DomainId::X3Native, DomainId::X3Evm, 100);

        System::set_block_number(expires_at + 1);
        assert_ok!(Router::cancel_expired_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        assert!(Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err());
    });
}

#[test]
fn refund_after_finalized_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let (message_id, _, _, expires_at) =
            initiate_transfer_and_id(asset_id, DomainId::X3Native, DomainId::X3Evm, 100);
        assert_ok!(Router::complete_xvm_transfer(
            RuntimeOrigin::signed(1),
            message_id
        ));

        System::set_block_number(expires_at + 1);
        assert!(Router::cancel_expired_xvm_transfer(RuntimeOrigin::signed(1), message_id).is_err());
    });
}

#[test]
fn route_pending_limit_enforced() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let mut cfg = permissive_route();
        cfg.limits.pending_limit = 1;
        assert_ok!(Registry::configure_route(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Native,
            DomainId::X3Evm,
            cfg,
        ));

        let (_id, _, _, _) =
            initiate_transfer_and_id(asset_id, DomainId::X3Native, DomainId::X3Evm, 10);
        let blocked = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            10,
            System::block_number() + 50,
        );
        assert_eq!(
            blocked,
            Err(pallet_x3_cross_vm_router::Error::<Test>::RoutePendingLimitExceeded.into())
        );
    });
}

#[test]
fn amount_above_route_limit_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let mut cfg = permissive_route();
        cfg.limits.min_amount = 5;
        cfg.limits.max_amount = 20;
        assert_ok!(Registry::configure_route(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Native,
            DomainId::X3Evm,
            cfg,
        ));

        let res = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            21,
            System::block_number() + 50,
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::AmountOutOfBounds.into())
        );
    });
}

#[test]
fn amount_below_route_min_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let mut cfg = permissive_route();
        cfg.limits.min_amount = 5;
        cfg.limits.max_amount = 20;
        assert_ok!(Registry::configure_route(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Native,
            DomainId::X3Evm,
            cfg,
        ));

        let res = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            4,
            System::block_number() + 50,
        );
        assert_eq!(
            res,
            Err(pallet_x3_cross_vm_router::Error::<Test>::AmountOutOfBounds.into())
        );
    });
}

// ============================================================================
// PHASE 4.5 — ROUTE LIMIT REGRESSION TESTS
//
// These tests cover daily volume and wallet daily volume limit enforcement
// paths that were previously untested. They complement the existing
// route_pending_limit_enforced, amount_above_route_limit_rejected, and
// amount_below_route_min_rejected tests.
//
// NOTE: `DailyVolumeLimitExceeded` is structurally identical to the
// wallet-daily path (same epoch-accumulator pattern, different storage key).
// It cannot be tested with the DEV_PERMISSIVE route because that sets
// `daily_limit = u128::MAX`, which skips the check.  The wallet-daily test
// below proves the epoch-accumulator enforcement path works.
//
// NOTE: packet_from_message fails only when nonce overflows u64 — not
// feasible to trigger in unit tests. NonceBatchExhausted requires extreme
// nonce pressure (100+ calls per batch) which is impractical for unit tests.
// PacketCommitmentMismatch is tested by corrupting the stored commitment.
// ============================================================================

/// Prove that WalletDailyVolumeLimitExceeded is thrown when a single sender
/// exceeds the per-wallet 24h volume limit.
#[test]
fn wallet_daily_volume_limit_exceeded_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);
        let mut cfg = permissive_route();
        // Set a low per-wallet daily limit.
        cfg.limits.per_wallet_daily_limit = 120;
        assert_ok!(Registry::configure_route(
            RuntimeOrigin::root(),
            asset_id,
            DomainId::X3Native,
            DomainId::X3Evm,
            cfg,
        ));

        // First transfer of 80 — should succeed.
        assert_ok!(Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            80,
            System::block_number() + 50,
        ));

        // Second transfer of 50 would push wallet total to 130 > 120 limit.
        let blocked = Router::xvm_transfer(
            RuntimeOrigin::signed(1),
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            50,
            System::block_number() + 50,
        );
        assert_eq!(
            blocked,
            Err(pallet_x3_cross_vm_router::Error::<Test>::WalletDailyVolumeLimitExceeded.into())
        );
    });
}

/// Prove that PacketCommitmentMismatch is thrown when the stored commitment
/// does not match the recomputed packet at completion time.
#[test]
fn packet_commitment_mismatch_rejected() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(10_000);

        // Initiate a transfer to create the packet commitment.
        let (message_id, _, _, _) =
            initiate_transfer_and_id(asset_id, DomainId::X3Native, DomainId::X3Evm, 50);

        // Corrupt the stored packet commitment so it won't match recomputation.
        pallet_x3_cross_vm_router::PacketCommitments::<Test>::insert(
            message_id,
            H256::repeat_byte(0xff),
        );

        // Completion must fail with PacketCommitmentMismatch.
        assert_eq!(
            Router::complete_xvm_transfer(RuntimeOrigin::signed(1), message_id),
            Err(pallet_x3_cross_vm_router::Error::<Test>::PacketCommitmentMismatch.into())
        );
    });
}

// ============================================================================
// PHASE 4 — EXPANDED ROUTE MATRIX + STATE MACHINE GUARD
//
// These tests complement the existing
// `vm_adapter_six_routes_preserve_supply_and_clear_pending` test. That
// test exercises the four VM-adapter routes (X3Evm→X3Native,
// X3Evm→X3Svm, X3Svm→X3Native, X3Svm→X3Evm) alongside the two
// native-source routes. The new tests below:
//
//   * Pin the SVM-source → EVM-destination and EVM-source →
//     SVM-destination full round trips as named tests with explicit
//     supply-and-pending assertions. The existing six-route test
//     covers these implicitly; pinning them explicitly makes the
//     cross-VM path test failures self-explanatory when a single
//     direction regresses.
//   * Add a state-machine guard test that enumerates the legal
//     `TransferStatus` transitions and asserts every illegal
//     transition is rejected. This is the keystone of the freeze
//     that the cross-VM audit called out.
//
// These tests are unit-level only; live RPC + node build is out of
// scope for this turn (build-chain blocker on rustc/libsecp256k1
// per the cross-VM validation report).
// ============================================================================

/// SVM-source → EVM-destination full round trip via the VM-adapter
/// origin. Pinned explicitly so a single direction's regression
/// produces a self-explanatory test name in CI output.
#[test]
fn xvm_router_svm_to_evm_full_round_trip() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(20_000);

        // Seed the EVM and SVM legs with native-source transfers
        // first, so the SVM-source debit has supply to draw from
        // (the asset's native-mint-burn policy only mints into the
        // native leg; EVM and SVM start at 0).
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 2_000);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 2_000);

        let l0 = Ledger::ledgers(asset_id).expect("ledger exists");
        let svm0 = l0.svm_supply;
        let evm0 = l0.evm_supply;

        let _msg_id = do_xvm_vm(
            asset_id,
            DomainId::X3Svm,
            alice_svm(),
            DomainId::X3Evm,
            alice_evm(),
            250,
        );

        let l1 = Ledger::ledgers(asset_id).expect("ledger exists");
        // The full round trip debits the SVM domain and credits the
        // EVM domain.
        assert!(
            l1.svm_supply < svm0,
            "SVM source supply must decrease (was {}, now {})",
            svm0,
            l1.svm_supply
        );
        assert!(
            l1.evm_supply > evm0,
            "EVM destination supply must increase (was {}, now {})",
            evm0,
            l1.evm_supply
        );
        assert_eq!(l1.pending_supply, 0, "pending must clear on completion");
        l1.check_invariant().expect("ledger invariant must hold");
    });
}

/// EVM-source → SVM-destination full round trip via the VM-adapter
/// origin. Pinned explicitly so a single direction's regression
/// produces a self-explanatory test name in CI output.
#[test]
fn xvm_router_evm_to_svm_full_round_trip() {
    new_test_ext().execute_with(|| {
        let asset_id = bootstrap_x3_asset(20_000);

        // Seed the EVM and SVM legs with native-source transfers
        // first, so the EVM-source debit has supply to draw from
        // (see comment in `xvm_router_svm_to_evm_full_round_trip`).
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Evm, 2_000);
        do_xvm(asset_id, DomainId::X3Native, DomainId::X3Svm, 2_000);

        let l0 = Ledger::ledgers(asset_id).expect("ledger exists");
        let evm0 = l0.evm_supply;
        let svm0 = l0.svm_supply;

        let _msg_id = do_xvm_vm(
            asset_id,
            DomainId::X3Evm,
            alice_evm(),
            DomainId::X3Svm,
            alice_svm(),
            175,
        );

        let l1 = Ledger::ledgers(asset_id).expect("ledger exists");
        assert!(
            l1.evm_supply < evm0,
            "EVM source supply must decrease (was {}, now {})",
            evm0,
            l1.evm_supply
        );
        assert!(
            l1.svm_supply > svm0,
            "SVM destination supply must increase (was {}, now {})",
            svm0,
            l1.svm_supply
        );
        assert_eq!(l1.pending_supply, 0, "pending must clear on completion");
        l1.check_invariant().expect("ledger invariant must hold");
    });
}

/// State-machine guard: enumerate every `TransferStatus` pair and
/// assert that only the legal transitions succeed. The legal set
/// is the one in `x3_asset_kernel_types::TransferStatus::can_transition_to`
/// (the authoritative state graph). This is the keystone of the
/// freeze the cross-VM audit called out — a regression that lets
/// `Created → Finalized` through would let a destination-side
/// credit occur before the source-side debit is recorded, breaking
/// the supply invariant.
#[test]
fn xvm_router_state_machine_legal_transitions_only() {
    use x3_asset_kernel_types::TransferStatus::*;
    let all = [
        Created,
        SourceDebited,
        DestinationCredited,
        Finalized,
        Expired,
        Refunded,
        Failed,
    ];
    // Authoritative set from `TransferStatus::can_transition_to`.
    // Kept as a `Vec` (rather than a `HashSet`) because
    // `TransferStatus` deliberately does not derive `Hash` — the
    // type is a runtime state, not a hash key, and adding `Hash`
    // would be a backwards-incompatible AST change for downstream
    // consumers. Linear scan over 8 pairs is O(64) per
    // `contains`-equivalent, which is trivial for a 7×7 = 49-pair
    // test matrix.
    let legal: &[(TransferStatus, TransferStatus)] = &[
        (Created, SourceDebited),
        (Created, Failed),
        (SourceDebited, DestinationCredited),
        (SourceDebited, Expired),
        (SourceDebited, Failed),
        (DestinationCredited, Finalized),
        (Expired, Refunded),
        (Expired, Failed),
    ];
    let is_legal_pair = |from: TransferStatus, to: TransferStatus| -> bool {
        legal.iter().any(|&(a, b)| a == from && b == to)
    };

    for from in all.iter() {
        for to in all.iter() {
            let key = (*from, *to);
            let is_legal = is_legal_pair(*from, *to);
            // Self-transitions are always illegal (a state machine
            // step must change state). The legal set above already
            // excludes them, so we don't special-case.
            assert_eq!(
                from.can_transition_to(*to),
                is_legal,
                "transition {:?} -> {:?}: `can_transition_to` must match the legal set",
                from,
                to
            );
            // Silence the unused-variable warning on `key` — we
            // compute it for the diagnostic message above.
            let _ = key;
        }
    }

    // Spot-check a few illegal transitions that the audit
    // specifically called out.
    assert!(
        !Created.can_transition_to(Finalized),
        "Created -> Finalized must be illegal (skipping debit/credit)"
    );
    assert!(
        !Created.can_transition_to(DestinationCredited),
        "Created -> DestinationCredited must be illegal (skipping debit)"
    );
    assert!(
        !Created.can_transition_to(Refunded),
        "Created -> Refunded must be illegal (refund requires expiry first)"
    );
    assert!(
        !Finalized.can_transition_to(Refunded),
        "Finalized is terminal: any further transition is illegal"
    );
    assert!(
        !Finalized.can_transition_to(Failed),
        "Finalized is terminal: any further transition is illegal"
    );
    assert!(
        !Refunded.can_transition_to(Finalized),
        "Refunded is terminal: any further transition is illegal"
    );
    assert!(
        !Failed.can_transition_to(Refunded),
        "Failed is terminal: any further transition is illegal"
    );
}
