// Stub wasm_binary.rs for patched frame-storage-access-test-runtime
// This provides a minimal WASM binary (empty module) to satisfy the include! macro

pub const WASM_BINARY: Option<&[u8]> = Some(&[
    0x00, 0x61, 0x73, 0x6d, // WASM magic
    0x01, 0x00, 0x00, 0x00, // Version 1
]);

pub const WASM_BINARY_BLOATY: Option<&[u8]> = WASM_BINARY;
