//! A standalone artifact carries a signature of its own (X3-LANG-009).
//!
//! An artifact that arrives as the argument of a signed extrinsic is already bound to its sender:
//! `a_tampered_artifact_cannot_be_executed_under_the_original_signature` (the `X3-native
//! lifecycles` gate) flips a byte inside the signed bytes and the node refuses the transaction.
//! An artifact that arrives any other way — a `.x3b` copied between machines, a file from a release
//! page — had nothing of the kind. Its envelope carries a checksum, and a checksum is arithmetic:
//! a forger edits the body, recomputes it, and the artifact is self-consistent again.
//!
//! These tests compile real `.x3` source, sign the resulting artifact, and hold
//! `X3Executor::execute_attested` to refusing anything the signer did not vouch for — before the
//! bytes reach the verifier or either engine.
#![cfg(all(feature = "std", feature = "compile"))]

use sp_core::{ed25519, Pair};
use x3_common::artifact::{
    artifact_digest, ArtifactAttestation, ArtifactKeyRegistry, ArtifactKeyStatus,
    RegisteredArtifactKey,
};
use x3_common::bytecode;
use x3_x3_integration::compiler_bridge::compile_legacy_source as compile_source;
use x3_x3_integration::{X3Executor, X3ExecutorConfig, X3IntegrationError};

const SOURCE: &str = "fn main() -> i64 { let mut total = 0; let mut i = 0; \
                      while i < 7 { total = total + i; i = i + 1; } return total; }";
/// 0+1+2+3+4+5+6, as the source computes it.
const EXPECTED: i64 = 21;

fn pair(seed: u8) -> ed25519::Pair {
    ed25519::Pair::from_seed(&[seed; 32])
}

fn hex_of(pair: &ed25519::Pair) -> String {
    pair.public().0.iter().map(|b| format!("{b:02x}")).collect()
}

fn registry(entries: &[(&str, &ed25519::Pair, ArtifactKeyStatus)]) -> ArtifactKeyRegistry {
    ArtifactKeyRegistry::from_entries(
        entries
            .iter()
            .map(|(key_id, pair, status)| RegisteredArtifactKey {
                key_id: key_id.to_string(),
                public_key: hex_of(pair),
                status: *status,
            })
            .collect(),
    )
    .expect("the registry is well formed")
}

fn artifact() -> Vec<u8> {
    compile_source(SOURCE).expect("the fixture source compiles")
}

fn returned_value(receipt: &x3_x3_integration::X3ExecutionReceipt) -> i64 {
    i64::from_le_bytes(
        receipt
            .return_data
            .as_slice()
            .try_into()
            .expect("the program returns an i64"),
    )
}

#[test]
fn a_signed_artifact_runs_and_returns_what_its_source_computes() {
    let signer = pair(7);
    let registry = registry(&[("release", &signer, ArtifactKeyStatus::Active)]);
    let bytes = artifact();
    let attestation = ArtifactAttestation::sign(&bytes, "release", &signer);

    let receipt = X3Executor::execute_attested(
        &bytes,
        &attestation,
        &registry,
        &[],
        X3ExecutorConfig::on_chain(),
    )
    .expect("an attested artifact executes");
    assert!(receipt.success);
    assert_eq!(returned_value(&receipt), EXPECTED);
}

/// The gap, measured: a forged artifact whose checksum has been repaired is indistinguishable from
/// a genuine one to every check the format itself offers, and is refused here.
#[test]
fn a_forged_artifact_with_a_repaired_checksum_is_refused_before_it_runs() {
    let signer = pair(7);
    let registry = registry(&[("release", &signer, ArtifactKeyStatus::Active)]);
    let bytes = artifact();
    let attestation = ArtifactAttestation::sign(&bytes, "release", &signer);

    // Flip a byte of the body and repair the envelope's own integrity field, which needs no key.
    let mut forged = bytes.clone();
    let last = forged.len() - 1;
    forged[last] ^= 0x01;
    let repaired = bytecode::checksum(&forged[bytecode::HEADER_LEN..]).to_le_bytes();
    forged[bytecode::CHECKSUM_OFFSET..bytecode::CHECKSUM_OFFSET + 4].copy_from_slice(&repaired);
    assert_ne!(forged, bytes, "the forgery changed the artifact");
    assert_eq!(
        bytecode::checksum(&forged[bytecode::HEADER_LEN..]).to_le_bytes(),
        forged[bytecode::CHECKSUM_OFFSET..bytecode::CHECKSUM_OFFSET + 4],
        "and is self-consistent by the envelope's own checksum"
    );

    let refusal = X3Executor::execute_attested(
        &forged,
        &attestation,
        &registry,
        &[],
        X3ExecutorConfig::on_chain(),
    )
    .expect_err("the signer did not vouch for these bytes");
    assert!(
        matches!(refusal, X3IntegrationError::UnattestedArtifact(_)),
        "{refusal:?}"
    );
}

/// The attestation is checked before the artifact is parsed, so bytes nobody vouched for are never
/// handed to the verifier or an engine: a payload that is not a module at all is refused as
/// unattested, not as malformed.
#[test]
fn an_unattested_payload_is_refused_without_being_parsed() {
    let signer = pair(7);
    let registry = registry(&[("release", &signer, ArtifactKeyStatus::Active)]);
    let bytes = artifact();
    let attestation = ArtifactAttestation::sign(&bytes, "release", &signer);

    let not_a_module = b"this is not an X3BC module at all".to_vec();
    assert!(
        X3Executor::execute(&not_a_module, &[], X3ExecutorConfig::on_chain()).is_err(),
        "the control: the executor refuses it on its own too"
    );
    let refusal = X3Executor::execute_attested(
        &not_a_module,
        &attestation,
        &registry,
        &[],
        X3ExecutorConfig::on_chain(),
    )
    .expect_err("unattested bytes are not executed");
    assert!(
        matches!(refusal, X3IntegrationError::UnattestedArtifact(_)),
        "the refusal names the attestation, not the format: {refusal:?}"
    );
}

/// Rotation and revocation, on a real artifact: the key that signed last year's release keeps
/// verifying it, until it is revoked.
#[test]
fn rotating_a_signing_key_keeps_old_artifacts_runnable_and_revoking_it_does_not() {
    let old = pair(7);
    let new = pair(9);
    let bytes = artifact();
    let attestation = ArtifactAttestation::sign(&bytes, "release-2025", &old);
    let run = |registry: &ArtifactKeyRegistry| {
        X3Executor::execute_attested(
            &bytes,
            &attestation,
            registry,
            &[],
            X3ExecutorConfig::on_chain(),
        )
    };

    let rotated = registry(&[
        ("release-2025", &old, ArtifactKeyStatus::Retired),
        ("release-2026", &new, ArtifactKeyStatus::Active),
    ]);
    let receipt = run(&rotated).expect("a retired key's artifact still runs");
    assert_eq!(returned_value(&receipt), EXPECTED);
    assert!(!rotated.may_sign("release-2025"));

    let revoked = registry(&[
        ("release-2025", &old, ArtifactKeyStatus::Revoked),
        ("release-2026", &new, ArtifactKeyStatus::Active),
    ]);
    let refusal = run(&revoked).expect_err("a revoked key's artifacts do not run");
    assert!(
        format!("{refusal}").contains("revoked"),
        "the refusal says why: {refusal}"
    );
}

/// A signature from a key the caller's registry does not list is not trust, however well formed it
/// is: the attacker signs their own forgery with their own key.
#[test]
fn an_artifact_signed_by_a_stranger_does_not_run() {
    let signer = pair(7);
    let stranger = pair(11);
    let registry = registry(&[("release", &signer, ArtifactKeyStatus::Active)]);
    let bytes = artifact();

    for (name, attestation) in [
        (
            "an id nobody lists",
            ArtifactAttestation::sign(&bytes, "some-other-signer", &stranger),
        ),
        (
            "the listed id under the wrong key",
            ArtifactAttestation::sign(&bytes, "release", &stranger),
        ),
    ] {
        // The attestation is internally consistent: it is about these exact bytes.
        assert_eq!(
            attestation.artifact_digest,
            artifact_digest(&bytes),
            "{name}"
        );
        let refusal = X3Executor::execute_attested(
            &bytes,
            &attestation,
            &registry,
            &[],
            X3ExecutorConfig::on_chain(),
        )
        .expect_err(name);
        assert!(
            matches!(refusal, X3IntegrationError::UnattestedArtifact(_)),
            "{name}: {refusal:?}"
        );
    }
}

/// An attestation is a JSON sidecar in practice, so it has to survive the round trip that carrying
/// it beside the artifact means.
#[test]
fn an_attestation_round_trips_through_json() {
    let signer = pair(7);
    let registry = registry(&[("release", &signer, ArtifactKeyStatus::Active)]);
    let bytes = artifact();
    let attestation = ArtifactAttestation::sign(&bytes, "release", &signer);

    let sidecar = serde_json::to_string(&attestation).expect("an attestation serializes");
    let parsed: ArtifactAttestation = serde_json::from_str(&sidecar).expect("and parses back");
    assert_eq!(parsed, attestation);
    X3Executor::execute_attested(
        &bytes,
        &parsed,
        &registry,
        &[],
        X3ExecutorConfig::on_chain(),
    )
    .expect("the parsed attestation verifies exactly as the original did");
}
