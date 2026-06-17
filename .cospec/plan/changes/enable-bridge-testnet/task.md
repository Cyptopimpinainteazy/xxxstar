## Implementation

- [ ] 1.1 Fix workspace build — restore missing vendor directory
     【Target Object】`vendor/sp-runtime-interface/test-wasm/`
     【Purpose】Workspace root does not compile — missing directory blocks all `cargo check/build`
     【Method】Restore the `vendor/sp-runtime-interface/test-wasm/` directory from `.rc4-worktrees/` or git history; if neither source exists, re-clone the submodule from the upstream `sp-runtime-interface` repository
     【Dependencies】None
     【Content】
        - Check if `.rc4-worktrees/old/vendor/sp-runtime-interface/test-wasm/` exists; if yes, copy it to `vendor/sp-runtime-interface/test-wasm/`
        - If `.rc4-worktrees/` does not exist, restore from git history using `git checkout HEAD -- vendor/sp-runtime-interface/test-wasm/` or re-initialize the git submodule
        - Verify `cargo check --workspace` passes after restoration
        - If the directory still cannot be restored, create a minimal placeholder `Cargo.toml` and `lib.rs` that compiles to unblock the workspace

- [ ] 1.2 Fix workspace build — restore missing liquidity-core crate
     【Target Object】`crates/x3-liquidity-core/`
     【Purpose】`Cargo.toml` line 104 lists `"crates/x3-liquidity-core"` as workspace member but the crate may be incomplete or missing source files
     【Method】Verify the crate directory exists and contains valid source; if missing, restore from `.rc4-worktrees/old/crates/x3-liquidity-core/`
     【Dependencies】None
     【Content】
        - Verify `crates/x3-liquidity-core/` exists with `Cargo.toml` and `src/` directory
        - If the directory is empty or missing, copy from `.rc4-worktrees/old/crates/x3-liquidity-core/`
        - If `.rc4-worktrees/` is unavailable, restore from git history: `git checkout HEAD -- crates/x3-liquidity-core/`
        - Verify `cargo check --workspace` passes

- [ ] 1.3 Fix Cargo.lock version conflicts
     【Target Object】`Cargo.lock` (root), `Cargo.toml` (root)
     【Purpose】3x `sp-*` crate versions (git + crates.io) conflict, `trie-db v0.30.0` flagged future-incompatible — blocks clean workspace compilation
     【Method】Run `cargo update` to resolve semver-compatible conflicts; if conflicts remain, manually align `sp-*` dependency versions in root `Cargo.toml` to use a single source (crates.io or git, not both)
     【Dependencies】1.1, 1.2
     【Content】
        - Run `cargo update` and check if conflicts resolve automatically
        - If `sp-*` crates still have mixed sources (git + crates.io), inspect root `Cargo.toml` for duplicate or conflicting `sp-*` dependency entries
        - Align all `sp-*` dependencies to use the same source (prefer crates.io versions matching the Substrate release used by the workspace)
        - For `trie-db v0.30.0`, check if a newer compatible version exists; if not, add an `[allow]` entry in `deny.toml` or suppress the future-incompatible warning
        - Verify `cargo check --workspace` passes without version-related errors

- [ ] 1.4 Toggle bridge feature flags for testnet
     【Target Object】`TESTNET_FEATURE_FLAGS.toml` (root)
     【Purpose】External bridges are `DISABLED_BLOCKED` — must be `GUARDED_TESTNET` for bridge testnet; BTC gateway must be `GUARDED_TESTNET`; atomic gateway must be `LIVE_TESTNET`
     【Method】Edit the feature flags file to change the three flag values
     【Dependencies】None
     【Content】
        - Change `external_bridges_mainnet = "DISABLED_BLOCKED"` → `"GUARDED_TESTNET"`
        - Change `btc_mainnet_gateway = "SIM_TESTNET"` → `"GUARDED_TESTNET"`
        - Change `atomic_gateway = "GUARDED_TESTNET"` → `"LIVE_TESTNET"`
        - Verify the file parses correctly (e.g., `toml2json` or a simple parse check)
        - Commit the flag changes with a descriptive message referencing the bridge testnet enablement

- [ ] 1.5 Enable external bridges in genesis config
     【Target Object】`deployment/chain-specs/x3-testnet-raw.json` (genesis.raw.top section)
     【Purpose】`ExternalBridgesEnabled` storage defaults to `false` — must be `true` for testnet so bridge extrinsics are accepted
     【Method】Add the `ExternalBridgesEnabled` storage key-value pair to the testnet chain spec's genesis raw storage top section
     【Dependencies】None
     【Content】
        - Locate the storage key for `ExternalBridgesEnabled` in the pallet's storage metadata (or compute it via `twox_128` hash of the pallet name + storage name)
        - Add the key-value pair `"<storage_key>": "0x01"` (true) to the `genesis.raw.top` object in `x3-testnet-raw.json`
        - If using a plain chain spec instead, add `ExternalBridgesEnabled: true` to the `genesis.runtime.palletConfig` section
        - Verify the chain spec is valid JSON and can be loaded by the node (`./target/release/x3-node build-spec --chain x3-testnet-raw.json --raw`)
        - If the pallet's genesis config is not exposed in the chain spec, modify `pallets/x3-cross-vm-router/src/lib.rs` to add a genesis config builder that sets `ExternalBridgesEnabled` from a parameter

- [ ] 1.6 Wire NoOpCrossChainValidator for testnet
     【Target Object】`pallets/x3-settlement-engine/src/bridge_integration.rs` (testnet runtime config section)
     【Purpose】Use the existing `NoOpCrossChainValidator` (accepts all proofs) for testnet to bypass proof verification
     【Method】In the testnet runtime configuration block within `bridge_integration.rs`, set `CrossChainValidatorProvider` to `NoOpCrossChainValidator`
     【Dependencies】None
     【Content】
        - Locate the `testnet` or `TestnetConfig` implementation block in `pallets/x3-settlement-engine/src/bridge_integration.rs`
        - Set `type CrossChainValidatorProvider = NoOpCrossChainValidator;` in the testnet config
        - Ensure `NoOpCrossChainValidator` is imported (it should already exist in the crate; if not, add the import)
        - Verify the change compiles with `cargo check -p pallet-x3-settlement-engine`
        - Add a comment explaining this is testnet-only and must be replaced with a real validator for mainnet

- [ ] 1.7 Fix 4 stub EVM precompiles (modexp, bn128Add, bn128Mul, bn128Pairing)
     【Target Object】`crates/evm-integration/src/mini_evm.rs` lines 440-466
     【Purpose】These 4 Ethereum precompiles return errors in `no_std mini_evm` — blocks DApps that rely on them (e.g., Tornado Cash, ZK rollups)
     【Method】Implement real precompile logic using `num-bigint` for modexp and `bn128` arithmetic libraries for the BN128 operations; feature-gate to `std` builds only
     【Dependencies】`num-bigint` crate, `bn128` or `substrate-bn` crate (check if already in dependency tree)
     【Content】
        - `precompile_modexp`: Implement modular exponentiation using `num-bigint::BigUint::modpow()`; handle edge cases (zero base, zero exponent, modulus = 1); return `Ok(output)` with 32-byte padded result
        - `precompile_bn128_add`: Implement BN128 G1 point addition using `substrate-bn` or equivalent; validate input is exactly 128 bytes; handle point-at-infinity edge case
        - `precompile_bn128_mul`: Implement BN128 G1 scalar multiplication; validate input is exactly 96 bytes; handle zero scalar edge case
        - `precompile_bn128_pairing`: Implement BN128 pairing check; validate input length is multiple of 192 bytes; return `0x01` (true) or `0x00` (false) in 32-byte encoding
        - Add `#[cfg(feature = "std")]` gate to all four implementations; in `no_std` builds, return `Err(PrecompileError::Unsupported)` to preserve existing behavior
        - Add gas cost calculation matching Ethereum precompile gas schedules (modexp: dynamic based on input length, BN128: fixed per operation)
        - Add unit tests for each precompile with known test vectors from Ethereum test suite
        - Verify `cargo check --workspace` passes

- [ ] 1.8 Gate mock executors behind test feature
     【Target Object】`crates/x3-integration/src/hostcalls.rs` lines 425-428
     【Purpose】`MockEvmExecutor` and `MockSvmExecutor` used in test code — ensure they're gated behind `#[cfg(test)]` so they don't bloat production binaries
     【Method】Verify the mock executors are only compiled in test builds; add `#[cfg(test)]` guard if missing
     【Dependencies】None
     【Content】
        - Inspect lines 425-428 of `crates/x3-integration/src/hostcalls.rs` for `MockEvmExecutor` and `MockSvmExecutor` definitions
        - If `#[cfg(test)]` or `#[cfg(feature = "test-helpers")]` is already present, confirm it's correct and no action needed
        - If not gated, wrap the mock executor definitions and all their usage sites with `#[cfg(test)]`
        - Verify `cargo check --workspace` passes (production build should not include mock code)
        - Verify `cargo test --workspace` still passes (tests can still use the mocks)

- [ ] 1.9 Generate testnet chain spec with bridges enabled
     【Target Object】`deployment/chain-specs/` (output directory), `node/src/chain_spec.rs` (chain spec generator)
     【Purpose】Create a testnet chain spec JSON with bridges enabled, testnet authorities, and testnet genesis config for the multi-validator deployment
     【Method】Use the node's built-in chain spec generator (`./target/release/x3-node build-spec`) with the bridge-enabled feature flags, or modify the existing testnet spec generator in `node/src/chain_spec.rs`
     【Dependencies】1.5
     【Content】
        - If the node binary has a `testnet-bridge` chain spec preset, run: `./target/release/x3-node build-spec --chain testnet-bridge > deployment/chain-specs/x3-testnet-bridge-plain.json`
        - If no preset exists, modify `node/src/chain_spec.rs` to add a `testnet_bridge_config()` function that sets `ExternalBridgesEnabled = true`, testnet validator authorities (5-7 well-known test keys), and testnet token allocations including a faucet account
        - Generate the raw chain spec: `./target/release/x3-node build-spec --chain deployment/chain-specs/x3-testnet-bridge-plain.json --raw > deployment/chain-specs/x3-testnet-bridge-raw.json`
        - Verify the raw spec loads without errors: `./target/release/x3-node --chain deployment/chain-specs/x3-testnet-bridge-raw.json --validator --alice`
        - Commit both `x3-testnet-bridge-plain.json` and `x3-testnet-bridge-raw.json`

- [ ] 1.10 Deploy 5-7 validator staging testnet
     【Target Object】`infra/docker-compose.yml`, `deployment/genesis/`, `deployment/keys/`
     【Purpose】Deploy a multi-validator testnet with bridge support for E2E testing
     【Method】Use existing deployment scripts and infrastructure configs; provision validator nodes using Docker or systemd with the bridge-enabled chain spec
     【Dependencies】1.1, 1.2, 1.3, 1.9
     【Content】
        - Generate validator keys for 5-7 nodes using `./target/release/x3-node key generate` and store in `deployment/keys/`
        - Create or update `infra/docker-compose.yml` with 5-7 validator services, each with unique `--name`, `--validator`, `--chain` pointing to the bridge chain spec, and `--node-key` for peer discovery
        - Configure each validator with `--bootnodes` pointing to the first validator (bootnode)
        - Set up `deployment/genesis/` with the authority set containing all 5-7 validator stash/controller accounts
        - Start validators and verify block production (all validators should produce blocks in round-robin Aura)
        - Verify bridge extrinsics are callable via RPC (e.g., `x3CrossVmRouter.externalBridgesEnabled()` returns `true`)
        - Add health check monitoring (Prometheus metrics endpoint on each validator)

- [ ] 1.11 Deploy block explorer
     【Target Object】`docker/docker-compose.yml` (support infrastructure stack)
     【Purpose】Block explorer for testnet visibility — allows developers to view blocks, transactions, and bridge events
     【Method】Deploy the existing explorer service defined in `docker/docker-compose.yml` (Polkadot.js Apps or custom explorer) configured to connect to the testnet RPC endpoint
     【Dependencies】1.10
     【Content】
        - In `docker/docker-compose.yml`, ensure the explorer service is configured with `RPC_URL` pointing to the testnet RPC gateway (e.g., `http://x3-testnet-rpc:9933`)
        - If using Polkadot.js Apps, set `WS_URL` environment variable to the testnet WebSocket endpoint
        - Start the explorer: `docker compose -f docker/docker-compose.yml up -d explorer`
        - Verify blocks are visible and searchable in the explorer UI
        - Verify bridge transactions (e.g., `x3CrossVmRouter.deposit()`) appear in the explorer's event log

- [ ] 1.12 Deploy indexer + PostgreSQL
     【Target Object】`docker/docker-compose.yml` (support infrastructure stack)
     【Purpose】Indexer for querying bridge events and transaction history — enables rich querying of on-chain data
     【Method】Deploy the existing indexer service (Subsquid or custom) with PostgreSQL backend defined in `docker/docker-compose.yml`
     【Dependencies】1.10
     【Content】
        - In `docker/docker-compose.yml`, ensure the indexer service is configured with `CHAIN_RPC_URL` pointing to the testnet RPC endpoint
        - Ensure PostgreSQL service is configured with persistent volume for data storage
        - Start the indexer stack: `docker compose -f docker/docker-compose.yml up -d postgres indexer`
        - Verify block ingestion is working (indexer logs show increasing block numbers)
        - Verify bridge events (e.g., `BridgeDeposit`, `BridgeWithdrawal`) are indexed and queryable via the indexer's GraphQL endpoint

- [ ] 1.13 Deploy faucet
     【Target Object】`docker/docker-compose.yml` (support infrastructure stack)
     【Purpose】Testnet faucet for distributing test tokens to developers and users
     【Method】Deploy the existing faucet service defined in `docker/docker-compose.yml` configured with address-level rate caps and a pre-funded faucet account
     【Dependencies】1.10
     【Content】
        - In `docker/docker-compose.yml`, ensure the faucet service is configured with `FAUCET_ACCOUNT_SEED` (or private key), `RPC_URL` pointing to testnet, and `RATE_CAP` (e.g., 100 tokens per address per day)
        - Start the faucet: `docker compose -f docker/docker-compose.yml up -d faucet`
        - Verify test tokens are claimable by submitting a request to the faucet endpoint
        - Verify faucet works with bridge-enabled accounts (accounts that have interacted with bridge pallets)

- [ ] 1.14 Deploy public RPC gateway with rate limiting
     【Target Object】`infra/rpc/` (RPC router configs), `docker/docker-compose.yml` (support infrastructure stack)
     【Purpose】Public RPC endpoint for external users to interact with bridge testnet — includes rate limiting, health scoring, and quorum verification
     【Method】Deploy the existing RPC router (HAProxy or custom router) from `infra/rpc/` configured with rate limits and pointing to the validator RPC endpoints
     【Dependencies】1.10
     【Content】
        - Configure the RPC router in `infra/rpc/router/` (or `infra/rpc/haproxy/`) with upstream pointing to all 5-7 validator RPC ports
        - Set rate limits: 100 requests/second per IP, 1000 requests/second total
        - Enable health scoring: mark upstream as down after 3 consecutive failures
        - Enable quorum verification for state queries (require 2/3+ validators to agree)
        - Add the RPC gateway service to `docker/docker-compose.yml` or deploy via systemd
        - Verify external users can query testnet state (e.g., `system_health`, `chain_getBlock`)
        - Verify bridge RPC methods are accessible (e.g., `x3CrossVmRouter_externalBridgesEnabled`)

- [ ] 1.15 Add CI gates for bridge pallets
     【Target Object】`.github/workflows/ci.yml` (or a new `.github/workflows/bridge-ci.yml`)
     【Purpose】Currently CI only gates 4 of 40+ pallets — bridge pallets have no CI enforcement, risking regressions
     【Method】Add CI workflow jobs for bridge-related pallets that run `cargo test` and `cargo clippy` on each PR
     【Dependencies】1.1, 1.2, 1.3
     【Content】
        - Add CI job for `pallets/x3-cross-vm-router` tests: `cargo test -p pallet-x3-cross-vm-router`
        - Add CI job for `pallets/x3-settlement-engine` tests: `cargo test -p pallet-x3-settlement-engine`
        - Add CI job for `crates/x3-bridge` tests: `cargo test -p x3-bridge`
        - Add CI job for `crates/x3-bridge-adapters` tests: `cargo test -p x3-bridge-adapters`
        - Add CI job for `crates/x3-verification-router` tests: `cargo test -p x3-verification-router`
        - Add CI job for `crates/x3-relayer` tests: `cargo test -p x3-relayer`
        - Add `cargo clippy` checks for each of the above packages
        - Make these jobs required for merging to the testnet branch (add to branch protection rules or use `required` status check)
        - Add a CI job for `cargo check --workspace` to ensure no build regressions across the entire workspace

- [ ] 1.16 Verify bridge end-to-end flow on testnet
     【Target Object】Testnet deployment (all services running: validators, explorer, indexer, faucet, RPC gateway)
     【Purpose】Confirm the full bridge flow works end-to-end — internal cross-VM transfers and external bridge deposit/withdrawal
     【Method】Execute bridge deposit/withdrawal flow on testnet using the RPC gateway and verify state, events, and indexing
     【Dependencies】1.10, 1.11, 1.12, 1.13
     【Content】
        - Execute internal cross-VM transfer: Native → EVM → SVM using the cross-VM router pallet; verify balances update correctly on each side
        - Execute bridge deposit flow (if external chain available): submit a deposit extrinsic on the external chain gateway; verify the `BridgeDeposit` event is emitted and the destination account balance increases
        - Execute bridge withdrawal flow: submit a withdrawal extrinsic; verify the `BridgeWithdrawal` event is emitted and the source account balance decreases
        - Verify supply invariant: total supply before and after bridge operations should remain equal (no token minting/burning leaks)
        - Verify events are emitted correctly with correct parameters (source chain, destination chain, amount, sender, recipient)
        - Verify indexer captures bridge events and they are queryable via the indexer's GraphQL endpoint
        - Verify explorer shows bridge transactions with correct event data
