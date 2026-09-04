# X3 VM and Compiler Workspace

Quick start for working on the X3 VM and compiler components.

Build
```
cd x3-lang
# Because the workspace is still under construction, build crate individually as needed
cargo check --manifest-path crates/x3-common/Cargo.toml
cargo check --manifest-path crates/x3-ast/Cargo.toml
cargo check --manifest-path crates/x3-lexer/Cargo.toml
cargo check --manifest-path vm/Cargo.toml
cargo check --manifest-path compiler/Cargo.toml
```

Run VM tests (individual crates):
```
# Run VM crate tests
cargo test --manifest-path vm/Cargo.toml
```

Design notes
- Bytecode is fixed-width (4 bytes per instruction) except when extended immediates are needed.
- The verifier enforces instruction alignment and valid opcodes.
- The executor is deterministic and gas metering is done before execution.
