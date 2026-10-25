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

use x3_x3_integration::compiler_bridge::compile_legacy_source as compile_source;
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

// ─── Block context ──────────────────────────────────────────────────────────────────────────────

/// Source reads the block it runs in. `ctx_*` are host calls (`x3_common::intrinsics`), resolved
/// and typed like `evm_sload`, so a program that declares its own `ctx_chain_id` still wins.
#[test]
fn source_reads_the_block_context_it_is_given() {
    let bytes = compile_source(
        "fn main() -> i64 { \
           let lease_ends = ctx_timestamp() + 30000; \
           if ctx_chain_id() != 650000 { return 0 - 1; } \
           return ctx_block_height() * 1000000 + (lease_ends - ctx_timestamp()) + ctx_value(); \
         }",
    )
    .unwrap_or_else(|e| panic!("must compile: {e}"));
    let context = x3_x3_integration::X3BlockContext {
        sender: None,
        block_height: Some(42),
        timestamp_ms: Some(1_760_000_000_000),
        value: Some(0),
        chain_id: Some(650_000),
    };
    let receipt =
        X3Executor::execute_on_chain_with_context(&bytes, 1_000_000, &[], false, &context).unwrap();
    assert!(
        receipt.success,
        "{}",
        String::from_utf8_lossy(&receipt.return_data)
    );
    assert_eq!(
        receipt.return_data,
        (42i64 * 1_000_000 + 30_000).to_le_bytes().to_vec()
    );

    // With no context the same program is refused by the first read, not answered with zeros.
    let receipt = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).unwrap();
    assert!(!receipt.success);
    assert!(String::from_utf8_lossy(&receipt.return_data).contains("ContextUnavailable"));
}

#[test]
fn a_programs_own_function_named_like_a_context_read_still_wins() {
    let bytes = compile_source(
        "fn ctx_chain_id() -> i64 { return 7; } fn main() -> i64 { return ctx_chain_id(); }",
    )
    .unwrap_or_else(|e| panic!("must compile: {e}"));
    let receipt = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).unwrap();
    assert!(receipt.success);
    assert_eq!(receipt.return_data, 7i64.to_le_bytes().to_vec());
}

// ─── Arrays and tuples ──────────────────────────────────────────────────────────────────────────

fn on_chain(source: &str) -> X3ExecutionReceiptView {
    let bytes = compile_source(source).unwrap_or_else(|e| panic!("must compile: {e}\n{source}"));
    let receipt = X3Executor::execute_on_chain(&bytes, 1_000_000, &[], false).unwrap();
    X3ExecutionReceiptView {
        success: receipt.success,
        data: receipt.return_data,
    }
}

struct X3ExecutionReceiptView {
    success: bool,
    data: Vec<u8>,
}

fn returns(source: &str, expected: i64) {
    let view = on_chain(source);
    assert!(
        view.success,
        "{}\n{source}",
        String::from_utf8_lossy(&view.data)
    );
    assert_eq!(view.data, expected.to_le_bytes().to_vec(), "{source}");
}

#[test]
fn an_array_literal_is_indexed_measured_and_summed_in_a_loop() {
    returns(
        "fn main() -> i64 { let xs = [3, 5, 8, 13]; let mut i = 0; let mut sum = 0; \
         while i < xs.len() { sum = sum + xs[i]; i = i + 1; } return sum * 10 + xs.len(); }",
        294,
    );
}

/// `a[i] = v` rebinds `a`; a copy taken before the write keeps its old element (value semantics,
/// which is what lets MIR stay in SSA form and is what the engine does with `mov`).
#[test]
fn element_assignment_has_value_semantics() {
    returns(
        "fn main() -> i64 { let mut a = [1, 2, 3]; let before = a; a[1] = 20; \
         return a[1] * 100 + before[1]; }",
        2002,
    );
}

#[test]
fn an_array_built_in_a_loop_and_passed_to_a_function() {
    returns(
        "fn total(xs: [i64; 4]) -> i64 { let mut i = 0; let mut s = 0; \
         while i < xs.len() { s = s + xs[i]; i = i + 1; } return s; } \
         fn main() -> i64 { let mut a = [0, 0, 0, 0]; let mut i = 0; \
         while i < 4 { a[i] = i * i; i = i + 1; } return total(a); }",
        14,
    );
}

#[test]
fn a_tuple_is_built_and_read_by_position() {
    returns(
        "fn main() -> i64 { let t = (7, 9); return t[0] * 10 + t[1]; }",
        79,
    );
}

#[test]
fn reading_past_the_end_fails_the_execution_and_writes_nothing() {
    let view = on_chain("fn main() -> i64 { evm_sstore(0, 1); let a = [1, 2]; return a[5]; }");
    assert!(!view.success);
    assert!(String::from_utf8_lossy(&view.data).contains("ArrayIndexOutOfBounds"));
}

#[test]
fn aggregate_misuse_is_refused_at_compile_time() {
    let cases = [
        (
            "fn main() -> i64 { let a = [1, 2]; a[0] = 5; return a[0]; }",
            "immutable",
        ),
        (
            "fn main() -> i64 { let t = (1, 2); let i = 1; return t[i]; }",
            "tuple index",
        ),
        (
            "fn main() -> i64 { let mut a = [1, 2]; a[0] = true; return a[0]; }",
            "Mismatch",
        ),
        (
            "fn main() -> i64 { let a = [1, true]; return 0; }",
            "InconsistentArrayElements",
        ),
        (
            "fn f() -> [i64; 2] { return [1, 2]; } fn main() -> i64 { f()[0] = 3; return 0; }",
            "named array",
        ),
    ];
    for (source, expected) in cases {
        let why = refusal(source);
        assert!(
            why.to_lowercase().contains(&expected.to_lowercase()),
            "{source}\n  refused with: {why}\n  expected it to mention `{expected}`"
        );
    }
}

// ─── Structs ────────────────────────────────────────────────────────────────────────────────────

#[test]
fn a_struct_is_built_read_by_field_and_passed_to_a_function() {
    returns(
        "struct Point { x: i64, y: i64 } \
         fn dot(a: Point, b: Point) -> i64 { return a.x * b.x + a.y * b.y; } \
         fn main() -> i64 { let p = Point { x: 3, y: 4 }; let q = Point { y: 2, x: 5 }; \
         return dot(p, q); }",
        23,
    );
}

#[test]
fn a_field_assignment_has_value_semantics() {
    returns(
        "struct Acct { bal: i64, nonce: i64 } \
         fn main() -> i64 { let mut a = Acct { bal: 10, nonce: 0 }; let before = a; \
         a.bal = a.bal + 5; a.nonce = 1; return a.bal * 100 + a.nonce * 10 + before.bal; }",
        1520,
    );
}

#[test]
fn structs_nest_and_hold_arrays_and_are_returned() {
    returns(
        "struct Leg { venue: i64, amounts: [i64; 2] } \
         struct Route { first: Leg, second: Leg } \
         fn make(v: i64) -> Leg { return Leg { venue: v, amounts: [v, v * 2] }; } \
         fn main() -> i64 { let r = Route { first: make(1), second: make(3) }; \
         return r.second.amounts[1] * 10 + r.first.venue; }",
        61,
    );
}

#[test]
fn a_field_updated_in_a_loop_keeps_every_iteration() {
    returns(
        "struct Acc { sum: i64, n: i64 } \
         fn main() -> i64 { let mut acc = Acc { sum: 0, n: 0 }; let mut i = 1; \
         while i <= 4 { acc.sum = acc.sum + i; acc.n = acc.n + 1; i = i + 1; } \
         return acc.sum * 10 + acc.n; }",
        104,
    );
}

#[test]
fn a_struct_literals_fields_are_evaluated_in_the_order_written() {
    // `b` is written first, so its store happens first and `a` reads it.
    returns(
        "struct P { a: i64, b: i64 } \
         fn put(v: i64) -> i64 { evm_sstore(0, v); return v; } \
         fn main() -> i64 { let p = P { b: put(7), a: evm_sload(0) }; return p.a * 10 + p.b; }",
        77,
    );
}

#[test]
fn a_tuple_element_is_read_and_written_by_position() {
    returns(
        "fn main() -> i64 { let mut t = (1, 2); t.1 = 9; return t.0 * 10 + t.1; }",
        19,
    );
}

#[test]
fn a_struct_value_returned_from_main_is_its_fields() {
    let view =
        on_chain("struct P { a: i64, b: bool } fn main() -> P { return P { a: 5, b: true }; }");
    assert!(view.success, "{}", String::from_utf8_lossy(&view.data));
    let expected = x3_vm::Value::Tuple(vec![x3_vm::Value::I64(5), x3_vm::Value::Bool(true)]);
    assert_eq!(view.data, expected.return_bytes());
}

#[test]
fn struct_misuse_is_refused_at_compile_time() {
    let cases = [
        ("struct P { a: i64 } fn main() -> i64 { let p = P { a: 1, b: 2 }; return 0; }", "NoSuchField"),
        ("struct P { a: i64, b: i64 } fn main() -> i64 { let p = P { a: 1 }; return 0; }", "leaving out"),
        ("struct P { a: i64 } fn main() -> i64 { let p = P { a: 1, a: 2 }; return 0; }", "twice"),
        ("struct P { a: i64 } fn main() -> i64 { let p = P { a: true }; return 0; }", "Mismatch"),
        ("fn main() -> i64 { let p = Q { a: 1 }; return 0; }", "UnknownType"),
        ("struct P { a: i64 } fn main() -> i64 { let p = P { a: 1 }; return p.z; }", "NoSuchField"),
        ("struct P { a: i64 } fn main() -> i64 { let p = P { a: 1 }; p.a = 2; return p.a; }", "immutable"),
        ("struct P { a: i64 } fn main() -> i64 { let mut p = P { a: 1 }; p.a = true; return p.a; }", "Mismatch"),
        ("struct P { a: P } fn main() -> i64 { return 0; }", "RecursiveType"),
        ("struct P { a: Q } struct Q { p: P } fn main() -> i64 { return 0; }", "RecursiveType"),
        ("struct P { a: Nope } fn main() -> i64 { return 0; }", "UnknownType"),
        ("struct P { a: i64, a: i64 } fn main() -> i64 { return 0; }", "twice"),
        ("struct P { a: i64 } struct P { b: i64 } fn main() -> i64 { return 0; }", "twice"),
        // Nominal: the same fields under another name are another type.
        (
            "struct P { a: i64 } struct Q { a: i64 } fn f(p: P) -> i64 { return p.a; } \
             fn main() -> i64 { return f(Q { a: 1 }); }",
            "Mismatch",
        ),
        ("fn main() -> i64 { let t = (1, 2); return t.2; }", "NoSuchField"),
    ];
    for (source, expected) in cases {
        let why = refusal(source);
        assert!(
            why.to_lowercase().contains(&expected.to_lowercase()),
            "{source}\n  refused with: {why}\n  expected it to mention `{expected}`"
        );
    }
}

#[test]
fn a_condition_followed_by_a_block_is_not_read_as_a_struct_literal() {
    returns(
        "struct S { v: i64 } fn main() -> i64 { let flag = true; let mut r = 0; \
         if flag { r = 1; } while r < 3 { r = r + 1; } \
         if (S { v: 2 }).v == 2 { r = r * 10; } return r; }",
        30,
    );
}
