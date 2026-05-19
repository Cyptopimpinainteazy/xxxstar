# X3 Atomic Star — Node Deployment Guide

> This guide covers deploying a real X3 node (not the mock RPC server).
> The mock server (`scripts/mock-rpc-server.js`) is for frontend dev only —
> it produces fake blocks and has no chain state.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Build the Node Binary](#2-build-the-node-binary)
3. [Generate a Chain Spec](#3-generate-a-chain-spec)
4. [Run a Single Dev Node](#4-run-a-single-dev-node)
5. [Run a 3-Validator Local Testnet](#5-run-a-3-validator-local-testnet)
6. [Connect a Wallet](#6-connect-a-wallet)
7. [Send a Cross-VM Transfer](#7-send-a-cross-vm-transfer)
8. [Observe the Supply Invariant](#8-observe-the-supply-invariant)
9. [Failed Settlement Recovery](#9-failed-settlement-recovery)
10. [Validator Operations](#10-validator-operations)
11. [Monitoring](#11-monitoring)
12. [Troubleshooting](#12-troubleshooting)

---

## 1. Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 4 cores | 8 cores |
| RAM | 8 GB | 16 GB |
| Disk | 50 GB SSD | 200 GB NVMe SSD |
| OS | Ubuntu 22.04 / Debian 12 | Ubuntu 22.04 LTS |
| Network | 10 Mbps | 100 Mbps, static IP |

### Software Dependencies

```bash
# Rust (toolchain is pinned via rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Protobuf compiler (required by substrate)
sudo apt-get update
sudo apt-get install -y protobuf-compiler clang llvm

# wasm32 target (required for runtime compilation)
rustup target add wasm32-unknown-unknown

# Verify toolchain (should match rust-toolchain.toml)
rustc --version
```

---

## 2. Build the Node Binary

```bash
# Build the real node binary (takes 10–30 minutes first time)
cargo build --release -p x3-chain-node

# Verify the binary exists and works
./target/release/x3-chain-node --version
# Expected output: x3-chain-node x.y.z-<commit>
```

> **Important:** Never use `scripts/mock-rpc-server.js` for anything other than
> frontend UI development. It produces fake blocks and has no real consensus.
> The `start-x3-chain.sh` script will exit with an error if the binary is missing
> rather than silently falling back to the mock.

---

## 3. Generate a Chain Spec

### Dev spec (ephemeral, single node, Alice/Bob keys)
```bash
./target/release/x3-chain-node build-spec \
  --chain dev \
  --disable-default-bootnode \
  > chain-specs/x3-dev.json
```

### Local testnet spec (3 validators)
```bash
# Human-readable spec
./target/release/x3-chain-node build-spec \
  --chain local \
  --disable-default-bootnode \
  > chain-specs/x3-local.json

# Convert to raw (required for actual node launch)
./target/release/x3-chain-node build-spec \
  --chain chain-specs/x3-local.json \
  --raw \
  --disable-default-bootnode \
  > chain-specs/x3-local-raw.json
```

---

## 4. Run a Single Dev Node

```bash
# Option A: Use the start script (recommended — exits if binary missing)
./scripts/start-x3-chain.sh

# Option B: Direct invocation
./target/release/x3-chain-node \
  --chain dev \
  --tmp \
  --rpc-port 9933 \
  --ws-port 9944 \
  --prometheus-port 9616 \
  --prometheus-external \
  --rpc-cors all \
  --validator \
  --alice

# Verify the node is running
curl -s http://localhost:9933 \
  -H 'Content-Type: application/json' \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' | jq
```

---

## 5. Run a 3-Validator Local Testnet

```bash
# Start 3 validators
./scripts/testnet-full-launch.sh

# Verify all validators are peered (should show 2 peers each)
for PORT in 9933 9934 9935; do
  echo -n "Validator $PORT peers: "
  curl -s http://localhost:$PORT \
    -H 'Content-Type: application/json' \
    -d '{"id":1,"jsonrpc":"2.0","method":"system_peers","params":[]}' \
    | jq '.result | length'
done
```

---

## 6. Connect a Wallet

### Polkadot.js Apps
1. Open [https://polkadot.js.org/apps](https://polkadot.js.org/apps)
2. Click the network selector (top left)
3. Select "Development" → "Local Node" → `ws://127.0.0.1:9944`
4. Block number should be incrementing with GRANDPA finality active

---

## 7. Send a Cross-VM Transfer

Via **Developer → Extrinsics** in Polkadot.js Apps:
- Pallet: `x3CrossVmRouter`
- Call: `xvmTransfer`
- Parameters: `asset_id=1`, `destination_vm=Evm`, `amount=1000000000`, `ttl_blocks=50`

State lifecycle:
1. `XvmTransferInitiated` event → source debited, `PendingTransfers` populated
2. `complete_xvm_transfer` called → `XvmTransferCompleted` event, destination credited
3. `PendingTransfers` cleared, `pending_supply` returns to 0

---

## 8. Observe the Supply Invariant

The invariant `represented_total ≤ canonical_supply` is enforced at every operation.

```bash
# Run invariant proof tests
cargo test -p pallet-x3-cross-vm-router test_canonical_supply_never_breaks -- --nocapture

# Run the full cross-VM test suite (all 13+ tests)
cargo test -p pallet-x3-cross-vm-router -- --nocapture
```

---

## 9. Failed Settlement Recovery

When a transfer's TTL expires before completion:
- `cancel_expired_xvm_transfer` is called automatically by `on_idle`
- Source domain supply is restored
- `XvmTransferCancelled` event is emitted
- `pending_supply` returns to 0

Manual recovery:
- Via Polkadot.js Extrinsics: `x3CrossVmRouter.cancelExpiredXvmTransfer(transfer_id)`

---

## 10. Validator Operations

```bash
# Rotate session keys
curl -s http://localhost:9933 \
  -H 'Content-Type: application/json' \
  -d '{"id":1,"jsonrpc":"2.0","method":"author_rotateKeys","params":[]}' | jq .result

# Then set keys on-chain via:
# Developer → Extrinsics → session → setKeys
```

GRANDPA requires ≥ ⌊2/3n⌋ + 1 validators for finality:
- 3-validator testnet: all 3 must be online
- 7-validator testnet: 5 must be online

---

## 11. Monitoring

Prometheus metrics at `:9616/metrics`:

```
substrate_block_height{status="best"}
substrate_block_height{status="finalized"}
substrate_sub_libp2p_peers_count
substrate_finality_grandpa_round
x3_xvm_transfers_pending
x3_xvm_transfers_completed_total
x3_supply_invariant_violations_total  # Must always be 0
```

Alert: `x3_supply_invariant_violations_total > 0` → **CRITICAL, halt operations**

---

## 12. Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `start-x3-chain.sh: binary not found` | Not built | `cargo build --release -p x3-chain-node` |
| No blocks produced | Missing `--validator` flag | Add `--validator --alice` (dev) or set session keys |
| GRANDPA stalled | Not enough validators online | Bring up ≥ ⌊2/3n⌋ + 1 validators |
| Transfer stuck in pending | TTL expired, OCW not running | Call `cancelExpiredXvmTransfer` manually |
| Supply invariant violation | Bug in pallet | File critical issue, do not continue operating node |
