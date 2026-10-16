//! The compiler refuses bad source; it never panics on it.
//!
//! A node that accepts X3 source (a deployment RPC, a gateway compiling submitted programs) runs
//! this pipeline on attacker-chosen bytes, so a panic anywhere from the lexer to the bytecode
//! emitter is a denial of service. Three generators, from shallow to deep:
//!
//! - arbitrary strings (exercise the lexer);
//! - soups of X3 tokens (get past the lexer and into the parser's recovery paths);
//! - token-level mutations of programs that compile (get past the parser and into the type
//!   checker, lowering, optimizer and emitter with almost-valid input).
//!
//! `PROPTEST_CASES` scales every generator; CI's default keeps the run short.

use std::panic::{catch_unwind, AssertUnwindSafe};

use proptest::prelude::*;
use x3_compiler::{CompilationOptions, Compiler};

/// Programs that compile, covering the language's constructs.
const SEEDS: &[&str] = &[
    "fn main() -> i64 { evm_sstore(3, 41); return evm_sload(3) + 1; }",
    "fn put(v: i64) { evm_sstore(1, v); } fn main() -> i64 { put(5); put(6); return evm_sload(1); }",
    "fn main() -> i64 { let mut i = 0; while i < 3 { evm_sstore(0, evm_sload(0) + 10); i = i + 1; } return evm_sload(0); }",
    "fn f(a: i64, b: i64) -> i64 { if a > b { return a - b; } else { return b - a; } } fn main() -> i64 { return f(3, 9) * 2; }",
    "fn main() -> i64 { let a = [1, 2, 3]; let mut s = 0; let mut i = 0; while i < a.len() { s = s + a[i]; i = i + 1; } return s; }",
    "fn main() -> i64 { let mut a = [0, 0, 0]; a[1] = 7; let t = (a[1], 2); return t[0] + t[1]; }",
    "fn sum(xs: [i64; 3]) -> i64 { return xs[0] + xs[1] + xs[2]; } fn main() -> i64 { return sum([4, 5, 6]); }",
    "fn main() -> i64 { let x = ctx_chain_id(); let h = ctx_block_height(); return x + h; }",
    "fn main() -> i64 { let mut n = 10; let mut acc = 1; while n > 0 { acc = acc * 2 % 1000; n = n - 1; } return acc; }",
    "fn main() -> bool { let a = 3; let b = 4; return a < b && !(a == b) || false; }",
    "struct P { x: i64, y: i64 } fn main() -> i64 { let mut p = P { y: 2, x: 1 }; p.x = p.x + p.y; return p.x; }",
    "struct L { v: i64, a: [i64; 2] } struct R { l: L } fn main() -> i64 { let r = R { l: L { v: 1, a: [2, 3] } }; return r.l.a[1] + r.l.v; }",
    "fn main() -> i64 { let mut t = (1, 2); t.1 = 5; return t.0 + t.1; }",
];

const TOKENS: &[&str] = &[
    "fn",
    "let",
    "mut",
    "if",
    "else",
    "while",
    "for",
    "in",
    "return",
    "atomic",
    "struct",
    "true",
    "false",
    "main",
    "x",
    "y",
    "i64",
    "u64",
    "bool",
    "len",
    "evm_sload",
    "evm_sstore",
    "ctx_chain_id",
    "(",
    ")",
    "{",
    "}",
    "[",
    "]",
    ",",
    ";",
    ":",
    "->",
    "=",
    "==",
    "!=",
    "<",
    ">",
    "<=",
    ">=",
    "+",
    "-",
    "*",
    "/",
    "%",
    "&&",
    "||",
    "!",
    ".",
    "0",
    "1",
    "255",
    "9223372036854775807",
    "-9223372036854775808",
    "18446744073709551616",
    "\"s\"",
    "'",
    "#",
    "//",
    "/*",
    "*/",
    "@",
    "_",
    "..",
    "::",
];

/// Compile on a thread with Rust's default spawned-thread stack (2 MiB, what a worker thread in a
/// node gets), so unbounded recursion shows up here rather than only in production.
fn compiles_without_panicking(source: String) -> Result<(), String> {
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            catch_unwind(AssertUnwindSafe(|| {
                let _ = Compiler::compile(&source, CompilationOptions::contract_mode());
            }))
            .map_err(|payload| {
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                format!("panicked: {message}\nsource: {source:?}")
            })
        })
        .expect("spawn");
    handle.join().unwrap_or_else(|_| Err("thread died".into()))
}

fn tokenize(source: &str) -> Vec<String> {
    const PAIRS: [&str; 8] = ["->", "==", "!=", "<=", ">=", "&&", "||", "::"];
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_alphanumeric() || ch == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(chars[start..i].iter().collect());
        } else {
            let pair: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            if PAIRS.contains(&pair.as_str()) {
                out.push(pair);
                i += 2;
            } else {
                out.push(ch.to_string());
                i += 1;
            }
        }
    }
    out
}

#[derive(Debug, Clone)]
enum Edit {
    Delete(usize),
    Duplicate(usize),
    Swap(usize, usize),
    Replace(usize, usize),
    Insert(usize, usize),
}

fn apply(tokens: &mut Vec<String>, edit: &Edit) {
    if tokens.is_empty() {
        return;
    }
    let n = tokens.len();
    match *edit {
        Edit::Delete(i) => {
            tokens.remove(i % n);
        }
        Edit::Duplicate(i) => {
            let t = tokens[i % n].clone();
            tokens.insert(i % n, t);
        }
        Edit::Swap(i, j) => tokens.swap(i % n, j % n),
        Edit::Replace(i, t) => tokens[i % n] = TOKENS[t % TOKENS.len()].to_string(),
        Edit::Insert(i, t) => tokens.insert(i % (n + 1), TOKENS[t % TOKENS.len()].to_string()),
    }
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        any::<usize>().prop_map(Edit::Delete),
        any::<usize>().prop_map(Edit::Duplicate),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Swap(a, b)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Replace(a, b)),
        (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Edit::Insert(a, b)),
    ]
}

#[test]
fn every_seed_compiles_before_and_after_tokenizing() {
    for seed in SEEDS {
        Compiler::compile(seed, CompilationOptions::contract_mode())
            .unwrap_or_else(|e| panic!("seed must compile: {seed}\n{e:?}"));
        let respaced = tokenize(seed).join(" ");
        Compiler::compile(&respaced, CompilationOptions::contract_mode())
            .unwrap_or_else(|e| panic!("re-spaced seed must compile: {respaced}\n{e:?}"));
    }
}

proptest! {
    #[test]
    fn arbitrary_text_never_panics(source in ".{0,200}") {
        prop_assert_eq!(compiles_without_panicking(source), Ok(()));
    }

    #[test]
    fn token_soup_never_panics(picks in prop::collection::vec(any::<usize>(), 0..60)) {
        let source = picks
            .iter()
            .map(|i| TOKENS[i % TOKENS.len()])
            .collect::<Vec<_>>()
            .join(" ");
        prop_assert_eq!(compiles_without_panicking(source), Ok(()));
    }

    #[test]
    fn mutated_programs_never_panic(
        seed in 0..SEEDS.len(),
        edits in prop::collection::vec(edit(), 1..4),
    ) {
        let mut tokens = tokenize(SEEDS[seed]);
        for e in &edits {
            apply(&mut tokens, e);
        }
        prop_assert_eq!(compiles_without_panicking(tokens.join(" ")), Ok(()));
    }
}

/// Nesting is bounded by a limit the parser reports, not by the stack.
fn nested(open: &str, close: &str, depth: usize) -> String {
    format!(
        "fn main() -> i64 {{ {} return 1; {} return 0; }}",
        open.repeat(depth),
        close.repeat(depth)
    )
}

/// Under the limit, at it, past it, and far past it.
const DEPTHS: [usize; 5] = [32, 63, 64, 1_000, 20_000];

#[test]
fn deep_parentheses_are_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!(
            "fn main() -> i64 {{ return {}1{}; }}",
            "(".repeat(depth),
            ")".repeat(depth)
        );
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

#[test]
fn deep_unary_chains_are_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!("fn main() -> i64 {{ return {}1; }}", "-".repeat(depth));
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

#[test]
fn deep_array_literals_are_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!(
            "fn main() -> i64 {{ let a = {}1{}; return 0; }}",
            "[".repeat(depth),
            "]".repeat(depth)
        );
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

#[test]
fn deep_if_blocks_are_refused_not_overflowed() {
    for depth in DEPTHS {
        assert_eq!(
            compiles_without_panicking(nested("if true { ", "}", depth)),
            Ok(()),
            "depth {depth}"
        );
    }
}

#[test]
fn deep_while_blocks_are_refused_not_overflowed() {
    for depth in DEPTHS {
        assert_eq!(
            compiles_without_panicking(nested("while false { ", "}", depth)),
            Ok(()),
            "depth {depth}"
        );
    }
}

#[test]
fn deep_atomic_blocks_are_refused_not_overflowed() {
    for depth in DEPTHS {
        assert_eq!(
            compiles_without_panicking(nested("atomic { ", "}", depth)),
            Ok(()),
            "depth {depth}"
        );
    }
}

#[test]
fn long_else_if_chains_are_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!(
            "fn main() -> i64 {{ let x = 3; if x == 0 {{ return 0; }} {} else {{ return 9; }} }}",
            "else if x == 1 { return 1; } ".repeat(depth)
        );
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

#[test]
fn deep_mixed_nesting_is_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!(
            "fn main() -> i64 {{ {} return 1; {} return 0; }}",
            "while false { if (1 + [2][0]) > 0 { ".repeat(depth),
            "} }".repeat(depth)
        );
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

#[test]
fn deep_nested_types_are_refused_not_overflowed() {
    for depth in DEPTHS {
        let source = format!(
            "fn f(a: {}i64{}) -> i64 {{ return 0; }} fn main() -> i64 {{ return 0; }}",
            "[".repeat(depth),
            "; 1]".repeat(depth)
        );
        assert_eq!(compiles_without_panicking(source), Ok(()), "depth {depth}");
    }
}

/// A program at the limit still compiles; one level past it is a parse error, not a crash.
#[test]
fn the_nesting_limit_is_a_reported_error() {
    let at = |depth: usize| {
        let source = nested("if true { ", "}", depth);
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || {
                Compiler::compile(&source, CompilationOptions::contract_mode())
                    .map(|_| ())
                    .map_err(|e| format!("{e:?}"))
            })
            .expect("spawn")
            .join()
            .expect("no overflow")
    };
    assert!(at(60).is_ok(), "{:?}", at(60).err());
    let refused = format!("{:?}", at(65).expect_err("too deep"));
    assert!(refused.contains("maximum nesting depth"), "{refused}");
}

/// The mutation generator is only worth something if its programs get past the parser. Run a
/// fixed sample and require that a real share of them compile all the way to bytecode and a real
/// share fail *after* parsing — so the passes behind the parser are what is being exercised.
#[test]
fn mutations_reach_the_passes_behind_the_parser() {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as usize
    };
    let (mut compiled, mut past_parser, total) = (0, 0, 2_000);
    for _ in 0..total {
        let mut tokens = tokenize(SEEDS[next() % SEEDS.len()]);
        let edit = match next() % 5 {
            0 => Edit::Delete(next()),
            1 => Edit::Duplicate(next()),
            2 => Edit::Swap(next(), next()),
            3 => Edit::Replace(next(), next()),
            _ => Edit::Insert(next(), next()),
        };
        apply(&mut tokens, &edit);
        match Compiler::compile(&tokens.join(" "), CompilationOptions::contract_mode()) {
            Ok(_) => compiled += 1,
            Err(error) if !format!("{error:?}").starts_with("Parser") => past_parser += 1,
            Err(_) => {}
        }
    }
    assert!(
        compiled * 50 >= total,
        "only {compiled}/{total} mutants compiled"
    );
    assert!(
        past_parser * 20 >= total,
        "only {past_parser}/{total} mutants failed after parsing"
    );
}
