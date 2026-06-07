//! Custom dylint lints enforcing determinism in x3-chain runtime code.
//!
//! | Lint | Level | Description |
//! |------|-------|-------------|
//! | `HASHMAP_NONDETERMINISM`      | warn | `HashMap`/`HashSet` → use `BTreeMap`/`BTreeSet` |
//! | `SYSTEM_TIME_NONDETERMINISM`  | warn | `SystemTime::now()` → use pallet block-timestamp |
//! | `FLOAT_ARITHMETIC_NONDETERMINISM` | warn | `f32`/`f64` arithmetic is platform-dependent |
//! | `PANIC_IN_RUNTIME`            | warn | `panic!`/`unwrap()`/`expect()` crash nodes without reverts |
//! | `STATIC_MUT_NONDETERMINISM`   | warn | `static mut` = per-node global state, breaks consensus |

#![feature(rustc_private)]

// Required to make rustc_private rlib crates available.
extern crate rustc_driver;

extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_hir::{Expr, ExprKind, Item, ItemKind, Mutability, QPath};
use rustc_lint::{LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, declare_lint_pass, Session};

declare_lint! {
    /// Replace with `BTreeMap`/`BTreeSet` for deterministic iteration.
    pub HASHMAP_NONDETERMINISM,
    Warn,
    "use BTreeMap/BTreeSet instead of HashMap/HashSet for deterministic iteration in runtime code"
}

declare_lint! {
    /// Use `pallet_timestamp::Pallet::<T>::get()` for on-chain time instead.
    pub SYSTEM_TIME_NONDETERMINISM,
    Warn,
    "do not use SystemTime::now() or Instant::now() in runtime code; read block timestamp from pallet storage"
}

declare_lint! {
    /// `f32`/`f64` arithmetic is not IEEE 754 reproducible across different CPUs,
    /// compilers, or optimisation levels.  Two validators may produce different
    /// `f64` results for the same computation, diverging their state roots.
    ///
    /// **Fix:** use integer arithmetic, fixed-point, or a deterministic bigint
    /// library (e.g. `sp_arithmetic::FixedU128`).
    pub FLOAT_ARITHMETIC_NONDETERMINISM,
    Warn,
    "floating-point arithmetic is non-deterministic across platforms; use integer or fixed-point arithmetic"
}

declare_lint! {
    /// `panic!`, `.unwrap()`, and `.expect()` in runtime dispatch code abort the
    /// STF without reverting storage, leaving state inconsistent across nodes.
    ///
    /// **Fix:** return `Err(Error::<T>::SomeVariant)` or use `ensure!(cond, Error::<T>::...)`.
    pub PANIC_IN_RUNTIME,
    Warn,
    "panic/unwrap/expect in runtime code crashes nodes without storage revert; return Err(...) instead"
}

declare_lint! {
    /// `static mut` items hold per-process mutable state that is NOT shared
    /// between pallets or reset between blocks.  Two validators may observe
    /// different values if one was restarted, producing divergent state roots.
    ///
    /// **Fix:** store data in FRAME storage (`#[pallet::storage]`) or in
    /// `sp_externalities`.
    pub STATIC_MUT_NONDETERMINISM,
    Warn,
    "static mut is per-node mutable state; store data in FRAME storage to keep all validators in sync"
}

declare_lint! {
    /// `thread_local!` storage is per-thread and therefore per-node.
    /// Two validators executing the same extrinsic may observe different TLS
    /// values, producing divergent state roots.
    ///
    /// **Fix:** store shared state in FRAME `#[pallet::storage]` or `sp_externalities`.
    pub THREAD_LOCAL_NONDETERMINISM,
    Warn,
    "thread_local! is per-node state and breaks consensus determinism; use FRAME storage instead"
}

declare_lint! {
    /// Calls to `std::fs`, `std::net`, or `std::process::Command` perform
    /// blocking I/O that is unavailable in the WASM runtime and will differ
    /// between nodes (file contents, network responses, exit codes).
    ///
    /// **Fix:** remove all I/O from runtime dispatch code; use off-chain workers
    /// (`#[pallet::hooks]` `offchain_worker`) for any external I/O.
    pub BLOCKING_IO_NONDETERMINISM,
    Warn,
    "blocking I/O (fs/net/process) is unavailable in WASM runtime and non-deterministic; use off-chain workers"
}

declare_lint_pass!(DeterminismLints => [
    HASHMAP_NONDETERMINISM,
    SYSTEM_TIME_NONDETERMINISM,
    FLOAT_ARITHMETIC_NONDETERMINISM,
    PANIC_IN_RUNTIME,
    STATIC_MUT_NONDETERMINISM,
    THREAD_LOCAL_NONDETERMINISM,
    BLOCKING_IO_NONDETERMINISM,
]);

impl LateLintPass<'_> for DeterminismLints {
    fn check_path(
        &mut self,
        cx: &LateContext<'_>,
        path: &rustc_hir::Path<'_>,
        _id: rustc_hir::HirId,
    ) {
        let segs: Vec<&str> = path.segments.iter().map(|s| s.ident.name.as_str()).collect();

        let has_coll = segs.iter().any(|&s| s == "collections");
        let has_hash_type = segs.iter().any(|&s| s == "HashMap" || s == "HashSet");
        if has_coll && has_hash_type {
            cx.span_lint(HASHMAP_NONDETERMINISM, path.span, |diag| {
                diag.help("replace with `BTreeMap`/`BTreeSet` for deterministic iteration order");
            });
        }

        let has_time_mod = segs.iter().any(|&s| s == "time");
        let has_time_type = segs.iter().any(|&s| s == "SystemTime" || s == "Instant");
        if has_time_mod && has_time_type {
            cx.span_lint(SYSTEM_TIME_NONDETERMINISM, path.span, |diag| {
                diag.help("use `pallet_timestamp::Pallet::<T>::get()` for on-chain time");
            });
        }

        // Detect f32 / f64 type paths (e.g. `let x: f64 = ...` or `f64::from(...)`)
        let has_float = segs.iter().any(|&s| s == "f32" || s == "f64");
        if has_float {
            cx.span_lint(FLOAT_ARITHMETIC_NONDETERMINISM, path.span, |diag| {
                diag.help("use `sp_arithmetic::FixedU128` or integer arithmetic instead of f32/f64");
            });
        }

        // Detect thread_local! accessor paths (the macro creates __getit fns)
        if segs.iter().any(|&s| s == "__getit" || s == "LocalKey") {
            cx.span_lint(THREAD_LOCAL_NONDETERMINISM, path.span, |diag| {
                diag.help("replace thread_local! with FRAME #[pallet::storage] or sp_externalities");
            });
        }

        // Detect blocking I/O paths: std::fs, std::net, std::process
        let has_blocking_ns = segs.iter().any(|&s| s == "fs" || s == "net" || s == "process");
        let has_std = segs.iter().any(|&s| s == "std" || s == "std_io");
        if has_blocking_ns && has_std {
            cx.span_lint(BLOCKING_IO_NONDETERMINISM, path.span, |diag| {
                diag.help("I/O is unavailable in WASM runtime; move to an off-chain worker instead");
            });
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'_>, expr: &Expr<'_>) {
        // ── method-call checks: .unwrap(), .expect(), .unwrap_or_else … ──────
        if let ExprKind::MethodCall(method, _recv, _args, _span) = &expr.kind {
            let name = method.ident.name.as_str();
            if matches!(name, "unwrap" | "expect" | "unwrap_unchecked") {
                cx.span_lint(PANIC_IN_RUNTIME, expr.span, |diag| {
                    diag.help("return `Err(Error::<T>::...)` instead of calling .unwrap()/.expect() in runtime code");
                });
            }
        }

        // ── function-call checks ─────────────────────────────────────────────
        let ExprKind::Call(func, _args) = &expr.kind else {
            return;
        };

        if let ExprKind::Path(QPath::Resolved(_, path)) = &func.kind {
            let path_str: String = path
                .segments
                .iter()
                .map(|s| s.ident.name.as_str())
                .collect::<Vec<_>>()
                .join("::");

            // HashMap/HashSet constructors
            let is_hashmap_ctor = (path_str.contains("HashMap") || path_str.contains("HashSet"))
                && (path_str.ends_with("new") || path_str.ends_with("with_capacity"));
            if is_hashmap_ctor {
                cx.span_lint(HASHMAP_NONDETERMINISM, expr.span, |diag| {
                    diag.help("replace with `BTreeMap::new()` / `BTreeSet::new()`");
                });
            }

            // panic!() desugars to a call through core::panicking
            if path_str.contains("panicking") || path_str.ends_with("panic") {
                cx.span_lint(PANIC_IN_RUNTIME, expr.span, |diag| {
                    diag.help("use `fail!(Error::<T>::...)` or return `Err(...)` instead of panicking in runtime code");
                });
            }
        }
    }

    // ── check_item: catch static mut items ──────────────────────────────────
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if let ItemKind::Static(mutability, _ident, _ty, _body) = item.kind {
            if mutability == Mutability::Mut {
                cx.span_lint(STATIC_MUT_NONDETERMINISM, item.span, |diag| {
                    diag.help("replace `static mut` with FRAME `#[pallet::storage]` or `sp_externalities`");
                });
            }
        }
    }
}

#[no_mangle]
pub fn register_lints(_sess: &Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        &HASHMAP_NONDETERMINISM,
        &SYSTEM_TIME_NONDETERMINISM,
        &FLOAT_ARITHMETIC_NONDETERMINISM,
        &PANIC_IN_RUNTIME,
        &STATIC_MUT_NONDETERMINISM,
        &THREAD_LOCAL_NONDETERMINISM,
        &BLOCKING_IO_NONDETERMINISM,
    ]);
    lint_store.register_late_pass(|_| Box::new(DeterminismLints));
}

#[no_mangle]
pub fn register_pre_expansion_lints(_sess: &Session, _lint_store: &mut LintStore) {}

#[no_mangle]
pub fn register_early_lint_passes(_sess: &Session, _lint_store: &mut LintStore) {}

/// Version marker required by the dylint driver to verify ABI compatibility.
#[no_mangle]
pub extern "C" fn dylint_version() -> *mut ::std::os::raw::c_char {
    ::std::ffi::CString::new(dylint_linting::DYLINT_VERSION)
        .unwrap()
        .into_raw()
}
