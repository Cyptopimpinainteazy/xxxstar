# RC5 GenesisBuilder Blocker Diagnosis

## Exact failing command

```bash
OUT_DIR=reports/rc5-preflight-check \
LOG_DIR=logs/rc5-preflight-check \
BINARY=target/rc5-build/release/x3-chain-node \
RC5_FORCE_CLEAN_BUILD=0 \
DURATION_SECONDS=30 \
SNAPSHOT_INTERVAL_SECONDS=10 \
RUN_SETTLEMENT_SMOKE=0 \
scripts/mainnet/rc5_internal_alpha_72h.sh
```

## Exact error output

```text
`build-spec` command failed: Other: wasm call error Other: Exported method GenesisBuilder_build_state is not found
Error: Service(Other("wasm call error Other: Exported method GenesisBuilder_build_state is not found"))
```

## sp-genesis-builder dependency exists?

Before patch:
- Workspace `Cargo.toml`: missing
- `runtime/Cargo.toml`: missing

After patch:
- Workspace `Cargo.toml`: present as `sp-genesis-builder` on polkadot-sdk `stable2512`
- `runtime/Cargo.toml`: present with `default-features = false`
- `runtime/Cargo.toml` `std` feature: includes `sp-genesis-builder/std`

## GenesisBuilder API implemented?

Before patch:
- `runtime/src/lib.rs`: no `impl sp_genesis_builder::GenesisBuilder<Block> for Runtime` in active runtime

After patch:
- `runtime/src/lib.rs`: added `GenesisBuilder<Block>` implementation inside `impl_runtime_apis!`
- Implemented methods:
  - `build_state(json: Vec<u8>) -> sp_genesis_builder::Result`
  - `get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>>`
  - `preset_names() -> Vec<sp_genesis_builder::PresetId>`

## Exact file/function missing

- Missing runtime API implementation block in active runtime:
  - `runtime/src/lib.rs`
  - inside `impl_runtime_apis!`
  - missing `sp_genesis_builder::GenesisBuilder<Block>` impl and exported method `GenesisBuilder_build_state`

## Node build-spec expectation

- `node/src/command.rs` executes `BuildSpec` via `cmd.run(config.chain_spec, config.network)`.
- The build-spec path requires runtime exports for genesis-building methods, including `GenesisBuilder_build_state`.
- When runtime API is missing, node reports the above exported-method error and preflight fails.

## Runtime/node SDK line check

- Workspace Substrate dependencies point to `https://github.com/paritytech/polkadot-sdk` on branch `stable2512`.
- Node and runtime both resolve through workspace deps on the same SDK line.

## Exact patch plan executed

1. Add workspace dependency: `sp-genesis-builder` in `Cargo.toml`.
2. Add runtime dependency: `sp-genesis-builder` in `runtime/Cargo.toml`.
3. Add `sp-genesis-builder/std` to runtime `std` feature set.
4. Add `impl sp_genesis_builder::GenesisBuilder<Block> for Runtime` in `runtime/src/lib.rs` using `frame_support::genesis_builder_helper` with `RuntimeGenesisConfig`.
5. Re-run validation commands (check/build/build-spec/preflight).

## Validation outcomes after patch

1. `cargo check -p x3-chain-runtime`
  - PASS (runtime compiles with GenesisBuilder API wiring).

2. `cargo build --release -p x3-chain-node`
  - BLOCKED in this environment by pre-existing wasm build/toolchain issues unrelated to GenesisBuilder wiring:
    - mixed rustc artifacts in `target/release` before clean
    - `num-traits` wasm error (`E0223`) in `target/rc5-build` runtime wasm build path

3. `target/rc5-build/release/x3-chain-node build-spec --chain local3` + raw generation
  - Plain build-spec output can be generated after stripping banner prelude.
  - Raw build-spec still fails on current (old) RC5 binary with:
    - `Exported method GenesisBuilder_build_state is not found`

4. `scripts/mainnet/rc5_internal_alpha_72h.sh` short preflight
  - FAIL (expected fail-closed behavior) with same missing export on existing RC5 binary.

## Current release gate state

- GenesisBuilder runtime API is now wired in source and runtime-check validated.
- RC5 remains correctly blocked until a fresh node binary embedding this runtime fix is produced in an environment where runtime wasm build succeeds.
