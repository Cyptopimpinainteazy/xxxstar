# Feature Flags

This workspace treats feature flags as an API surface. New flags must be added
here and justified in the owning crate.

## Node

Source: `node/Cargo.toml`

- `default = ["cli"]`: enables CLI entrypoints for the node binary.
- `cli`: local marker for CLI-only code.
- `frontier`: enables Frontier JSON-RPC wiring and matching runtime adapters.
- `native`: enables native runtime execution hooks via `x3-chain-runtime/native`.
- `runtime-benchmarks`: enables benchmarking code in the runtime and CLI.
- `try-runtime`: enables Substrate try-runtime checks.

## Runtime

Source: `runtime/Cargo.toml`

- `default = ["std"]`: standard host build for node execution and tooling.
- `dev`: enables `pallet-sudo` for local/dev networks only.
- `std`: enables host-side std support across runtime dependencies.
- `runtime-benchmarks`: exposes FRAME benchmark code paths.
- `native`: marker for native-only helpers.
- `native-real-vm-adapters`: opts into native EVM/SVM adapters for developer
  workflows; default behavior keeps WASM and native execution aligned.
- `frontier`: enables Frontier runtime integration.
- `disable-runtime-api`: compiles the runtime without selected runtime APIs.
- `try-runtime`: enables FRAME try-runtime hooks.

## Dylint / Determinism

Source: `Cargo.toml`

- `workspace.metadata.dylint.libraries`: registers `crates/dylint-determinism`
  as the runtime panic/unwrap lint source.

## Rule

- Runtime and node flags must be documented here before merge.
- Native-only flags must explain why WASM parity is preserved.
- Experimental flags belong behind opt-in features and must name an owner.
