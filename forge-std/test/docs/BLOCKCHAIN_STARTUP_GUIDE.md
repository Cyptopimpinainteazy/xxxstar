# X3 Chain Blockchain Startup Guide

> Quick reference for running the X3 Chain dual-VM (EVM + SVM + X3VM) blockchain node.

## Prerequisites

```bash
# Rust toolchain (1.75+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup target add wasm32-unknown-unknown

# System dependencies (Ubuntu/Debian)
sudo apt install -y build-essential clang libclang-dev protobuf-compiler
```

## Quick Start

### 1. Build the Node

```bash
# Release build (recommended)
cargo build --release

# Binary location
./target/release/x3-chain-node --version
```

### 2. Run Development Node

```bash
# Fastest way - temporary storage, auto-purges on restart
./target/release/x3-chain-node --dev --tmp

# Or use the convenience script
./run-dev-node.sh
```

### 3. Access Points

| Service | URL |
|---------|-----|
| HTTP RPC | http://127.0.0.1:9944 |
| WebSocket | ws://127.0.0.1:9944 |
| Prometheus | http://127.0.0.1:9615/metrics |

---

## Node Configurations

### Development Mode (Single Validator)

```bash
./target/release/x3-chain-node \
  --dev \
  --tmp \
  --rpc-cors all \
  --rpc-methods unsafe
```

**Pre-funded accounts:**
- Alice: `5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY`
- Bob: `5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty`

### Local Testnet (Multi-Node)

**Node 1 (Alice):**
```bash
./target/release/x3-chain-node \
  --chain local \
  --alice \
  --port 30333 \
  --rpc-port 9944 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001
```

**Node 2 (Bob):**
```bash
./target/release/x3-chain-node \
  --chain local \
  --bob \
  --port 30334 \
  --rpc-port 9945 \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwM6wVCJxk8LmCtw3gpU6p
```

### Production Mode

```bash
./target/release/x3-chain-node \
  --chain staging \
  --name "my-validator" \
  --validator \
  --base-path /var/lib/x3-chain \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --prometheus-external \
  --telemetry-url "wss://telemetry.x3-chain.io/submit 0"
```

---

## Key Management

### Generate Session Keys

```bash
# Via RPC (node must be running)
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "author_rotateKeys", "params":[]}' \
  http://localhost:9944
```

### Insert Keys Manually

```bash
./target/release/x3-chain-node key insert \
  --base-path /var/lib/x3-chain \
  --chain staging \
  --scheme sr25519 \
  --suri "your secret seed phrase" \
  --key-type aura

./target/release/x3-chain-node key insert \
  --base-path /var/lib/x3-chain \
  --chain staging \
  --scheme ed25519 \
  --suri "your secret seed phrase" \
  --key-type gran
```

---

## RPC Quick Reference

### Check Node Status

```bash
# Health check
curl -s http://localhost:9944/health

# System info
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  http://localhost:9944
```

### X3 Kernel RPCs

```bash
# Check authorization status
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "atlasKernel_isAuthorized", "params":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"]}' \
  http://localhost:9944

# Get canonical balance
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "atlasKernel_getCanonicalBalance", "params":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY", "0x0000000000000000000000000000000000000000000000000000000000000000"]}' \
  http://localhost:9944
```

### EVM RPCs (Frontier)

```bash
# Get block number
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "eth_blockNumber"}' \
  http://localhost:9944

# Get balance (EVM)
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "eth_getBalance", "params":["0x0000000000000000000000000000000000000000", "latest"]}' \
  http://localhost:9944
```

---

## Chain Purge & Reset

```bash
# Purge development chain
./target/release/x3-chain-node purge-chain --dev -y

# Purge specific chain
./target/release/x3-chain-node purge-chain --chain local -y

# Purge with custom base path
./target/release/x3-chain-node purge-chain \
  --base-path /var/lib/x3-chain \
  --chain staging -y
```

---

## VM Execution

### Submit Comit Transaction (Atomic Cross-VM)

Using polkadot.js or SDK:

```typescript
// TypeScript SDK example
import { AtlasSphereClient } from '@x3-chain/sdk';

const client = new AtlasSphereClient('ws://localhost:9944');

// Create atomic trade across EVM and SVM
const comit = await client.comit.create({
  evmPayload: '0x...', // EVM calldata
  svmPayload: '...', // BPF instruction data
  prepareRoot: '0x...',
});

await comit.submit();
```

### Direct EVM Call

```bash
# Deploy contract via eth_sendRawTransaction
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "eth_sendRawTransaction", "params":["0x..."]}' \
  http://localhost:9944
```

---

## Monitoring

### Prometheus Metrics

```bash
# Scrape metrics
curl http://localhost:9615/metrics

# Key metrics to watch:
# - substrate_block_height
# - substrate_ready_transactions_number
# - substrate_peers_count
# - x3_kernel_comits_total
```

### Logs

```bash
# Verbose logging
RUST_LOG=info,x3=debug ./target/release/x3-chain-node --dev --tmp

# Specific modules
RUST_LOG=pallet_x3_kernel=trace ./target/release/x3-chain-node --dev --tmp
```

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| `bulk memory support is not enabled` | Fixed in latest build (patched sc-executor) |
| Node won't start | Run `purge-chain` then restart |
| No peers connecting | Check firewall allows port 30333 |
| RPC connection refused | Ensure `--rpc-cors all` for remote access |
| WASM build fails | `rustup target add wasm32-unknown-unknown` |

### Debug Mode

```bash
# Run with backtrace
RUST_BACKTRACE=1 ./target/release/x3-chain-node --dev --tmp

# Run with all logs
RUST_LOG=debug ./target/release/x3-chain-node --dev --tmp 2>&1 | tee node.log
```

---

## Testnet Connection

### Public Testnet

```bash
# Connect to X3 Chain testnet
./target/release/x3-chain-node \
  --chain testnet \
  --name "my-node" \
  --bootnodes /dns/bootnode.testnet.x3-chain.io/tcp/30333/p2p/...
```

**Testnet RPC:** `http://rpc.testnet.x3-chain.io:9944`  
**Faucet:** `https://faucet.testnet.x3-chain.io`

---

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────┐
│                    X3 Chain Node                        │
├─────────────────────────────────────────────────────────────┤
│  Consensus: Aura (6s blocks) + GRANDPA (finality)          │
├─────────────────────────────────────────────────────────────┤
│  X3 Kernel Pallet                                        │
│  ├── Comit Transactions (atomic cross-VM)                  │
│  ├── Canonical Ledger (asset tracking)                     │
│  └── Authorization (account permissions)                   │
├─────────────────────────────────────────────────────────────┤
│  VM Adapters                                                │
│  ├── EVM (Frontier pallet-evm, Shanghai)                   │
│  ├── SVM (solana-rbpf BPF executor)                        │
│  └── X3VM (native X3 VM)                                │
├─────────────────────────────────────────────────────────────┤
│  External Chains (103+ EVM chains)                         │
│  └── Atomic swaps via bridge contracts                     │
└─────────────────────────────────────────────────────────────┘
```

---

## Next Steps

1. **Explore the Explorer**: `cd apps/explorer && npm install && npm run dev`
2. **Use the Wallet**: `cd apps/wallet && npm install && npm run dev`
3. **Read the RPC Guide**: [docs/RPC_INTEGRATION_GUIDE.md](RPC_INTEGRATION_GUIDE.md)
4. **Deploy Contracts**: [docs/tutorials/](tutorials/)

---

*Generated: December 2025 | X3 Chain v0.1.0*
