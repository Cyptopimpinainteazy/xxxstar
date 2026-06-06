//! Compiler determinism tests
//!
//! Invariants: LANG-COMPILE-001

use blake2::{Blake2s256, Digest};
use invariant_macros::invariant;
use x3_compiler::{CompilationOptions, Compiler};

#[test]
#[invariant("LANG-COMPILE-001")]
fn compile_idempotent_in_crate() {
    let source = r#"
        fn add(a: i64, b: i64) -> i64 {
            let c = a + b;
            return c;
        }

        fn main() -> i64 {
            return add(10, 20);
        }
    "#;

    let opts = CompilationOptions::opt2()
        .with_verbose(false)
        .with_emit_stats(false)
        .with_debug(false);

    let out0 = Compiler::compile(source, opts.clone()).expect("Initial compile failed");
    let bytes0 = out0.bytecode.to_bytes();
    let mut h0 = Blake2s256::new();
    h0.update(&bytes0);
    let h0 = h0.finalize();

    for i in 0..20 {
        let out = Compiler::compile(source, opts.clone()).expect("Repeated compile failed");
        let bytes = out.bytecode.to_bytes();
        let mut hasher = Blake2s256::new();
        hasher.update(&bytes);
        let h = hasher.finalize();
        assert_eq!(
            h0.as_slice(),
            h.as_slice(),
            "Bytecode hash changed on repetition {}",
            i
        );
    }
}
