use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Generate stub wasm_binary.rs to avoid building actual WASM
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("wasm_binary.rs");
    
    fs::write(
        &dest_path,
        r#"
// Stub wasm_binary.rs for patched frame-storage-access-test-runtime
pub const WASM_BINARY: Option<&[u8]> = Some(&[
    0x00, 0x61, 0x73, 0x6d, // WASM magic
    0x01, 0x00, 0x00, 0x00, // Version 1
]);

pub const WASM_BINARY_BLOATY: Option<&[u8]> = WASM_BINARY;
"#,
    ).unwrap();
    
    println!("cargo:rerun-if-changed=build.rs");
}
