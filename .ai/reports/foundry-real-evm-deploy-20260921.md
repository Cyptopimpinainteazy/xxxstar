# Foundry real EVM deployment — evidence

Date: 2026-09-21
Commit: `a4f63684c` (local `master`)
Source branch capability: `fix/foundry-real-evm-deploy`

## What changed

- `crates/x3-foundry-core/src/deployer.rs` no longer simulates deployment.
  `simulate_deploy`, `simulate_block_number`, `estimate_gas`, and the
  timestamp-based `compute_tx_hash` are gone.
- `crates/x3-foundry-core/src/evm_deploy.rs` (new) signs and submits a genuine
  contract-creation transaction through ethers and returns the node receipt's
  `address`, `tx_hash`, `block_number`, and `gas_used`.
- `crates/x3-foundry-auditor/src/lib.rs` exposes
  `compile_contract_bytecode`, shared with the deployer, so Solidity is
  compiled by the real `forge build` in one place.
- `crates/x3-oracle/src/pyth_oracle.rs` gained `Default` and `or_default()`
  to satisfy clippy `new_without_default` / `unwrap_or_default`.

## Proof commands (pinned toolchain 1.90.0)

```text
CARGO_TARGET_DIR=/tmp/x3target190 \
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
rustup run 1.90.0 cargo check -p x3-foundry-core -p x3-foundry-auditor
```

Result: PASS (`Finished dev profile`)

```text
CARGO_TARGET_DIR=/tmp/x3target190 \
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
rustup run 1.90.0 cargo clippy -p x3-foundry-core -p x3-foundry-auditor -p x3-oracle --all-targets -- -D warnings
```

Result: PASS

```text
rustup run stable cargo test -p x3-foundry-auditor
```

Result: 29 passed / 0 failed

```text
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include \
rustup run stable cargo test -p x3-foundry-core --lib
```

Result: 52 passed / 0 failed. The three real-anvil end-to-end tests
(`test_deploy_contracts_real_anvil_end_to_end`,
`test_cross_chain_real_anvil_end_to_end`, `test_full_pipeline`) require binding
a local TCP port, so they were run with network escalation; the other 49 run in
the normal sandbox.

## Boundary

This proves real local-anvil EVM deployment and honest invalid-key refusal. It
does not claim mainnet deployment: named chains resolve through env overrides
(`ETH_RPC_URL`, `X3_NODE_RPC`, `LOCAL_RPC_URL`), and no live mainnet deployment
was performed.
