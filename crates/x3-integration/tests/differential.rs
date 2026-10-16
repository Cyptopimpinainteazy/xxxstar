//! Differential end-to-end tests for the chain compiler.
//!
//! **Since the runtime switched engines:** blocks execute `crates/x3-vm` through
//! `X3Executor::execute_on_chain` in every build (`tests/chain_engine.rs`). `mini_x3` is no longer
//! on any chain path; it stays as the reference this file holds the new engine to, until the old
//! engines are retired. Where the text below says a block runs `mini_x3`, read "used to run".
//!
//! One program is compiled at every optimization level and in two function orders (as written,
//! and with `main` moved to the front), and every build is executed on **both** of the runtime's
//! engines — `X3Executor` (the std on-chain path) and `mini_x3` (the no-std reader the kernel's
//! adapter uses). Every one of those runs must produce the value the source states.
//!
//! This is the property the compiler has to have for a chain: the optimizer may change how a
//! program computes, never what it computes; the order a program declares its functions in is not
//! part of its meaning; and the two engines that read the same X3BC may not disagree about it. A
//! single test that compiled once, at one level, could not see any of those three failures.
//!
//! Every mismatch is collected and reported together, because the number of broken shapes is what
//! says whether a fix is general or specific to one program.

#![cfg(all(feature = "std", feature = "compile"))]

use x3_compiler::{CompilationOptions, Compiler};
use x3_x3_integration::mini_x3::{self, MiniValue};
use x3_x3_integration::{X3Executor, X3ExecutorConfig};

/// Enough for every program here, and small enough that a loop that fails to terminate is
/// reported as gas exhaustion in milliseconds rather than hanging the suite.
const KERNEL_GAS: u64 = 1_000_000;

fn levels() -> Vec<(&'static str, CompilationOptions)> {
    vec![
        ("O0", CompilationOptions::no_opt()),
        ("O1", CompilationOptions::basic()),
        ("O2", CompilationOptions::opt2()),
        ("O3", CompilationOptions::opt3()),
        ("contract", CompilationOptions::contract_mode()),
    ]
}

/// Split `source` into top-level `fn` items and put `main` first.
///
/// Only used on the programs in this file, which are written with one item per `fn` at column 0,
/// so the split is by line rather than by a parser — the point is to give the compiler a second
/// ordering of the *same* program, not to test a reordering tool.
fn main_first(source: &str) -> Option<String> {
    let mut items: Vec<String> = Vec::new();
    let mut prelude = String::new();
    for line in source.lines() {
        if line.starts_with("fn ") {
            items.push(String::new());
        }
        match items.last_mut() {
            Some(item) => {
                item.push_str(line);
                item.push('\n');
            }
            None => {
                prelude.push_str(line);
                prelude.push('\n');
            }
        }
    }
    let main = items.iter().position(|item| item.starts_with("fn main("))?;
    if main == 0 {
        return None;
    }
    let entry = items.remove(main);
    items.insert(0, entry);
    Some(format!("{prelude}{}", items.concat()))
}

/// Compile and run one build of one program on both engines; `Err` describes the first divergence.
fn run_one(source: &str, options: CompilationOptions, expected: i64) -> Result<(), String> {
    let output = Compiler::compile(source, options).map_err(|e| format!("compile: {e:?}"))?;
    run_bytes(&output.bytecode.to_bytes(), expected)
}

/// Execute one artifact on both engines; `Err` describes the first divergence from `expected`.
fn run_bytes(bytes: &[u8], expected: i64) -> Result<(), String> {
    // The runtime validates before it executes; a compiled program it refuses cannot reach a chain.
    mini_x3::validate_x3bc(bytes)
        .map_err(|e| format!("the runtime's validator refused it: {e:?}"))?;
    let receipt = X3Executor::execute(bytes, &[], X3ExecutorConfig::on_chain())
        .map_err(|e| format!("X3Executor: {e:?}"))?;
    if !receipt.success {
        return Err(format!(
            "X3Executor: failed: {}",
            String::from_utf8_lossy(&receipt.return_data)
        ));
    }
    if receipt.return_data != expected.to_le_bytes().to_vec() {
        let got = <[u8; 8]>::try_from(receipt.return_data.as_slice())
            .map(i64::from_le_bytes)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| format!("{:?}", receipt.return_data));
        return Err(format!("X3Executor returned {got}, expected {expected}"));
    }

    let kernel = mini_x3::execute_x3bc(bytes, KERNEL_GAS).map_err(|e| format!("mini_x3: {e:?}"))?;
    if kernel.return_val != MiniValue::I64(expected) {
        return Err(format!(
            "mini_x3 returned {:?}, expected {expected}",
            kernel.return_val
        ));
    }
    Ok(())
}

/// Run every build of every case and return one line per failing build.
fn check_all(cases: &[(&str, &str, i64)]) -> Vec<String> {
    let mut failures = Vec::new();
    for (name, source, expected) in cases {
        let mut orders = vec![("as-written", source.to_string())];
        if let Some(reordered) = main_first(source) {
            orders.push(("main-first", reordered));
        }
        for (order, text) in &orders {
            for (level, options) in levels() {
                if let Err(why) = run_one(text, options, *expected) {
                    failures.push(format!("{name} [{level}, {order}]: {why}"));
                }
            }
        }
    }
    failures
}

fn assert_no_failures(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} build(s) diverged from the value the source states:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

const CONTROL_FLOW: &[(&str, &str, i64)] = &[
    (
        "nested_if",
        "fn pick(a: i64, b: i64) -> i64 {\n    if a > 0 {\n        if b > 0 {\n            return 1;\n        } else {\n            return 2;\n        }\n    } else {\n        if b > 0 {\n            return 3;\n        }\n    }\n    return 4;\n}\n\nfn main() -> i64 {\n    return pick(1, 1) * 1000 + pick(1, -1) * 100 + pick(-1, 1) * 10 + pick(-1, -1);\n}\n",
        1234,
    ),
    (
        "nested_while",
        "fn grid(n: i64) -> i64 {\n    let mut total = 0;\n    let mut i = 0;\n    while i < n {\n        let mut j = 0;\n        while j < n {\n            total = total + i * n + j;\n            j = j + 1;\n        }\n        i = i + 1;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return grid(4);\n}\n",
        120,
    ),
    (
        "while_in_if",
        "fn f(flag: i64) -> i64 {\n    let mut total = 0;\n    if flag > 0 {\n        let mut i = 0;\n        while i < 5 {\n            total = total + 2;\n            i = i + 1;\n        }\n    } else {\n        total = 99;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f(1) * 100 + f(0);\n}\n",
        1099,
    ),
    (
        "if_in_while",
        "fn evens(n: i64) -> i64 {\n    let mut i = 0;\n    let mut count = 0;\n    while i < n {\n        if i % 2 == 0 {\n            count = count + 1;\n        } else {\n            count = count + 10;\n        }\n        i = i + 1;\n    }\n    return count;\n}\n\nfn main() -> i64 {\n    return evens(7);\n}\n",
        34,
    ),
    (
        "nested_break",
        "fn f() -> i64 {\n    let mut i = 0;\n    let mut hits = 0;\n    while i < 5 {\n        let mut j = 0;\n        while j < 100 {\n            if j == 3 {\n                break;\n            }\n            hits = hits + 1;\n            j = j + 1;\n        }\n        i = i + 1;\n    }\n    return hits;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        15,
    ),
    (
        "nested_continue",
        "fn f() -> i64 {\n    let mut i = 0;\n    let mut total = 0;\n    while i < 3 {\n        i = i + 1;\n        let mut j = 0;\n        while j < 4 {\n            j = j + 1;\n            if j == 2 {\n                continue;\n            }\n            total = total + j;\n        }\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        24,
    ),
    (
        "return_in_loop",
        "fn first_over(limit: i64) -> i64 {\n    let mut i = 0;\n    while i < 1000 {\n        if i * i > limit {\n            return i;\n        }\n        i = i + 1;\n    }\n    return -1;\n}\n\nfn main() -> i64 {\n    return first_over(50);\n}\n",
        8,
    ),
    (
        "call_in_loop",
        "fn sq(x: i64) -> i64 {\n    return x * x;\n}\n\nfn sum_squares(n: i64) -> i64 {\n    let mut i = 1;\n    let mut total = 0;\n    while i <= n {\n        total = total + sq(i);\n        i = i + 1;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return sum_squares(5);\n}\n",
        55,
    ),
    (
        "call_in_branch",
        "fn double(x: i64) -> i64 {\n    return x + x;\n}\n\nfn triple(x: i64) -> i64 {\n    return x * 3;\n}\n\nfn f(x: i64) -> i64 {\n    if x > 10 {\n        return double(x);\n    }\n    return triple(x);\n}\n\nfn main() -> i64 {\n    return f(20) + f(2);\n}\n",
        46,
    ),
    (
        "multiple_returns",
        "fn sign(x: i64) -> i64 {\n    if x > 0 {\n        return 1;\n    }\n    if x < 0 {\n        return -1;\n    }\n    return 0;\n}\n\nfn main() -> i64 {\n    return sign(9) * 100 + sign(-9) * 10 + sign(0);\n}\n",
        90,
    ),
    (
        "loop_with_break_and_mutation",
        "fn f() -> i64 {\n    let mut n = 0;\n    loop {\n        n = n + 7;\n        if n > 30 {\n            break;\n        }\n    }\n    return n;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        35,
    ),
    (
        "countdown",
        "fn countdown(start: i64) -> i64 {\n    let mut n = start;\n    let mut steps = 0;\n    while n > 0 {\n        n = n - 3;\n        steps = steps + 1;\n    }\n    return steps;\n}\n\nfn main() -> i64 {\n    return countdown(10);\n}\n",
        4,
    ),
];

/// A mutable variable must be its own storage: writing it may not change the value it was
/// initialised from, nor another variable initialised from the same value.
const MUTABLE_CELLS: &[(&str, &str, i64)] = &[
    (
        "cell_does_not_alias_its_initialiser",
        "fn main() -> i64 {\n    let a = 5;\n    let mut x = a;\n    x = 7;\n    return a * 10 + x;\n}\n",
        57,
    ),
    (
        "two_cells_from_one_value",
        "fn main() -> i64 {\n    let a = 5;\n    let mut x = a;\n    let mut y = a;\n    x = x + 1;\n    y = y + 2;\n    return a * 100 + x * 10 + y;\n}\n",
        567,
    ),
    (
        "cell_from_param_does_not_alias",
        "fn f(p: i64) -> i64 {\n    let mut x = p;\n    x = x * 3;\n    return p * 100 + x;\n}\n\nfn main() -> i64 {\n    return f(4);\n}\n",
        412,
    ),
    (
        "cell_initialised_from_expression_then_same_expression",
        "fn f(a: i64, b: i64) -> i64 {\n    let mut x = a + b;\n    x = 100;\n    let y = a + b;\n    return x + y;\n}\n\nfn main() -> i64 {\n    return f(2, 3);\n}\n",
        105,
    ),
    (
        "inner_cell_reset_every_outer_iteration",
        "fn f() -> i64 {\n    let mut total = 0;\n    let mut i = 0;\n    while i < 3 {\n        let mut j = 0;\n        while j < 2 {\n            j = j + 1;\n            total = total + 1;\n        }\n        i = i + 1;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        6,
    ),
    (
        "same_literal_two_cells",
        "fn main() -> i64 {\n    let mut x = 0;\n    let mut y = 0;\n    x = x + 1;\n    y = y + 5;\n    x = x + 1;\n    return x * 10 + y;\n}\n",
        25,
    ),
];

#[test]
fn mutable_variables_are_their_own_storage() {
    assert_no_failures(check_all(MUTABLE_CELLS));
}

const FOR_LOOPS: &[(&str, &str, i64)] = &[
    (
        "for_range",
        "fn sum(n: i64) -> i64 {\n    let mut total = 0;\n    for (let i in 0..n) {\n        total = total + i;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return sum(5);\n}\n",
        10,
    ),
    (
        "for_range_continue",
        "fn sum_odd(n: i64) -> i64 {\n    let mut total = 0;\n    for (let i in 0..n) {\n        if i % 2 == 0 {\n            continue;\n        }\n        total = total + i;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return sum_odd(8);\n}\n",
        16,
    ),
    (
        "for_range_break",
        "fn f() -> i64 {\n    let mut last = 0;\n    for (let i in 0..100) {\n        if i == 7 {\n            break;\n        }\n        last = i;\n    }\n    return last;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        6,
    ),
    (
        "for_range_nonzero_start",
        "fn f() -> i64 {\n    let mut total = 0;\n    for (let i in 3..6) {\n        total = total + i;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        12,
    ),
    (
        "for_range_end_evaluated_once",
        "fn f() -> i64 {\n    let mut n = 3;\n    let mut total = 0;\n    for (let i in 0..n) {\n        n = n + 1;\n        total = total + 1;\n        if total > 50 {\n            break;\n        }\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
        3,
    ),
    (
        "for_cstyle",
        "fn f(n: i64) -> i64 {\n    let mut total = 0;\n    for (let mut i = 0; i < n; i = i + 1) {\n        total = total + i;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f(5);\n}\n",
        10,
    ),
    (
        "for_cstyle_continue",
        "fn f(n: i64) -> i64 {\n    let mut total = 0;\n    for (let mut i = 0; i < n; i = i + 1) {\n        if i == 2 {\n            continue;\n        }\n        total = total + i;\n    }\n    return total;\n}\n\nfn main() -> i64 {\n    return f(5);\n}\n",
        8,
    ),
];

const EXPRESSIONS: &[(&str, &str, i64)] = &[
    (
        "arith",
        "fn main() -> i64 {\n    let a = 17;\n    let b = 5;\n    return (a + b) * 1000000 + (a - b) * 10000 + (a * b) * 10 + a / b + a % b - 5;\n}\n",
        22_120_850,
    ),
    (
        "negative",
        "fn neg(x: i64) -> i64 {\n    return -x;\n}\n\nfn main() -> i64 {\n    return neg(5) * 3 - neg(-2);\n}\n",
        -17,
    ),
    (
        "logic",
        "fn b2i(c: bool) -> i64 {\n    if c {\n        return 1;\n    }\n    return 0;\n}\n\nfn main() -> i64 {\n    let t = 3 > 2;\n    let f = 2 > 3;\n    return b2i(t && t) * 10000 + b2i(t && f) * 1000 + b2i(f || t) * 100 + b2i(f || f) * 10 + b2i(!f);\n}\n",
        10101,
    ),
    (
        "comparisons",
        "fn b2i(c: bool) -> i64 {\n    if c {\n        return 1;\n    }\n    return 0;\n}\n\nfn main() -> i64 {\n    let a = 4;\n    let b = 9;\n    return b2i(a < b) * 100000 + b2i(a <= b) * 10000 + b2i(a > b) * 1000 + b2i(a >= b) * 100 + b2i(a == b) * 10 + b2i(a != b);\n}\n",
        110001,
    ),
    (
        "many_temporaries",
        "fn main() -> i64 {\n    let a = 1;\n    let b = 2;\n    let c = 3;\n    let d = 4;\n    let e = 5;\n    let f = 6;\n    let g = 7;\n    let h = 8;\n    let i = 9;\n    let j = 10;\n    let k = a * b + c * d + e * f + g * h + i * j;\n    let l = (a + b) * (c + d) * (e + f) - (g + h) * (i + j);\n    return k * 10000 + l;\n}\n",
        1_899_946,
    ),
    (
        "deep_calls",
        "fn l4(x: i64) -> i64 {\n    return x + 4;\n}\n\nfn l3(x: i64) -> i64 {\n    return l4(x * 2) + 3;\n}\n\nfn l2(x: i64) -> i64 {\n    return l3(x + 1) * 2;\n}\n\nfn l1(x: i64, y: i64) -> i64 {\n    return l2(x) + l2(y);\n}\n\nfn main() -> i64 {\n    return l1(1, 2);\n}\n",
        48,
    ),
    (
        "recursion",
        "fn fact(n: i64) -> i64 {\n    if n <= 1 {\n        return 1;\n    }\n    return n * fact(n - 1);\n}\n\nfn main() -> i64 {\n    return fact(10);\n}\n",
        3_628_800,
    ),
    (
        "div_overflow_wraps",
        "fn div(a: i64, b: i64) -> i64 {\n    return a / b;\n}\n\nfn main() -> i64 {\n    let min = -9223372036854775807 - 1;\n    return div(min, -1);\n}\n",
        i64::MIN,
    ),
    (
        "rem_overflow_wraps",
        "fn rem(a: i64, b: i64) -> i64 {\n    return a % b;\n}\n\nfn main() -> i64 {\n    let min = -9223372036854775807 - 1;\n    return rem(min, -1) + 5;\n}\n",
        5,
    ),
    (
        "folded_div_overflow_wraps",
        "fn main() -> i64 {\n    return (-9223372036854775807 - 1) / -1;\n}\n",
        i64::MIN,
    ),
    (
        "folded_rem_overflow_wraps",
        "fn main() -> i64 {\n    return (-9223372036854775807 - 1) % -1 + 3;\n}\n",
        3,
    ),
    (
        "folded_add_overflow_wraps",
        "fn main() -> i64 {\n    return 9223372036854775807 + 1;\n}\n",
        i64::MIN,
    ),
    (
        "args_preserved_across_call",
        "fn id(x: i64) -> i64 {\n    return x;\n}\n\nfn f(a: i64, b: i64, c: i64) -> i64 {\n    let x = id(100);\n    return a * 100 + b * 10 + c + x;\n}\n\nfn main() -> i64 {\n    return f(1, 2, 3);\n}\n",
        223,
    ),
];

/// Control flow: every nesting the language allows, with the answer read off the source.
#[test]
fn control_flow_agrees_across_levels_orders_and_engines() {
    assert_no_failures(check_all(CONTROL_FLOW));
}

/// `for` loops, which the HIR desugars to `while`.
#[test]
fn for_loops_agree_across_levels_orders_and_engines() {
    assert_no_failures(check_all(FOR_LOOPS));
}

/// Arithmetic, comparison, logic and unary operators; many live temporaries at once.
#[test]
fn expressions_agree_across_levels_orders_and_engines() {
    assert_no_failures(check_all(EXPRESSIONS));
}

/// The compiler's own fixture corpus, at every level and in both orders.
#[test]
fn the_fixture_corpus_agrees_across_levels_orders_and_engines() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../x3-compiler/tests/fixtures");
    let corpus: &[(&str, i64)] = &[
        ("fib.x3", 55),
        ("loop_ops.x3", 16),
        ("match_cond.x3", 5),
        ("branch_fold.x3", 30),
        ("loop_sum.x3", 15),
        ("loop_break.x3", 21),
        ("loop_continue.x3", 12),
    ];
    let sources: Vec<(String, String, i64)> = corpus
        .iter()
        .map(|(name, expected)| {
            let source = std::fs::read_to_string(dir.join(name))
                .unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"));
            (name.to_string(), source, *expected)
        })
        .collect();
    let cases: Vec<(&str, &str, i64)> = sources
        .iter()
        .map(|(n, s, e)| (n.as_str(), s.as_str(), *e))
        .collect();
    assert_no_failures(check_all(&cases));
}

/// A construct the compiler cannot give its meaning to must be refused at compile time, not
/// compiled to a program that computes something else.
#[test]
fn constructs_without_a_lowering_are_refused_not_miscompiled() {
    let refused: &[(&str, &str)] = &[
        (
            "field_access",
            "fn f(x: i64) -> i64 {\n    return x.value;\n}\n\nfn main() -> i64 {\n    return f(3);\n}\n",
        ),
        (
            "range_value",
            "fn main() -> i64 {\n    let r = 0..3;\n    return 1;\n}\n",
        ),
    ];
    let mut accepted = Vec::new();
    for (name, source) in refused {
        for (level, options) in levels() {
            if Compiler::compile(source, options).is_ok() {
                accepted.push(format!("{name} [{level}]"));
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "these compiled although the compiler has no lowering that preserves their meaning:\n  {}",
        accepted.join("\n  ")
    );
}

/// Division by zero is a runtime error of the program, at every optimization level: the compiler
/// may not crash on it (a literal zero divisor used to panic the folder), may not fold it into a
/// constant, and neither engine may panic executing it.
#[test]
fn division_by_zero_compiles_and_fails_cleanly_on_both_engines() {
    let sources = [
        "fn main() -> i64 {\n    return 1 / 0;\n}\n",
        "fn main() -> i64 {\n    return 7 % 0;\n}\n",
        "fn div(a: i64, b: i64) -> i64 {\n    return a / b;\n}\n\nfn main() -> i64 {\n    return div(9, 0);\n}\n",
    ];
    for source in sources {
        for (level, options) in levels() {
            let compiled = std::panic::catch_unwind(|| Compiler::compile(source, options.clone()));
            let output = match compiled {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    panic!("[{level}] must compile (the error is the program's): {e:?}\n{source}")
                }
                Err(_) => panic!("[{level}] the compiler panicked on user source:\n{source}"),
            };
            let bytes = output.bytecode.to_bytes();
            let on_chain = std::panic::catch_unwind(|| {
                X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
            })
            .unwrap_or_else(|_| panic!("[{level}] X3Executor panicked:\n{source}"));
            let failed = match on_chain {
                Err(_) => true,
                Ok(receipt) => !receipt.success,
            };
            assert!(
                failed,
                "[{level}] X3Executor reported success for a division by zero:\n{source}"
            );
            let kernel = std::panic::catch_unwind(|| mini_x3::execute_x3bc(&bytes, KERNEL_GAS))
                .unwrap_or_else(|_| panic!("[{level}] mini_x3 panicked:\n{source}"));
            assert!(
                kernel.is_err(),
                "[{level}] mini_x3 reported success for a division by zero:\n{source}"
            );
        }
    }
}

/// Every optimizer pass, run **on its own**, must preserve what the unoptimized program computes.
///
/// The level tests above say *that* a pipeline miscompiles; this says *which pass* does, and it
/// holds each pass to the property independently of the order the pipeline happens to run them in —
/// a pass whose bug is masked by a later pass today is still a bug the next pipeline change exposes.
#[test]
fn every_optimizer_pass_preserves_semantics_on_its_own() {
    let mut failures = Vec::new();
    for (name, source, expected) in CONTROL_FLOW
        .iter()
        .chain(FOR_LOOPS)
        .chain(EXPRESSIONS)
        .chain(MUTABLE_CELLS)
    {
        let mut orders = vec![("as-written", source.to_string())];
        if let Some(reordered) = main_first(source) {
            orders.push(("main-first", reordered));
        }
        for (order, text) in &orders {
            let unoptimized =
                match Compiler::compile(text, CompilationOptions::no_opt().with_emit_mir(true)) {
                    Ok(output) => output
                        .artifacts
                        .and_then(|a| a.mir_unoptimized)
                        .expect("emit_mir retains the unoptimized MIR"),
                    // A program that does not compile at all is reported by the level tests.
                    Err(_) => continue,
                };
            let entry = entry_index(text);
            for pass in x3_opt::default_passes() {
                let mut mir = unoptimized.clone();
                if let Err(e) = pass.run(&mut mir) {
                    failures.push(format!(
                        "{name} [{}, {order}]: pass error: {e:?}",
                        pass.name()
                    ));
                    continue;
                }
                if let Some(entry) = entry {
                    let main = mir.functions.remove(entry);
                    mir.functions.insert(0, main);
                }
                let outcome = Compiler::compile_mir(&mir, CompilationOptions::no_opt())
                    .map_err(|e| format!("codegen: {e:?}"))
                    .and_then(|module| run_bytes(&module.to_bytes(), *expected));
                if let Err(why) = outcome {
                    failures.push(format!("{name} [{}, {order}]: {why}", pass.name()));
                }
            }
        }
    }
    assert_no_failures(failures);
}

/// Where `main` is among the source's `fn` items, which is its index in the MIR.
fn entry_index(source: &str) -> Option<usize> {
    source
        .lines()
        .filter(|line| line.starts_with("fn "))
        .position(|line| line.starts_with("fn main("))
}

/// Programs that are not well-typed or not well-formed. Each must be refused **at compile time**,
/// at every optimization level: the chain executes what the compiler emits, and a program the
/// compiler should have refused runs with whatever meaning the VM happens to give it.
const ILL_FORMED: &[(&str, &str)] = &[
    (
        "bool_arithmetic",
        "fn main() -> i64 {\n    let b = true;\n    return b + 1;\n}\n",
    ),
    (
        "return_type_mismatch",
        "fn f() -> bool {\n    return 5;\n}\n\nfn main() -> i64 {\n    if f() {\n        return 1;\n    }\n    return 0;\n}\n",
    ),
    (
        "argument_type_mismatch",
        "fn f(x: i64) -> i64 {\n    return x;\n}\n\nfn main() -> i64 {\n    return f(true);\n}\n",
    ),
    (
        "non_bool_condition",
        "fn main() -> i64 {\n    if 5 {\n        return 1;\n    }\n    return 0;\n}\n",
    ),
    (
        "unknown_identifier",
        "fn main() -> i64 {\n    return y;\n}\n",
    ),
    (
        "duplicate_function",
        "fn f() -> i64 {\n    return 1;\n}\n\nfn f() -> i64 {\n    return 2;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
    ),
    (
        "wrong_argument_count",
        "fn f(x: i64) -> i64 {\n    return x;\n}\n\nfn main() -> i64 {\n    return f(1, 2);\n}\n",
    ),
    (
        "assign_to_immutable",
        "fn main() -> i64 {\n    let x = 1;\n    x = 2;\n    return x;\n}\n",
    ),
    (
        "annotation_mismatch",
        "fn main() -> i64 {\n    let x: bool = 5;\n    return 1;\n}\n",
    ),
    (
        "missing_return",
        "fn f() -> i64 {\n    let x = 1;\n}\n\nfn main() -> i64 {\n    return f();\n}\n",
    ),
    (
        "compare_int_with_bool",
        "fn main() -> i64 {\n    if 1 < true {\n        return 1;\n    }\n    return 0;\n}\n",
    ),
    (
        "call_a_non_function",
        "fn main() -> i64 {\n    let x = 1;\n    return x();\n}\n",
    ),
    (
        "string_arithmetic",
        "fn main() -> i64 {\n    return \"a\" + 1;\n}\n",
    ),
    (
        "logical_and_on_integers",
        "fn main() -> i64 {\n    if 1 && 2 {\n        return 1;\n    }\n    return 0;\n}\n",
    ),
    (
        "assignment_changes_type",
        "fn main() -> i64 {\n    let mut x = 1;\n    x = true;\n    return 1;\n}\n",
    ),
    // Found by the `compile_and_run` fuzz target: the empty program compiled to a module the
    // chain refuses.
    ("empty_program", ""),
    // `compile_and_run` again: the lexer skipped control characters, so these compiled.
    (
        "nul_inside_a_statement",
        "fn main() -> i64 {\n    return\u{0} 1;\n}\n",
    ),
    (
        "backspace_before_an_expression",
        "fn main() -> i64 {\n    let x = 1;\n    \u{8}x;\n    return x;\n}\n",
    ),
    // Also from `compile_and_run`: `^` compiled to a call to a "builtin" no engine implements.
    (
        "power_operator",
        "fn main() -> i64 {\n    let x = 2;\n    return x ^ 3;\n}\n",
    ),
    (
        "no_main",
        "fn helper() -> i64 {\n    return 1;\n}\n",
    ),
    (
        "main_with_parameters",
        "fn main(x: i64) -> i64 {\n    return x;\n}\n",
    ),
    (
        "main_returns_the_wrong_type",
        "fn main() -> i64 {\n    return true;\n}\n",
    ),
];

#[test]
fn ill_formed_programs_are_refused_at_compile_time() {
    let mut accepted = Vec::new();
    for (name, source) in ILL_FORMED {
        for (level, options) in levels() {
            match std::panic::catch_unwind(|| Compiler::compile(source, options.clone())) {
                Ok(Ok(_)) => accepted.push(format!("{name} [{level}]")),
                Ok(Err(_)) => {}
                Err(_) => accepted.push(format!("{name} [{level}]: the compiler panicked")),
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "{} build(s) of ill-formed programs were accepted:\n  {}",
        accepted.len(),
        accepted.join("\n  ")
    );
}

/// Unbounded recursion ends the same way at every optimization level, on both engines. The
/// optimizer changes frame sizes, so the unoptimized build used to run out of register file first
/// and report `RegisterOutOfBounds` — a bad-operand error — while the optimized build hit the depth
/// limit (found by the `compile_and_run` fuzz target). Both are a stack overflow now.
#[test]
fn unbounded_recursion_is_a_stack_overflow_at_every_level() {
    let source = "fn f(n: i64) -> i64 {\n    let a = n + 1;\n    let b = a * 2;\n    let c = b - n;\n    return f(c);\n}\n\nfn main() -> i64 {\n    return f(0);\n}\n";
    for (level, options) in levels() {
        let bytes = Compiler::compile(source, options)
            .unwrap_or_else(|e| panic!("[{level}] must compile: {e:?}"))
            .bytecode
            .to_bytes();
        assert_eq!(
            mini_x3::execute_x3bc(&bytes, KERNEL_GAS).map(|r| r.return_val),
            Err(mini_x3::X3Error::StackOverflow),
            "[{level}] mini_x3"
        );
        let std = X3Executor::execute(&bytes, &[], X3ExecutorConfig::on_chain())
            .expect("a runtime fault is an unsuccessful receipt");
        assert!(!std.success, "[{level}] x3-vm reported success");
        let reason = String::from_utf8_lossy(&std.return_data);
        assert!(
            reason.contains("StackOverflow"),
            "[{level}] x3-vm: {reason}"
        );
    }
}
