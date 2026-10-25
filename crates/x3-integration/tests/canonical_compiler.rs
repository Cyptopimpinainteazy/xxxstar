#![cfg(all(feature = "std", feature = "compile"))]

use x3_x3_integration::compiler_bridge::compile_source;

#[test]
fn production_bridge_emits_exactly_the_canonical_compilers_artifact() {
    let source = include_str!("../../../x3-lang/examples/trading_core_v1.x3");
    let expected =
        x3_lang_compiler::compile_with_mode(source, x3_lang_compiler::CompilationMode::Testnet)
            .expect("canonical trading source must compile");
    let actual = compile_source(source).expect("production bridge must compile canonical source");
    assert_eq!(actual, expected);
}

#[test]
fn bridge_preserves_the_explicit_mode_and_binds_provenance() {
    use x3_x3_integration::compiler_bridge::{compile_artifact, compile_source_with_mode};
    let source = include_str!("../../../x3-lang/examples/native_trading.x3");
    for mode in [
        x3_lang_compiler::CompilationMode::Dev,
        x3_lang_compiler::CompilationMode::Testnet,
        x3_lang_compiler::CompilationMode::Mainnet,
    ] {
        let direct = x3_lang_compiler::compile_with_mode(source, mode).map_err(|e| e.to_string());
        let bridged = compile_source_with_mode(source, mode);
        match direct {
            Ok(expected) => {
                assert_eq!(bridged.unwrap(), expected);
                let result = compile_artifact(source, mode).unwrap();
                assert_eq!(result.mode, mode);
                assert_eq!(
                    result.source_hash,
                    sp_io::hashing::sha2_256(source.as_bytes())
                );
                assert_eq!(result.artifact_hash, sp_io::hashing::sha2_256(&expected));
                assert_eq!(result.bytecode, expected);
            }
            Err(_) => assert!(bridged.is_err(), "bridge bypassed {mode:?} rejection"),
        }
    }
}

#[test]
fn deployment_requirement_cannot_silently_replace_the_compiled_source_policy() {
    use x3_x3_integration::{compiler_bridge::compile_source_with_policy, CompilationPolicy};
    let source = include_str!("../../../x3-lang/examples/native_trading.x3");
    assert!(
        compile_source_with_policy(source, CompilationPolicy::private_submission_required())
            .is_err()
    );
    let private = source.replace(
        "require_private_submission: false",
        "require_private_submission: true",
    );
    let bytes =
        compile_source_with_policy(&private, CompilationPolicy::private_submission_required())
            .unwrap();
    assert!(
        x3_x3_integration::canonical::decode_artifact(&bytes)
            .unwrap()
            .requires_private_submission
    );
}

#[test]
fn canonical_verification_requires_a_host_to_execute_and_refuses_bad_artifacts() {
    let source = include_str!("../../../x3-lang/examples/native_trading.x3");
    let bytes = compile_source(source).unwrap();
    x3_x3_integration::X3Executor::verify_on_chain(&bytes).unwrap();
    assert!(x3_x3_integration::X3Executor::execute_on_chain(&bytes, 100_000, &[], false).is_err());
    let mut changed = bytes;
    changed[2] = 255;
    assert!(x3_x3_integration::X3Executor::verify_on_chain(&changed).is_err());
    assert!(compile_source("not a valid .x3 program").is_err());
}
