# X3 Atomic Star

A Substrate-based blockchain with native cross-VM execution across X3Native, X3Evm, and X3Svm domains.

> **Current Status:** v0.4 Internal Testnet Candidate
> See [CURRENT_MAINNET_STATUS.md](./CURRENT_MAINNET_STATUS.md) for an accurate, honest assessment.

---

## Quick Start

### 1. Install prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Protobuf + clang (required by Substrate)
sudo apt-get update
sudo apt-get install -y protobuf-compiler clang llvm

# WASM target (required for runtime build)
rustup target add wasm32-unknown-unknown
```

### 2. Build the node binary

```bash
cargo build --release -p x3-chain-node

# Verify it works
./target/release/x3-chain-node --version
```

### 3. Run a dev node

```bash
./scripts/start-x3-chain.sh
# or directly:
./target/release/x3-chain-node --chain dev --tmp --rpc-port 9933 --validator --alice
```

### 4. Run a 3-validator local testnet

```bash
./scripts/testnet-full-launch.sh
```

### 5. Connect a wallet

Open [Polkadot.js Apps](https://polkadot.js.org/apps) → Development → Local Node → `ws://127.0.0.1:9944`

---

## Architecture

```
X3 Atomic Star
├── Consensus: Aura (block production) + GRANDPA (finality)
├── Execution Domains:
│   ├── X3Native  — native token transfers and staking
│   ├── X3Evm     — EVM-compatible execution
│   └── X3Svm     — SVM (Solana VM) compatible execution
├── Cross-VM Router: atomic 6-route matrix with supply invariant
├── Supply Ledger: canonical_supply ≥ represented_total enforced per-op
├── Settlement Engine: atomic state machine with refund path
└── External Bridges: DISABLED at genesis, governance-gated post-audit
```

---

## Running Tests

```bash
# Cross-VM router proofs (6-route matrix, invariants, replay protection)
cargo test -p pallet-x3-cross-vm-router -- --nocapture

# Supply ledger invariant tests
cargo test -p pallet-x3-supply-ledger -- --nocapture

# Settlement engine state machine
cargo test -p pallet-x3-settlement-engine -- --nocapture

# Full workspace
cargo test --workspace
```

---

## Development vs Production

| Script | Purpose | Use For |
|--------|---------|---------|
| `scripts/start-x3-chain.sh` | Starts **real** node binary | All testing |
| `scripts/testnet-full-launch.sh` | Starts 3-validator real testnet | Integration testing |
| `scripts/start-mock-rpc-dev.sh` | Starts fake RPC mock | **Frontend UI dev only** |

> **Warning:** `scripts/mock-rpc-server.js` produces **fake blocks** with no real consensus.
> It is clearly marked DEV ONLY and must never be used for testnet or mainnet.

---

## CI

All merges to `main` require all jobs in `.github/workflows/ci.yml` to pass:

- Format, Clippy (deny warnings)
- `cargo check -p x3-chain-runtime`
- `cargo check -p x3-chain-node`
- `cargo test -p pallet-x3-cross-vm-router` (13 production-proof tests)
- `cargo test -p pallet-x3-supply-ledger`
- `cargo test -p pallet-x3-settlement-engine`
- `cargo test -p pallet-x3-atomic-kernel`

---

## Feature Flags

Features disabled at RC-1 (require governance + audit to enable):

| Feature | Guard |
|---------|-------|
| `external-gateway` | `compile_error!` if combined with `mainnet-rc1` |
| `parallel-executor` | `compile_error!` if combined with `mainnet-rc1` |
| External bridges | `ExternalBridgesEnabled = false` in genesis |

---

## Documentation

- [CURRENT_MAINNET_STATUS.md](./CURRENT_MAINNET_STATUS.md) — honest status report
- [MAINNET_LAUNCH_CHECKLIST.md](./MAINNET_LAUNCH_CHECKLIST.md) — binary pass/fail gate tracker
- [docs/deployment/X3_NODE_DEPLOYMENT.md](./docs/deployment/X3_NODE_DEPLOYMENT.md) — full node deployment guide
- [MAINNET_RC1_SCOPE.md](./MAINNET_RC1_SCOPE.md) — what is and is not in RC-1
