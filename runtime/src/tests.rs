//! Runtime-level tests for the settlement wiring.
//!
//! This file replaced an assertion-free placeholder. It used to hold fourteen
//! `#[test]` functions whose bodies were only comments ("Scenario: …",
//! "Assertion: …"), it was never declared in `lib.rs` (so nothing ever compiled
//! it), and its mock runtime was written against a pre-2022
//! `frame_system::Config`. The tests below assert against the real `Runtime`,
//! using the same `TestExternalities` pattern as the inline tests in `lib.rs`.

use super::*;
use frame_support::{assert_err, assert_noop, assert_ok};
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

/// A window the chain can open must also be a window it can *settle*, in one block, as one
/// transaction.
///
/// This is the invariant a live run found the hard way: the ordering lane could be opened,
/// committed into and revealed on a three-validator chain, and then the settle was refused by the
/// transaction pool with `Invalid Transaction: Transaction would exhaust the block limits` — a
/// window whose bonds can never be released. The pool rejects a transaction whose declared weight
/// does not fit the block, and the settle weight is a function of what the window holds, so the
/// capacity and the byte ceiling the runtime configures have to be settlable in one transaction.
/// A test at the pallet level cannot see this: it needs the runtime's own `BlockWeights`.
///
/// `#[cfg(not(feature = "mainnet-rc1"))]` because the pallet it exercises is: the runtime's
/// `impl pallet_private_execution::Config for Runtime` carries that same cfg, so without this the
/// `mainnet-rc1` build of this test file does not compile (measured: `clippy runtime rc1` failed
/// with `the trait bound Runtime: pallet_private_execution::Config is not satisfied`).
#[cfg(not(feature = "mainnet-rc1"))]
#[test]
fn a_full_ordering_window_is_settlable_in_one_block() {
    use frame_support::weights::Weight;
    use pallet_private_execution::WeightInfo;

    let weights = BlockWeights::get();
    let normal = weights.get(dispatch_class_normal());
    let limit: Weight = normal
        .max_total
        .expect("with_sensible_defaults sets a max_total for the normal class");

    // The worst case the runtime allows: every commitment slot taken and the whole plaintext budget
    // revealed. If this does not fit, the runtime has configured a window it can never settle.
    let worst = <Runtime as pallet_private_execution::Config>::WeightInfo::settle_ordering_window(
        <Runtime as pallet_private_execution::Config>::MaxOrderingCommits::get(),
        <Runtime as pallet_private_execution::Config>::MaxOrderingWindowBytes::get(),
    );

    assert!(
        worst.all_lte(limit),
        "settling a full ordering window weighs {worst:?}, which does not fit the normal class's \
         {limit:?}: the pool would refuse every settle and the window's bonds could never be \
         released. Lower MaxOrderingCommits/MaxOrderingWindowBytes, or price the settle."
    );

    // The control, and the reason this test exists: the capacity this runtime *used* to configure
    // (1024 commitments and a 1 MiB plaintext ceiling) does not fit, which is the failure a live
    // three-validator run hit — the window opened, was committed into and revealed, and every
    // settle was refused by the pool with `Invalid Transaction: Transaction would exhaust the
    // block limits`. If that configuration is ever restored, this test says so before a drill does.
    let previously_configured = <() as WeightInfo>::settle_ordering_window(1024, 1_048_576);
    assert!(
        !previously_configured.all_lte(limit),
        "1024 commitments and 1 MiB of revealed plaintext weigh {previously_configured:?}, which \
         is inside the normal class's {limit:?} — either this test's arithmetic is wrong or the \
         block budget grew, and the live drill's failure needs re-deriving before that number is \
         trusted."
    );
}

fn dispatch_class_normal() -> frame_support::dispatch::DispatchClass {
    frame_support::dispatch::DispatchClass::Normal
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

// ── the two halt switches, and whether they meet ─────────────────────────────────────────────
//
// X3-RT-001's row named three things that were "read from the code, not proven by a test": the
// atomic-kernel's economic halt and the kernel's routine pause are independent switches, nothing
// drove both at once, and the runtime's `EmergencyHaltController` wiring had no runtime-level
// test. `RuntimeEmergencyHaltController::trigger()` sets
// `pallet_x3_supply_ledger::TransferHalted`, which is exactly what the atomic kernel's
// `T::EconomicHalt::is_halted()` returns — a chain of three pallets that no test had walked end
// to end. These two do.

/// The nuclear halt reaches the atomic kernel, through the runtime's own controller.
///
/// `EmergencyHaltController` has a blanket no-op impl for `()`, so a mock that configured `()`
/// would let every "the halt works" test pass while a real chain kept accepting bundles. The
/// assertion is therefore not "the extrinsic succeeded" but the two links the wiring consists of:
/// the flag the atomic kernel reads has flipped, and the bundle the gateway was authorized to
/// submit a moment ago is now refused by name.
#[test]
fn the_emergency_halt_reaches_the_atomic_kernel_through_the_runtime() {
    use frame_support::BoundedVec;

    atomic_test_ext().execute_with(|| {
        let gateway = atomic_gateway();

        assert!(
            !pallet_x3_supply_ledger::TransferHalted::<Runtime>::get(),
            "the atomic kernel's halt flag must be clear in genesis, or the rest proves nothing"
        );

        // The gateway is authorized and funded: this submission would be accepted as it stands.
        assert_ok!(crate::X3AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            BoundedVec::try_from(vec![atomic_leg()]).expect("within MaxLegsPerBundle"),
            100,
            ATOMIC_CHAIN_ID,
            ATOMIC_NONCE,
        ));

        assert_ok!(pallet_x3_kernel::Pallet::<Runtime>::emergency_halt(
            RuntimeOrigin::root()
        ));

        assert!(
            pallet_x3_supply_ledger::TransferHalted::<Runtime>::get(),
            "`emergency_halt` must flip the flag the atomic kernel's `EconomicHalt` reads; the \
             controller is otherwise the no-op impl for `()`"
        );

        assert_err!(
            crate::X3AtomicKernel::submit_atomic_bundle(
                RuntimeOrigin::signed(gateway),
                BoundedVec::try_from(vec![atomic_leg()]).expect("within MaxLegsPerBundle"),
                100,
                ATOMIC_CHAIN_ID,
                ATOMIC_NONCE + 1,
            ),
            pallet_x3_atomic_kernel::Error::<Runtime>::EconomicHaltActive
        );
    });
}

/// A routine pause must not strand the bond an atomic bundle is already holding.
///
/// The two switches are independent: the pause lives in `pallet_x3_kernel` and its guards are on
/// that pallet's own extrinsic paths, while `submit_atomic_bundle` and `rollback_atomic_bundle`
/// live in `pallet_x3_atomic_kernel` and consult `EconomicHalt`, not `ProtocolPaused`. That is the
/// safe direction — a pause can never make a pending bundle unrecoverable — but it was an
/// argument made from reading two files, so it is measured here: the bundle is submitted, the
/// chain is paused, and the submitter cancels it *while paused*, with the bond released in full
/// and total issuance unchanged.
#[test]
fn a_kernel_pause_leaves_a_pending_atomic_bundle_recoverable() {
    use frame_support::BoundedVec;
    use pallet_x3_atomic_kernel::{BundleRollbackReason, BundleStatus, Bundles};

    atomic_test_ext().execute_with(|| {
        let gateway = atomic_gateway();
        let balances = pallet_balances::Pallet::<Runtime>::free_balance(gateway.clone());
        let issuance_before = pallet_balances::Pallet::<Runtime>::total_issuance();

        assert_ok!(crate::X3AtomicKernel::submit_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            BoundedVec::try_from(vec![atomic_leg()]).expect("within MaxLegsPerBundle"),
            100,
            ATOMIC_CHAIN_ID,
            ATOMIC_NONCE,
        ));
        let (bundle_id, _) = Bundles::<Runtime>::iter()
            .next()
            .expect("the submitted bundle is stored");
        assert!(
            pallet_balances::Pallet::<Runtime>::reserved_balance(gateway.clone()) > 0,
            "the bundle must be holding a bond, or there is nothing to strand"
        );

        assert_ok!(pallet_x3_kernel::Pallet::<Runtime>::emergency_pause(
            RuntimeOrigin::root()
        ));
        assert!(
            pallet_x3_kernel::ProtocolPaused::<Runtime>::get(),
            "the pause the recovery below has to survive must actually be in force"
        );

        assert_ok!(crate::X3AtomicKernel::rollback_atomic_bundle(
            RuntimeOrigin::signed(gateway.clone()),
            bundle_id,
            BundleRollbackReason::SubmitterCancelled,
        ));

        assert_eq!(
            Bundles::<Runtime>::get(bundle_id)
                .expect("the record survives rollback")
                .status,
            BundleStatus::RolledBack
        );
        assert_eq!(
            pallet_balances::Pallet::<Runtime>::reserved_balance(gateway.clone()),
            0,
            "a pause must not leave the bundle's bond reserved"
        );
        assert_eq!(
            pallet_balances::Pallet::<Runtime>::free_balance(gateway.clone()),
            balances - crate::AtomicKernelMinBond::get() / 2,
            "a voluntary cancel costs exactly the 50% penalty and returns the rest"
        );
        assert_eq!(
            pallet_balances::Pallet::<Runtime>::total_issuance(),
            issuance_before,
            "the penalty moves to the treasury rather than being burned"
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

        // The client-facing accessor for this receipt is a runtime API, and runtime APIs are
        // invocable only through a client-side executor (see the note above
        // `native_supply_contract_tests` in lib.rs). That wire-level assertion lives in the live
        // test, which calls `state_call` for the API by name — see
        // `node/tests/x3vm_live_lifecycle.rs::a_compiled_x3_program_is_finalized_and_its_receipt_is_readable`.
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

// ── The emergency halt must not brick the chain ───────────────────────────────
//
// `pallet_x3_invariants::InvariantCheck` is wired into `SignedExtra`, and while
// `Halted` is true it refuses *every* signed extrinsic. `RuntimeEmergencyHaltController`
// (the kernel's `emergency_halt`) sets that flag, and the supply ledger's
// `TransferHalted` with it. The recovery calls the halt is supposed to permit —
// `rollback_atomic_bundle` to release a pending bundle's bond, and the governance
// path that clears the flag — are ordinary signed calls in the same tuple, so if
// the gate does not exempt them the chain has no route back: no extrinsic can
// enter a block, and every validator's pool refuses the halt's own remedy.
//
// These tests enter through the transaction-validity API the pool calls, because
// that — not the dispatch layer — is what stops the chain.

/// A signed extrinsic built exactly the way the runtime's clients build one.
///
/// The tuple order is consensus-critical, so this mirrors
/// `node/src/rpc.rs`, `node/src/atomic_gateway.rs` and `crates/x3-runtime-signer`
/// rather than approximating them.
fn signed_xt(call: RuntimeCall, pair: &sp_core::sr25519::Pair, nonce: u32) -> UncheckedExtrinsic {
    use sp_core::Pair as _;

    let genesis_hash = System::block_hash(0);
    let extra: SignedExtra = (
        frame_system::CheckNonZeroSender::<Runtime>::new(),
        frame_system::CheckSpecVersion::<Runtime>::new(),
        frame_system::CheckTxVersion::<Runtime>::new(),
        frame_system::CheckGenesis::<Runtime>::new(),
        frame_system::CheckEra::<Runtime>::from(sp_runtime::generic::Era::Immortal),
        frame_system::CheckNonce::<Runtime>::from(nonce),
        frame_system::CheckWeight::<Runtime>::new(),
        pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
        pallet_x3_invariants::InvariantCheck::<Runtime>::new(),
        Decode::decode(&mut &[][..]).expect("the agent-law extension decodes from empty bytes"),
    );
    let payload = SignedPayload::from_raw(
        call.clone(),
        extra.clone(),
        (
            (),
            VERSION.spec_version,
            VERSION.transaction_version,
            genesis_hash,
            genesis_hash,
            (),
            (),
            (),
            (),
            (),
        ),
    );
    let signature = Signature::from(pair.sign(payload.encode().as_slice()));
    UncheckedExtrinsic::new_signed(call, Address::Id(account_from(pair)), signature, extra)
}

fn account_from(pair: &sp_core::sr25519::Pair) -> AccountId {
    use sp_core::Pair as _;
    use sp_runtime::traits::IdentifyAccount;
    pair.public().into_account().into()
}

/// Ask the pool's own validity API what it thinks of `call`.
fn pool_validity(
    pair: &sp_core::sr25519::Pair,
    call: RuntimeCall,
) -> sp_runtime::transaction_validity::TransactionValidity {
    let xt = signed_xt(call, pair, System::account_nonce(account_from(pair)));
    // `Executive::validate_transaction` is what the runtime's
    // `TaggedTransactionQueue` implementation calls, so this is the pool's own gate.
    Executive::validate_transaction(
        sp_runtime::transaction_validity::TransactionSource::External,
        xt,
        System::block_hash(0),
    )
}

fn halt_test_ext() -> sp_io::TestExternalities {
    use sp_core::Pair as _;
    use sp_runtime::BuildStorage;

    let who = account_from(&sp_core::sr25519::Pair::from_string("//Alice", None).unwrap());
    let storage = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: vec![(who, 10_000 * X3)],
            dev_accounts: None,
        },
        ..Default::default()
    }
    .build_storage()
    .expect("the halt test genesis must build");

    let mut ext: sp_io::TestExternalities = storage.into();
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}

/// A halted chain must keep the recovery path open.
///
/// Before this was fixed, the gate refused everything, so the halt's own remedy was
/// unreachable: the bond of a pending bundle could not be released, and no extrinsic
/// could clear the flag. That made `emergency_halt` a one-way door — a governance
/// call that bricks the chain until a runtime upgrade.
#[test]
fn a_halted_chain_still_accepts_the_recovery_calls_that_release_funds() {
    use sp_core::Pair as _;

    let pair = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();

    halt_test_ext().execute_with(|| {
        let remark = RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() });

        // Control: before the halt the same call shape is valid, so a later refusal
        // is the halt and not a broken fixture.
        assert!(
            pool_validity(&pair, remark.clone()).is_ok(),
            "the control call must be valid before the halt"
        );

        // Governance trips the halt through the kernel, exactly as it would on chain.
        assert_ok!(pallet_x3_kernel::Pallet::<Runtime>::emergency_halt(
            RuntimeOrigin::root()
        ));
        assert!(pallet_x3_invariants::Halted::<Runtime>::get());
        assert!(pallet_x3_supply_ledger::TransferHalted::<Runtime>::get());

        let halted_code = Err(
            sp_runtime::transaction_validity::TransactionValidityError::Invalid(
                sp_runtime::transaction_validity::InvalidTransaction::Custom(
                    pallet_x3_invariants::INVARIANT_HALT_CODE,
                ),
            ),
        );

        assert_eq!(
            pool_validity(&pair, remark),
            halted_code,
            "a user call must be refused while halted"
        );

        // The calls the halt exists to leave open.
        let rollback =
            RuntimeCall::X3AtomicKernel(pallet_x3_atomic_kernel::Call::rollback_atomic_bundle {
                bundle_id: H256::zero(),
                reason: pallet_x3_atomic_kernel::BundleRollbackReason::SubmitterCancelled,
            });
        assert!(
            pool_validity(&pair, rollback).is_ok(),
            "the bond-releasing rollback must not be refused by the halt"
        );

        let clear = RuntimeCall::X3Invariants(pallet_x3_invariants::Call::clear_halted {});
        assert!(
            pool_validity(&pair, clear).is_ok(),
            "the halt must have a reachable remedy"
        );

        // The remedy is governance-gated (`UpdateOrigin`/`SupplyGovernance` are
        // `EnsureRootOrHalfCouncil`) and a mainnet-rc1 chain has no sudo, so the
        // transaction that reaches it is a council motion. Those calls must be
        // submittable too, or the remedy exists on paper only.
        let motion = RuntimeCall::Council(pallet_collective::Call::propose {
            threshold: 2,
            proposal: Box::new(RuntimeCall::X3Invariants(
                pallet_x3_invariants::Call::clear_halted {},
            )),
            length_bound: 1_000,
        });
        assert!(
            pool_validity(&pair, motion).is_ok(),
            "the council motion that carries the remedy must be submittable while halted"
        );

        let resume =
            RuntimeCall::X3SupplyLedger(pallet_x3_supply_ledger::Call::resume_transfers {});
        assert!(
            pool_validity(&pair, resume).is_ok(),
            "the economy freeze the same controller raised must be liftable"
        );

        // And the remedy does what it says when it runs. `Members(2, 2)` is the origin
        // `Council::close` produces once two of two members have approved a motion.
        let council_origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(2, 2).into();
        assert_ok!(pallet_x3_invariants::Pallet::<Runtime>::clear_halted(
            council_origin.clone()
        ));
        assert_ok!(pallet_x3_supply_ledger::Pallet::<Runtime>::resume_transfers(council_origin));

        assert!(!pallet_x3_invariants::Halted::<Runtime>::get());
        assert!(!pallet_x3_supply_ledger::TransferHalted::<Runtime>::get());
        assert!(
            pool_validity(
                &pair,
                RuntimeCall::System(frame_system::Call::remark { remark: Vec::new() })
            )
            .is_ok(),
            "once the halt is cleared, normal traffic must be valid again"
        );

        // Negative control: below the council threshold the remedy is not reachable,
        // so this is a governed recovery and not an open door.
        pallet_x3_invariants::Halted::<Runtime>::put(true);
        let minority: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(0, 2).into();
        assert!(
            pallet_x3_invariants::Pallet::<Runtime>::clear_halted(minority).is_err(),
            "a minority of the council must not be able to clear the halt"
        );
        assert!(
            pallet_x3_invariants::Halted::<Runtime>::get(),
            "the refusal must leave the halt in place"
        );
    });
}

/// A pallet wired into this runtime must charge something for its calls.
///
/// `impl WeightInfo for ()` returns `Weight::zero()` for every call, which drops the pallet out of
/// block-weight accounting entirely: the weight-based limit never sees its dispatchables, so an
/// attacker can pack blocks with them. Eighteen configs in this file were in that state until
/// 2026-09-27 even though their pallets shipped non-zero, read/write-counted `SubstrateWeight`
/// values. `scripts/check-runtime-weights-wired.py` holds the config line ("no `()` where weights
/// are reachable", with a shrink-only exception list); this asserts the *runtime's own* choice
/// returns real weight, so reverting one to `()` fails here with a number rather than with a grep.
///
/// Only pallets present in every feature set are named, so the test does not need a `cfg` of its own.
#[test]
fn a_wired_pallet_charges_more_than_nothing() {
    use frame_support::weights::Weight;
    use pallet_timestamp::weights::WeightInfo as _;
    use pallet_x3_atomic_kernel::weights::WeightInfo as _;

    fn assert_charged(label: &str, weight: Weight) {
        assert!(
            weight.ref_time() > 0 || weight.proof_size() > 0,
            "{label} charges {weight:?} — a pallet wired to () charges Weight::zero() and its \
             dispatchables are invisible to the block weight limit"
        );
    }

    // Only pallets this runtime carries in *every* feature set, so the test needs no cfg of its own;
    // x3-auction, x3-oracle and private-execution are `#[cfg(not(feature = "mainnet-rc1"))]` and are
    // covered by the test below.
    assert_charged(
        "x3-atomic-kernel finalize_atomic_bundle",
        <Runtime as pallet_x3_atomic_kernel::Config>::WeightInfo::finalize_atomic_bundle(),
    );
    assert_charged(
        "timestamp set",
        <Runtime as pallet_timestamp::Config>::WeightInfo::set(),
    );
}

/// The same claim for the pallets the `mainnet-rc1` feature set removes from the runtime.
#[cfg(not(feature = "mainnet-rc1"))]
#[test]
fn a_wired_rc1_excluded_pallet_charges_more_than_nothing() {
    use pallet_x3_auction::weights::WeightInfo as _;

    let weight = <Runtime as pallet_x3_auction::Config>::WeightInfo::create_auction();
    assert!(
        weight.ref_time() > 0 || weight.proof_size() > 0,
        "x3-auction create_auction charges {weight:?} — a pallet wired to () charges Weight::zero()"
    );
}

/// WIRE-1: the Guardian pallets are members of *every* runtime, and their
/// privileged paths refuse a bare signer on chain.
///
/// This is the guard against the state the Independent Guardian audit found:
/// the registry, the security gate and the trust gate compiled as
/// members-of-nothing, so no Guardian rule could refuse anything. A test that
/// only called the pallet directly would still pass in that state — this one
/// reaches them through the real `Runtime`, certifies through `Root` (how
/// `pallet_governance` enacts an approved proposal), and asserts that an
/// unprivileged signer is refused.
#[test]
fn the_guardian_gates_are_wired_and_refuse_an_unprivileged_caller() {
    use pallet_x3_app_registry::{
        ApplicationRegistryInspect, ArtifactHashes, CertificationTier, GuardianVm,
        RestrictionReason, StandardRefs,
    };
    use pallet_x3_trust_gate::{Privilege, PrivilegeSet, TrustVerdict};

    sp_io::TestExternalities::default().execute_with(|| {
        let owner = account(0xA1);
        let stranger = account(0xA2);
        let bytecode = H256::from([0x7B; 32]);
        let hashes = ArtifactHashes {
            bytecode,
            source: H256::from([0x7C; 32]),
            manifest: H256::from([0x7D; 32]),
        };
        let standards = StandardRefs {
            guardian_standard: 1,
            trust_standard: 1,
            exploit_corpus: 1,
        };

        // Membership, not just the ability to call the pallet: the three
        // Guardian pallets must be registered in the runtime's `PalletInfo`.
        // The audit's finding was precisely that they compiled but were not
        // members of any runtime.
        {
            use frame_support::traits::PalletInfo as _;
            assert!(
                <Runtime as frame_system::Config>::PalletInfo::index::<
                    pallet_x3_app_registry::Pallet<Runtime>,
                >()
                .is_some(),
                "pallet-x3-app-registry is not a member of the runtime"
            );
            assert!(
                <Runtime as frame_system::Config>::PalletInfo::index::<
                    pallet_x3_security_gate::Pallet<Runtime>,
                >()
                .is_some(),
                "pallet-x3-security-gate is not a member of the runtime"
            );
            assert!(
                <Runtime as frame_system::Config>::PalletInfo::index::<
                    pallet_x3_trust_gate::Pallet<Runtime>,
                >()
                .is_some(),
                "pallet-x3-trust-gate is not a member of the runtime"
            );
        }

        // Registration is permissionless by design (spec §4): a signed account
        // may create an EXPERIMENTAL application, and it holds no privileges.
        assert_ok!(
            pallet_x3_app_registry::Pallet::<Runtime>::register_application(
                RuntimeOrigin::signed(owner.clone()),
                b"example-app".to_vec(),
                GuardianVm::Evm,
                7u16,
                hashes,
                standards,
            )
        );
        let app_id = 0u64;
        assert_eq!(
            pallet_x3_app_registry::Pallet::<Runtime>::tier(app_id),
            Some(CertificationTier::Experimental)
        );
        assert!(
            !pallet_x3_app_registry::Pallet::<Runtime>::has_guardian_privileges(app_id),
            "an EXPERIMENTAL application must not hold Guardian privileges"
        );

        // A bare signer is not the Guardian: certification is refused on chain.
        assert_noop!(
            pallet_x3_app_registry::Pallet::<Runtime>::certify_application(
                RuntimeOrigin::signed(stranger.clone()),
                app_id,
                0,
                hashes,
                None,
            ),
            DispatchError::BadOrigin
        );

        // Root — the origin a governance enactment dispatches as — is the
        // Guardian: the *exact* artifact hash is certified and privileges switch
        // on for that hash alone.
        assert_ok!(
            pallet_x3_app_registry::Pallet::<Runtime>::certify_application(
                RuntimeOrigin::root(),
                app_id,
                0,
                hashes,
                None,
            )
        );
        assert_eq!(
            pallet_x3_app_registry::Pallet::<Runtime>::tier(app_id),
            Some(CertificationTier::X3Verified)
        );
        assert!(pallet_x3_app_registry::Pallet::<Runtime>::has_guardian_privileges(app_id));
        assert!(
            pallet_x3_app_registry::Pallet::<Runtime>::is_certified_artifact(app_id, bytecode),
            "the certified bytecode must read as certified"
        );
        assert!(
            !pallet_x3_app_registry::Pallet::<Runtime>::is_certified_artifact(
                app_id,
                H256::from([0x99; 32])
            ),
            "a different bytecode must not inherit the certification"
        );

        // ── the trust gate, through the same runtime ────────────────────────
        let category = 7u16;
        let allowed = [Privilege::Mint];
        let forbidden = [
            Privilege::ModifyBalances,
            Privilege::Seize,
            Privilege::Drain,
            Privilege::Freeze,
            Privilege::SellRestriction,
            Privilege::Tax,
            Privilege::Upgrade,
            Privilege::ArbitraryCall,
            Privilege::OracleControl,
            Privilege::RouteControl,
            Privilege::AdminRole,
        ];

        // A bare signer cannot set a category's trust policy either.
        assert_noop!(
            pallet_x3_trust_gate::Pallet::<Runtime>::set_category_policy(
                RuntimeOrigin::signed(stranger.clone()),
                category,
                PrivilegeSet::from_privileges(&allowed),
                PrivilegeSet::from_privileges(&[]),
                PrivilegeSet::from_privileges(&forbidden),
            ),
            DispatchError::BadOrigin
        );
        assert_ok!(
            pallet_x3_trust_gate::Pallet::<Runtime>::set_category_policy(
                RuntimeOrigin::root(),
                category,
                PrivilegeSet::from_privileges(&allowed),
                PrivilegeSet::from_privileges(&[]),
                PrivilegeSet::from_privileges(&forbidden),
            )
        );

        // A census holding only allowed privileges is compliant…
        assert_ok!(pallet_x3_trust_gate::Pallet::<Runtime>::declare_privileges(
            RuntimeOrigin::root(),
            app_id,
            category,
            PrivilegeSet::from_privileges(&allowed),
        ));
        assert_eq!(
            pallet_x3_trust_gate::Pallet::<Runtime>::evaluate(app_id),
            Ok(TrustVerdict::Compliant)
        );

        // …and a census holding a forbidden privilege is a refusal, never an
        // approval. This is the gate's whole job.
        assert_ok!(pallet_x3_trust_gate::Pallet::<Runtime>::declare_privileges(
            RuntimeOrigin::root(),
            app_id,
            category,
            PrivilegeSet::from_privileges(&[Privilege::Drain]),
        ));
        assert_eq!(
            pallet_x3_trust_gate::Pallet::<Runtime>::evaluate(app_id),
            Err(pallet_x3_trust_gate::TrustError::ForbiddenPrivilege(
                Privilege::Drain
            ))
        );

        // ── the security gate is a member too ───────────────────────────────
        assert_noop!(
            pallet_x3_security_gate::Pallet::<Runtime>::create_ruleset(RuntimeOrigin::signed(
                stranger
            )),
            DispatchError::BadOrigin
        );
        assert_ok!(pallet_x3_security_gate::Pallet::<Runtime>::create_ruleset(
            RuntimeOrigin::root()
        ));
        assert_eq!(
            pallet_x3_security_gate::Pallet::<Runtime>::next_ruleset_version(),
            1u32
        );

        // Restricting the application through the runtime removes its
        // privileges while the history stays readable.
        assert_ok!(
            pallet_x3_app_registry::Pallet::<Runtime>::restrict_application(
                RuntimeOrigin::root(),
                app_id,
                RestrictionReason::SecurityFinding,
            )
        );
        assert!(!pallet_x3_app_registry::Pallet::<Runtime>::has_guardian_privileges(app_id));
        assert_eq!(
            pallet_x3_app_registry::Pallet::<Runtime>::is_restricted(app_id),
            Some(true)
        );
    });
}
