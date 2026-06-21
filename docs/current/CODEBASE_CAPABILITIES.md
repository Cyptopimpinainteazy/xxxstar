# Codebase Capabilities

**Updated:** 2026-06-17

## Bridge-Enabled Runtime

Evidence:

- `runtime/Cargo.toml` defines `testnet = []` and documents it as the bridge-enabled testnet runtime feature.
- `runtime/src/lib.rs` wires `pallet_x3_crosschain_gateway::Config`.
- `runtime/src/lib.rs` uses `NoOpCrossChainValidator` under `#[cfg(feature = "testnet")]`.
- `cargo check -p x3-chain-runtime --features testnet` completed successfully during this pass.

## Gateway And Settlement

Evidence:

- `pallets/x3-crosschain-gateway/src/lib.rs`
- `pallets/x3-settlement-engine/src/bridge_integration.rs`
- `pallets/x3-settlement-engine/src/btc_gateway.rs`
- `pallets/x3-cross-vm-router/src/lib.rs`
- `pallets/x3-supply-ledger/src/lib.rs`

The runtime includes gateway, settlement, cross-VM router, and supply-ledger
components needed for a bridge testnet path.

## Verification Router

Evidence:

- `crates/x3-verification-router/src/lib.rs`
- `crates/x3-verification-router/src/evm_receipt.rs`
- `cargo test -p x3-verification-router` passed 13 tests during this pass.

Verifier strategies present:

- EVM receipt verifier.
- Solana finalized verifier.
- Bitcoin SPV verifier.
- Validator quorum verifier.
- X3 internal verifier.

## Relayer

Evidence:

- `crates/x3-relayer/src/main.rs`
- `crates/x3-relayer/src/relayer.rs`
- `crates/x3-relayer/src/submitter.rs`
- `crates/x3-relayer/src/watchers/evm.rs`
- `crates/x3-relayer/src/watchers/svm.rs`
- `cargo check -p x3-relayer` completed successfully during this pass.

Relayer behavior in code:

- EVM header/event watching.
- SVM header watching.
- Finality tracking.
- Verification router integration.
- Risk engine integration.
- RPC submitter with retry configuration.
- Governance pause awareness.

## External Bridge Adapters

Evidence:

- `crates/x3-bridge-adapters/src/ethereum.rs`
- `crates/x3-bridge-adapters/src/solana.rs`
- `crates/x3-bridge-adapters/src/bitcoin.rs`
- `cargo test -p x3-bridge-adapters` passed 17 tests during this pass.

## EVM Contracts

Evidence:

- `X3-contracts/evm/contracts/X3ExternalGateway.sol`
- `X3-contracts/evm/contracts/X3VmERC20.sol`
- `X3-contracts/evm/contracts/X3KernelBridge.sol`
- `X3-contracts/evm/contracts/interfaces/IX3Verification.sol`
- `X3-contracts/evm/test/X3ExternalGateway.t.sol`

The e2e script deploys `TestOnlyVerifier` and `X3ExternalGateway` to Anvil for
a local bridge test.

`cd X3-contracts/evm && forge test` passed 15 tests during this pass.

## Testnet Scripts

Evidence:

- `scripts/testnet/run-7-validators-local.sh`
- `scripts/testnet/verify-bridge-e2e.sh`
- `scripts/testnet/testnet_rc_gate.sh`
- `scripts/testnet/x3_testnet_up.sh`
- `scripts/testnet/x3_testnet_health.sh`
- `scripts/e2e-atomic-trade.sh`
- `.github/workflows/zombienet-integration.yml`
- `.github/workflows/try-runtime-upgrade.yml`
- `.github/workflows/x3-bridge-fixture-regeneration.yml`

## What This Means

The codebase has enough bridge/runtime/relayer/testnet surface to document and
operate a bridged testnet harness. The honest line is not "paper only"; it is a
real bridge-enabled testnet path with local verification commands.

The current Anvil-backed e2e proves gateway event emission, relayer event
pickup, X3 node RPC submission, runtime kernel extrinsic submission, and X3
canonical ledger mutation after deposit relay. Production wrapped-asset minting
and the real external proof-validation profile still need to be wired and tested
before this can be called a complete production bridge.
