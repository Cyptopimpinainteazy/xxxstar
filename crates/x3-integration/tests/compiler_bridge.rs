//! Compiler-to-runtime boundary tests.
//!
//! These tests require the real workspace compiler. No fake or empty bytecode is
//! accepted by the integration layer.

#![cfg(all(feature = "std", feature = "compile"))]

use x3_backend::BytecodeModule;
use x3_x3_integration::compiler_bridge::{
    compile_legacy_source as compile_source,
    compile_legacy_source_with_policy as compile_source_with_policy,
};
use x3_x3_integration::mini_x3::{self, MiniValue};
use x3_x3_integration::{CompilationPolicy, X3Executor, X3ExecutorConfig};

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
        ("fib.x3", 55),           // fib(10)
        ("loop_ops.x3", 16),      // sum_three(1,2,3) + multiply_accumulate(2,3,4)
        ("match_cond.x3", 5),     // classify(-5)+classify(0)+classify(5)+classify(50)+classify(500)
        ("branch_fold.x3", 30),   // (5+10)*2, the branch that returns 999 is folded away
        ("loop_sum.x3", 15),      // sum_to(5) = 1+2+3+4+5, which needs a loop back-edge
        ("loop_break.x3", 21),    // count_to(6) = 1+..+6, which needs `break` to leave the loop
        ("loop_continue.x3", 12), // sum_except(5, 3) = 1+2+4+5, which needs `continue` to skip one
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

/// Floating point is computed in simulation and refused on-chain — and both halves are asserted.
///
/// The verifier's on-chain options deny float opcodes (`VerifyOptions::on_chain` sets
/// `deny_float_arithmetic`), because a float result that depends on the platform's rounding is not a
/// deterministic state transition. So a float program must not run on the on-chain path: what this
/// test requires is that the *arithmetic* is real in simulation (`1.5 + 2.5 == 4.0`, so the branch
/// returns 7) and that the same artifact is refused on-chain by that policy rather than computed.
///
/// The refusal is the interesting half for the compiler: before the float flag reached the backend
/// it emitted an integer add for `1.5 + 2.5`, which failed at run time with `TypeMismatch("i64",
/// "F64(1.5)")` — a type error where the language owes the reader either the operation or a policy
/// refusal (TICKET-133).
#[test]
fn float_arithmetic_runs_in_simulation_and_is_refused_on_chain() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../x3-compiler/tests/fixtures/float_math.x3");
    let source = std::fs::read_to_string(&path).expect("the fixture must be readable");
    let bytes = compile_source(&source).expect("the fixture must compile");

    let receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::simulation())
        .expect("simulation must execute a float program");
    assert!(
        receipt.success,
        "1.5 + 2.5 == 4.0 takes the branch: {}",
        String::from_utf8_lossy(&receipt.return_data)
    );
    assert_eq!(
        receipt.return_data,
        7i64.to_le_bytes().to_vec(),
        "the float arithmetic must produce the value the source states"
    );

    let refused = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain());
    assert!(
        refused.is_err(),
        "an on-chain run must refuse float arithmetic, not compute it: {refused:?}"
    );
    let message = format!("{:?}", refused.expect_err("refused"));
    assert!(
        message.contains("ForbiddenOnChain"),
        "and say that it is the float opcode policy, not a parse or type failure: {message}"
    );
}

/// X3-MEV-002's chain-intake half: a deployment policy can require private submission, and the
/// requirement is carried by the artifact's own bytes.
///
/// The MEV/privacy rows recorded a structural gap for weeks — "the chain-intake pipeline carries no
/// submission policy ... a program that reaches the chain cannot demand private submission at all"
/// — because the compiler this chain runs had no notion of a submission policy. A demand the
/// compiler does not record cannot be enforced by anything downstream, so these two tests pin both
/// the positive and the negative case at the *compiler boundary*: the flag is set when the policy
/// asks for it, and it is absent when the policy does not, with the rest of the artifact unchanged.
///
/// The enforcing half is `pallet-x3-kernel`'s intake check (see
/// `pallets/x3-kernel/src/private_submission_intake.rs`), which reads this bit out of the header and
/// refuses the program when the runtime's own `PrivateSubmissionChannel` says the chain cannot meet
/// it. Testing only one of the two halves would prove nothing: a recorded demand nobody reads, or a
/// check nothing ever trips.
#[test]
fn the_private_submission_policy_is_recorded_in_the_artifact_header() {
    let source = "fn main() -> i64 {\n    return 7;\n}\n";

    let demanded =
        compile_source_with_policy(source, CompilationPolicy::private_submission_required())
            .expect("valid .x3 source must compile under the private-submission policy");
    let plain = compile_source(source).expect("valid .x3 source must compile");

    assert!(
        x3_common::bytecode::requires_private_submission(&demanded),
        "the artifact compiled under the policy must carry the demand in its own header"
    );
    assert!(
        !x3_common::bytecode::requires_private_submission(&plain),
        "the default policy must not invent a demand"
    );

    // The demand must not change anything else about the artifact: it is a capability, not a
    // different program. Byte-for-byte equality apart from the feature word is the strongest form
    // of that claim available here.
    let offset = x3_common::bytecode::FEATURE_FLAGS_OFFSET;
    assert_eq!(
        demanded.len(),
        plain.len(),
        "recording a capability must not change the artifact's length"
    );
    assert_eq!(
        &demanded[..offset],
        &plain[..offset],
        "the header before the feature word must be identical"
    );
    assert_eq!(
        &demanded[offset + 4..],
        &plain[offset + 4..],
        "the body and the rest of the header must be identical"
    );

    // And a module carrying the demand is still a valid module: both readers the runtime has accept
    // it, so the refusal a chain performs is the *policy* and not a decode failure.
    BytecodeModule::from_bytes(&demanded).expect("the std reader must accept a demanding artifact");
    mini_x3::validate_x3bc(&demanded).expect("the no-std reader must accept a demanding artifact");
}

/// The engine's half of the compiled capability, in both interpreters.
///
/// X3-MEV-002's demand is recorded by the compiler and enforced at intake by `pallet-x3-kernel`; the
/// engines a program runs on also have to honour it, or an artifact that reached an interpreter by a
/// route which skipped the intake check would run in the clear. `mini_x3` used to read the header's
/// feature word and *discard* it (`let _features = ...`), which is the shape a policy takes when
/// nothing can enforce it — so this test drives the artifact through both engines and requires the
/// same answer from each.
#[test]
fn both_engines_refuse_a_demanding_artifact_without_a_private_channel() {
    let demanding = compile_source_with_policy(
        "fn main() -> i64 {\n    return 5;\n}\n",
        CompilationPolicy::private_submission_required(),
    )
    .expect("the fixture must compile");
    let plain =
        compile_source("fn main() -> i64 {\n    return 5;\n}\n").expect("the fixture must compile");

    // mini_x3 — the interpreter a block runs, through the wrapper that supplies no posture and the
    // one that supplies it.
    assert_eq!(
        mini_x3::execute_x3bc(&demanding, 100_000).err(),
        Some(mini_x3::X3Error::PrivateSubmissionRequired),
        "the no-std engine must refuse a demanding artifact when nothing said a private channel exists"
    );
    mini_x3::execute_x3bc_with_slots_and_policy(&demanding, 100_000, &[], true)
        .expect("the same bytes must run once the context offers a private channel");
    mini_x3::execute_x3bc(&plain, 100_000)
        .expect("a program that does not demand privacy must be unaffected");

    // The std engine, through the executor the node's adapters call.
    let refused = X3Executor::execute(&demanding, &[], X3ExecutorConfig::on_chain());
    assert!(
        refused.is_err(),
        "the std engine must refuse the same artifact: {refused:?}"
    );
    assert!(
        X3Executor::execute(
            &demanding,
            &[],
            X3ExecutorConfig::on_chain().with_private_submission_available()
        )
        .is_ok(),
        "and must run it once the context declares a private channel"
    );
    assert!(
        X3Executor::execute(&plain, &[], X3ExecutorConfig::on_chain()).is_ok(),
        "the default policy must not be gated"
    );

    // Both engines must agree about *which* artifacts are gated, which is the property that makes
    // this a policy rather than two independent opinions.
    assert_eq!(
        mini_x3::execute_x3bc(&demanding, 100_000).is_err(),
        X3Executor::execute(&demanding, &[], X3ExecutorConfig::on_chain()).is_err(),
    );
}
