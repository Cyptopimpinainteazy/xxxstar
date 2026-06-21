## Implementation

- [ ] 1.1 Verify workspace build compiles and resolve Cargo.lock conflicts
     【Target Object】`Cargo.toml` (root workspace), `Cargo.lock`
     【Purpose】Ensure the workspace compiles cleanly after all changes in the previous commit; resolve any remaining Cargo.lock version conflicts that block compilation
     【Method】Run `cargo check --workspace` and capture errors; if failures occur, identify root cause (likely Cargo.lock semver conflicts) and run `cargo update` to resolve
     【Dependencies】None
     【Content】
        - Run `cargo check --workspace 2>&1` and capture full output
        - If compilation fails, inspect error messages to identify version conflicts (e.g., duplicate `sp-*` crate versions from mixed git/crates.io sources, `trie-db` future-incompatible warnings)
        - Run `cargo update` to resolve semver-compatible conflicts automatically
        - If conflicts persist after `cargo update`, manually align conflicting dependency versions in root `Cargo.toml` to use a single source
        - Re-run `cargo check --workspace` and confirm it passes cleanly with zero errors
        - Note: the vendor `test-wasm` directory issue is already resolved in the prior commit; no action needed here

- [ ] 1.2 Enable ExternalBridgesEnabled in genesis config via raw storage key injection
     【Target Object】`deployment/chain-specs/x3-testnet-raw.json` (genesis.raw.top section)
     【Purpose】`ExternalBridgesEnabled` is a `StorageValue` in pallet `x3_cross_vm_router` that defaults to `false` — must be `true` for testnet so bridge extrinsics (`register_external_root`, `emergency_pause_bridge`) are accepted
     【Method】Inject the raw storage key-value pair for `ExternalBridgesEnabled` into the testnet chain spec's `genesis.raw.top` JSON object; no `chain_spec.rs` changes needed since the pallet has no `GenesisConfig` — the storage value is set directly via the raw spec
     【Dependencies】None
     【Content】
        - Compute the storage key for `ExternalBridgesEnabled` using `twox_128("X3CrossVmRouter") + twox_128("ExternalBridgesEnabled")` — the known key is `0x1ea3c00b772dc6623f323eb3179639f18997eadf5206160f7717460ca1aec5a8`
        - Add `"0x1ea3c00b772dc6623f323eb3179639f18997eadf5206160f7717460ca1aec5a8": "0x01"` to the `genesis.raw.top` object in `deployment/chain-specs/x3-testnet-raw.json`
        - Verify the chain spec is valid JSON (e.g., `python3 -m json.tool deployment/chain-specs/x3-testnet-raw.json`)
        - Verify `cargo check --workspace` still passes (no code changes needed, only JSON data)
        - Note: `node/src/chain_spec.rs` does NOT need modification because `pallet_x3_cross_vm_router` has no `GenesisConfig` struct — `ExternalBridgesEnabled` is a `StorageValue` that can only be set via raw storage key injection in the chain spec JSON

- [ ] 1.3 Update enable-bridge-testnet/task.md and proposal.md to reflect actual completion status
     【Target Object】`.cospec/plan/changes/enable-bridge-testnet/task.md`, `.cospec/plan/changes/enable-bridge-testnet/proposal.md`
     【Purpose】The original task.md has all items unchecked — many are already done in commit `2f8753f89`. Update to reflect actual completion status and reference this new change for remaining items.
     【Method】Mark completed items as done, update remaining items with accurate status and cross-reference to this `finish-bridge-testnet` change
     【Dependencies】1.1, 1.2
     【Content】
        - Mark 1.2 (liquidity-core) as ✅ done — crate exists and is properly configured
        - Mark 1.4 (feature flags) as ✅ done — flags toggled in TESTNET_FEATURE_FLAGS.toml
        - Mark 1.6 (NoOpCrossChainValidator) as ✅ done — wired in bridge_integration.rs
        - Mark 1.7 (EVM precompiles) as ✅ done — 4 precompiles implemented in mini_evm.rs
        - Mark 1.8 (mock executors) as ✅ done — gated behind test feature
        - Mark 1.10-1.14 (deployment infra) as ✅ done — scripts, docker, keys, genesis, RPC infra all exist
        - Mark 1.15 (CI gates) as ✅ done — bridge pallet CI gates added to ci.yml
        - Mark 1.16 (E2E verification) as ✅ done — verify-bridge-e2e.sh exists (303 lines, 7 checks)
        - Update 1.1, 1.3, 1.5, 1.9 with current status and reference this new `finish-bridge-testnet` change
        - If `enable-bridge-testnet/proposal.md` references unfinished items, add a note linking to this change
