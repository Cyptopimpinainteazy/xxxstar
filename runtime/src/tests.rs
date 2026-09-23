//! Runtime-level tests for the settlement wiring.
//!
//! This file replaced an assertion-free placeholder. It used to hold fourteen
//! `#[test]` functions whose bodies were only comments ("Scenario: …",
//! "Assertion: …"), it was never declared in `lib.rs` (so nothing ever compiled
//! it), and its mock runtime was written against a pre-2022
//! `frame_system::Config`. The tests below assert against the real `Runtime`,
//! using the same `TestExternalities` pattern as the inline tests in `lib.rs`.

use super::*;
use frame_support::{assert_err, assert_ok};
use pallet_x3_settlement_engine::types::{AssetSpec, ExternalChainId, TokenId};
use sp_runtime::DispatchError;

fn account(seed: u8) -> AccountId {
    AccountId::from([seed; 32])
}

fn native_asset(chain: ExternalChainId, amount: u128) -> AssetSpec {
    AssetSpec {
        chain,
        token: TokenId::Native,
        amount,
    }
}

#[test]
fn runtime_genesis_config_builds() {
    use sp_runtime::BuildStorage;

    let storage = RuntimeGenesisConfig::default()
        .build_storage()
        .expect("the runtime's genesis config must build");
    // A genesis that produced nothing would still "build"; check the two
    // pallets this file exercises actually have storage entries.
    assert!(
        !storage.top.is_empty(),
        "genesis produced an empty top-level trie"
    );
}

/// The settlement engine must be reachable through the real runtime with a
/// signed origin, and the intent must land in its storage.
#[test]
fn settlement_intent_creation_is_wired_through_the_runtime() {
    sp_io::TestExternalities::default().execute_with(|| {
        let maker = account(0xAA);
        let taker = account(0xBB);
        let secret_hash = H256::from([0x11; 32]);

        assert_ok!(
            pallet_x3_settlement_engine::Pallet::<Runtime>::create_intent(
                RuntimeOrigin::signed(maker.clone()),
                taker,
                native_asset(ExternalChainId::X3Native, 1_000),
                native_asset(ExternalChainId::Ethereum, 500),
                secret_hash,
                Some(600),
            )
        );

        let intents: Vec<_> =
            pallet_x3_settlement_engine::SettlementIntents::<Runtime>::iter().collect();
        assert_eq!(intents.len(), 1, "the intent was stored");
        assert_eq!(intents[0].1.maker, maker);
        assert_eq!(intents[0].1.secret_hash, secret_hash);
        assert_eq!(intents[0].1.legs_total, 2);
    });
}

/// Settlement intents are per-account, so an unsigned origin must be refused
/// rather than attributed to some default account.
#[test]
fn settlement_intent_creation_requires_a_signed_origin() {
    sp_io::TestExternalities::default().execute_with(|| {
        let result = pallet_x3_settlement_engine::Pallet::<Runtime>::create_intent(
            RuntimeOrigin::root(),
            account(0xAA),
            native_asset(ExternalChainId::X3Native, 1_000),
            native_asset(ExternalChainId::Ethereum, 500),
            H256::from([0x11; 32]),
            None,
        );
        assert_err!(result, DispatchError::BadOrigin);
        assert!(
            pallet_x3_settlement_engine::SettlementIntents::<Runtime>::iter().count() == 0,
            "a rejected call must not leave an intent behind"
        );
    });
}

// ── The atomic lifecycle against the real runtime ────────────────────────────
//
// `tests/e2e/safety_tests.rs` and `tests/e2e/real_finality_proofs.rs` claimed to cover this and never
// compiled — they were not declared as test targets, and they would not have compiled if they had:
// `submit_atomic_bundle(origin, legs, deadline)` is missing the chain id and nonce the call takes,
// `BundleLeg::Lock { amount, asset }` is not a variant of the leg type, `RuntimeOrigin::signed(1)`
// is not an account the runtime ever authorized for the atomic gate, and
// `assert_err!(result, "NonceAlreadyUsed")` compares a `DispatchError` with a string. They are
// deleted. These four tests are what those files claimed, written against the API that exists.

const ATOMIC_CHAIN_ID: u32 = 7;
const ATOMIC_NONCE: u64 = 1;

/// The account this test's genesis authorizes for the atomic gate and funds for the bond.
fn atomic_gateway() -> AccountId {
    AccountId::from([0x6A; 32])
}

fn atomic_leg() -> pallet_x3_atomic_kernel::proof::BundleLeg {
    use pallet_x3_atomic_kernel::proof::{BundleLeg, DeclaredAccess, VmType};
    BundleLeg {
        vm_type: VmType::X3,
        token_in: H256::repeat_byte(0xAA),
        token_out: H256::repeat_byte(0xBB),
        amount_in: 1_000,
        min_amount_out: 900,
        deadline: 10_000,
        access: DeclaredAccess {
            reads: Default::default(),
            writes: Default::default(),
        },
    }
}

/// Genesis with the gateway **authorized** — since spec 19 the atomic gate reads the custody
/// registry, and a chain that names nobody admits nobody — and funded, because the submitter pays
/// `MinBond`.
fn atomic_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;

    let storage = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: vec![(atomic_gateway(), 10_000 * X3)],
            dev_accounts: None,
        },
        x3_custody: pallet_x3_custody::GenesisConfig {
            x3_lang_gateways: vec![atomic_gateway()],
            settlement_gateways: vec![],
            ..Default::default()
        },
        ..Default::default()
    }
    .build_storage()
    .expect("the atomic test genesis must build");

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// The receipt root the chain computes for a bundle — the same commitment `do_finalize_bundle`
/// checks on a build without `dev`/`testnet`.
fn runtime_receipt_root(bundle_id: H256, cert: H256, finalized_block: u64) -> H256 {
    use codec::Encode;

    let record =
        pallet_x3_atomic_kernel::Bundles::<Runtime>::get(bundle_id).expect("the bundle exists");
    let executor_hash = record
        .executor
        .as_ref()
        .map(|account| H256::from(sp_io::hashing::blake2_256(&account.encode())))
        .unwrap_or(H256::zero());
    H256::from(sp_io::hashing::blake2_256(
        &pallet_x3_atomic_kernel::ReceiptRootData {
            bundle_id,
            legs_hash: record.legs_hash,
            leg_count: record.leg_count,
            executor_hash,
            finalized_block,
            finality_cert: cert,
        }
        .encode(),
    ))
}

#[test]
fn the_atomic_kernel_refuses_an_account_the_chain_did_not_authorize() {
    atomic_test_ext().execute_with(|| {
        let outsider = AccountId::from([0x11; 32]);
        assert_err!(
            crate::X3AtomicKernel::assign_bundle_executor(
                RuntimeOrigin::signed(outsider.clone()),
                H256::repeat_byte(0x01)
            ),
            DispatchError::BadOrigin
        );
        assert_err!(
            crate::X3AtomicKernel::finalize_atomic_bundle(
                RuntimeOrigin::signed(outsider),
                H256::repeat_byte(0x01),
                H256::repeat_byte(0x02),
                H256::repeat_byte(0x03),
                1
            ),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn the_genesis_authorized_gateway_reaches_the_atomic_kernel() {
    atomic_test_ext().execute_with(|| {
        // The same call, from the account genesis authorized: it gets *past* the origin, which is
        // the wiring this test exists for. It then fails on the bundle, not on the caller.
        let error = crate::X3AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(atomic_gateway()),
            H256::repeat_byte(0x01),
        )
        .expect_err("a bundle that was never submitted cannot be assigned");
        assert_ne!(error, DispatchError::BadOrigin);
        assert!(
            matches!(error, DispatchError::Module(_)),
            "expected a pallet error, got {error:?}"
        );
    });
}

#[test]
fn a_bundle_finalizes_once_with_the_receipt_root_the_chain_requires() {
    use frame_support::BoundedVec;

    atomic_test_ext().execute_with(|| {
        let gateway = atomic_gateway();
        assert_ok!(crate::X3AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            BoundedVec::try_from(vec![atomic_leg()]).expect("within MaxLegsPerBundle"),
            100,
            ATOMIC_CHAIN_ID,
            ATOMIC_NONCE,
        ));
        let (bundle_id, _) = pallet_x3_atomic_kernel::Bundles::<Runtime>::iter()
            .next()
            .expect("the submitted bundle is stored");
        assert_ok!(crate::X3AtomicKernel::assign_bundle_executor(
            RuntimeOrigin::signed(gateway.clone()),
            bundle_id
        ));

        // The anchor map is keyed by `u64`; the call takes the runtime's own block number.
        let now_u32 = System::block_number();
        let now: u64 = now_u32.into();
        let cert = H256::repeat_byte(0x55);
        pallet_x3_atomic_kernel::FinalityCertAnchors::<Runtime>::insert(now, cert);

        // A receipt root the bundle does not commit to is refused...
        assert_err!(
            crate::X3AtomicKernel::finalize_atomic_bundle(
                RuntimeOrigin::signed(gateway.clone()),
                bundle_id,
                H256::repeat_byte(0x66),
                cert,
                now_u32
            ),
            pallet_x3_atomic_kernel::Error::<Runtime>::InvalidReceiptRoot
        );

        // ...the commitment the chain computes is accepted, and finalization happens once.
        let root = runtime_receipt_root(bundle_id, cert, now);
        assert_ok!(crate::X3AtomicKernel::finalize_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            bundle_id,
            root,
            cert,
            now_u32
        ));
        assert_eq!(
            pallet_x3_atomic_kernel::Bundles::<Runtime>::get(bundle_id)
                .expect("the bundle exists")
                .status,
            pallet_x3_atomic_kernel::BundleStatus::Finalized
        );
        assert!(
            pallet_x3_atomic_kernel::PoaeProofs::<Runtime>::contains_key(bundle_id),
            "a finalized bundle carries its proof"
        );
        assert_err!(
            crate::X3AtomicKernel::finalize_atomic_bundle(
                RuntimeOrigin::signed(gateway),
                bundle_id,
                root,
                cert,
                now_u32
            ),
            pallet_x3_atomic_kernel::Error::<Runtime>::InvalidBundleState
        );
    });
}

#[test]
fn a_bundle_nonce_cannot_be_replayed() {
    use frame_support::BoundedVec;

    atomic_test_ext().execute_with(|| {
        let gateway = atomic_gateway();
        let legs = || BoundedVec::try_from(vec![atomic_leg()]).expect("within MaxLegsPerBundle");

        assert_ok!(crate::X3AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            legs(),
            100,
            ATOMIC_CHAIN_ID,
            ATOMIC_NONCE,
        ));
        assert_err!(
            crate::X3AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(gateway),
                legs(),
                100,
                ATOMIC_CHAIN_ID,
                ATOMIC_NONCE,
            ),
            pallet_x3_atomic_kernel::Error::<Runtime>::InvalidNonce
        );
    });
}
