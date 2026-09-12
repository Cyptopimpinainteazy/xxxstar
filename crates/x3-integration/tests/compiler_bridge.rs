//! Compiler-to-runtime boundary tests.
//!
//! These tests require the real workspace compiler. No fake or empty bytecode is
//! accepted by the integration layer.

#![cfg(all(feature = "std", feature = "compile"))]

use x3_backend::BytecodeModule;
use x3_x3_integration::compiler_bridge::compile_source;

#[test]
fn compile_source_emits_runtime_loadable_bytecode() {
    let source = r#"
        fn main() -> i64 {
            return 1;
        }
    "#;

    let bytes = compile_source(source).expect("valid .x3 source must compile");
    assert!(bytes.starts_with(b"X3BC"), "missing canonical X3 bytecode magic");

    let module = BytecodeModule::from_bytes(&bytes)
        .expect("compiler output must be accepted by the runtime bytecode loader");
    assert!(!module.code.is_empty(), "compiled module must contain instructions");
}

#[test]
fn compile_source_rejects_invalid_source() {
    let error = compile_source("this is not valid x3 source")
        .expect_err("invalid source must fail closed");
    assert!(
        error.to_string().contains("compil"),
        "error should identify the compilation boundary: {error}"
    );
}
