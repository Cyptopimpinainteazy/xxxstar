# Bridged Testnet Runbook

**Updated:** 2026-06-17

This runbook describes the bridged testnet path present in the codebase.

## What It Runs

- X3 Substrate node/runtime built with the `testnet` feature.
- Cross-chain gateway pallet in the runtime.
- Settlement engine bridge integration.
- Relayer process watching external chain headers/events and submitting proofs.
- Optional Anvil local EVM chain with `X3ExternalGateway`.
- Bridge verification scripts that check RPC, storage, route RPCs, finality, peers, and chain spec state.

## Build Checks

```bash
cargo check -p x3-chain-runtime --features testnet
cargo check -p x3-relayer
cargo test -p x3-verification-router
cargo test -p x3-bridge-adapters
```

## Start A Local Validator Network

```bash
bash scripts/testnet/run-7-validators-local.sh
```

If you want the older all-in-one launcher:

```bash
bash scripts/testnet-full-launch.sh
```

## Verify Bridge Runtime State

```bash
bash scripts/testnet/verify-bridge-e2e.sh --rpc-url http://127.0.0.1:9944 --count 7 --base-rpc-port 9944
```

The script checks:

- At least three validator RPC endpoints respond.
- `ExternalBridgesEnabled` storage is true.
- `ExternalBridgeAuditGate` storage is true.
- `x3_getBridgeStatus` responds.
- `x3_getAtomicRoute` responds.
- `atomicTrade_simulate` responds.
- Finality progresses.
- Validator peer connectivity is healthy.
- Chain spec bridge settings are present.

## Run Anvil + Gateway + Relayer E2E

Prerequisites:

- `forge`
- `anvil`
- `cast`
- `jq`
- release binaries for `x3-chain-node` and `x3-relayer`

```bash
cargo build --release -p x3-chain-node -p x3-relayer
bash scripts/e2e-atomic-trade.sh
```

That script:

- Starts Anvil on port `8545`.
- Deploys `TestOnlyVerifier`.
- Deploys `X3ExternalGateway`.
- Starts `x3-chain-node`.
- Starts `x3-relayer`.
- Deploys a local `MockERC20`, mints to the Anvil deployer, whitelists it on the gateway, and approves the gateway.
- Calls `depositToX3(address,bytes,uint256,uint256)` on the gateway.
- Checks for `DepositLocked` events and relayer proof-submission logs.

Expected result:

```txt
EVM DepositLocked events found: 1
Relayer successfully submitted deposit proof to X3
E2E Atomic Trade Test: PASSED
```

## Relayer Configuration

The relayer supports YAML and environment configuration.

Common environment variables:

```bash
export X3_RPC="http://127.0.0.1:9944"
export ETH_RPC="http://127.0.0.1:8545"
export ETH_GATEWAY="<gateway-address>"
export X3_NETWORK="local"
export POLL_INTERVAL="2"
export DB_PATH="/tmp/x3-relayer.db"
```

Default YAML path:

```txt
crates/x3-relayer/relayer-config.testnet.yaml
```

## Important Runtime Detail

In `runtime/Cargo.toml`, the `testnet` feature is explicitly documented as the
bridge-enabled testnet runtime feature. In `runtime/src/lib.rs`, `#[cfg(feature =
"testnet")]` selects `NoOpCrossChainValidator` for settlement proof acceptance.
That is appropriate for a bridge testnet harness, not for a value-bearing
production bridge.

The current Anvil e2e validates relay into X3 node RPC and through the runtime
kernel path. The `x3_submitCrossVmTransaction` endpoint decodes the local
deposit relay payload, submits the kernel `submit_cross_vm_operation` extrinsic,
and the harness asserts Alice's X3 canonical ledger balance increases after the
relay. Production wrapped-asset minting and non-local proof validation remain
separate hardening work.

## Troubleshooting

- If `verify-bridge-e2e.sh` reports `ExternalBridgesEnabled` as null, regenerate or inspect the testnet chain spec.
- If RPC checks fail, confirm validator RPC ports start at `9944` and increment by one.
- If Anvil e2e fails, inspect the relayer and node log sections printed by `scripts/e2e-atomic-trade.sh`.
- If Foundry commands fail, install/update Foundry and rerun `forge test` inside `X3-contracts/evm`.
