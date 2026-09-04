# Change: Enable Full Bridge Testnet on X3 Atomic Star

## Rationale
The X3 Atomic Star blockchain is currently at v0.4 Internal Testnet Candidate with external bridges disabled. The bridge code is substantially more complete than previously documented — the real blockers are configuration flags, build issues, and a few stub precompiles. This change enables a functional bridge testnet by fixing the build, toggling feature flags, and deploying infrastructure.

## Changes
- Fix workspace build (missing vendor directory, missing liquidity-core crate, Cargo.lock conflicts)
- Toggle feature flags: `external_bridges_mainnet` → `GUARDED_TESTNET`, `atomic_gateway` → `LIVE_TESTNET`
- Set `ExternalBridgesEnabled = true` in genesis config
- Use `NoOpCrossChainValidator` for testnet proof bypass
- Deploy 5-7 validator staging testnet with explorer, indexer, faucet
- Fix 4 stub EVM precompiles (modexp, bn128Add/Mul/Pairing) if needed
- Remove mock executors from test code paths
- Wire CI gates for bridge pallets

## Impact
- **Affected Specifications**: Bridge/Router, Cross-VM Execution, Settlement Engine, Infrastructure
- **Affected Code**:
  - `Cargo.toml` (root): Fix workspace member list, restore missing crates
  - `Cargo.lock`: Resolve version conflicts
  - `TESTNET_FEATURE_FLAGS.toml`: Toggle bridge flags to GUARDED_TESTNET
  - `pallets/x3-cross-vm-router/src/lib.rs`: Set ExternalBridgesEnabled default or genesis config
  - `pallets/x3-settlement-engine/src/bridge_integration.rs`: Wire NoOpCrossChainValidator for testnet
  - `crates/evm-integration/src/mini_evm.rs`: Fix 4 stub precompiles (modexp, bn128Add/Mul/Pairing)
  - `crates/x3-integration/src/hostcalls.rs`: Gate mock executors behind test feature
  - `deployment/chain-specs/`: Generate testnet chain spec with bridges enabled
  - `infra/`, `k8s/`, `docker/`: Deploy validator, explorer, indexer, faucet
  - `.github/workflows/`: Add CI gates for bridge pallets
