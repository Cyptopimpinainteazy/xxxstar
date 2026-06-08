# Rule: Runtime Wiring Required

## Purpose
A file existing in the repo does not mean it is wired into the runtime. Code must be reachable from the actual execution path.

## Required Behavior
- Before claiming a module/feature is "wired", trace it from the entry point (main.rs, lib.rs, CLI, API handler, pallet construct_runtime!).
- For Rust pallets: verify pallet is listed in `runtime/src/lib.rs` `construct_runtime!` macro.
- For Rust crates: verify crate is in `Cargo.toml` workspace members and imported.
- For Solidity: verify contract is deployed or deployable via migration/hardhat scripts.
- For CLI tools: verify command is registered and reachable from `main()`.
- For API endpoints: verify route is registered in the server/router.

## Forbidden Behavior
- Do NOT claim a pallet is "ready" if it's not in the runtime's `construct_runtime!`.
- Do NOT claim a crate is "integrated" if it's only a Cargo.toml entry with no imports.
- Do NOT claim a feature "works" if it's only reachable via test harness but not the real binary.
- Do NOT claim an adapter is wired if it's compiled but never instantiated.

## Proof Required
- Show the import/registration line in the runtime or main entry point.
- Grep for `mod`, `use`, or `impl` references linking the code to execution.
- If unwired, state UNWIRED clearly and note what's missing.