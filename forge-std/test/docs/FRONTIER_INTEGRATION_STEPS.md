# Frontier Integration Steps

This document captures a prioritized plan to complete Frontier (pallet-evm) integration and JSON-RPC wiring for X3 Chain.

## Objective
- Resolve Frontier version compatibility and pin the correct Frontier/Polkadot versions.
- Enable full Frontier JSON-RPC (eth_*, net_*, web3_*) via `fc-rpc` and `fp-rpc`.
- Replace mock adapters with `NativeEvmAdapter` that uses runtime `pallet-evm::Runner` or `FrontierEvmExecutor`.
- Validate end-to-end comit flows with real EVM/SVM adapters.

## Safe Staged Plan

1. Pin versions
   - Match `frontier` crate versions to the exact Substrate rev used in the workspace (`948fbd2`).
   - Option A: Use a Frontier git branch compatible with `rev 948fbd2` (recommended).
   - Option B: Add a patch to `Cargo.toml` to force compatible versions in `patch.crates-io`.

2. Node feature toggles
   - Add an optional `frontier` feature in `node/Cargo.toml` that enables `fc-rpc`, `fp-rpc`, and `fp-evm` dependencies.
   - Keep the existing `EthCompatApi` as a minimal fallback.

3. Adapter wiring
   - Ensure `runtime` sets `type EvmAdapter = native_vm_adapters::NativeEvmAdapter;` for native builds (already implemented).
   - Add a `NativeEvmAdapter` implementation (done) that calls `pallet_evm::Runner::call` (already present in `runtime/src/lib.rs`).
   - Update `pallets/x3-kernel/src/adapters.rs` `real_adapters` to be consistent with runtime adapters or mark as deprecated.

4. JSON-RPC wiring
   - Using `fc-rpc`/`fp-rpc` docs, create a `create_frontier_rpc` function in `node/src/rpc.rs` to register all eth_* endpoints.
   - When `feature = "frontier"` is enabled, `create_full` should merge these modules.
   - Implement a WebSocket subscription integration for Frontier events (eth_newPendingTransaction, logs, etc.).

5. E2E tests & CI
   - Add integration tests that run the node with `--features frontier` and run a few canonical EVM workflows.
   - Add a CI matrix entry to run the node build with `--features frontier`.

6. Deterministic WASM and version pin
   - If Frontier requires a newer substrate, either pin to a compatible Substrate tag or switch to a `fork` branch with all patch changes applied.
   - Add `Cargo.lock` maintenance scripts to keep `frontier` pins in sync.

## Quick attempts
- Local smoke test (without fw dependencies): `cargo run -p node -- --dev` and use `eth_compat` RPCs
- With `frontier` feature (if dependencies stabilized): `cargo run -p node --features frontier -- --dev`

## Notes
- For now, `pallets/x3-kernel` uses mock adapters in WASM builds while runtime uses `NativeEvmAdapter` in native builds. That is a valid compromise for testing and security.
- The real `FrontierEvmExecutor` implementation requires runtime-level `pallet_evm::Config` types; thus adapter implementation should remain in `runtime/src/lib.rs`.

## Next steps
- I can draft PRs for:
  1. Adding `frontier` feature in `node/Cargo.toml` with the required dependencies.
  2. Wiring the `fc-rpc` modules into `create_full` guarded by feature.
  3. Adding CI job to build with `--features frontier`.

Would you like me to proceed with the PR draft for the dependency pin (Option A) or first wire the optional RPC feature with the stub we added? 
