//! The validator profile uses the same codec, verifier and trading engine as tooling.
use x3_lang_compiler::{compile_with_mode, CompilationMode};
use x3_lang_execution::{
    artifact::{decode_artifact, decode_trading_operation},
    spec::opcodes::*,
};

fn artifact() -> Vec<u8> {
    compile_with_mode(
        include_str!("../../examples/native_trading.x3"),
        CompilationMode::Testnet,
    )
    .unwrap()
}
#[test]
fn canonical_compilation_is_accepted_with_the_shared_gas_schedule() {
    let bytes = artifact();
    let decoded = decode_artifact(&bytes).unwrap();
    assert_eq!(decoded.operations.len(), 9);
    assert_eq!(
        decoded.gas_used,
        bytes.len() as u64 + 9 * base_gas_cost(TRADING_BEGIN) as u64
    );
    assert_eq!(
        x3_lang_compiler::emitter::decode_trading_program(&bytes).unwrap(),
        decoded.operations
    );
}
#[test]
fn every_truncated_artifact_is_refused() {
    let bytes = artifact();
    for length in 0..bytes.len() {
        assert!(
            decode_artifact(&bytes[..length]).is_err(),
            "accepted prefix of {length} bytes"
        );
    }
}
#[test]
fn all_version_bindings_and_future_bytecode_versions_fail_closed() {
    let bytes = artifact();
    for offset in [0, 2, 4, 6, 8, 10] {
        let mut changed = bytes.clone();
        changed[offset] = 255;
        assert!(
            decode_artifact(&changed).is_err(),
            "accepted changed version at {offset}"
        );
    }
}
#[test]
fn unsupported_operations_are_rejected_instead_of_skipped() {
    let mut bytes = artifact();
    bytes.extend_from_slice(&[REQUIRE, 0, 1, 0]);
    assert!(decode_artifact(&bytes).unwrap_err().contains("unsupported"));
}
#[test]
fn json_payload_cannot_impersonate_another_opcode() {
    assert!(decode_trading_operation(TRADING_COMMIT, br#""AssertAllDebtsClosed""#).is_err());
}
#[test]
fn compiled_private_policy_cannot_lose_its_executable_gate() {
    let source = include_str!("../../examples/native_trading.x3")
        .replace("require_private_submission: false", "require_private_submission: true");
    let mut bytes = compile_with_mode(&source, CompilationMode::Testnet).unwrap();
    let decoded = decode_artifact(&bytes).unwrap();
    assert!(decoded.requires_private_submission);
    // Remove precisely the MODE_CHECK record, preserving instruction alignment.
    let pc = 1 + VERSIONS_RECORD_LEN;
    assert_eq!(bytes[pc], MODE_CHECK);
    let len = usize::from(u16::from_le_bytes([bytes[pc + 1], bytes[pc + 2]]));
    let end = (pc + 3 + len + 3) & !3;
    bytes.drain(pc..end);
    assert!(decode_artifact(&bytes).unwrap_err().contains("private"));
}

#[test]
fn unknown_policy_fields_are_refused_instead_of_lost() {
    let bytes = artifact();
    let artifact = decode_artifact(&bytes).unwrap();
    let mut operation = serde_json::to_value(&artifact.operations[0]).unwrap();
    operation["BeginAtomicTrade"]["policy"]["future_security_requirement"] = serde_json::json!(true);
    let payload = serde_json::to_vec(&operation).unwrap();
    assert!(x3_lang_execution::artifact::decode_trading_operation(TRADING_BEGIN, &payload).is_err());
}
