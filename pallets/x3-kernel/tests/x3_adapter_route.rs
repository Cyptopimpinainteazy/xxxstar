//! A compiled `.x3` program, driven through the adapter the kernel is configured with in production.
//!
//! The pallet's own tests configure `TestX3Adapter` in `mock.rs`, which fabricates a receipt — fine
//! for the pallet's logic, and no evidence that a compiled program reaches the kernel's execution
//! route. X3-LANG-004's row asked for this test by name: "No test drives a compiled program's call
//! into the atomic kernel: the bridge proves compile → runtime-loadable bytecode, and the route from
//! there to `X3AtomicKernel` is the open half". What follows closes that half at the adapter
//! boundary: the artifact is real (compiled from source here), and the receipt is the kernel's own
//! `ExecutionReceipt` type with its version stamp.
#![cfg(feature = "std")]

use pallet_x3_kernel::X3VmAdapter;
use pallet_x3_kernel::{X3ExecutorAdapter, EXECUTION_RECEIPT_VERSION};

fn compile(source: &str) -> Vec<u8> {
    x3_x3_integration::compiler_bridge::compile_source(source).expect("valid source must compile")
}

#[test]
fn a_compiled_program_is_executed_by_the_production_adapter() {
    // Two programs that return different values: one cannot tell "the receipt reports the program's
    // value" from "the receipt reports a constant".
    for expected in [42i64, 7i64] {
        let source = format!("fn main() -> i64 {{\n    return {expected};\n}}\n");
        let bytes = compile(&source);

        // What the kernel checks before it executes anything.
        X3VmAdapter::validate(&bytes).expect("the kernel's validate must accept a compiled module");
        let estimate =
            X3VmAdapter::estimate_gas(&bytes).expect("the kernel must be able to price it");
        assert!(estimate > 0, "an executable module costs something");

        let receipt = X3VmAdapter::execute(&bytes, 1_000_000)
            .expect("the production adapter must execute a compiled module");
        assert!(
            receipt.success,
            "the program returns, so the kernel's receipt must succeed"
        );
        assert_eq!(
            receipt.return_data,
            expected.to_le_bytes().to_vec(),
            "the kernel's receipt must carry the value the source states"
        );
        assert!(
            receipt.gas_used > 0,
            "the kernel's receipt must report metered gas"
        );
        assert_eq!(
            receipt.version, EXECUTION_RECEIPT_VERSION,
            "and it must be stamped with the kernel's own receipt version"
        );
    }
}

#[test]
fn the_production_adapter_refuses_a_corrupted_module() {
    let mut bytes = compile("fn main() -> i64 {\n    return 5;\n}\n");
    // Flip a byte in the body (past the header) — the checksum covers it, so the kernel must refuse
    // rather than execute a program whose bytes were edited in transit.
    bytes[30] ^= 0xFF;
    assert!(
        X3VmAdapter::validate(&bytes).is_err(),
        "validate must refuse a corrupted module"
    );
    assert!(
        X3VmAdapter::execute(&bytes, 1_000_000).is_err(),
        "and execute must not run it"
    );
}
