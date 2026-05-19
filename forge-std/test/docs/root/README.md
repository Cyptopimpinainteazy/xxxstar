# X3 Chain L1

[![Build Status](https://github.com/Cyptopimpinainteazy/x3-chain/actions/workflows/ci.yml/badge.svg)](https://github.com/Cyptopimpinainteazy/x3-chain/actions/workflows/ci.yml) [![SVM Counter Integration](https://github.com/Cyptopimpinainteazy/x3-chain/actions/workflows/svm-counter-integration.yml/badge.svg)](https://github.com/Cyptopimpinainteazy/x3-chain/actions/workflows/svm-counter-integration.yml) [![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](./LICENSE)

X3 Chain is a next-generation Layer-1 blockchain purpose-built to host dual virtual machines (EVM + SVM), enabling native interoperability between Ethereum-style smart contracts and Solana-style Sealevel programs. The network is optimized for cross-domain composability, featuring a native asset layer, predictable execution semantics, and atomic cross-chain operations to bridge ecosystem liquidity without trusted intermediaries.

## Table of Contents

1. [Vision & Core Features](#vision--core-features)
2. [Current Status](#current-status)
3. [Architecture Overview](#architecture-overview)
4. [Development Setup](#development-setup)
5. [Build Instructions](#build-instructions)
6. [Quick Start](#quick-start)
7. [Running a Node](#running-a-node)
8. [RPC API Surface](#rpc-api-surface)
9. [Consensus](#consensus)
10. [Network Configuration](#network-configuration)
11. [Key Management](#key-management)
12. [Basic Usage Examples](#basic-usage-examples)
13. [Account Authorization](#account-authorization)
14. [Testing & Quality Gates](#testing--quality-gates)
15. [Contribution Guidelines](#contribution-guidelines)
16. [Roadmap Snapshot](#roadmap-snapshot)
17. [Developer Templates](#developer-templates)
18. [Resources & Further Reading](#resources--further-reading)
19. [License](#license)

---

## Vision & Core Features

- **Dual-VM Execution (EVM + SVM):** Run Solidity/Vyper contracts and Sealevel programs side-by-side with deterministic consensus ordering. X3 Chain exposes a unified account abstraction to simplify cross-VM asset flows.
- **Native Asset Layer:** The X3 native asset powers staking, fee markets, and rewards. Additional assets can be registered via asset pallets and used across both VMs without wrapping.
- **Atomic Cross-Chain Operations:** Built-in message-lane primitives let developers submit atomic transactions that span multiple domains, eliminating the need for fragile multi-step bridging.
- **High-Performance Substrate Foundation:** Built on Substrate for modularity, runtime upgrades, and rich tooling while maintaining a custom runtime tuned for X3’ heterogeneous VM workloads.

## Current Status

🎉 **X3 Chain Testnet v1 is NOW LIVE!**

- ✅ **Testnet Deployment:** Public testnet with 3+ validators, RPC endpoints, and faucet service operational
- ✅ **X3 Kernel MVP:** Comit submission, nonce management, asset registry, and canonical ledger primitives implemented and wired into runtime
- ✅ **Runtime Integration:** Aura + GRANDPA consensus, transaction payment, and X3 Kernel fully integrated for end-to-end Comit processing
- ✅ **Node Service & RPC:** Node starts with Aura + GRANDPA consensus, networking (peer discovery and sync), with HTTP JSON-RPC on `127.0.0.1:9933` and WebSocket JSON-RPC on `127.0.0.1:9944`
- ✅ **X3 Kernel RPC:** Five X3 Kernel RPC methods exposed via `node/src/rpc.rs::create_full()` for querying canonical ledger, asset metadata, authorization status, and authorities
- ⚠️ **Dual-VM Adapters (EVM/SVM):** Using mock executors for testnet; real Frontier/SVM execution integration in development
- 🚧 **Governance:** Sudo remains enabled for development; governance pallet integration **NOT YET IMPLEMENTED**

**Testnet RPC**: `http://rpc.testnet.x3-chain.io:9933`  
**Faucet**: `https://faucet.testnet.x3-chain.io`  
**See**: `docs/reports/TESTNET_ANNOUNCEMENT.md` for details

---

## Architecture Overview

| Component               | Status            | Summary                                                                                                                                        |
| ----------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| **Runtime**             | Dev-ready         | FRAME-based runtime integrating Aura + GRANDPA consensus, balances, transaction payment, sudo (for dev), and scaffolding for VM orchestration. |
| **Dual VM Layer**       | In Development    | Frontier-based EVM adapter and SVM bridge are being wired into the canonical ledger with forthcoming execution and RPC exposure.               |
| **Cross-Domain Kernel** | Implemented (MVP) | X3 Kernel pallet anchors Comit submission, asset registry, and canonical ledger updates powering dual-VM coordination.                      |
| **Node Service**        | In Progress       | Provides RPC, telemetry, and networking services with hooks for future Frontier JSON-RPC and SVM execution interfaces.                         |
| **Tooling**             | In Progress       | CLI utilities cover chain specs and key handling; Comit crafting helpers and SDK improvements are scheduled next.                              |
### Developer Tools: BMAD Method
The repository includes a small integration wrapper for BMAD Method under `crates/vibe-bmad`. BMAD is an AI-driven agile development tool that helps with planning, workflows and architecture decisions.

Quick usage:
```sh
cd crates/vibe-bmad
npm run install-bmad
npm run workflow-init
```
BMAD requires Node.js >= 20. See `crates/vibe-bmad/docs/root/README.md` for more details.

---

## Development Setup

### Prerequisites

- **Operating System:** Linux or macOS (Windows via WSL2 recommended).
- **Rust Toolchain:** `rustup` with the stable toolchain (or project-specified override via `rust-toolchain.toml`).
  ```bash
  rustup toolchain install stable
  rustup default stable
  rustup target add wasm32-unknown-unknown
  rustup component add rustfmt clippy
  ```
  Verify installation:
  ```bash
  rustup show active-toolchain
  cargo --version
  ```
- **Build Dependencies:** `cmake`, `pkg-config`, `openssl`, `libclang`, and `clang` (required by Substrate).
  ```bash
  # Debian/Ubuntu
  sudo apt update
  sudo apt install -y build-essential cmake pkg-config libssl-dev git clang libclang-dev
  ```
- **Substrate Dependencies:** Refer to the [official Substrate prerequisites](https://docs.substrate.io/install/) for platform-specific instructions.
- **Optional:** `just`, `direnv`, Docker, and Polkadot.js apps for local interactions.

### Repository Setup

```bash
git clone https://github.com/your-org/x3-chain.git
cd x3-chain
```

---

## Build Instructions

Compile the X3 Chain node and runtime artifacts:

```bash
cargo build --release
```

Key artifacts:

- `target/release/x3-chain-node` – Native node binary.
- `runtime/wasm/x3_chain_runtime.compact.wasm` – Runtime WASM (generated via build script).

For iterative development builds:

```bash
cargo build
```

## Quick Start

1. **Build the binaries**

   ```bash
   cargo build --release
   ```

2. **Launch a development node**

   ```bash
   ./target/release/x3-chain-node --dev --tmp
   ```

   The node will start with Aura + GRANDPA consensus and expose HTTP JSON-RPC on `127.0.0.1:9933` and WebSocket JSON-RPC on `127.0.0.1:9944`.

3. **Query X3 Kernel via HTTP JSON-RPC**

   Check authorized accounts:
   ```bash
   curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
        -d '{"id":1,"jsonrpc":"2.0","method":"atlasKernel_getAuthorizedAccounts","params":[null]}'
   ```

   Get current authorities:
   ```bash
   curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
        -d '{"id":1,"jsonrpc":"2.0","method":"atlasKernel_getAuthorities","params":[null]}'
   ```

   Query canonical balance (example):
   ```bash
   curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
        -d '{"id":1,"jsonrpc":"2.0","method":"atlasKernel_getCanonicalBalance","params":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",0,null]}'
   ```

4. **Submit Comit extrinsic over WebSocket**

   ```bash
   cargo install --locked subxt-cli
   subxt extrinsic submit \
     --url ws://127.0.0.1:9944 \
     --suri //Alice \
     atlasKernel submitComit \
     0x0102030405060708090a0b0c0d0e0f00112233445566778899aabbccddeeff00 \
     0x \
     0x0102 \
     0 \
     1000000000000 \
     0x0000000000000000000000000000000000000000000000000000000000000000
   ```

   Watch Comit events:
   ```bash
   subxt event watch --url ws://127.0.0.1:9944 atlasKernel
   ```

### JSON-RPC API Surface

The X3 Chain node exposes a comprehensive JSON-RPC interface assembled in `node/src/rpc.rs::create_full()` combining standard Substrate system RPCs, transaction-payment RPCs, X3-specific kernel RPCs, and optional Ethereum/SVM-compatible RPC endpoints.

**Endpoints:**
- **HTTP RPC:** `http://127.0.0.1:9933` (curl, automation, health probes)
- **WebSocket RPC:** `ws://127.0.0.1:9944` (Polkadot.js, subxt, subscriptions for `newHeads`, `finalizedHeads`, events)

---

#### **Substrate System RPCs** (Standard)

These are wired through `io.merge(sc_rpc::system::SystemApi::to_rpc_methods())` and provide core node metadata, health monitoring, and peer information:

- `system_health` – Node health status (peers, syncing, finalized blocks)
- `system_name` – Node implementation name
- `system_version` – Node version string
- `system_chain` – Chain name (e.g., "X3 Chain")
- `system_chainType` – Chain type (Development, Local, Live)
- `system_properties` – Chain properties (token symbol, decimals, SS58 format)
- `system_accountNextIndex` – Next nonce for an account
- `system_addReservedPeer` – Add reserved peer (permissioned networks)
- `system_removeReservedPeer` – Remove reserved peer
- `system_peers` – List connected peers
- `system_networkState` – Full network state (peer IDs, addresses)
- `system_nodeRoles` – Node role (full, authority, light)
- `system_localPeerId` – Local peer ID
- `system_localListenAddresses` – Listening multiaddrs

---

#### **Chain RPCs** (Standard)

Wired via `io.merge(sc_rpc::chain::ChainApi::to_rpc_methods())` for block and header retrieval:

- `chain_getHeader` – Block header by hash
- `chain_getBlock` – Full block by hash
- `chain_getBlockHash` – Block hash by number
- `chain_getFinalizedHead` – Hash of finalized head
- `chain_subscribeNewHeads` – Subscribe to new block headers (WebSocket)
- `chain_subscribeFinalizedHeads` – Subscribe to finalized headers (WebSocket)
- `chain_unsubscribeNewHeads` – Unsubscribe from new heads
- `chain_unsubscribeFinalizedHeads` – Unsubscribe from finalized heads

---

#### **State RPCs** (Standard)

Wired via `io.merge(sc_rpc::state::StateApi::to_rpc_methods())` for direct storage queries and metadata:

- `state_call` – Call into runtime API at block hash
- `state_getMetadata` – Runtime metadata (types, pallets, calls, events)
- `state_getStorage` – Storage value at key
- `state_getStorageHash` – Storage hash at key
- `state_getStorageSize` – Storage size at key
- `state_queryStorage` – Query storage across block range
- `state_queryStorageAt` – Query storage at specific block
- `state_subscribeStorage` – Subscribe to storage changes (WebSocket)
- `state_unsubscribeStorage` – Unsubscribe from storage changes
- `state_getRuntimeVersion` – Runtime version (spec_name, spec_version, apis)
- `state_subscribeRuntimeVersion` – Subscribe to runtime version changes (WebSocket)
- `state_unsubscribeRuntimeVersion` – Unsubscribe from runtime version changes

---

#### **Transaction Payment RPCs** (Standard)

Wired via `io.merge(pallet_transaction_payment_rpc::TransactionPayment::to_rpc_methods())` for fee estimation:

- `payment_queryInfo` – Fee estimation for an extrinsic (weight, partial_fee, class)
- `payment_queryFeeDetails` – Detailed fee breakdown (base, length, adjusted_weight_fee)
- `payment_queryCallInfo` – Fee estimation for runtime call without extrinsic
- `payment_queryCallFeeDetails` – Detailed fee breakdown for runtime call

---

#### **X3 Kernel RPCs** (Custom)

Custom X3-specific methods defined in `pallet_x3_kernel::rpc` and wired in `node/src/rpc.rs`. These query canonical ledger state, asset metadata, and authorization:

- `atlasKernel_getCanonicalBalance(account, asset_id, at?)` – Query canonical ledger balance for account and asset
- `atlasKernel_getAssetMetadata(asset_id, at?)` – Get asset symbol, decimals, and metadata
- `atlasKernel_isAuthorized(account, at?)` – Check if account is authorized for privileged operations
- `atlasKernel_getAuthorizedAccounts(at?)` – List all authorized accounts
- `atlasKernel_getAuthorities(at?)` – Get current validator authority set

**Example:**
```bash
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"atlasKernel_getCanonicalBalance","params":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",0,null]}'
```

---

#### **Ethereum-Compatible RPCs** (Experimental via Frontier)

⚠️ **Status: Experimental** – Wired via `node/src/rpc_frontier.rs::create_frontier_rpc()` when `frontier` feature is enabled. These methods are backed by runtime API calls to the X3 Kernel EVM adapter and provide Ethereum JSON-RPC compatibility for EVM contract interactions.

**Available Methods:**
- `eth_getBalance(address, block?)` – Query native balance for EVM address (returns hex wei)
- `eth_getCode(address, block?)` – Retrieve contract bytecode for EVM address
- `eth_getStorageAt(address, slot, block?)` – Read EVM storage slot
- `eth_getTransactionCount(address, block?)` – Get account nonce
- `eth_call(tx_object, block?)` – Execute read-only EVM call (gas-free)
- `eth_estimateGas(tx_object)` – Estimate gas required for transaction
- `eth_sendRawTransaction(signed_rlp_tx)` – Submit signed Ethereum transaction (returns keccak256 tx hash)

**Partial Implementation Notes:**
- Transaction receipts (`eth_getTransactionReceipt`) not yet exposed
- Block and transaction history queries not yet implemented
- Event logs (`eth_getLogs`) not yet exposed
- Subscription methods (`eth_subscribe`, `eth_newFilter`) not yet implemented

**Example:**
```bash
# Query EVM balance
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"eth_getBalance","params":["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb","latest"]}'

# Execute read-only EVM call
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"eth_call","params":[{"to":"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb","data":"0x70a08231000000000000000000000000742d35Cc6634C0532925a3b844Bc9e7595f0bEb"},"latest"]}'
```

---

#### **SVM-Compatible RPCs** (Experimental)

⚠️ **Status: Experimental** – Wired via `node/src/rpc_frontier.rs::create_svm_rpc()` for querying SVM (Solana Virtual Machine) state. These methods are backed by runtime API calls to the X3 Kernel SVM adapter.

**Available Methods:**
- `svm_getBalance(pubkey)` – Query lamport balance for base58 or hex SVM pubkey (returns `{ "value": u64 }`)
- `svm_isProgram(pubkey)` – Check if pubkey has deployed executable program (returns `{ "result": bool }`)

**Partial Implementation Notes:**
- SVM transaction submission not yet exposed
- SVM account data queries not yet implemented
- SVM program execution RPC not yet exposed
- SVM logs and signatures not yet available

**Example:**
```bash
# Query SVM balance
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"svm_getBalance","params":["11111111111111111111111111111111"]}'
```

---

**Rate Limiting:** The RPC server integrates `RateLimiter` from `node/src/rpc.rs` for DOS protection. Default limits apply per-connection.

**FlashFinalityGadget:** `node/src/rpc.rs::create_full()` passes `finality_proof_provider` to enable finality proofs for light clients and cross-chain bridges.

   You should see `ComitSubmitted`, `ComitExecutionStarted`, `ComitExecutionCompleted`, and `ComitFinalized` events in order as the runtime processes the transaction.

---

## Security Scanning & Fuzzing (contest procedure)

To help with the million-dollar bug bounty, this repository provides a
lightweight framework for analysing every Solidity contract.  See
`scan_all.sh` at the repo root; running it will:

1. install npm dependencies (OpenZeppelin, etc.)
2. regenerate `remappings.txt` for solidity imports
3. run **Slither** on each non-test `.sol` file
4. run **Semgrep** across `contracts/`
5. execute every Foundry test/fuzz suite discovered under `contracts/`

```bash
./scan_all.sh | tee audit-output.txt
```

Fix the reported warnings / failing tests and re-run until the output
is clean.  Manual review should accompany these automated scans, focusing
on critical modules such as `X3AMM.sol`, `AtlasTreasury.sol`,
`AISwarmCoordinator.sol`, etc.  Additional invariants and fuzz harnesses
can be added in any package (`tests/security` already contains one).

A polished security results bundle (logs + corrected code) is the
submission for the contest.

---

## Running a Node

✅ **STATUS: Node binary is functional with networking and RPC server.**

**Launch a development node:**

```bash
./target/release/x3-chain-node --dev
```

Useful flags:

- `--tmp` – Run with an in-memory DB (cleared on exit).
- `--rpc-port <PORT>` – Override the default HTTP RPC port (default: `9933`).
- `--ws-port <PORT>` – Override the default WebSocket RPC port (default: `9944`).
- `--rpc-cors all` – Allow RPC calls from any origin (development only).
- `--log runtime=debug` – Increase logging verbosity for runtime modules.

**Default HTTP RPC Bind Address:** `127.0.0.1:9933`  
**Default WebSocket RPC Bind Address:** `127.0.0.1:9944`

The node starts with:
- **Aura consensus** for block authoring (6-second slot duration)
- **GRANDPA finality** for Byzantine fault-tolerant finalization
- **Networking** with libp2p peer discovery and block sync
- **HTTP JSON-RPC server** on `127.0.0.1:9933`
- **WebSocket JSON-RPC server** on `127.0.0.1:9944`

Stop the node with `Ctrl+C`. Logs are streamed to stdout by default.

**Limitations:**
- Frontier/SVM execution adapters are not yet wired (dual-VM execution uses mock receipts)
- Governance pallet not integrated (sudo remains enabled)

## Consensus

X3 Chain currently leverages the Aura block authoring engine paired with GRANDPA finality, delivering a 6-second target block time and deterministic development workflows. The core team is actively evaluating a future migration path toward a Tendermint-style BFT consensus set to enhance liveness under adversarial network conditions while preserving runtime upgrade flexibility.

---

## Network Configuration

X3 Chain ships with multiple chain specifications:

| Spec                     | Command                                               | Description                                                            |
| ------------------------ | ----------------------------------------------------- | ---------------------------------------------------------------------- |
| Development              | `x3-chain-node --dev`                             | Single-node authority, instant block production.                       |
| Local Testnet            | `x3-chain-node --chain local -d /tmp/x3-local` | Multi-node authority with deterministic keys (use `--alice`, `--bob`). |
| Staging/Mainnet (future) | `x3-chain-node --chain x3`                     | Use generated chain spec files committed by core team.                 |

Generate custom chain specs:

```bash
# Export plain chain spec
./target/release/x3-chain-node build-spec --disable-default-bootnode > x3.json

# Export raw chain spec (ready for launch)
./target/release/x3-chain-node build-spec --chain x3.json --raw --disable-default-bootnode > x3-raw.json
```

Distribute the raw chain spec to validators to ensure consensus on initial state.

---

## Key Management

Use Substrate tooling (`subkey`) to generate and manage keys:

```bash
# Install subkey (ships with Substrate toolchain)
cargo install --force subkey --git https://github.com/paritytech/substrate --locked

# Generate aura/grandpa keys
subkey generate --scheme sr25519   # Aura authority
subkey inspect "<SECRET_PHRASE>" --scheme ed25519  # Grandpa authority
```

Inject keys into the keystore:

```bash
./target/release/x3-chain-node \
  --chain x3 \
  --name "Validator-01" \
  --base-path /var/lib/x3 \
  key insert \
  --scheme sr25519 \
  --suri "<SECRET_PHRASE>" \
  --key-type aura
```

For production, store secret phrases securely (e.g., HSM, Hashicorp Vault). Never expose raw secret seeds in scripts or logs.

---

## Basic Usage Examples

### 1. Interact via RPC

Start the node (HTTP RPC on `127.0.0.1:9933`, WebSocket on `127.0.0.1:9944`):

```bash
./target/release/x3-chain-node --dev
```

Query authorized accounts via X3 Kernel RPC:

```bash
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
     -d '{"id":1,"jsonrpc":"2.0","method":"atlasKernel_getAuthorizedAccounts","params":[null]}'
```

**Available RPC Methods:**

The node exposes a comprehensive JSON-RPC interface combining:
- **Substrate System RPCs** (`system_*`) – Node health, metadata, peer information
- **Chain RPCs** (`chain_*`) – Block and header retrieval, subscriptions
- **State RPCs** (`state_*`) – Storage queries, runtime metadata, versioning
- **Transaction Payment RPCs** (`payment_*`) – Fee estimation and breakdown
- **X3 Kernel RPCs** (`atlasKernel_*`) – Canonical ledger, asset metadata, authorization
- **Ethereum-compatible RPCs** (`eth_*`) – EVM balance, code, storage, call, estimateGas, sendRawTransaction (experimental)
- **SVM-compatible RPCs** (`svm_*`) – SVM balance, program queries (experimental)

See the **JSON-RPC API Surface** section below for complete method reference and examples.

**X3 Kernel RPC Methods:**
- `atlasKernel_getCanonicalBalance(account, asset_id, at?)` – Query canonical ledger balance
- `atlasKernel_getAssetMetadata(asset_id, at?)` – Get asset symbol and decimals
- `atlasKernel_isAuthorized(account, at?)` – Check account authorization status
- `atlasKernel_getAuthorizedAccounts(at?)` – List all authorized accounts
- `atlasKernel_getAuthorities(at?)` – Get current authority set

### 2. Deploy Solidity Contracts

1. Start node with EVM RPC compatibility (future release flag).
2. Use Hardhat/Foundry endpoint: `http://127.0.0.1:8545`.
3. Deploy contract as on any Ethereum-compatible network.

### 3. Execute SVM Programs (Roadmap)

- Build SVM program with Solana toolchain.
- Submit via X3 Chain SVM adaptor pallet (forthcoming).
- Monitor execution via RPC subscription.

### 4. Atomic Cross-Chain Operation (Simulated)

1. Start the X3 Chain node alongside your target counterparty chain (e.g., a local Substrate relay or Ethereum devnet) ensuring both expose RPC endpoints.
2. Use the X3 CLI (roadmap) to draft a cross-domain manifest and submit it via the `x3_chain_cross_chain_submit` RPC; during active development you can mock this with `author_submitExtrinsic` carrying the kernel pallet call.
3. Observe the composite transaction status via `system.events` and confirm both VM executions are finalized in the same block.
4. Inspect `atlasKernel.lanes` RPC (coming soon) or node logs tagged `x3-kernel` to verify lane commitments and relay messages.

### Account Authorization

The X3 Kernel implements an account authorization system that controls which accounts can submit Comits. Authorized accounts are managed through privileged extrinsics and checked during submission.

#### Authorization Management

**Authorize an Account:**
```rust
// Requires root privileges (via sudo in development)
atlasKernel.authorize_account(account_id)
// Some tooling may expose this as camelCase: authorizeAccount
```

**Deauthorize an Account:**
```rust
// Requires root privileges (via sudo in development)
atlasKernel.deauthorize_account(account_id)
// Some tooling may expose this as camelCase: deauthorizeAccount
```

**Check Authorization Status:**
```rust
// Runtime API
is_authorized(account_id) -> bool
```

#### Authorization in Code

Authorization is enforced in the `auth_check()` function by verifying membership in `AuthorizedAccounts` storage:

```rust
// In submit_comit extrinsic
let operation_context = Self::encode_submit_comit_context(&who, comit_id);
Self::auth_check(&who, &operation_context)?;
```

The `auth_check()` implementation:
```rust
fn auth_check(caller: &T::AccountId, _operation_context: &[u8]) -> Result<(), DispatchError> {
    #[cfg(not(feature = "dev-bypass"))]
    {
        if AuthorizedAccounts::<T>::contains_key(caller) {
            Ok(())
        } else {
            Err(Error::<T>::Unauthorized.into())
        }
    }
}
```

#### Example: Authorizing Alice for Comit Submission

Using `subxt` CLI (once node is functional):

```bash
# Alice authorizes Bob's account (Alice has sudo privileges)
subxt extrinsic submit \
  --url ws://127.0.0.1:9944 \
  --suri //Alice \
  sudo sudo \
  atlasKernel authorizeAccount 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty

# Verify authorization
subxt rpc call \
  --url ws://127.0.0.1:9944 \
  atlasKernel_isAuthorized \
  5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty
```

#### Authorization Storage

Authorized accounts are stored in:
```rust
#[pallet::storage]
pub type AuthorizedAccounts<T: Config> = 
    StorageMap<_, Blake2_128Concat, T::AccountId, (), ValueQuery>;
```

Accounts are present with unit value `()` when authorized and removed from storage when deauthorized. Authorization is checked using `contains_key()` rather than reading a boolean value.

#### Events

The system emits events for authorization changes:
```rust
Event::AccountAuthorized { account } // When account is authorized
Event::AccountDeauthorized { account } // When account is deauthorized
```

#### Testing Authorization

Test coverage includes:
- `authorize_account_successful`: Root can authorize accounts
- `deauthorize_account_successful`: Root can revoke authorization
- `submit_comit_fails_for_unauthorized_account`: Unauthorized accounts cannot submit (returns `Error::<T>::Unauthorized`)
- All 43 pallet tests verify authorization logic with `dev-bypass` feature disabled in production mode

---

## Testing & Quality Gates

Run unit tests across workspace members:

```bash
cargo test --all
```

Linting and formatting:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Runtime checks:

```bash
cargo test -p x3-chain-runtime
cargo build -p x3-chain-runtime --release --features runtime-benchmarks
```

Continuous Integration (GitHub Actions) enforces these gates, alongside WASM runtime builds for determinism.

---

## ☢️ YOLO FINISHER v5.0 — Nuclear Finalization

The repository enforces the **YOLO FINISHER v5.0** protocol for finalization.
No code is considered "shipped" until it passes the 100/100 readiness gate.

### Core Rules

1. **FAIL IF UNCERTAIN**: In the face of ambiguity, choose the safest interpretation and implement it. Never defer.
2. **INTENT RECOVERY**: Unused code defaults to *intended* and must be wired. Delete only if proven dead.
3. **SYMMETRY**: Every architectural element needs its counterpart (Write/Read, Emit/Consume).
4. **CONFIG LAW**: Every configuration flag must actively alter behavior and be validated on startup.
5. **COLD START**: The system must bootstrap and succeed on a fresh machine with zero state.

### Finalization Stack

Run the nuclear finisher stack in sequence:
`CARTOGRAPHER` → `ARCHAEOLOGIST` → `BREAKER` → `AUDITOR` → `INTENT ANALYST` → `INTEGRATOR` → `VERIFIER` → `FIXER` → `ECONOMIST` → `CHAOS ENGINE` → `COMPLETION JUDGE`

### Usage

```bash
# Start the autonomous finalization daemon
npm run finisher:nuclear

# Run specific gates
make finish-score   # Check readiness score
make finish-audit   # Security & Economics audit
make finish-chaos   # Chaos & Fuzzer injection
```

See `.github/prompts/finisher-*.prompt.md` for detailed agent roles.

---

## Contribution Guidelines

1. **Fork & Branch:** Create feature branches from `main`.
2. **Coding Standards:** Follow Rust best practices, ensure `cargo fmt` and `cargo clippy` pass.
3. **Commits:** Use descriptive messages; reference GitHub issues or PRs when applicable.
4. **Testing:** Add unit/integration tests covering new functionality.
5. **PR Review:** Request review from X3 Chain maintainers; expect automated checks before merge.
6. **Security:** Report vulnerabilities privately to the core team (security@x3-chain.io) before public disclosure.

Please read `CONTRIBUTING.md` (forthcoming) for detailed policies, CLA requirements, and governance.

---

## Roadmap Snapshot

- ✅ X3 Kernel pallet MVP landed with Comit submission, asset registry, and canonical ledger primitives now available on-chain.
- ✅ Runtime integrates Aura + GRANDPA consensus, transaction payment, and X3 Kernel wiring for end-to-end Comit handling.
- 🚧 Dual VM adapters (Frontier EVM + SVM bridge) under active development with a developer preview targeted for the next milestone.
- 🔜 Next up: Tendermint-style consensus evaluation, governance pallet activation to retire sudo, and comprehensive runtime benchmarking.

Progress is tracked publicly via our GitHub Projects board.

---

## Developer Templates

- Curated template matrix: [/docs/docs/docs/templates/X3_DEVELOPER_TEMPLATES.md](/docs/docs/docs/templates/X3_DEVELOPER_TEMPLATES.md)
- Local starter folders: [/docs/templates/x3-chain/README.md](/docs/templates/x3-chain/README.md)
- Quick matrix bootstrap helper: `./scripts/bootstrap-x3-template-matrix.sh`

---

## Resources & Further Reading

- Local master docs index: [../master/INDEX.md](/docs/master/INDEX.md)
- X3 Chain Documentation (coming soon): [https://docs.x3-chain.io](https://docs.x3-chain.io)
- X3 Chain Cross-Chain Primer: [https://labs.x3-chain.io/cross-chain-primer](https://labs.x3-chain.io/cross-chain-primer)
- Substrate Developer Hub: [https://docs.substrate.io](https://docs.substrate.io)
- FRAME Runtime Overview: [https://docs.substrate.io/build/runtime/](https://docs.substrate.io/build/runtime/)
- Substrate Node Template: [https://github.com/substrate-developer-hub/substrate-node-template](https://github.com/substrate-developer-hub/substrate-node-template)
- Polkadot.js Apps: [https://polkadot.js.org/apps/](https://polkadot.js.org/apps/)
- Solana Sealevel Docs: [https://docs.solana.com/developing/programming-model/overview](https://docs.solana.com/developing/programming-model/overview)
- Ethereum JSON-RPC Spec: [https://ethereum.org/en/developers/docs/apis/json-rpc/](https://ethereum.org/en/developers/docs/apis/json-rpc/)

---

## License

X3 Chain is released under the Apache License 2.0. See `LICENSE` for details.
