# X3 Chain – Complete Functional Blockchain Roadmap

## Overview

This roadmap describes the step-by-step path to transform X3 Chain from a **proof-of-concept prototype** (dual-VM kernel + stubs) into a **fully functional Layer-1 blockchain** capable of:

- ✅ Deterministic dual-VM (EVM + SVM) transaction execution
- ✅ Atomic cross-domain operations with canonical ledger finality
- ✅ Production-grade consensus, networking, and RPC surfaces
- ✅ Developer tooling (SDKs, wallet, explorer, CLI utilities)
- ✅ Deployment infrastructure (testnet, staging, mainnet)

---

## Estimated Timeline

- **Phase 1–2:** 2–3 weeks (Core Runtime & Node Hardening)
- **Phase 3–4:** 3–4 weeks (EVM & SVM Integration)
- **Phase 5:** 2–3 weeks (Cross-Domain Orchestration)
- **Phase 6–7:** 2–3 weeks (Developer Tooling & Testing)
- **Phase 8:** 1–2 weeks (Testnet Launch & QA)

**Total: ~4–6 months** to production-ready Alpha launch.

---

## Phase 1: Core Runtime Hardening & Validation

**Goal:** Ensure the X3 Kernel pallet and runtime are production-grade, fully tested, and deterministic for Wasm.

### 1.1 Complete X3 Kernel Pallet Logic

**Status:** MVP implemented; mock VM dispatchers in place.

**Tasks:**

- [ ] **Finalize Comit data model** (`pallets/x3-kernel/src/lib.rs`):
  - Lock down `Comit`, `ExecutionReceipt`, `SphereState` structures (add versioning for backward compatibility).
  - Implement deterministic SCALE encoding for all types to ensure consistent hashing.
  - Add serde support for JSON RPC interfaces.

- [ ] **Enhance nonce and replay protection**:
  - Ensure nonce increments are atomic and cannot be bypassed.
  - Add test coverage for concurrent nonce submissions (race conditions).
  - Document nonce lifecycle in test fixtures.

- [ ] **Upgrade prepare_root verification**:
  - Currently uses `blake2_256`; add support for configurable hash schemes (future Merkle proofs).
  - Write comprehensive test suite covering hash collision resistance.
  - Benchmark performance; optimize if prepare_root computation becomes a bottleneck.

- [ ] **Canonical ledger constraints & overflow handling**:
  - Add overflow/underflow guards in balance updates.
  - Implement atomic multi-asset transfers (A pays B, C pays D in single Comit).
  - Add comprehensive error types for ledger violations (e.g., `InsufficientBalance`, `UnknownAsset`, `MaxAssetsExceeded`).

- [ ] **Event finality and indexing**:
  - Ensure `ComitSubmitted`, `ComitFinalized`, `ComitFailed` events are properly indexed for block explorers.
  - Add event filtering helpers to support off-chain indexing via subquery or The Graph.

**Acceptance Criteria:**

- All existing tests pass.
- Add 20+ new test cases covering edge cases (overflow, nonce replay, empty Comits, max payload sizes).
- Code is no_std compliant and generates deterministic Wasm.
- Runtime weights are benchmarked.

---

### 1.2 Runtime Configuration & Constants

**Status:** Base constants defined; need refinement.

**Tasks:**

- [ ] **Review and lock down runtime constants** (`runtime/src/lib.rs`):
  - `MaxPayloadLength`: 32 KiB — verify against realistic EVM/SVM payload sizes. Consider increasing if needed.
  - `ExistentialDeposit`: 100 µX3 — review for mainnet economics.
  - `TransactionByteFee`: 10 µX3/byte — validate fee structure matches gas-to-X3 conversion.
  - `MaxAssetsPerAccount`: 32 — reasonable for dev; document mainnet scaling expectations.

- [ ] **Add configurable consensus parameters**:
  - `BlockTime`: Currently 6 seconds. Document expected finality time (Aura + GRANDPA).
  - `SessionLength`: Define validator rotation period (e.g., 1 hour = 600 blocks).
  - `MaxAuthorities`: 32 validators — sufficient for launch; scalability plan post-mainnet.

- [ ] **Implement configurable fee market** (optional; can be Phase 2):
  - Base fee multiplier (similar to EIP-1559).
  - Priority fee tiers.
  - Per-VM execution cost multipliers (EVM vs. SVM execution pricing).

**Acceptance Criteria:**

- All constants are documented with rationale and mainnet override guidance.
- Constants can be updated via governance (post-MVP).
- Runtime builds and tests pass with locked constants.

---

### 1.3 Deterministic Wasm Build & Reproducibility

**Status:** Build script in place; needs validation.

**Tasks:**

- [ ] **Lock Rust toolchain version** (`rust-toolchain.toml`):
  - Pin exact Substrate/FRAME crate versions.
  - Ensure all dependencies are reproducible (check for git refs without pinned commits).
  - Document build environment (OS, Rust version, dependencies).

- [ ] **Verify Wasm build reproducibility**:
  - Build runtime twice on different machines; compare `.wasm` checksums.
  - Document any non-determinism sources (timestamps, random seeds, etc.).
  - Add CI step to validate Wasm determinism on every commit.

- [ ] **Create release build script** (`scripts/build-release.sh`):
  - Automate deterministic native & Wasm build.
  - Output checksums and version metadata.
  - Tag releases in git with build artifacts.

**Acceptance Criteria:**

- Wasm builds are reproducible across dev machines and CI.
- Release script produces auditable build artifacts.
- CI enforces reproducibility checks.

---

### 1.4 Comprehensive Test Suite for Kernel Pallet

**Status:** ~100 tests exist; need expansion for edge cases and integration.

**Tasks:**

- [ ] **Unit tests** (`pallets/x3-kernel/src/tests.rs` + `mock.rs`):
  - Expand to 200+ tests covering:
    - Valid and invalid Comit submissions (all error paths).
    - Asset registration and ledger constraints.
    - Nonce overflow and race conditions.
    - Prepare root verification (correct, mismatched, zero roots).
  - Add property-based tests using `proptest` (e.g., nonce monotonicity, ledger invariants).

- [ ] **Integration tests** (new file: `tests/kernel_integration.rs`):
  - Test kernel pallet interactions with balances, transaction payment, and system pallets.
  - Verify fee deduction flows.
  - Test account registration and x3 ID mapping.

- [ ] **Fuzzing** (optional; can be Phase 2):
  - Fuzz Comit parsing and prepare_root computation.
  - Fuzz canonical ledger operations.

**Acceptance Criteria:**

- 95%+ code coverage on kernel pallet.
- All tests pass locally and in CI.
- Fuzzing (if done) runs for at least 1 week without crashes.

---

## Phase 2: Node Service & Networking

**Goal:** Harden the node binary, RPC surface, and network stack for multi-node setups.

### 2.1 Node CLI & Chain Specification

**Status:** Basic CLI in place; needs testnet specs.

**Tasks:**

- [ ] **Enhance node CLI** (`node/src/cli.rs`, `node/src/command.rs`):
  - Add `--dev`, `--local`, `--staging`, `--mainnet` chain specs.
  - Support custom chain spec via `--chain` flag with validation.
  - Add key injection utilities (`key insert --suri ...`).
  - Document CLI help text and examples.

- [ ] **Generate and commit chain specs**:
  - **Development** (`specs/dev.json`): Single-node authority, sudo enabled.
  - **Local Testnet** (`specs/local.json`): 3–4 node setup, deterministic keys (Alice, Bob, Charlie, Dave).
  - **Staging** (`specs/staging.json`): Public testnet spec (to be deployed).
  - **Mainnet** (`specs/mainnet.json`): Final mainnet spec (genesis, validators, sudo disabled).

- [ ] **Validator key generation tooling**:
  - Write script (`scripts/generate-keys.sh`) to create Aura + GRANDPA keys for validators.
  - Document key derivation and storage (HSM recommendations for mainnet).
  - Add subkey integration if not already in place.

**Acceptance Criteria:**

- Node starts with all four chain specs.
- Multi-node local testnet can be spun up and reaches consensus.
- CLI help is clear and examples work.

---

### 2.2 RPC Surface & Subscriptions

**Status:** Basic RPC available; needs enrichment.

**Tasks:**

- [ ] **Core RPC endpoints** (already provided by substrate; ensure enabled):
  - `chain_*`: Block queries, header info, finalized head.
  - `state_*`: Storage queries, proof generation.
  - `system_*`: Chain info, peer count, sync status.
  - `author_submitExtrinsic`: Transaction submission.

- [ ] **X3-specific RPC methods** (new file: `node/src/rpc.rs`):
  - `atlasKernel_getCanonicalBalance`: Query account asset balance.
  - `atlasKernel_getAssetMetadata`: Fetch asset symbol, decimals.
  - `atlasKernel_getComitStatus`: Query Comit submission status (pending, finalized, failed).
  - `atlasKernel_getAccountNonce`: Retrieve next expected nonce.
  - `system_getAccountInfo`: Combined account metadata (nonce, balance, x3 ID).

- [ ] **Subscriptions** (event monitoring):
  - `atlasKernel_subscribeComits`: Real-time Comit submission/finalization events.
  - Ensure subscription management handles client disconnects gracefully.

- [ ] **Documentation** (`docs/RPC.md`):
  - Full RPC reference with example curl/wscat commands.
  - Type definitions and error codes.

**Acceptance Criteria:**

- All core and custom RPC endpoints respond correctly.
- Integration tests verify RPC against a running node.
- Documentation is complete and examples work.

---

### 2.3 Networking & Peer Discovery

**Status:** Default Substrate libp2p; works for small networks.

**Tasks:**

- [ ] **Network configuration**:
  - Set appropriate `--max-peers` (e.g., 50 for testnet).
  - Configure bootnodes for stagenet/mainnet in chain specs.
  - Enable peer reputation management to handle malicious peers.

- [ ] **Telemetry integration** (optional but recommended):
  - Configure Substrate telemetry for monitoring network health.
  - Add custom telemetry endpoint for testnet health apps/dash-legacy-2-legacy-2board.

- [ ] **Network resilience testing**:
  - Test node behavior under partition, high latency, and packet loss.
  - Verify consensus liveness and safety under adversarial conditions.

**Acceptance Criteria:**

- Multi-node network reaches consensus reliably.
- Node recovers gracefully from network partitions.
- Peer count and sync status are queryable via RPC.

---

### 2.4 Storage & Database Configuration

**Status:** RocksDB default; needs optimization.

**Tasks:**

- [ ] **Storage optimization**:
  - Tune RocksDB compression and cache sizes for typical workload.
  - Add pruning strategy (default: archive mode; optional pruning for resource-constrained validators).
  - Monitor storage growth rate and project disk requirements.

- [ ] **State snapshots** (nice-to-have):
  - Export/import state snapshots for fast node sync.
  - Document snapshot generation and verification.

**Acceptance Criteria:**

- Storage is optimized and tuned.
- Node startup time and sync speed are acceptable (<5 min for dev setup).

---

## Phase 3: EVM Integration (Frontier)

**Goal:** Deploy a production-grade EVM execution layer with canonical ledger coupling.

### 3.1 Integrate pallet-evm & Frontier

**Status:** Scaffolded; not yet integrated into runtime.

**Tasks:**

- [ ] **Add pallet-evm dependency** (`runtime/Cargo.toml`):
  - Pull in `pallet-evm` from Frontier.
  - Ensure version compatibility with current Substrate (test on stable branch).

- [ ] **Implement EVM pallet configuration** (`runtime/src/lib.rs`):
  - Configure `pallet_evm::Config`:
    - `ChainId = 1337` (or testnet ID).
    - `AccountProvider`: Custom account provider mapping H160 to substrate AccountId.
    - `FeeCalculator`: Denominate gas fees in X3 (conversion: 1 X3 = N gwei).
    - `WeightToFee`: Calculate weight cost in X3.
    - `GasWeightMapping`: Translate gas to substrate weight units.
    - `Runner`: Default runner or custom for canonical ledger hooks.

- [ ] **Implement canonical ledger hooks**:
  - Create adapter pallet `pallet-canonical-ledger-adapter` (new pallet in `pallets/`).
  - Implement `pallet_evm::Config::OnChargeTransaction` to deduct fees from canonical ledger.
  - Implement `pallet_evm::Config::OnChargeWithdraw` and `OnChargeExtend` if using advanced fee models.
  - Ensure all balance reads/writes route through canonical ledger storage map.

- [ ] **Wire into runtime** (`runtime/src/lib.rs`):
  - Add `Evm: pallet_evm`.
  - Update `construct_runtime!` macro.
  - Add EVM to `AllPalletsWithSystem`.

**Acceptance Criteria:**

- Runtime compiles and generates valid Wasm.
- EVM pallet can accept dummy transactions (mock execution initially).
- Tests verify canonical ledger reads/writes.

---

### 3.2 Implement EVM Execution & Account Mapping

**Status:** Stubs only; needs full implementation.

**Tasks:**

- [ ] **Account mapping** (H160 ↔ AccountId):
  - Implement `pallet_evm::Config::AccountProvider`.
  - Store mapping in storage: `EvmAccountMapping<H160> -> AccountId`.
  - Auto-derive H160 from substrate account (e.g., use account pubkey hash).
  - Handle account creation and cleanup on contract deployment/destruction.

- [ ] **Contract storage integration**:
  - Ensure EVM contract code and storage are persisted in substrate storage.
  - Implement storage key derivation to avoid collisions.

- [ ] **Gas metering**:
  - Configure `GasWeightMapping` to translate gas to substrate weight.
  - Implement `FeeCalculator` to compute X3 fee from gas.
  - Add overhead cost for cross-VM calls (if applicable).

- [ ] **EVM Executor tests**:
  - Write tests for simple contract deployments (e.g., Greeter, ERC-20).
  - Test balance transfers via EVM.
  - Verify gas deduction from canonical ledger.

**Acceptance Criteria:**

- EVM accounts are created and mapped correctly.
- Simple Solidity contract (e.g., `pragma solidity ^0.8.0; contract Greeter { string public greeting = "Hello"; }`) deploys and executes.
- Gas fees are deducted from canonical ledger and credited to block author.
- Tests pass for contract deployment, execution, and state queries.

---

### 3.3 Ethereum JSON-RPC Layer (Frontier)

**Status:** Not yet implemented.

**Tasks:**

- [ ] **Add pallet-ethereum** (`runtime/Cargo.toml`):
  - Provides transaction validation and event emitter for Ethereum-compatible RPC.

- [ ] **Implement Frontier RPC server** (new file: `node/src/rpc_evm.rs`):
  - Expose Ethereum JSON-RPC methods via separate port (e.g., 8545).
  - Key methods:
    - `eth_blockNumber`, `eth_chainId`, `eth_gasPrice`
    - `eth_getBalance`, `eth_getCode`, `eth_getStorageAt`
    - `eth_call`, `eth_estimateGas`
    - `eth_sendRawTransaction`
    - `eth_getTransactionReceipt`, `eth_getTransactionByHash`
    - `eth_getLogs`, `eth_subscribe` (WebSocket)
  - Use Frontier's built-in RPC handler or custom adapter.

- [ ] **Transaction validation & mempool**:
  - Implement custom transaction validation for Ethereum-formatted transactions.
  - Ensure transactions are queued in substrate's transaction pool.
  - Handle nonce-based ordering and replacement.

- [ ] **Wallet integration tests**:
  - Configure MetaMask to point to localhost:8545.
  - Test sending a simple ETH transfer via MetaMask.
  - Verify transaction appears in block and balance updates.

**Acceptance Criteria:**

- Frontier JSON-RPC server is listening on port 8545.
- MetaMask can connect and display balance.
- Simple ERC-20 contract can be deployed via Hardhat/Truffle.
- Transaction fees are charged in X3.

---

### 3.4 EVM Tooling & Examples

**Status:** Not yet created.

**Tasks:**

- [ ] **Example contracts**:
  - Greeter: `contracts/evm/Greeter.sol`
  - ERC-20: `contracts/evm/ERC20Token.sol`
  - DEX (AMM): `contracts/evm/SimpleDEX.sol`
  - Store deploy scripts in `contracts/evm/scripts/`.

- [ ] **Hardhat configuration**:
  - Create `hardhat.config.js` pointing to local node.
  - Add deploy scripts and tests.
  - Document: `docs/HARDHAT.md`.

- [ ] **Foundry support** (optional):
  - Create `foundry.toml` for Solidity testing.
  - Provide cast/forge examples.

**Acceptance Criteria:**

- Hardhat deploy scripts work end-to-end.
- Example contracts are deployable and testable.
- Documentation guides developers through EVM workflow.

---

## Phase 4: SVM Integration

**Goal:** Deploy Solana Virtual Machine execution with deterministic verification.

### 4.1 SVM Sidecar Service Architecture

**Status:** Concept only; needs implementation.

**Tasks:**

- [ ] **Design SVM sidecar** (new binary: `svm-sidecar/`):
  - **Role:** Listen for Comit events; execute SVM programs deterministically; emit receipts.
  - **Architecture:**
    - Subscribe to node RPC for `ComitSubmitted` events.
    - Deserialize `svm_payload` (Borsh-encoded Solana transaction).
    - Execute using Solana SDK / Agave runtime (in sandbox).
    - Generate receipt with state diffs, logs, and signatures.
    - Submit receipt to node via `submit_svm_receipt` extrinsic.

- [ ] **Sidecar crate structure**:
  - `svm-sidecar/Cargo.toml`: Dependencies on solana-sdk, tokio, tonic.
  - `svm-sidecar/src/main.rs`: Sidecar entry point.
  - `svm-sidecar/src/executor.rs`: SVM execution wrapper.
  - `svm-sidecar/src/receipt.rs`: Receipt generation and validation.
  - `svm-sidecar/src/rpc_client.rs`: Node RPC communication.

- [ ] **Determinism & sandboxing**:
  - Run SVM execution in isolated process/container.
  - Disable randomness, system calls (e.g., time, entropy).
  - Capture execution traces for verification.

**Acceptance Criteria:**
- Sidecar can connect to node RPC and subscribe to Comit events.
- Sidecar executes a simple SVM program (e.g., system_program transfer).
- Receipt is generated and contains all necessary state diffs.

---

### 4.2 SVM Pallet & Receipt Verification

**Status:** Design in `pallets/svm-integration/docs/root/README.md`; not implemented.

**Tasks:**

- [ ] **Create SVM pallet** (`pallets/svm-runtime/src/lib.rs`):
  - Storage maps:
    - `PendingSvmExecutions<H256>`: Pending Comit-to-execution mappings.
    - `SvmReceipts<H256>`: Verified receipts by Comit ID.
  - Extrinsic: `submit_svm_receipt(comit_id, account_diffs, logs, execution_trace)`
  - Events: `SvmReceiptSubmitted`, `SvmReceiptVerified`, `SvmReceiptFailed`

- [ ] **Implement receipt verification**:
  - Signature checks: Verify Solana signature on transaction.
  - Replay detection: Ensure receipt isn't resubmitted.
  - State root validation: Recompute expected state root from account diffs; compare with receipt's root.
  - Determinism checks: Verify execution matches expected hash.

- [ ] **Canonical ledger integration**:
  - On successful verification, call `atlasKernel::apply_svm_delta(account_diffs)`.
  - Update balances atomically in canonical ledger.
  - Emit `ComitFinalized` event.

- [ ] **Test suite**:
  - Test valid receipt submissions.
  - Test invalid signatures, mismatched roots, replay attacks.
  - Test canonical ledger updates.

**Acceptance Criteria:**
- SVM pallet compiles and integrates into runtime.
- Receipt verification logic is sound and tested.
- Receipts successfully update canonical ledger.

---

### 4.3 SVM Program Examples & Testing

**Status:** Concept; needs implementation.

**Tasks:**

- [ ] **Example SVM programs**:
  - Token transfer: `programs/svm/transfer/src/lib.rs`
  - Token balance query: `programs/svm/balance_query/src/lib.rs`
  - Simple state mutation: `programs/svm/counter/src/lib.rs`
  - Store in `programs/svm/`.

- [ ] **SVM testing harness**:
  - Build programs and generate `.so` artifacts.
  - Create test fixtures for program execution.
  - Integration tests: submit SVM Comits and verify finalization.

- [ ] **Solana Playground support** (optional):
  - Document how to deploy and test SVM programs in Solana Playground.

**Acceptance Criteria:**
- Example SVM programs compile and run.
- Integration tests verify SVM execution and canonical ledger updates.

---

# ⚛️ PHASE 5: Cross-Domain Orchestration

**Goal:** Enable atomic cross-VM operations (EVM ↔ SVM calls, DEX swaps, etc.).

### 5.1 Dual-VM Dispatcher Implementation

**Status:** Trait defined; mock implementations only.

**Tasks:**

- [ ] **Replace mock implementations** (`pallets/x3-kernel/src/lib.rs`):
  - Implement `DualVmDispatcher::execute_evm_tx` to call into EVM pallet.
  - Implement `DualVmDispatcher::execute_svm_tx` to dispatch to SVM sidecar.
  - Implement `execute_dual_tx` to orchestrate both sequentially.

- [ ] **Execution ordering & atomicity**:
  - Define execution order: EVM first, then SVM (or vice versa; document rationale).
  - Implement rollback logic if either VM fails.
  - Atomic state commits: apply both state diffs or neither.

- [ ] **Cross-VM message passing** (nice-to-have for Phase 5+):
  - Define message protocol for EVM → SVM and SVM → EVM calls.
  - Implement message routing in dispatcher.
  - Add tests for cross-VM interactions.

**Acceptance Criteria:**
- `execute_dual_tx` correctly dispatches to both VMs.
- State changes are atomic (both succeed or both fail).
- Tests verify cross-VM atomicity.

---

### 5.2 Atomic Swap Example (Cross-VM DEX)

**Status:** Concept; needs implementation.

**Tasks:**

- [ ] **Design cross-VM swap**:
  - User on EVM holds Token A; user on SVM holds Token B.
  - Single Comit atomically swaps A → B and B → A.

- [ ] **Implement swap contracts**:
  - EVM: `SwapInitiator.sol` (user deposits Token A; waits for SVM finalization).
  - SVM: `SwapCompleter` program (receives Token A; sends Token B).
  - Coordinator logic in X3 Kernel to enforce both sides execute or both fail.

- [ ] **Integration test**:
  - Create two test accounts (one EVM, one SVM).
  - Submit atomic swap Comit.
  - Verify both sides finalized and balances updated.

**Acceptance Criteria:**
- Atomic swap contract is deployable and testable.
- Cross-VM swaps execute atomically.
- Documentation explains swap flow and Comit structure.

---

# 🛠️ PHASE 6: Developer Tooling & SDKs

**Goal:** Provide high-level APIs for dApp developers.

### 6.1 TypeScript SDK

**Status:** Basic structure in `packages/ts-sdk/`; needs implementation.

**Tasks:**

- [ ] **Core SDK functionality** (`packages/ts-sdk/src/`):
  - `index.ts`: Main exports and API.
  - `client.ts`: Node RPC client wrapper using subxt.js or web3.js.
  - `signer.ts`: Local key management and transaction signing.
  - `comit.ts`: Comit construction and submission.
  - `types.ts`: TypeScript definitions for all on-chain types.

- [ ] **High-level API**:
  ```typescript
  const client = new AtlasClient({ rpc: "http://localhost:9944" });
  
  // Query canonical balance
  const balance = await client.getCanonicalBalance(accountId, assetId);
  
  // Submit EVM transaction
  const txHash = await client.submitEvmTx(hexEncodedTx);
  
  // Submit SVM transaction
  const comitId = await client.submitComit({
    evm: hexEvm,
    svm: hexSvm,
    fee: 1000n,
  });
  ```

- [ ] **Testing**:
  - Unit tests for all SDK functions.
  - Integration tests against running node.
  - Example scripts in `packages/ts-sdk/examples/`.

**Acceptance Criteria:**
- SDK compiles and exports are correct.
- All RPC methods are wrapped with type safety.
- Example scripts run end-to-end.

---

### 6.2 Python SDK

**Status:** Basic structure in `packages/py-sdk/`; needs implementation.

**Tasks:**

- [ ] **Core SDK functionality** (`packages/py-sdk/x3_chain/`):
  - `__init__.py`: Main exports.
  - `client.py`: Node RPC client.
  - `signer.py`: Key management.
  - `comit.py`: Comit construction.
  - `types.py`: Type definitions.

- [ ] **High-level API** (mirror TypeScript):
  - Query canonical balance.
  - Submit EVM/SVM transactions.
  - Event subscriptions.

- [ ] **Testing & Documentation**:
  - Unit and integration tests.
  - Example scripts in `packages/py-sdk/examples/`.
  - Docstrings and usage guide.

**Acceptance Criteria:**
- SDK is pip-installable.
- Example scripts run successfully.
- Documentation is complete.

---

### 6.3 Wallet & Explorer

**Status:** App structure in `apps/wallet/`, `apps/explorer/`; needs implementation.

**Tasks:**

- [ ] **Wallet** (`apps/wallet/`):
  - Account creation and key management (seed phrase backup).
  - Balance queries (EVM and SVM).
  - Transaction building and signing.
  - Integration with MetaMask for EVM; custom UI for SVM.
  - Support for Comit submission.

- [ ] **Explorer** (`apps/explorer/`):
  - Block explorer: list blocks, transactions, validators.
  - Account viewer: balance, assets, transaction history.
  - Comit viewer: search by ID; display status, fees, VM payloads.
  - Real-time event feed.
  - Dark/light mode.

- [ ] **Frontend tech stack**:
  - Framework: React (already in package.json).
  - RPC client: ethers.js (EVM) + custom TS SDK (SVM).
  - UI: Tailwind CSS or Material UI.
  - Build: Vite or Next.js.

**Acceptance Criteria:**
- Wallet can create accounts and display balances.
- Explorer shows live block data.
- Both apps connect to local/testnet node.

---

### 6.4 CLI Utilities

**Status:** Node CLI exists; needs SDK/helper tools.

**Tasks:**

- [ ] **Comit CLI tool** (`tools/comit-cli/`):
  - Subcommands:
    - `comit create --evm <hex> --svm <hex> --fee <amount>`
    - `comit sign --key <path> --data <hex>`
    - `comit submit --url <rpc> --comit <hex>`
    - `comit query --url <rpc> --id <comit-id>`
  - Useful for testing and scripting.

- [ ] **Key management CLI**:
  - `x3-key generate`: Create new keypair.
  - `x3-key import --seed <phrase>`: Import from seed.
  - `x3-key export --key <path>`: Export public key.

- [ ] **Integration testing CLI**:
  - `x3-test deploy-contract --solidity <file>`: Deploy EVM contract.
  - `x3-test submit-program --rust <file>`: Deploy SVM program.
  - `x3-test check-balance --account <id>`: Verify balance.

**Acceptance Criteria:**
- CLI tools are usable and documented.
- Example workflows (e.g., deploy contract + swap) are scripted.

---

# 🧪 PHASE 7: Comprehensive Testing & QA

**Goal:** Validate the entire system under realistic workloads.

### 7.1 Integration Test Suite

**Status:** Basic tests exist; need expansion.

**Tasks:**

- [ ] **End-to-end test scenarios**:
  - Deploy EVM contract.
  - Create SVM account.
  - Submit atomic cross-VM Comit.
  - Query final state via RPC.
  - Verify canonical ledger consistency.

- [ ] **Stress testing**:
  - Generate 100+ Comits per second.
  - Verify block finalization under load.
  - Monitor node memory and CPU.

- [ ] **Fuzzing**:
  - Fuzz Comit payloads and prepare roots.
  - Fuzz contract code (EVM, SVM).
  - Test node resilience to malformed extrinsics.

**Acceptance Criteria:**
- All integration tests pass.
- Node remains stable under 100+ Comits/sec.
- Fuzzing runs for 1+ week without issues.

---

### 7.2 Consensus & Safety Testing

**Status:** Basic consensus in place; needs adversarial testing.

**Tasks:**

- [ ] **Multi-node consensus tests**:
  - 5-node network reaches finality.
  - Network tolerates 1 byzantine validator.
  - Forking is prevented.

- [ ] **Network partition recovery**:
  - Partition network into two groups.
  - Verify no divergence.
  - Heal partition and verify re-sync.

- [ ] **Validator rotation**:
  - Add/remove validators.
  - Verify consensus continues.

**Acceptance Criteria:**
- Multi-node testnet achieves finality reliably.
- Consensus is safe under adversarial conditions.

---

### 7.3 Performance Benchmarking

**Status:** Not yet done.

**Tasks:**

- [ ] **Benchmark key operations**:
  - Comit submission latency.
  - EVM transaction execution time.
  - SVM program execution time.
  - Canonical ledger lookup/update.
  - RPC query response time.

- [ ] **Capacity planning**:
  - Max Comits per block.
  - Max contracts per block.
  - Storage growth rate.
  - Network bandwidth usage.

- [ ] **Report** (`docs/BENCHMARKS.md`):
  - Publish benchmark results.
  - Compare against Ethereum and Solana.
  - Document optimization opportunities.

**Acceptance Criteria:**
- Benchmarks are reproducible.
- Performance meets or exceeds targets (define targets).
- Report guides mainnet deployment.

---

# 🚀 PHASE 8: Testnet Launch & Monitoring

**Goal:** Deploy a public testnet for early adopters and gather feedback.

### 8.1 Testnet Deployment

**Status:** Infrastructure scaffolded; needs launch.

**Tasks:**

- [ ] **Infrastructure setup** (`infra/`):
  - Deploy 5+ validators on testnet infrastructure (AWS, Hetzner, etc.).
  - Configure load balancer for RPC endpoints.
  - Set up monitoring (Prometheus, Grafana).
  - Configure alerting (PagerDuty, Slack).

- [ ] **Chain spec for testnet**:
  - Generate `specs/testnet.json` with testnet authorities.
  - Distribute to validator operators.

- [ ] **Faucet for test X3**:
  - Deploy faucet service (`tools/faucet/`).
  - Allow users to claim test X3 for testing.
  - Rate limiting to prevent spam.

**Acceptance Criteria:**
- 5+ validators are operational.
- Testnet reaches finality regularly.
- Faucet distributes test X3.
- Monitoring apps/dash-legacy-2-legacy-2boards show real-time metrics.

---

### 8.2 Documentation & Community

**Status:** Partial docs exist; need completion.

**Tasks:**

- [ ] **Complete documentation**:
  - `docs/GETTING_STARTED.md`: Quick start guide.
  - `docs/DEPLOY_CONTRACT.md`: EVM contract deployment tutorial.
  - `docs/DEPLOY_PROGRAM.md`: SVM program deployment tutorial.
  - `docs/ATOMIC_SWAP.md`: Cross-VM swap walkthrough.
  - `docs/API_REFERENCE.md`: Full RPC and SDK reference.
  - `docs/TESTNET.md`: Testnet info, faucet, explorers.

- [ ] **Community resources**:
  - Discord server for developers.
  - GitHub Discussions for questions.
  - Example dApps and repos.

- [ ] **Grant program** (optional):
  - Allocate testnet X3 for developers building on X3 Chain.
  - Showcase community projects.

**Acceptance Criteria:**
- Documentation is comprehensive and easy to follow.
- Community channels are active.
- Early dApps are deploying on testnet.

---

### 8.3 Bug Bounty & Security Audit

**Status:** Not yet conducted.

**Tasks:**

- [ ] **Internal security review**:
  - Code review of kernel pallet, VM integrations, consensus.
  - Check for common vulnerabilities (overflow, reentrancy, etc.).

- [ ] **External audit**:
  - Engage reputable auditor (e.g., Quantstamp, Trail of Bits) for 4-week audit.
  - Remediate findings.

- [ ] **Bug bounty program**:
  - Launch public bug bounty on HackerOne or similar.
  - Allocate bounty pool (e.g., 50k X3).
  - Publish program rules and scope.

**Acceptance Criteria:**
- Internal review is complete; no critical issues remain.
- External audit passes with no critical findings.
- Bug bounty is live and receiving submissions.

---

# 🎯 PHASE 9: Mainnet Preparation & Launch

**Goal:** Prepare for production deployment.

### 9.1 Mainnet Readiness Checklist

**Tasks:**

- [ ] **Security & stability**:
  - ✅ 3+ external audits completed (or 2 audits + 6-month testnet uptime).
  - ✅ Bug bounty program ran for 3+ months.
  - ✅ Zero critical/high-severity issues.

- [ ] **Performance & scalability**:
  - ✅ Benchmarks show 100+ Comits/sec throughput.
  - ✅ Latency <10s block time consistently.
  - ✅ Storage requirements documented and acceptable.

- [ ] **Governance**:
  - ✅ On-chain governance pallet deployed and tested.
  - ✅ Sudo is disabled (transitioned to governance).
  - ✅ Treasury and staking parameters are locked.

- [ ] **Economic model**:
  - ✅ Token economics are finalized and published.
  - ✅ Inflation schedule is locked.
  - ✅ Fee structure is optimized for mainnet.

- [ ] **Operations**:
  - ✅ Validator set is decentralized (50+ validators target).
  - ✅ Monitoring and alerting are operational.
  - ✅ Disaster recovery procedures are documented and tested.

**Acceptance Criteria:**
- All checklist items are green.
- Mainnet is ready for launch.

---

### 9.2 Mainnet Genesis & Launch

**Tasks:**

- [ ] **Genesis block**:
  - Finalize validators for mainnet.
  - Generate mainnet chain spec with genesis balances.
  - Distribute to validators.

- [ ] **Mainnet launch**:
  - Coordinate validator startup.
  - Monitor first blocks and finality.
  - Announce mainnet live.

- [ ] **Post-launch monitoring**:
  - 24/7 incident response.
  - Real-time apps/dash-legacy-2-legacy-2boards.
  - Community support channels.

**Acceptance Criteria:**
- Mainnet launches successfully.
- First blocks finalize within expected time.
- Validator set is healthy and decentralized.

---

### 9.3 Post-Mainnet Roadmap

**Future phases** (beyond launch):

- **Phase 10:** Interoperability (bridge to Ethereum, Solana, other L1s).
- **Phase 11:** Scaling layer (Rollups, Parachains).
- **Phase 12:** Privacy (zk-proofs, MEV protection).
- **Phase 13:** Governance (DAO tooling, proposal framework).

---

## 📊 Implementation Timeline Summary

| Phase | Focus | Duration | Parallel Work |
|-------|-------|----------|---------------|
| 1 | Core Runtime | 2–3 weeks | SDKs (Phase 6 foundations) |
| 2 | Node & Networking | 1–2 weeks | EVM integration (Phase 3 setup) |
| 3 | EVM Integration | 2–3 weeks | SVM design (Phase 4 prep) |
| 4 | SVM Integration | 2–3 weeks | Tooling (Phase 6 dev) |
| 5 | Cross-Domain | 1–2 weeks | Testing prep |
| 6 | Developer Tooling | 1–2 weeks | Community docs |
| 7 | Testing & QA | 2–3 weeks | Parallel with 6 |
| 8 | Testnet Launch | 1–2 weeks | Documentation |
| 9 | Mainnet Prep | 2–4 weeks | Audits, governance |
| **Total** | **Full Stack** | **~4–6 months** | **Iterative & overlapping** |

---

## 🎯 Success Metrics

At the end of Phase 9, the blockchain is considered **fully functional** when:

1. ✅ **Consensus:** Multi-node network achieves finality reliably (>99.9% uptime).
2. ✅ **Execution:** Both EVM and SVM transactions execute deterministically; canonical ledger is single source of truth.
3. ✅ **Atomicity:** Cross-domain Comits execute atomically; either both VMs finalize or both fail.
4. ✅ **Security:** Audited; no critical vulnerabilities; bug bounty program ran successfully.
5. ✅ **Tooling:** SDKs, wallet, explorer, CLI are production-ready and well-documented.
6. ✅ **Throughput:** Sustains 100+ Comits/sec; block time <10s; finality <30s.
7. ✅ **Community:** Mainnet is live; 50+ validators; active ecosystem of dApps.

---

## Next Steps

1. **Prioritize Phase 1 tasks** and assign owners.
2. **Set up CI/CD** to automate testing and Wasm builds.
3. **Create GitHub issues** for each task with acceptance criteria.
4. **Schedule weekly sync** to track progress and unblock issues.
5. **Engage community** early (testnet, docs, examples).

---

Good luck! 🚀
