use std::{env, fs, path::PathBuf};

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

fn write_cached_wasm_binary(cached: &PathBuf) {
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
        // built artifact is available. Prefer that artifact and only fall back
        // to the None stub on a truly clean checkout.
        if let Some(cached) = cached_runtime_wasm_path() {
            write_cached_wasm_binary(&cached);
            return;
        }
        write_wasm_binary_stub("SKIP_WASM_BUILD is set; skipping runtime WASM build");
        return;
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
}
