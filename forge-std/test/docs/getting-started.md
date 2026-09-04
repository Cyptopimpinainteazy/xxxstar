# Getting Started with X3 Chain

This guide will help you build and run X3 Chain from source. All commands are based on the actual repository implementation.

## Prerequisites

### System Requirements
- **Operating System**: Linux or macOS (Windows via WSL2)
- **Rust**: 1.89.0 or later (as specified in `rust-toolchain.toml`)
- **Build Tools**: `cmake`, `pkg-config`, `libssl-dev`, `clang`
- **Memory**: Minimum 8GB RAM (16GB recommended)
- **Disk Space**: 20GB free space for build artifacts

### Install Rust Toolchain

```bash
# Install rustup if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# The repository includes rust-toolchain.toml which will auto-select Rust 1.89.0
# Verify installation
rustup show
cargo --version

# Add WebAssembly target (required for runtime)
rustup target add wasm32-unknown-unknown
```

### Install Build Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential cmake pkg-config libssl-dev git clang libclang-dev
```

**macOS:**
```bash
brew install cmake pkg-config openssl
```

---

## Step 1: Clone and Build

### Clone the Repository

```bash
git clone https://github.com/your-org/x3-chain.git
cd x3-chain
```

### Build the Node Binary

This compiles the blockchain node (`x3-chain-node`):

```bash
cargo build --release --package x3-chain-node
```

Build artifacts location: `target/release/x3-chain-node`

Expected build time: 30-60 minutes on first build (uses all CPU cores)

### Build the X3 CLI Tool (Optional)

The X3 CLI provides development tools for compiling X3 contracts:

```bash
cargo build --release --package x3-cli
```

Build artifact location: `target/release/x3`

To enable full SDK features (deploy, simulate, tx commands):

```bash
cargo build --release --package x3-cli --features sdk
```

---

## Step 2: Run Your First Node

### Option A: Development Mode (Quickest)

Run a single-node development chain with temporary storage:

```bash
./target/release/x3-chain-node --dev --tmp
```

This starts:
- ✅ Single validator node (Alice)
- ✅ Aura + GRANDPA consensus
- ✅ HTTP JSON-RPC on `127.0.0.1:9933`
- ✅ WebSocket JSON-RPC on `127.0.0.1:9944`
- ⚠️ All data deleted when node stops (use `--tmp` flag)

**To persist data**, omit `--tmp`:
```bash
./target/release/x3-chain-node --dev
```

Data location: `~/.local/share/x3-chain-node/chains/dev/`

### Option B: Local Testnet (Multi-Validator)

For production-like environment with multiple validators:

```bash
cd deployment
./deploy-local-testnet.sh
```

This script:
- Generates validator keys (3 validators + 1 bootnode)
- Creates chain specification
- Starts 4 systemd services
- Configures persistent storage

See [deployment/README.md](./deployment/README.md) for details.

---

## Step 3: Verify Node is Running

### Check Node Logs

If running in foreground mode, you should see:
```
✅ Role: AUTHORITY
✅ Best: #1234 (0x1a2b3c...)
✅ Finalized #1230 (0x4d5e6f...)
```

### Query via RPC

**HTTP JSON-RPC** (system health):
```bash
curl http://127.0.0.1:9933 -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}'
```

Expected response:
```json
{"jsonrpc":"2.0","result":{"isSyncing":false,"peers":0,"shouldHavePeers":false},"id":1}
```

**WebSocket** (subscribe to new blocks):
```bash
# Install websocat for WebSocket testing
cargo install websocat

# Subscribe to new block headers
websocat ws://127.0.0.1:9944 \
  -t -text '{"id":1,"jsonrpc":"2.0","method":"chain_subscribeNewHeads","params":[]}'
```

---

## Step 4: Using the X3 CLI

The X3 CLI provides development tools for compiling and testing X3 contracts.

### Initialize a New Project

```bash
./target/release/x3 init my-project
cd my-project
```

This creates:
- `x3.toml` - Project configuration
- `src/` - Source code directory
- `tests/` - Test directory

### Compile a Contract

**Compile all contracts in project:**
```bash
./target/release/x3 build
```

**Compile a single X3 source file** (standalone):
```bash
./target/release/x3 compile src/my_contract.x3
```

Output: `my_contract.wasm` and `my_contract.abi.json`

### Run Tests

```bash
./target/release/x3 test
```

### Interactive REPL

Start an interactive X3 language REPL:

```bash
./target/release/x3 repl
```

Try:
```x3
let x: i32 = 42;
let y = x * 2;
print(y);
```

---

## Step 5: Interact with the Blockchain (SDK Features)

To use blockchain interaction commands, rebuild with SDK features:

```bash
cargo build --release --package x3-cli --features sdk
```

### Account Management

**Create a new account:**
```bash
./target/release/x3 account create --name alice
```

**List accounts:**
```bash
./target/release/x3 account list
```

### Query Blockchain State

**Get account balance:**
```bash
./target/release/x3 query balance 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
```

**Get canonical ledger state:**
```bash
./target/release/x3 query ledger --asset-id 0 --account <ACCOUNT_ID>
```

### Deploy a Contract

```bash
./target/release/x3 deploy \
  --wasm target/wasm32-unknown-unknown/release/my_contract.wasm \
  --account alice \
  --url ws://127.0.0.1:9944
```

### Send Transactions

```bash
./target/release/x3 tx transfer \
  --to 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY \
  --amount 1000000000000 \
  --account alice
```

### Simulate Transactions

Test a transaction without broadcasting:

```bash
./target/release/x3 simulate \
  --comit <COMIT_HEX> \
  --url ws://127.0.0.1:9944
```

---

## Step 6: Cross-Chain Atomic Swaps

X3 CLI includes native cross-chain swap support for 103 EVM chains:

### List Supported Chains

```bash
./target/release/x3 chains list
```

**Search for specific chains:**
```bash
./target/release/x3 chains search ethereum
./target/release/x3 chains search polygon
```

### Execute Atomic Swap

```bash
./target/release/x3 swap \
  --from-chain ethereum \
  --to-chain polygon \
  --amount 1000000000000000000 \
  --token 0x... \
  --account alice
```

---

## Common Commands Reference

### Node Operations

| Command | Purpose |
|---------|---------|
| `./target/release/x3-chain-node --dev --tmp` | Start development node (temporary storage) |
| `./target/release/x3-chain-node --dev` | Start development node (persistent storage) |
| `./target/release/x3-chain-node --help` | Show all node options |

### X3 CLI (Contract Development)

| Command | Purpose |
|---------|---------|
| `x3 init <name>` | Create new project |
| `x3 build` | Build all contracts |
| `x3 compile <file>` | Compile single file |
| `x3 test` | Run tests |
| `x3 repl` | Start interactive REPL |
| `x3 docgen` | Generate documentation |

### X3 CLI (SDK Features - Requires `--features sdk`)

| Command | Purpose |
|---------|---------|
| `x3 account create` | Create account |
| `x3 account list` | List accounts |
| `x3 query balance <addr>` | Query balance |
| `x3 deploy --wasm <path>` | Deploy contract |
| `x3 tx transfer --to <addr>` | Send transfer |
| `x3 simulate --comit <hex>` | Simulate transaction |
| `x3 swap --from-chain <chain>` | Cross-chain swap |
| `x3 chains list` | List supported chains |

---

## Next Steps

- **Architecture**: See [TRI_VM_ARCHITECTURE.md](./TRI_VM_ARCHITECTURE.md) for EVM+SVM+X3VM design
- **Language Spec**: See [X3_LANGUAGE_SPECIFICATION.md](./X3_LANGUAGE_SPECIFICATION.md) for X3 language reference
- **Deployment**: See [deployment/README.md](./deployment/README.md) for production deployment
- **Testing**: See [testing/README.md](./testing/README.md) for test frameworks
- **Security**: See [../00-START-HERE-MAINNET-READINESS.md](../00-START-HERE-MAINNET-READINESS.md) for audit status

---

## Troubleshooting

### Build Failures

**Error: "linker `cc` not found"**
```bash
# Ubuntu/Debian
sudo apt install build-essential

# macOS
xcode-select --install
```

**Error: "failed to load source for dependency"**
```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Node Won't Start

**Error: "Port 9944 already in use"**
```bash
# Find and kill existing process
lsof -i :9944
kill -9 <PID>

# Or use different ports
./target/release/x3-chain-node --dev --ws-port 9945 --rpc-port 9934
```

**Error: "Database version mismatch"**
```bash
# Purge chain data
./target/release/x3-chain-node purge-chain --dev
```

### RPC Connection Issues

**Error: "Connection refused"**
- Ensure node is running (`ps aux | grep x3-chain-node`)
- Verify RPC is enabled (development mode enables by default)
- Check firewall rules

---

## Support & Community

- **Documentation**: [docs/master/INDEX.md](./master/INDEX.md)
- **Issues**: [GitHub Issues](https://github.com/your-org/x3-chain/issues)
- **Security**: See [../S0_BLOCKERS_REMEDIATION_PLAN.md](../S0_BLOCKERS_REMEDIATION_PLAN.md) for current security status
