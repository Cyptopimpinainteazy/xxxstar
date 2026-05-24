//! Domain-specific end-to-end tests: routes, swaps, auctions.
//!
//! These tests validate that the x3-compiler correctly handles programs
//! modelling real DeFi and on-chain coordination use cases.

use x3_compiler::{CompilationOptions, Compiler, OptLevel};

// ── helpers ──────────────────────────────────────────────────────────────────

fn compile_fixture_domain(name: &str, opt: OptLevel) -> x3_compiler::CompilationOutput {
    let path = format!("{}/tests/fixtures/{}.x3", env!("CARGO_MANIFEST_DIR"), name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("fixture not found: {}.x3", name));
    let options = CompilationOptions {
        opt_level: opt,
        emit_stats: true,
        ..Default::default()
    };
    Compiler::compile(&source, options)
        .unwrap_or_else(|e| panic!("compile failed for {}.x3: {:?}", name, e))
}

fn bytecode_size(name: &str, opt: OptLevel) -> usize {
    compile_fixture_domain(name, opt).bytecode.code.len()
}

// ── route tests ──────────────────────────────────────────────────────────────

#[test]
fn test_route_compiles_o2() {
    let out = compile_fixture_domain("route", OptLevel::Default);
    assert!(out.bytecode.code.len() > 0, "route.x3 should produce non-empty bytecode");
}

#[test]
fn test_route_o3_not_larger_than_o0() {
    let o0 = bytecode_size("route", OptLevel::None);
    let o3 = bytecode_size("route", OptLevel::Aggressive);
    assert!(
        o3 <= o0,
        "O3 bytecode should be <= O0 for route.x3: O0={o0}, O3={o3}"
    );
}

#[test]
fn test_route_deterministic() {
    let opts = CompilationOptions::opt2();
    let path = format!("{}/tests/fixtures/route.x3", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path).expect("route.x3 fixture not found");

    let r1 = Compiler::compile(&source, opts.clone()).unwrap();
    let r2 = Compiler::compile(&source, opts.clone()).unwrap();
    assert_eq!(
        r1.bytecode.code, r2.bytecode.code,
        "route.x3 must produce identical bytecode on repeated compilation"
    );
}

// ── swap tests ───────────────────────────────────────────────────────────────

#[test]
fn test_swap_compiles_o2() {
    let out = compile_fixture_domain("swap", OptLevel::Default);
    assert!(out.bytecode.code.len() > 0, "swap.x3 should produce non-empty bytecode");
}

#[test]
fn test_swap_function_count() {
    // swap.x3 defines 4 functions: fee_adjusted_input, swap_output, swap_with_fee, main
    // The bytecode emitter assigns synthetic names (fn_0, fn_1, …); verify the count.
    let out = compile_fixture_domain("swap", OptLevel::Default);
    assert_eq!(
        out.bytecode.functions.len(),
        4,
        "swap.x3 should produce exactly 4 function entries; got: {:?}",
        out.bytecode.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_swap_o3_not_larger_than_o0() {
    let o0 = bytecode_size("swap", OptLevel::None);
    let o3 = bytecode_size("swap", OptLevel::Aggressive);
    assert!(
        o3 <= o0,
        "O3 bytecode should be <= O0 for swap.x3: O0={o0}, O3={o3}"
    );
}

// ── auction tests ─────────────────────────────────────────────────────────────

#[test]
fn test_auction_compiles_o2() {
    let out = compile_fixture_domain("auction", OptLevel::Default);
    assert!(out.bytecode.code.len() > 0, "auction.x3 should produce non-empty bytecode");
}

#[test]
fn test_auction_function_count() {
    // auction.x3 defines 3 functions: highest_bid, is_reserve_met, main
    // The bytecode emitter assigns synthetic names (fn_0, fn_1, …); verify the count.
    let out = compile_fixture_domain("auction", OptLevel::Default);
    assert_eq!(
        out.bytecode.functions.len(),
        3,
        "auction.x3 should produce exactly 3 function entries; got: {:?}",
        out.bytecode.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_auction_all_opt_levels_succeed() {
    for opt in [OptLevel::None, OptLevel::Basic, OptLevel::Default, OptLevel::Aggressive] {
        let out = compile_fixture_domain("auction", opt);
        assert!(
            out.bytecode.code.len() > 0,
            "auction.x3 should compile at {:?}",
            opt
        );
    }
}
