//! Compilation performance benchmarks for x3-compiler.
//!
//! Measures wall-clock time for end-to-end Source → Bytecode compilation
//! at various optimization levels and for different program complexities.
//!
//! Run with:
//!   cargo bench --manifest-path crates/x3-compiler/Cargo.toml
//!
//! Results are written to target/criterion/.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use x3_compiler::{CompilationOptions, Compiler, OptLevel};

// ── bench helpers ─────────────────────────────────────────────────────────────

fn read_fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{}.x3",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {}.x3", name))
}

// ── benchmarks ───────────────────────────────────────────────────────────────

/// Micro: compile a minimal two-function program at O0 and O3.
fn bench_compile_simple(c: &mut Criterion) {
    let source = r#"
        fn add(a: i64, b: i64) -> i64 {
            return a + b;
        }

        fn main() -> i64 {
            return add(3, 4);
        }
    "#;

    let mut group = c.benchmark_group("compile_simple");

    group.bench_function("O0", |b| {
        b.iter(|| {
            let opts = CompilationOptions {
                opt_level: OptLevel::None,
                ..Default::default()
            };
            Compiler::compile(black_box(source), opts).unwrap()
        })
    });

    group.bench_function("O3", |b| {
        b.iter(|| {
            let opts = CompilationOptions {
                opt_level: OptLevel::Aggressive,
                ..Default::default()
            };
            Compiler::compile(black_box(source), opts).unwrap()
        })
    });

    group.finish();
}

/// Domain: compile route.x3 (multi-hop cost, 3 helper functions) at O2.
fn bench_compile_route(c: &mut Criterion) {
    let source = read_fixture("route");

    c.bench_function("compile_route_o2", |b| {
        b.iter(|| {
            let opts = CompilationOptions::opt2();
            Compiler::compile(black_box(source.as_str()), opts).unwrap()
        })
    });
}

/// Domain: compile swap.x3 (AMM, 4 functions, integer division) at O2.
fn bench_compile_swap(c: &mut Criterion) {
    let source = read_fixture("swap");

    c.bench_function("compile_swap_o2", |b| {
        b.iter(|| {
            let opts = CompilationOptions::opt2();
            Compiler::compile(black_box(source.as_str()), opts).unwrap()
        })
    });
}

/// Recursive: compile fib.x3 at O3 (constant-fold + inline opportunities).
fn bench_compile_fib_o3(c: &mut Criterion) {
    let source = read_fixture("fib");

    c.bench_function("compile_fib_o3", |b| {
        b.iter(|| {
            let opts = CompilationOptions {
                opt_level: OptLevel::Aggressive,
                ..Default::default()
            };
            Compiler::compile(black_box(source.as_str()), opts).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_compile_simple,
    bench_compile_route,
    bench_compile_swap,
    bench_compile_fib_o3,
);
criterion_main!(benches);
