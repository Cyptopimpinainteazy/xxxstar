//! Compiler-to-runtime boundary tests.
//!
//! These tests require the real workspace compiler. No fake or empty bytecode is
//! accepted by the integration layer.

#![cfg(all(feature = "std", feature = "compile"))]

use x3_backend::BytecodeModule;
use x3_x3_integration::compiler_bridge::compile_source;
use x3_x3_integration::mini_x3::{self, MiniValue};
use x3_x3_integration::{X3Executor, X3ExecutorConfig};

#[test]
fn compile_source_emits_runtime_loadable_bytecode() {
    let source = r#"
        fn main() -> i64 {
            return 1;
        }
    "#;

    let bytes = compile_source(source).expect("valid .x3 source must compile");
    assert!(
        bytes.starts_with(b"X3BC"),
        "missing canonical X3 bytecode magic"
    );

    let module = BytecodeModule::from_bytes(&bytes)
        .expect("compiler output must be accepted by the runtime bytecode loader");
    assert!(
        !module.code.is_empty(),
        "compiled module must contain instructions"
    );
}

#[test]
fn compile_source_rejects_invalid_source() {
    let error =
        compile_source("this is not valid x3 source").expect_err("invalid source must fail closed");
    // Case-insensitive: the message is `Compilation failed: Parser(...)`, and the assertion used to
    // look for the lowercase "compil" — so this test could not pass, whatever the compiler did.
    assert!(
        error.to_string().to_lowercase().contains("compil"),
        "error should identify the compilation boundary: {error}"
    );
}

/// The whole chain in one test: `.x3` source -> compiler -> X3BC envelope -> **both** of the
/// runtime's readers -> execution -> a receipt whose figures follow from the program.
///
/// This is the test `X3-LANG-001`'s row records as missing ("Genuine E2E test still required"). The
/// tests before it prove the links separately — `compile_source_emits_runtime_loadable_bytecode`
/// stops at "the loader accepts the bytes", and the executor's own tests start from bytecode they
/// built — so nothing exercised the seam where a compiled program becomes an executed receipt.
///
/// Two programs returning different values, because one program cannot tell "the receipt reports
/// what the program returned" from "the receipt reports a constant": the assertion is on the value
/// the source states, in both engines.
#[test]
fn compile_encode_decode_execute_receipt_end_to_end() {
    for expected in [42i64, 7i64] {
        let source = format!("fn main() -> i64 {{\n    return {expected};\n}}\n");

        // compile -> the canonical envelope.
        let bytes = compile_source(&source).expect("valid .x3 source must compile");
        assert!(
            bytes.starts_with(b"X3BC"),
            "the artifact must carry the canonical X3BC magic, not a private framing"
        );

        // decode -> two independent readers of the same format. `BytecodeModule` is the std reader;
        // `mini_x3` is the no-std reader the runtime adapter uses, and it reads the version,
        // checksum and min-version header rather than skipping them.
        let module = BytecodeModule::from_bytes(&bytes).expect("the std reader must accept it");
        assert!(
            !module.code.is_empty(),
            "a compiled module must contain instructions"
        );
        mini_x3::validate_x3bc(&bytes).expect("the runtime's no-std reader must accept it");

        // execute -> the on-chain executor, then the kernel-side one, on the same artifact.
        let receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
            .expect("a program that returns must execute on the on-chain path");
        assert!(
            receipt.success,
            "the program returns, so the execution succeeds"
        );
        assert!(receipt.gas_used > 0, "execution must be metered, not free");
        assert!(
            receipt.instructions_executed > 0,
            "the receipt must report the work the VM did"
        );
        assert_eq!(
            receipt.return_data,
            expected.to_le_bytes().to_vec(),
            "the receipt must report what the program returned, not a constant"
        );

        let kernel = mini_x3::execute_x3bc(&bytes, 100_000).expect("kernel-side execution");
        assert_eq!(
            kernel.return_val,
            MiniValue::I64(expected),
            "the kernel-side engine must agree with the source and with the std executor"
        );
        assert!(
            kernel.gas_used > 0,
            "the kernel-side execution is metered too"
        );
    }
}

/// The same chain, over the shapes a compiler has to emit differently, because one program cannot
/// show that the *encodings* agree — a register, a constant-pool index, a jump target and a call
/// each put a different kind of operand in the stream, and the emitter and the verifier disagree
/// per operand kind, not wholesale.
///
/// Each case is a program the source states the answer to, so a wrong answer is a failure and not a
/// crash: this is the test that would have caught the two-byte register operand at the first
/// shape that used a register (`return 1 + 2`), where the first test caught it only for a bare
/// literal (TICKET-130).
#[test]
fn every_operand_kind_compiles_verifies_and_executes() {
    // (source, expected i64 result)
    let cases: &[(&str, i64)] = &[
        ("fn main() -> i64 {\n    return 42;\n}\n", 42),
        ("fn main() -> i64 {\n    return 1 + 2;\n}\n", 3),
        ("fn main() -> i64 {\n    return 10 - 4;\n}\n", 6),
        ("fn main() -> i64 {\n    let x = 7;\n    return x;\n}\n", 7),
        ("fn main() -> i64 {\n    if 1 < 2 {\n        return 3;\n    }\n    return 4;\n}\n", 3),
        ("fn main() -> i64 {\n    return 1000;\n}\n", 1000),
        (
            "fn add(a: i64, b: i64) -> i64 {\n    return a + b;\n}\n\nfn main() -> i64 {\n    return add(2, 3);\n}\n",
            5,
        ),
    ];

    for (source, expected) in cases {
        let bytes = compile_source(source)
            .unwrap_or_else(|error| panic!("must compile: {error}\n--- source ---\n{source}"));
        mini_x3::validate_x3bc(&bytes).unwrap_or_else(|error| {
            panic!("the runtime's reader must accept what the compiler emitted: {error:?}\n--- source ---\n{source}")
        });
        let receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
            .unwrap_or_else(|error| {
                panic!("must verify and execute: {error:?}\n--- source ---\n{source}")
            });
        assert!(
            receipt.success,
            "the program returns, so the execution succeeds; the VM said: {}\n--- source ---\n{source}",
            String::from_utf8_lossy(&receipt.return_data)
        );
        assert_eq!(
            receipt.return_data,
            expected.to_le_bytes().to_vec(),
            "the receipt must report the value the source states: {source}"
        );
        assert!(
            receipt.instructions_executed > 0,
            "a receipt that reports no instructions did not count them: {source}"
        );
    }
}

/// The same chain over the compiler's own fixture corpus: recursion, several functions with
/// parameters, chained comparisons and a constant-folded branch.
///
/// The fixtures under `crates/x3-compiler/tests/fixtures/` are the compiler's e2e inputs, so they are
/// programs the compiler is already expected to handle — the question this asks is whether the
/// *runtime* agrees, which is a different one. Each expected value is read off the source
/// (`fib(10)` is 55, `classify(-5) + classify(0) + …` is 5), so a wrong answer fails rather than
/// crashing, and a shape that frames differently (a recursive call, a chain of comparisons) fails
/// here rather than in a user's program.
#[test]
fn the_compiler_fixture_corpus_executes_to_the_value_its_source_states() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../x3-compiler/tests/fixtures");
    let cases: &[(&str, i64)] = &[
        ("fib.x3", 55),         // fib(10)
        ("loop_ops.x3", 16),    // sum_three(1,2,3) + multiply_accumulate(2,3,4)
        ("match_cond.x3", 5),   // classify(-5)+classify(0)+classify(5)+classify(50)+classify(500)
        ("branch_fold.x3", 30), // (5+10)*2, the branch that returns 999 is folded away
        ("loop_sum.x3", 15),    // sum_to(5) = 1+2+3+4+5, which needs a loop back-edge
    ];

    // Every failing shape is collected and reported together: a corpus check that stops at the
    // first failure hides how many shapes are broken, which is the number that decides whether the
    // encodings agree in general or only for one program.
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (name, expected) in cases {
        let path = dir.join(name);
        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                failures.push(format!("{name}: cannot read {}: {error}", path.display()));
                continue;
            }
        };
        let bytes = match compile_source(&source) {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push(format!("{name}: must compile: {error}"));
                continue;
            }
        };
        if let Err(error) = mini_x3::validate_x3bc(&bytes) {
            failures.push(format!(
                "{name}: the runtime's reader must accept it: {error:?}"
            ));
            continue;
        }
        let receipt = match X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain()) {
            Ok(receipt) => receipt,
            Err(error) => {
                failures.push(format!("{name}: must verify and execute: {error:?}"));
                continue;
            }
        };
        if !receipt.success {
            failures.push(format!(
                "{name}: the VM said: {}",
                String::from_utf8_lossy(&receipt.return_data)
            ));
            continue;
        }
        let got = match <[u8; 8]>::try_from(receipt.return_data.as_slice()) {
            Ok(bytes) => i64::from_le_bytes(bytes),
            Err(_) => {
                failures.push(format!(
                    "{name}: the receipt returned {} bytes, not an i64",
                    receipt.return_data.len()
                ));
                continue;
            }
        };
        if got != *expected {
            failures.push(format!(
                "{name}: the receipt reports {got}, the source computes {expected}"
            ));
            continue;
        }
        if receipt.instructions_executed == 0 {
            failures.push(format!(
                "{name}: a receipt that reports no instructions did not count them"
            ));
            continue;
        }
        // The kernel-side engine has to agree with the std one on the same artifact: they are two
        // implementations of the format, and a program that runs on one and not the other is a
        // disagreement to report, not a pass.
        match mini_x3::execute_x3bc(&bytes, 1_000_000) {
            Ok(result) => match result.return_val {
                MiniValue::I64(value) if value == *expected => {}
                other => failures.push(format!(
                    "{name}: the kernel-side engine returned {other:?}, the source computes {expected}"
                )),
            },
            Err(error) => failures.push(format!("{name}: the kernel-side engine refused it: {error:?}")),
        }
        checked += 1;
    }
    assert!(
        failures.is_empty(),
        "the compiler's own fixtures must run on the runtime ({} of {} ran):\n  {}",
        checked,
        cases.len(),
        failures.join("\n  ")
    );
}
