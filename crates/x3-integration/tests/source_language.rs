//! What a `.x3` program can say, measured end to end: source → compiler → X3BC → both engines.
//!
//! Three things the language accepted, or refused, for the wrong reason (X3-LANG-001):
//!
//! - **Host calls.** `evm_sload`/`evm_sstore` existed as opcodes the backend could emit and both
//!   interpreters could execute, but the resolver called them undefined variables, so no program
//!   could read or write its own storage. They are callable now (`x3_common::intrinsics`).
//! - **Top-level constants.** `const K: i64 = 8;` parsed and type-checked, then failed MIR
//!   lowering with `value for symbol SymbolId(0) missing`. Constants are inlined now; a mutable
//!   global, which would need storage, is refused by name.
//! - **Operators on strings.** `"x" == "x"` type-checked and then failed *at run time* on both
//!   engines with `TypeMismatch`. It is refused at compile time now.
//!
//! Every program here runs on the std engine, on `mini_x3` (the engine a block runs), and through
//! `execute_on_chain`, and the three have to agree.
#![cfg(all(feature = "std", feature = "compile"))]

use x3_x3_integration::compiler_bridge::compile_source;
use x3_x3_integration::mini_x3::{self, MiniValue};
use x3_x3_integration::{X3Executor, X3ExecutorConfig};

/// Run `source` on all three paths and return the value they agree on.
fn agreed_value(source: &str) -> i64 {
    let bytes = compile_source(source).unwrap_or_else(|e| panic!("must compile: {e}\n{source}"));

    let std_receipt = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
        .unwrap_or_else(|e| panic!("std engine: {e:?}\n{source}"));
    assert!(
        std_receipt.success,
        "std engine failed: {}\n{source}",
        String::from_utf8_lossy(&std_receipt.return_data)
    );
    let std_value = i64::from_le_bytes(std_receipt.return_data.as_slice().try_into().unwrap());

    let mini_value = match mini_x3::execute_x3bc(&bytes, 1_000_000)
        .unwrap_or_else(|e| panic!("mini_x3: {e:?}\n{source}"))
        .return_val
    {
        MiniValue::I64(value) => value,
        other => panic!("mini_x3 returned {other:?}\n{source}"),
    };

    let chain = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false)
        .unwrap_or_else(|e| panic!("on-chain path: {e:?}\n{source}"));
    assert!(chain.success, "on-chain path failed\n{source}");
    let chain_value = i64::from_le_bytes(chain.return_data.as_slice().try_into().unwrap());

    assert_eq!(
        (std_value, mini_value),
        (chain_value, chain_value),
        "the engines disagree\n{source}"
    );
    chain_value
}

fn refusal(source: &str) -> String {
    match compile_source(source) {
        Ok(_) => panic!("must be refused at compile time:\n{source}"),
        Err(error) => error.to_string(),
    }
}

// ─── Host calls ─────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_program_reads_back_what_it_stored() {
    assert_eq!(
        agreed_value("fn main() -> i64 { evm_sstore(3, 41); return evm_sload(3) + 1; }"),
        42
    );
    assert_eq!(
        agreed_value("fn main() -> i64 { return evm_sload(9); }"),
        0,
        "a slot that was never written reads as zero"
    );
}

/// The optimizer must not merge the two loads across the store between them, nor drop a store
/// whose value nobody reads: a host call is a call, and every pass treats a call as an effect.
#[test]
fn loads_are_not_merged_across_a_store_and_stores_are_not_dropped() {
    assert_eq!(
        agreed_value(
            "fn main() -> i64 { let a = evm_sload(2); evm_sstore(2, 7); let b = evm_sload(2); \
             return a * 100 + b; }"
        ),
        7
    );
    assert_eq!(
        agreed_value(
            "fn put(v: i64) { evm_sstore(1, v); } \
             fn main() -> i64 { put(5); put(6); return evm_sload(1); }"
        ),
        6
    );
    assert_eq!(
        agreed_value(
            "fn main() -> i64 { let mut i = 0; \
             while i < 3 { evm_sstore(0, evm_sload(0) + 10); i = i + 1; } \
             return evm_sload(0); }"
        ),
        30
    );
}

/// What the program stores is what the chain would persist: the on-chain receipt is the journal
/// of the program's writes, in order, each with the value it replaced, so a slot written twice
/// appears twice and the second entry's old value is the first entry's new one.
#[test]
fn the_on_chain_receipt_carries_the_programs_storage_writes() {
    let bytes =
        compile_source("fn main() -> i64 { evm_sstore(1, 5); evm_sstore(1, 6); return 0; }")
            .unwrap();
    let receipt = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).unwrap();
    assert!(receipt.success);
    let writes = &receipt.storage_writes;
    assert_eq!(writes.len(), 2, "{writes:?}");
    assert_eq!(
        writes[0].key, writes[1].key,
        "both writes name the same slot"
    );
    assert_eq!(writes[0].old_value, None, "the slot was empty");
    assert_eq!(
        writes[1].old_value, writes[0].new_value,
        "the journal chains"
    );
    assert_ne!(writes[0].new_value, writes[1].new_value);

    let readonly = compile_source("fn main() -> i64 { return evm_sload(1); }").unwrap();
    let receipt = X3Executor::execute_on_chain(&readonly, 1_000_000, &[], false).unwrap();
    assert!(receipt.storage_writes.is_empty(), "a read writes nothing");
}

#[test]
fn a_host_call_is_checked_like_any_other_call() {
    let wrong_arity = refusal("fn main() -> i64 { return evm_sload(1, 2); }");
    assert!(wrong_arity.contains("WrongArgumentCount"), "{wrong_arity}");

    // A negative slot is a run-time fault on every path, not a silent wrap to a huge slot.
    let bytes = compile_source("fn main() -> i64 { evm_sstore(0 - 1, 1); return 0; }").unwrap();
    let chain = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).unwrap();
    assert!(!chain.success);
    assert!(
        chain.storage_writes.is_empty(),
        "a faulted program writes nothing"
    );
    assert!(mini_x3::execute_x3bc(&bytes, 1_000_000).is_err());
}

#[test]
fn a_programs_own_function_shadows_the_host_call() {
    assert_eq!(
        agreed_value(
            "fn evm_sload(x: i64) -> i64 { return x + 1000; } \
             fn main() -> i64 { return evm_sload(1); }"
        ),
        1001
    );
}

// ─── Top-level constants ────────────────────────────────────────────────────────────────────────

#[test]
fn top_level_constants_compute_what_the_same_expression_inline_would() {
    assert_eq!(
        agreed_value(
            "const BASE: i64 = 8; const SCALE: i64 = BASE * 3 - 4; let OFFSET = 0 - 2; \
             fn h() -> i64 { return SCALE + OFFSET; } \
             fn main() -> i64 { return h() + BASE; }"
        ),
        26
    );
}

#[test]
fn a_constant_that_is_not_a_constant_is_refused_by_name() {
    let cycle = refusal("const A: i64 = B; const B: i64 = A; fn main() -> i64 { return A; }");
    assert!(cycle.contains("cycle"), "{cycle}");

    let call =
        refusal("fn f() -> i64 { return 1; } const C: i64 = f(); fn main() -> i64 { return C; }");
    assert!(call.contains("constant expression"), "{call}");

    let mutable = refusal("let mut g = 7; fn main() -> i64 { return g; }");
    assert!(mutable.contains("MutableGlobal"), "{mutable}");

    // The resolver already refuses this one; the HIR check behind it names it the same way.
    let assigned = refusal("const K: i64 = 1; fn main() -> i64 { K = 2; return K; }");
    assert!(assigned.contains("Immutable"), "{assigned}");
}

// ─── Strings ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn an_operator_on_strings_is_refused_at_compile_time_not_at_run_time() {
    for op in ["==", "!=", "<", "+"] {
        let error = refusal(&format!(
            "fn main() -> i64 {{ let a = \"x\"; let b = \"y\"; let c = a {op} b; return 1; }}"
        ));
        // `+` is refused by the type checker; the comparisons it allows are refused by MIR.
        assert!(error.to_lowercase().contains("string"), "{op}: {error}");
    }
    // Binding, passing and returning a string still compile and run.
    assert_eq!(
        agreed_value(
            "fn tag() -> string { return \"abc\"; } fn one(s: string) -> i64 { return 1; } \
             fn main() -> i64 { let s = tag(); return one(s) + one(\"q\"); }"
        ),
        2
    );
}

// ─── Cross-VM calls (X3-LANG-001) ───────────────────────────────────────────────────────────────

/// The chain's EVM and SVM keep no state between executions (the production runtime runs
/// `mini_evm` and the payload's own SVM program), so a cross-VM call has nothing to reach. It is
/// refused by name with that reason, not reported as an unknown identifier.
#[test]
fn a_cross_vm_call_is_refused_with_the_reason_it_cannot_run() {
    for call in ["evm_call(1, 2, 3, 4)", "svm_invoke(1, 2)", "evm_balance(1)"] {
        let error = refusal(&format!("fn main() -> i64 {{ return {call}; }}"));
        assert!(error.contains("UnavailableCrossVmCall"), "{call}: {error}");
    }
    // A program's own function with that name is still its own.
    assert_eq!(
        agreed_value(
            "fn evm_call(a: i64) -> i64 { return a + 1; } \
             fn main() -> i64 { return evm_call(41); }"
        ),
        42
    );
}

// ─── Control-flow parity across all three execution paths ──────────────────────────────────────

/// These programs are compiled from source, not hand-assembled. A compiler which misplaces
/// a branch target, leaks a loop-local register, or drops a call will disagree with the
/// mathematical result in at least one of the std, mini and on-chain execution paths.
#[test]
fn branches_execute_only_the_selected_arm() {
    assert_eq!(
        agreed_value(
            "fn main() -> i64 { let mut x = 3; if x > 2 { x = 41; } else { x = 7; } return x + 1; }"
        ),
        42
    );
    assert_eq!(
        agreed_value(
            "fn main() -> i64 { let mut x = 3; if x < 2 { x = 41; } else { x = 7; } return x + 1; }"
        ),
        8
    );
}

#[test]
fn loop_iterations_update_state_and_respect_branching() {
    assert_eq!(
        agreed_value(
            "fn main() -> i64 { let mut i = 0; let mut sum = 0; while i < 7 {              if i < 4 { sum = sum + i; } else { sum = sum + 2; }              i = i + 1; } return sum; }"
        ),
        12
    );
    assert_eq!(
        agreed_value("fn main() -> i64 { let mut i = 0; while i < 0 { i = i + 1; } return i; }"),
        0
    );
}

#[test]
fn nested_function_calls_and_arithmetic_agree() {
    assert_eq!(
        agreed_value(
            "fn twice(x: i64) -> i64 { return x * 2; }              fn plus(x: i64, y: i64) -> i64 { return x + y; }              fn main() -> i64 { return plus(twice(19), 4); }"
        ),
        42
    );
}
