use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// The feature set that decides which `construct_runtime!` variant is compiled,
/// and therefore which pallets exist in `RuntimeGenesisConfig`. A cached runtime
/// blob built for a different set can never be embedded into this build: the
/// node would boot a runtime whose genesis schema does not match the chain spec
/// it was handed ("unknown field `depinMarketplace`, expected one of ...").
///
/// This mirrors what `substrate-wasm-builder` passes to the nested build: the
/// outer feature list minus `std` (verified against the generated
/// `target/<profile>/wbuild/x3-chain-runtime/Cargo.toml`). Deriving it from
/// `CARGO_FEATURE_*` rather than a hardcoded list means a new variant feature is
/// covered automatically.
fn enabled_features_key() -> String {
    let mut features: Vec<String> = env::vars()
        .filter_map(|(key, _)| {
            key.strip_prefix("CARGO_FEATURE_")
                .map(|name| name.to_lowercase().replace('_', "-"))
        })
        // `std` is dropped because the WASM build never gets it; `default` is a
        // cargo marker, not a feature of this crate.
        .filter(|name| name != "std" && name != "default")
        .collect();
    features.sort();
    features.dedup();
    features.join(",")
}

/// Sidecar next to the built blob recording the feature set it was built with.
fn sidecar_path(wasm: &Path) -> PathBuf {
    let mut name = wasm
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "x3_chain_runtime.wasm".into());
    name.push(".features");
    wasm.with_file_name(name)
}

fn write_variant_sidecar(wasm: &Path, key: &str) {
    let _ = fs::write(sidecar_path(wasm), format!("{key}\n"));
}

fn cached_variant_matches(wasm: &Path, key: &str) -> Result<(), String> {
    let sidecar = sidecar_path(wasm);
    let recorded = fs::read_to_string(&sidecar).map_err(|_| {
        format!(
            "cached runtime WASM at {} has no feature sidecar ({})",
            wasm.display(),
            sidecar.display()
        )
    })?;
    let recorded = recorded.trim();
    if recorded == key {
        Ok(())
    } else {
        Err(format!(
            "cached runtime WASM was built for features [{recorded}] but this build is [{key}]"
        ))
    }
}

fn write_wasm_binary_stub(reason: &str) {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let wasm_binary_path = PathBuf::from(out_dir).join("wasm_binary.rs");
    let stub = r#"pub const WASM_BINARY: Option<&[u8]> = None;
pub const WASM_BINARY_BLOATY: Option<&[u8]> = None;
"#;

    fs::write(&wasm_binary_path, stub).expect("failed to write wasm_binary.rs stub");
    println!(
        "cargo:warning=runtime/build.rs wrote stub wasm_binary.rs ({reason}); embedded runtime WASM will be unavailable"
    );
}

/// Remove the runtime build outputs so `substrate-wasm-builder` has to rebuild them.
///
/// Its own freshness check is based on source timestamps, and this crate's feature set is
/// not a source file — so a blob built with `runtime-benchmarks` looks fresh to a build
/// without it and gets embedded anyway. The node then fails to start with "runtime
/// requires function imports which are not present on the host:
/// env:ext_benchmarking_*". Measured on 2026-09-25: `target/release/x3-chain-node` was
/// unrunnable for exactly that reason, and the sidecar written below said so only *after*
/// the previous build had overwritten it with its own key.
fn remove_stale_wasm_outputs(wasm: &Path) {
    let mut removed = Vec::new();
    for candidate in [
        wasm.to_path_buf(),
        wasm.with_file_name("x3_chain_runtime.compact.wasm"),
        wasm.with_file_name("x3_chain_runtime.compact.compressed.wasm"),
        sidecar_path(wasm),
    ] {
        if candidate.exists() && fs::remove_file(&candidate).is_ok() {
            removed.push(candidate);
        }
    }
    println!(
        "cargo:warning=the cached runtime WASM was built for a different feature set; removed {} so it is rebuilt for this one",
        removed
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn cached_runtime_wasm_path() -> Option<PathBuf> {
    let profile = env::var("PROFILE").ok()?;
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let workspace_root = manifest_dir.parent()?.to_path_buf();
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));

    let cached = target_dir
        .join(profile)
        .join("wbuild/x3-chain-runtime/x3_chain_runtime.wasm");
    cached.is_file().then_some(cached)
}

fn write_cached_wasm_binary(cached: &Path) {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set by cargo");
    let copied_wasm = PathBuf::from(&out_dir).join("x3_chain_runtime.wasm");
    fs::copy(cached, &copied_wasm).expect("failed to copy cached runtime WASM");

    let wasm_binary_path = PathBuf::from(&out_dir).join("wasm_binary.rs");
    let source = r#"pub const WASM_BINARY: Option<&[u8]> =
    Some(include_bytes!(concat!(env!("OUT_DIR"), "/x3_chain_runtime.wasm")));
pub const WASM_BINARY_BLOATY: Option<&[u8]> = None;
"#;

    fs::write(&wasm_binary_path, source).expect("failed to write wasm_binary.rs");
    println!(
        "cargo:warning=runtime/build.rs embedded cached runtime WASM from {}",
        cached.display()
    );
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SKIP_WASM_BUILD");
    println!("cargo:rerun-if-env-changed=TARGET");

    // If we're *already* compiling the runtime for the WASM target, don't try to
    // invoke `substrate-wasm-builder` again.
    if env::var("TARGET").as_deref() == Ok("wasm32-unknown-unknown") {
        write_wasm_binary_stub("TARGET=wasm32-unknown-unknown; skipping embedded WASM build");
        return;
    }

    // When `substrate-wasm-builder` invokes a nested cargo build to produce the
    // WASM runtime, it sets `SKIP_WASM_BUILD` to prevent recursive rebuilds.
    // Honor it here so the workspace build doesn't spiral into nested builds.
    if env::var_os("SKIP_WASM_BUILD").is_some() {
        // A build with `SKIP_WASM_BUILD` should not recompile WASM, but tests
        // and dev nodes still need a real embedded runtime when a previously
        // built artifact is available. That artifact is only usable when it was
        // built for *this* variant: the runtime variant decides which pallets
        // exist in `RuntimeGenesisConfig`, so embedding a blob from another
        // feature set makes the node reject its own chain spec at boot
        // ("unknown field `depinMarketplace`, expected one of ..."). Cache reuse
        // is therefore gated on the recorded feature set; a mismatch falls back
        // to the None stub, which fails loudly at the call site instead.
        let features = enabled_features_key();
        if let Some(cached) = cached_runtime_wasm_path() {
            match cached_variant_matches(&cached, &features) {
                Ok(()) => {
                    write_cached_wasm_binary(&cached);
                    return;
                }
                Err(reason) => {
                    write_wasm_binary_stub(&format!(
                        "SKIP_WASM_BUILD is set and the cached runtime WASM is not usable: {reason}. \
Rebuild the embedded runtime for this variant with `env -u SKIP_WASM_BUILD cargo build -p x3-chain-node` \
(or `cargo build -p x3-chain-runtime`), then re-run."
                    ));
                    return;
                }
            }
        }
        write_wasm_binary_stub("SKIP_WASM_BUILD is set; skipping runtime WASM build");
        return;
    }

    // `substrate-wasm-builder` will skip its build when the existing blob looks fresh, and
    // "fresh" is decided from source timestamps, not from this crate's feature set. A blob
    // built with `runtime-benchmarks` therefore survives a build without it, and the node
    // embeds a runtime whose host functions it does not provide — the binary cannot start.
    // The sidecar written at the end of this function is a *key*, so it has to be read
    // before the build as well: a mismatch means the stale outputs go, which forces
    // `build()` below to produce a blob for this feature set.
    if let Some(existing) = cached_runtime_wasm_path() {
        if cached_variant_matches(&existing, &enabled_features_key()).is_err() {
            remove_stale_wasm_outputs(&existing);
        }
    }

    // Always build WASM from source with correct flags

    // Set WASM_BUILD_NO_COLOR to avoid ANSI codes in build output
    env::set_var("WASM_BUILD_NO_COLOR", "1");
    // Disable wasm-opt completely - it can cause issues with reference types
    env::set_var("WASM_BUILD_USE_WASM_OPT", "0");

    // Use substrate-wasm-builder to build the WASM runtime
    // This will generate wasm_binary.rs in OUT_DIR automatically
    substrate_wasm_builder::WasmBuilder::new()
        .with_current_project()
        .append_to_rust_flags("-C target-cpu=mvp")
        .append_to_rust_flags("-C target-feature=-sign-ext,-reference-types,-bulk-memory")
        .export_heap_base()
        // Leave the output shim file name as the default (`wasm_binary.rs`).
        .build();

    // Record the feature set for this blob so a later `SKIP_WASM_BUILD` build can
    // prove the cache belongs to its own variant before embedding it.
    if let Some(built) = cached_runtime_wasm_path() {
        write_variant_sidecar(&built, &enabled_features_key());
    }
}
