# X3 Proving Harness

> Add `"proving"` to workspace members in root `Cargo.toml` to build.

## Quick start

```bash
# from workspace root
cargo build -p x3-proving-harness --release
./target/release/x3-prove                 # probe default chains, write scorecard
./target/release/x3-prove --help          # full flag reference
```

## What it checks (per chain)

| Check | RPC method (Substrate / EVM / SVM) |
|---|---|
| `quoting` | `system_health` / `eth_blockNumber` / `getHealth` |
| `bundle_construction` | `chain_getBlockHash` / `eth_getBlockByNumber` / `getLatestBlockhash` |
| `submission` | `state_getMetadata` / `eth_protocolVersion` / `getVersion` |
| `rollback` | `chain_getFinalizedHead` / `eth_getBlockByNumber("finalized")` / `getSlot` |
| `reconciliation` | `state_getStorage` / `eth_getBalance` / `getAccountInfo` |
| `state_verification` | `system_version` / `web3_clientVersion` / `getVersion` |

Each probe has a 5-second timeout. Offline nodes produce `passed: false` — the binary always exits cleanly.

## Output

Writes `proof/reports/compatibility_matrix.json` (override with `--output`).
Prints a human-readable table to stdout. Exits **0** if all chains pass, **1** otherwise.

## Adding a chain

1. Pass it via `--chains x3-native,evm-testnet,svm-testnet,my-chain`.
2. Chain-ID prefix selects the RPC family: `evm-*` → EVM (8545), `svm-*` → SVM (8899), anything else → Substrate (9944).
3. To override the default port, edit `chain_config_for` in `src/checks.rs`.

## Thresholds

| Flag | Default |
|---|---|
| `--threshold-bundle-success` | `0.95` |
| `--threshold-rollback` | `1.0` |
| `--threshold-reconciliation` | `0.99` |
