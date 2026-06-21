# Failures And TODO Ledger

**Updated:** 2026-06-18 (Phase 4 complete)

This file is the current repair ledger from the documentation/verification pass.
It prioritizes things that block a production-grade bridged testnet.

## Resolved In This Pass

### 0. Phase 3 Product Integration Verified Pre-Wired (2026-06-18)

Verified 4 GAP report claims about missing product integration were already implemented:

- **pallet-x3-dex in construct_runtime**: Present in all 4 variants (lines 454, 529, 610, 692).
- **Launchpad calls TokenFactory**: Trait-based `TokenFactoryCreate` abstraction; `graduate()` calls it at line 491; runtime wires via `LaunchpadTokenFactoryBridge`.
- **Cross-VM Router rate limits**: `daily_limit`, `per_wallet_daily_limit`, `pending_limit` enforced in `initiate_transfer` with `DailyVolume`/`WalletDailyVolume` accumulators.
- **pallet-x3-lp-locker**: Exists with full lib/mock/tests runtime wiring.

### 0. Phase 4 Test Infrastructure & Code Quality (2026-06-18)

Findings:

- **75 ignored tests in launch-gates/**: Reference copies in `pack-03-bridge-atomic`, `pack-04-invariant`, `pack-05-test-gap` — not workspace tests. No action needed.
- **24 real ignored tests in workspace**:
  - `crates/x3-atomic-swap/tests/rpc_live_test.rs`: 6 ignored
  - `crates/x3-atomic-swap/tests/atlas_htlc_deploy_test.rs`: 6 ignored
  - `crates/x3-sidecar/tests/e2e_gateway_integration.rs`: 2 ignored
  - `tests/phase_core/rpc_settlement_validation.rs`: 10 ignored
  - These are integration tests requiring live chain — acceptable for now.
- **CI gates 12 of ~55 pallets**: CI directly tests 12 pallets (kernel, router, ledger, settlement, dex, launchpad, token-factory, dapp-hub, auction, wallet, northern-swarm, lp-locker). Full CI (`full-ci.yml`) runs `cargo test --workspace`.
- **Supply O(n) iteration**: `pallets/x3-supply-ledger/src/lib.rs:163` iterates all assets in `on_finalize`. Could stall on thousands of assets. Acceptable for v0.4 with bounded asset count.
- **Fixed `expect()` in vm_revert.rs:79**: Changed to `unwrap_or_default()`.
- **`ag_agent_uses_recovery`**: Does not exist in codebase — stale GAP reference.

### 0. Phase 2 Readiness Check Fixes (2026-06-18)

Fixes:

- **Hardcoded readiness reports**: Replaced 5 placeholder `String::from(...)` functions in `crates/x3-readiness/src/lib.rs:509-541` (btc_gateway, swarm_health, reactor_benchmark, marketing_audit, grant_pipeline) with real feature-registry-driven reports that read mode, score, blockers from `FEATURE_REGISTRY.toml`. Also updated `main.rs` equivalents.

### 1. Phase 0/1 Build & Security Fixes (2026-06-18)

Fixes:

- **Build syntax error**: Removed extra `};` in `pallets/x3-atomic-kernel/src/lib.rs:766` — workspace now compiles clean.
- **`expect()` panic**: Replaced `expect("leg count already validated")` with proper error propagation via `map_err` in atomic-kernel.
- **Bond slashing type errors**: Fixed `From<u128>` and `saturating_div` issues using `sp_runtime::Perbill`.
- **Sha256 XOR bug**: Replaced XOR-based pseudo-hash in `crates/svm-integration/src/syscalls.rs` with real `sha2::Sha256`. Solves C-04.
- **Keccak256 syscall**: Added missing `Keccak256Syscall` to the std SVM syscall table.
- **CrossVmInvoke stub**: Changed from silent echo to explicit `CrossVmRejected` error.
- **dev-bypass dead code**: Removed unreachable `#[cfg(feature = "dev-bypass")]` auth bypass block from kernel.
- **Agent-law visibility**: Changed `pub fn internal_slash` and `pub fn blacklist_agent` to private — governance bypass surface.

### 1. Release node and relayer build

Command:

```bash
cargo build --release -p x3-chain-node -p x3-relayer
```

Result:

```txt
Finished `release` profile [optimized]
```

Fixes:

- Repaired `crossVm_getRecentTransfers` brace structure.
- Imported `AtlasKernelRuntimeApi` for runtime API access.
- Fixed the metrics subscription closure capture.

### 2. Full Foundry contract suite

Command:

```bash
cd X3-contracts/evm && forge test
```

Result:

```txt
15 tests passed, 0 failed, 0 skipped
```

Fixes:

- Replaced unsupported wildcard JSON vector counting in `FlashloanParity.t.sol`.
- Fixed the underpay revert expectation to include custom error arguments.

### 3. Anvil + X3 node + relayer e2e

Command:

```bash
bash scripts/e2e-atomic-trade.sh
```

Result:

```txt
EVM DepositLocked events found: 1
Relayer successfully submitted deposit proof to X3
E2E Atomic Trade Test: PASSED
```

Fixes:

- Run Foundry deploys from `X3-contracts/evm` so remappings resolve.
- Pass the correct `X3ExternalGateway` constructor args.
- Deploy, mint, whitelist, and approve a local `MockERC20` instead of using a mainnet USDC address on Anvil.
- Call the real `depositToX3(address,bytes,uint256,uint256)` method.
- Generate a local relayer YAML for Anvil chain `1337`.
- Start the node with an explicit local-only node key and hard-fail if RPC never opens.
- Wire `x3_submitCrossVmTransaction` RPC ingress into the local runtime kernel extrinsic path.
- Assert X3 canonical ledger balance changes after the deposit relay.
- Build a relayer-side signed deposit relay envelope containing the runtime `LockProof`.
- Reject raw deposit payloads at `x3_submitCrossVmTransaction`; the RPC now requires the signed envelope and checks the proof hash is bound to the decoded bridge operation.
- Submit local bridge-testnet wrapped asset registration and mint calls after the verified kernel relay.
- Assert wrapped balance, per-token wrapped supply, and total wrapped supply change after the deposit relay.
- Make partial e2e results exit non-zero.

## Current Hard Failure / Blocker

### Production/non-local proof validation is not complete

Location:

- `node/src/rpc.rs` `x3_submitCrossVmTransaction`
- `crates/x3-relayer/src/main.rs` `submit_deposit_proof_to_x3`
- `pallets/x3-kernel/src/lib.rs` `submit_cross_vm_operation`
- `pallets/x3-wrapped/src/lib.rs` `mint_wrapped`

Issue:

- The e2e now proves Anvil deposit event detection, relayer-built signed proof envelope submission, runtime kernel extrinsic submission, X3 canonical ledger mutation, and wrapped-asset mint/accounting mutation.
- The local relay profile still signs the proof envelope with the dev authority seed (`X3_RELAY_PROOF_SIGNER`, default `//Alice`).
- The runtime verifies the envelope as a normal `LockProof`, but this is not a production/non-local external proof-validation profile.
- Multi-validator bridge verification, threshold proof aggregation, and production key handling are still not complete.

Next action:

- Replace the dev-authority proof signer with the real verifier-backed proof envelope for non-local networks and add a multi-validator/threshold proof e2e profile.

## Passed Bridge-Relevant Checks

```bash
cargo metadata --no-deps --format-version 1
cargo check -p x3-chain-runtime --features testnet
cargo check -p x3-relayer
cargo test -p x3-verification-router
cargo test -p x3-bridge-adapters
cd X3-contracts/evm && forge test --match-path test/X3ExternalGateway.t.sol
cd X3-contracts/evm && forge test
cargo check -p x3-chain-node
cargo build --release -p x3-chain-node -p x3-relayer
bash -n scripts/e2e-atomic-trade.sh
bash scripts/e2e-atomic-trade.sh
```

Results:

- Workspace metadata resolved.
- Bridge-enabled runtime feature compiled.
- Relayer compiled.
- Verification router passed 13 tests.
- Bridge adapters passed 17 tests.
- EVM external gateway suite passed 10 tests.
- Full EVM contract suite passed 15 tests.
- Node package check passed.
- Release node and relayer build passed.
- E2E script syntax check passed.
- Anvil-backed bridge relay e2e passed with canonical ledger balance +1000, wrapped balance +1000, wrapped token supply +1000, and total wrapped supply +1000.

## High-Priority TODOs / Stubs

### 1. Testnet proof acceptance is intentionally loose

Location:

- `runtime/src/lib.rs:2393-2397`

Issue:

- `#[cfg(feature = "testnet")]` uses `NoOpCrossChainValidator`.
- This is useful for bridge testnet bring-up, but it means testnet proof acceptance is not equivalent to production proof validation.

Next action:

- Keep it for local/testnet harnesses, but add a separate verification profile that wires the real cross-chain validator and runs the same e2e bridge flow.

### 2. Runtime EVM adapter path is incomplete without `frontier`

Locations:

- `runtime/src/lib.rs:3213-3235`
- `runtime/src/lib.rs:3238-3256`
- `runtime/src/lib.rs:3259-3275`
- `runtime/src/lib.rs:3278-3295`
- `runtime/src/lib.rs:3298-3310`
- `runtime/src/lib.rs:3350-3364`
- `runtime/src/lib.rs:3593-3656`
- `runtime/src/lib.rs:3659-3670`
- `runtime/src/lib.rs:3819-3883`
- `runtime/src/lib.rs:3886-3892`

Issue:

- Several EVM mapping/balance/code/storage/call/deploy methods return empty/disabled results when `frontier` is not enabled.
- Release/testnet docs must describe this as a bridge harness unless the EVM execution feature is actually enabled and tested.

Next action:

- Decide the bridged testnet profile: `testnet` only, or `testnet + frontier`.
- Add a build/test command for the chosen profile.

### 3. Cross-VM bridge connector escrow RPC plumbing returns zero

Location:

- `crates/cross-vm-bridge/src/connector.rs:436-447`

Issue:

- `get_evm_bridge_escrow` and `get_svm_bridge_escrow` return zeroed addresses with log warnings.

Next action:

- Wire these to live node/runtime storage or explicit chain-spec config before relying on connector-level escrow discovery.

### 4. Wallet cryptographic signature verification is not implemented

Locations:

- `crates/x3-wallet/src/transaction_signer.rs:150-158`
- `crates/x3-wallet/src/transaction_signer.rs:332-336`

Issue:

- `verify_signature` returns `Err("Cryptographic signature verification not implemented")`.

Next action:

- Implement real signature verification for the supported signer scheme and replace the current expected-error test.

### 5. Atomic swap finality certificate is a zero placeholder (FIXED 2026-06-18)

Status:

- `run_flash_finality_voter()` in `node/src/service.rs:2043` now derives a cert hash from GRANDPA-finalized block hash via `blake2_256` when no Flash-Finality cert is available.
- `build_finalization_request()` in `crates/atomic-swap-orchestrator/src/lib.rs:322` now accepts `finality_cert` as a parameter instead of hardcoding `H256::zero()`.
- Off-chain storage key `b"x3ff:" + block_number_le` always gets a non-zero cert hash, enabling the unsigned `submit_finalization_result` path.
- Pallet OCW doc comments updated to reflect GRANDPA-derived certs.

## Medium-Priority TODOs / Cleanup

- `pallets/x3-verifier/src/tests.rs:1-7` contains a placeholder test.
- `pallets/x3-launchpad/src/weights.rs:1` and other pallet weight files mention generated weight stubs.
- Fuzz harness generators and generated fuzz targets contain TODOs for structure-specific decoding tests.
- GPU determinism tests under `tests/chaos/` and `tests/p4_gpu_kernel_integration.py` use CPU/mock/stub paths.
- `crates/x3-automation/src/lib.rs:208` documents current block number as placeholder runtime integration.
- `crates/x3-sdk/src/rpc.rs:296` and `crates/x3-sdk/src/svm.rs:291` contain placeholder implementation notes.
- `scripts/x3-status-report.sh:95` reports stub detection as unknown when not run.

## Scan Counts

Focused scan command:

```bash
rg -n "TODO|FIXME|todo!\\(|unimplemented!\\(|panic!\\(\\s*\\\"(stub|not implemented)|placeholder|stub|fake|hardcoded|NoOp|not implemented" \
  -S crates pallets runtime node scripts tests X3-contracts docs/current README.md CURRENT_MAINNET_STATUS.md DOCUMENTATION_INDEX.md \
  -g '!target/**' -g '!node_modules/**' -g '!X3-contracts/evm/lib/**' -g '!**/*.lock' --glob '!**/*.cdx.json'
```

Result:

- 592 focused hits.

These are not all defects. Many are tests, mock runtimes, detector scripts, or documentation. The high-priority list above is the current repair order.

## Next Repair Order

1. Replace the local Alice-signed relay proof wrapper with the real proof-validation profile.
2. Add wrapped-asset mint/accounting assertions beside the canonical ledger assertion.
3. Decide and verify the exact bridged testnet profile: `testnet` only or `testnet + frontier`.
4. Wire connector escrow discovery and replace zero-address fallbacks.
5. Clean warning backlog in runtime/bridge crates once the core bridge path is stateful.
