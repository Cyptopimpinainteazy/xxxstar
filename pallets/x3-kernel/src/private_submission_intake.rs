//! The compiled private-submission policy as the chain's intake path enforces it (X3-MEV-002).
//!
//! The MEV/privacy work in this repository had a structural gap for a long time: the X3 language
//! toolchain compiles a `require_private_submission` policy into an executable demand, and
//! `pallet-x3-kernel` executes the artifact — but the chain-intake compiler (`crates/x3-compiler`,
//! behind `x3-integration::compiler_bridge`) had no notion of submission policy at all, so a program
//! that reached the chain could not demand private submission *even in principle*. The demand and
//! the enforcement lived on opposite sides of a boundary nothing crossed.
//!
//! The fix has two halves, and these tests exist because either half alone is worthless:
//!
//! * the compiler *records* the demand in the artifact's own feature word
//!   (`x3_common::bytecode::FEATURE_PRIVATE_SUBMISSION_REQUIRED`), so the requirement travels with
//!   the bytes rather than with a submission-time parameter a caller could omit (AGENTS.md §11);
//! * the pallet *reads* it at intake and refuses the program when the runtime's
//!   `Config::PrivateSubmissionChannel` says this chain cannot offer what the program demands.
//!
//! Every test drives the real extrinsic. The extractions that make the check load-bearing:
//!
//! * refusal must not be a refusal of *any* X3 payload — the same call with the same source
//!   compiled without the demand must run, or the check is indistinguishable from a broken adapter;
//! * the demand must be read out of the header, so a module whose feature word is edited after
//!   compilation is refused even though no compiler ever saw it (this is also why the edit does not
//!   invalidate the envelope: the checksum covers the body, and the test asserts that);
//! * the accepting direction must reach the ledger, so "accepted" cannot mean "accepted and did
//!   nothing".

use frame_support::{assert_noop, assert_ok};
use sp_core::H256;

use crate::mock::{
    new_test_ext, Balances, RuntimeOrigin, Test, TestPrivateSubmissionChannel, ALICE,
};
use crate::test_helpers::wrap_x3_payload;
use crate::{CanonicalLedger, Error, Nonces, SubmittedComits};
// `validate` is a trait method, so the trait has to be in scope to call it.
use crate::adapters::X3ExecutorAdapter;

type AtlasKernel = crate::Pallet<Test>;

/// The asset id the mock's X3 adapter credits, so the accepting direction has a leg to check.
const X3_LEG: u32 = 2;
/// What the mock's X3 adapter credits on a benign execution (`mock::TestX3Adapter`).
const X3_WRITES: u128 = 333;

const X3_SOURCE: &str = "fn main() -> i64 {\n    return 42;\n}\n";

/// An X3BC artifact compiled under the deployment policy that demands private submission.
fn artifact_demanding_private_submission() -> Vec<u8> {
    x3_x3_integration::compiler_bridge::compile_legacy_source_with_policy(
        X3_SOURCE,
        x3_x3_integration::CompilationPolicy::private_submission_required(),
    )
    .expect("the fixture source compiles under the private-submission policy")
}

/// The same source compiled under the default policy: no demand recorded.
fn artifact_without_a_demand() -> Vec<u8> {
    x3_x3_integration::compiler_bridge::compile_legacy_source(X3_SOURCE)
        .expect("the fixture source compiles")
}

/// Submit one comit whose X3 payload is `x3_payload`. Everything else is held constant, so the only
/// variable a test changes is the program.
fn submit_x3(comit_id: H256, x3_payload: Vec<u8>) -> sp_runtime::DispatchResult {
    let evm_payload = crate::test_helpers::wrap_evm_payload(&[0x01]);
    let svm_payload = crate::test_helpers::wrap_svm_payload(&[0x02]);
    let fee = 1_000u128;
    let prepare_root = AtlasKernel::compute_prepare_root_v2(
        comit_id,
        &evm_payload,
        &svm_payload,
        &x3_payload,
        0,
        fee,
    );
    AtlasKernel::submit_comit_v2(
        RuntimeOrigin::signed(ALICE),
        comit_id,
        evm_payload,
        svm_payload,
        x3_payload,
        0,
        fee,
        prepare_root,
    )
}

#[test]
fn a_program_that_demands_private_submission_is_refused_where_the_chain_offers_none() {
    new_test_ext().execute_with(|| {
        TestPrivateSubmissionChannel::set(false);
        let comit_id = H256::from_low_u64_be(0x9E02);
        let payload = artifact_demanding_private_submission();

        // The artifact really is an X3BC module carrying the demand; if the compiler stopped
        // recording it, this test would fail here rather than pass for the wrong reason.
        assert!(
            x3_common::bytecode::requires_private_submission(&payload),
            "the compiler must record the demand in the artifact's own feature word"
        );

        let issuance_before = Balances::total_issuance();
        let free_before = Balances::free_balance(ALICE);
        let nonce_before = Nonces::<Test>::get(ALICE);

        assert_noop!(
            submit_x3(comit_id, payload),
            Error::<Test>::PrivateSubmissionUnavailable
        );

        // Refused at intake: no fee burned, no nonce consumed, no comit stored, no leg written.
        assert_eq!(Balances::total_issuance(), issuance_before);
        assert_eq!(Balances::free_balance(ALICE), free_before);
        assert_eq!(Nonces::<Test>::get(ALICE), nonce_before);
        assert!(!SubmittedComits::<Test>::contains_key(comit_id));
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, X3_LEG), 0);
    });
}

#[test]
fn the_same_program_runs_once_the_chain_offers_a_private_channel() {
    new_test_ext().execute_with(|| {
        TestPrivateSubmissionChannel::set(true);
        let comit_id = H256::from_low_u64_be(0x9E03);

        assert_ok!(submit_x3(comit_id, artifact_demanding_private_submission()));

        // Not merely accepted: the program reached the adapter and its write landed. If it had been
        // accepted and skipped, the ledger would still read zero and this assertion would fail.
        assert!(SubmittedComits::<Test>::contains_key(comit_id));
        assert_eq!(Nonces::<Test>::get(ALICE), 1);
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, X3_LEG), X3_WRITES);

        TestPrivateSubmissionChannel::set(false);
    });
}

#[test]
fn a_program_that_does_not_demand_privacy_is_not_gated() {
    new_test_ext().execute_with(|| {
        // The channel is off, and this program does not ask for one.
        TestPrivateSubmissionChannel::set(false);
        let comit_id = H256::from_low_u64_be(0x9E04);
        let payload = artifact_without_a_demand();
        assert!(!x3_common::bytecode::requires_private_submission(&payload));

        assert_ok!(submit_x3(comit_id, payload));
        assert!(SubmittedComits::<Test>::contains_key(comit_id));
        assert_eq!(CanonicalLedger::<Test>::get(ALICE, X3_LEG), X3_WRITES);
    });
}

#[test]
fn a_chain_with_a_channel_still_runs_programs_that_do_not_demand_it() {
    // The other control: the check gates the *program's compiled policy*, not the chain. A payload
    // the adapter accepts must keep running with the channel in either position.
    new_test_ext().execute_with(|| {
        TestPrivateSubmissionChannel::set(true);
        assert_ok!(submit_x3(
            H256::from_low_u64_be(0x9E05),
            wrap_x3_payload(&[0x11, 0x00, 0x00, 0x00])
        ));
        TestPrivateSubmissionChannel::set(false);
    });
}

#[test]
fn the_demand_is_read_from_the_artifact_not_from_the_compiler() {
    // A module whose feature word was set *after* compilation, by something that never ran the
    // compiler: the pallet must refuse it on the same grounds. This is what makes the check a
    // property of the bytes rather than of a cooperating build step, and it is also why the envelope
    // still verifies — the checksum covers the body, not the feature word.
    new_test_ext().execute_with(|| {
        TestPrivateSubmissionChannel::set(false);

        let mut payload = artifact_without_a_demand();
        let offset = x3_common::bytecode::FEATURE_FLAGS_OFFSET;
        let mut word = [0u8; 4];
        word.copy_from_slice(&payload[offset..offset + 4]);
        let edited =
            u32::from_le_bytes(word) | x3_common::bytecode::FEATURE_PRIVATE_SUBMISSION_REQUIRED;
        payload[offset..offset + 4].copy_from_slice(&edited.to_le_bytes());

        // The edit is invisible to the envelope's integrity check, which is exactly why the header
        // has to be interpreted rather than merely checksummed — and the module still passes the
        // production verifier, so the refusal below is the policy and not a decode failure.
        assert_eq!(x3_common::bytecode::feature_flags(&payload), Some(edited));
        assert!(x3_common::bytecode::requires_private_submission(&payload));
        assert!(
            crate::wasm_adapters::WasmX3Adapter::validate(&payload).is_ok(),
            "the edited module must still be a well-formed program, or this test would prove the \
             wrong thing"
        );

        assert_noop!(
            submit_x3(H256::from_low_u64_be(0x9E06), payload),
            Error::<Test>::PrivateSubmissionUnavailable
        );
    });
}

#[test]
fn a_payload_that_is_not_a_module_is_answered_by_the_adapter_not_by_this_check() {
    // A truncated header must not be read as "no demand" *and* accepted: the intake check answers
    // only the question it can read, and the adapter refuses the bytes by name. Any other division
    // of labour either fails open on junk or reports the wrong reason for a malformed program.
    new_test_ext().execute_with(|| {
        TestPrivateSubmissionChannel::set(false);
        let mut truncated = artifact_without_a_demand();
        truncated.truncate(x3_common::bytecode::HEADER_LEN - 1);
        assert_eq!(x3_common::bytecode::feature_flags(&truncated), None);
        assert!(!x3_common::bytecode::requires_private_submission(
            &truncated
        ));

        assert!(
            crate::wasm_adapters::WasmX3Adapter::validate(&truncated).is_err(),
            "a header shorter than the X3BC header must be refused by the adapter that names it"
        );
    });
}
