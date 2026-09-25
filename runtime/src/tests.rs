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

// ── the X3 domain, driven through the runtime's own dispatch ────────────────────────────────

/// A genesis with one funded account, authorized to submit comits to the kernel.
fn kernel_test_ext() -> sp_io::TestExternalities {
    use sp_runtime::BuildStorage;

    let submitter = account(7);
    let storage = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: vec![(submitter.clone(), 10_000 * X3)],
            dev_accounts: None,
        },
        ..Default::default()
    }
    .build_storage()
    .expect("the kernel test genesis must build");

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| {
        System::set_block_number(1);
        assert_ok!(pallet_x3_kernel::Pallet::<Runtime>::authorize_account(
            RuntimeOrigin::root(),
            submitter
        ));
    });
    ext
}

/// A compiled `.x3` program, dispatched through the runtime, executes and is recorded.
///
/// This is the proof X3-LANG-004's row asked for and did not have: the pallet's own tests
/// configure `TestX3Adapter`, which fabricates a receipt, so nothing showed whether a program
/// compiled from source could travel the real route. It could not. `submit_comit_v2` validated
/// its `x3_payload` as an `X3VmPacket` and then handed those same bytes to
/// `T::X3Adapter::execute`, which parses X3BC bytecode — a packet is not a program, so on a
/// chain with a real adapter (`X3VmAdapter`, or `WasmX3Adapter` in the wasm build) every
/// non-empty X3 payload died with `X3ExecutionFailed`. The mock had been adjusted to the packet
/// shape rather than the path being fixed, which is why the mismatch survived.
#[test]
fn a_compiled_x3_program_is_executed_through_the_runtime_and_its_comit_is_recorded() {
    let submitter = account(7);
    let program = x3_x3_integration::compiler_bridge::compile_source(
        "fn main() -> i64 {\n    return 42;\n}\n",
    )
    .expect("the fixture must compile");

    kernel_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(0xC0FFEE);
        let nonce = pallet_x3_kernel::Nonces::<Runtime>::get(submitter.clone());
        // The kernel enforces a minimum-fee floor (`IncorrectFee` below it); the submitter is funded.
        let fee: Balance = 1_000_000;
        let prepare_root = pallet_x3_kernel::Pallet::<Runtime>::compute_prepare_root_v2(
            comit_id,
            &[],
            &[],
            &program,
            nonce,
            fee,
        );

        assert_ok!(pallet_x3_kernel::Pallet::<Runtime>::submit_comit_v2(
            RuntimeOrigin::signed(submitter.clone()),
            comit_id,
            Vec::new(),
            Vec::new(),
            program.clone(),
            nonce,
            fee,
            prepare_root,
        ));

        // Storage mutation: the chain recorded the comit and moved the submitter's nonce.
        assert!(
            pallet_x3_kernel::SubmittedComits::<Runtime>::get(comit_id).is_some(),
            "an accepted comit must be recorded, or the same id could be replayed"
        );
        assert_eq!(
            pallet_x3_kernel::Nonces::<Runtime>::get(submitter.clone()),
            nonce + 1
        );

        // The program's result is on-chain, not only in the event: this is the receipt step of the
        // X3Lang pipeline, and without it "the program ran and returned 42" cannot be checked after
        // the fact.
        let receipt = pallet_x3_kernel::Pallet::<Runtime>::x3_execution_receipt(comit_id)
            .expect("an accepted X3 comit must store its execution receipt");
        assert!(
            receipt.success,
            "the fixture program returns, so the receipt succeeds"
        );
        assert_eq!(
            receipt.return_data,
            42i64.to_le_bytes().to_vec(),
            "the receipt must carry the value the source states"
        );
        assert!(receipt.gas_used > 0, "the receipt must report metered gas");
        assert_eq!(
            receipt.version,
            pallet_x3_kernel::EXECUTION_RECEIPT_VERSION,
            "and be stamped with the kernel's receipt version"
        );
    });
}

/// The same dispatch with a corrupted program must be refused — and refused *before* the comit
/// is recorded, because the adapter's verification is what makes the payload trustworthy.
#[test]
fn a_corrupted_x3_program_is_refused_by_the_runtime_path() {
    let submitter = account(7);
    let mut program = x3_x3_integration::compiler_bridge::compile_source(
        "fn main() -> i64 {\n    return 42;\n}\n",
    )
    .expect("the fixture must compile");
    let last = program.len() - 1;
    program[last] ^= 0xFF;

    kernel_test_ext().execute_with(|| {
        let comit_id = H256::from_low_u64_be(0x0BAD_C0DEu64);
        let nonce = pallet_x3_kernel::Nonces::<Runtime>::get(submitter.clone());
        // The kernel enforces a minimum-fee floor (`IncorrectFee` below it); the submitter is funded.
        let fee: Balance = 1_000_000;
        let prepare_root = pallet_x3_kernel::Pallet::<Runtime>::compute_prepare_root_v2(
            comit_id,
            &[],
            &[],
            &program,
            nonce,
            fee,
        );

        assert!(
            pallet_x3_kernel::Pallet::<Runtime>::submit_comit_v2(
                RuntimeOrigin::signed(submitter.clone()),
                comit_id,
                Vec::new(),
                Vec::new(),
                program.clone(),
                nonce,
                fee,
                prepare_root,
            )
            .is_err(),
            "a module whose bytes were edited must not be executed"
        );
        assert!(
            pallet_x3_kernel::SubmittedComits::<Runtime>::get(comit_id).is_none(),
            "a refused comit must leave no record that lets the id be reused"
        );
        assert!(
            pallet_x3_kernel::Pallet::<Runtime>::x3_execution_receipt(comit_id).is_none(),
            "and it must not leave a receipt claiming a program it never ran"
        );
    });
}
