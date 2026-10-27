# X3 Atomic Star

A Substrate-based blockchain with native cross-VM execution across X3Native, X3Evm, and X3Svm domains.

> **Release status:** v0.4 internal testnet candidate. **Not approved for a public testnet or mainnet.**
>
> [Launch scope](./LAUNCH_SCOPE.md) defines what is in scope.
> [Mainnet status](./CURRENT_MAINNET_STATUS.md) and
> [known failures](./docs/current/FAILURES_AND_TODOS.md) record historical
> assessments; use current GitHub Actions runs at the exact commit for build
> or test claims.
>
> **X3Lang:** The Python pipeline, Rust compiler and runtime paths are under
> integration. Do not call any compiler or native backend the production
> execution path until the canonical compiler, determinism and runtime checks
> pass at the release candidate's exact commit.

---

## Local development

These commands start a development node, not a public validator deployment.

### 1. Install prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Protobuf + clang (required by Substrate)
sudo apt-get update
sudo apt-get install -y protobuf-compiler clang llvm

# WASM targets (required for runtime build; wasm32v1-none is the clean
# no-std target used by the pinned polkadot-sdk stable2512 toolchain)
rustup target add wasm32-unknown-unknown wasm32v1-none
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

### 4. Start a single validator (easy)

```bash
./scripts/start-validator-easy.sh --chain deployment/chain-specs/x3-testnet-raw.json
```

### 5. Test multiple validators

Use the [staging testnet guide](./docs/STAGING_TESTNET_SETUP.md) and
review each launcher's process cleanup and data-directory settings first.
In particular, `scripts/testnet-full-launch.sh` can stop other
`x3-chain-node` processes during cleanup; do not use it on a shared
validator host without isolating the processes.

### 6. Connect a wallet

Open [Polkadot.js Apps](https://polkadot.js.org/apps) → Development → Local Node → `ws://127.0.0.1:9944`

---

## Architecture

The repository contains these components. Their presence does not establish
that every VM executes statefully, atomically or with production finality.

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

## CI and verification

The default branch is `master`. Pull requests to `master` run
[Local CI](./.github/workflows/local-ci.yml) on self-hosted X3 runners.
Other workflows cover compiler, trading, storage and release-specific checks.
A workflow file or queued run is **not** evidence that a check passed.
Verify the current commit SHA, completed jobs and applicable branch protections
before claiming readiness.

Useful local commands (not a substitute for the full exact-head CI suite):

- `cargo fmt --all -- --check` — format gate
- `cargo check -p x3-chain-runtime` — runtime compile gate
- `cargo check -p x3-chain-node` — node compile gate
- `cargo test -p pallet-x3-cross-vm-router` — 6-route matrix, invariants, replay protection (8 named production-proof tests verified individually)
- `cargo test -p pallet-x3-supply-ledger` — canonical supply invariant
- `cargo test -p pallet-x3-settlement-engine` — atomic settlement state machine
- `cargo test -p pallet-x3-atomic-kernel` — bundle lifecycle + PoAE proof
- `cargo clippy --workspace --all-targets -- -D warnings` — workspace-wide lint gate
- `cargo build --release -p x3-chain-node` — release binary gate

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

- [LAUNCH_SCOPE.md](./LAUNCH_SCOPE.md) — **single authoritative scope statement** (start here)
- [CURRENT_MAINNET_STATUS.md](./CURRENT_MAINNET_STATUS.md) — detailed subsystem scoreboard
- [RELEASE_GATES.md](./RELEASE_GATES.md) — mainnet release gates summary (binary pass/fail gate tracker, retired MAINNET_LAUNCH_CHECKLIST.md archived under Desktop/xxxstar/olddocs)
- [docs/X3_DEPLOYMENT_POLICY.md](./docs/X3_DEPLOYMENT_POLICY.md) — full node deployment policy and guide
- [docs/STAGING_TESTNET_SETUP.md](./docs/STAGING_TESTNET_SETUP.md) — 5-7 validator staging testnet guide
- [scripts/create-rc1-release.sh](./scripts/create-rc1-release.sh) — release helper; requires a separate operator release decision
