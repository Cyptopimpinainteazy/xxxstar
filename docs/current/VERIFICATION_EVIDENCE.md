# Verification Evidence

**Updated:** 2026-06-17

Commands run locally during this documentation pass.

## Passed

```bash
cargo metadata --no-deps --format-version 1
```

Result:

- Completed successfully.
- Reported 129 workspace packages.

```bash
cargo check -p x3-chain-runtime --features testnet
```

Result:

- Completed successfully.
- Confirms the bridge-enabled `testnet` runtime feature compiles.
- Warnings remain, mostly unused imports, deprecated Substrate APIs, and placeholder-style EVM runtime adapter parameters.

```bash
cargo check -p x3-relayer
```

Result:

- Completed successfully.
- Warning: unused imports in `crates/x3-relayer/src/relayer.rs`.

```bash
cargo test -p x3-verification-router
```

Result:

- Completed successfully.
- 13 tests passed.
- Warnings: unused imports in verification-router modules.

```bash
cargo test -p x3-bridge-adapters
```

Result:

- Completed successfully.
- 17 tests passed.

```bash
cd X3-contracts/evm && forge test --match-path test/X3ExternalGateway.t.sol
```

Result:

- Completed successfully.
- 10 tests passed.

```bash
cd X3-contracts/evm && forge test
```

Result:

- Completed successfully.
- 15 tests passed.

```bash
cargo check -p x3-chain-node
```

Result:

- Completed successfully.
- Warnings remain in dependent runtime/pallet crates.

```bash
cargo build --release -p x3-chain-node -p x3-relayer
```

Result:

- Completed successfully.
- Release binaries built for the local e2e harness.

```bash
bash scripts/e2e-atomic-trade.sh
```

Result:

- Completed successfully.
- Started Anvil.
- Deployed `TestOnlyVerifier`.
- Deployed `X3ExternalGateway`.
- Started `x3-chain-node`.
- Started `x3-relayer`.
- Deployed and approved local `MockERC20`.
- Called `depositToX3`.
- Found 1 `DepositLocked` event.
- Relayer submitted the deposit proof to X3 node RPC.
- `x3_submitCrossVmTransaction` submitted the runtime kernel extrinsic.
- Alice's X3 canonical ledger balance changed from `0` to `1000`.
- Script reported `E2E Atomic Trade Test: PASSED`.

## Remaining Verification Gap

The e2e now proves event relay, node RPC submission, runtime kernel extrinsic
submission, and canonical ledger mutation for the local relay profile. It does
not yet prove production wrapped-asset minting, external verifier-backed proof
validation, or a multi-validator bridge run.

## Not Run In This Pass

```bash
bash scripts/testnet/run-7-validators-local.sh
bash scripts/testnet/verify-bridge-e2e.sh --count 7 --base-rpc-port 9944
```

These remain the next checks for a multi-validator local network. The single-node
Anvil-backed e2e now runs.
