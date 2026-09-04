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
