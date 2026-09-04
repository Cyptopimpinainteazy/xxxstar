//! Tests for deterministic compilation
//!
//! Invariants: LANG-COMPILE-001
//!
//! This test compiles a representative X3 program multiple times and asserts
//! that the compiled bytecode hash remains identical across repeated compilations.

use blake2::{Blake2s256, Digest};
use invariant_macros::invariant;
use x3_compiler::{CompilationOptions, Compiler};

#[test]
#[invariant("LANG-COMPILE-001")]
fn compile_idempotent() {
    // Representative source covering function, arithmetic and return
    let source = r#"
        fn add(a: i64, b: i64) -> i64 {
            let c = a + b;
            return c;
        }

        fn main() -> i64 {
            return add(10, 20);
        }
    "#;

    let opts = CompilationOptions::opt2().with_verbose(false).with_emit_stats(false).with_debug(false);

    // First compile
    let out0 = Compiler::compile(source, opts.clone()).expect("Initial compile failed");
    let bytes0 = out0.bytecode.to_bytes();
    let mut hasher = Blake2s256::new();
    hasher.update(&bytes0);
    let h0 = hasher.finalize();

    // Repeat compiles
    for i in 0..50 {
        let out = Compiler::compile(source, opts.clone()).expect("Repeated compile failed");
        let bytes = out.bytecode.to_bytes();
        let mut hasher = Blake2s256::new();
        hasher.update(&bytes);
        let h = hasher.finalize();
        assert_eq!(h0.as_slice(), h.as_slice(), "Bytecode hash changed on repetition {}", i);
    }
}
