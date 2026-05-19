# BotChain Tri-VM Genesis

A complete, runnable end-to-end MVP demonstrating:
- **Tri-VM Architecture**: EVM-like VM, SVM-like smart contract VM, and a sidecar VM for AI agents
- **Mobile/Ethical Compiler**: Injects immutable "10 Commandments" and signs manifests
- **MarriageLicense Smart Contract**: BOT ERC-20 token + child minting
- **HTLC Atomic Swaps**: Bitcoin ↔ chain token atomic swaps
- **Agent Lifecycle**: Adam & Eve → child minting → parent log inheritance → fine-tune child model → IPFS manifests
- **Checker Service**: 1:1 validator for artifacts/training logs
- **Minimal DEX**: Uniswap v2-style liquidity mock

## Quick Start

### Prerequisites
- Docker & Docker Compose
- Make
- Node.js 18+ (for local development)
- Python 3.11+ (for local development)

### One-Click Boot

```bash
# Clone and enter directory
cd botchain-tri-vm-genesis

# Build all containers
make build

# Start all services (Hardhat, IPFS, Bitcoin regtest, Python services)
make up

# Run the full lifecycle simulation
make simulate

# Run all tests
make test
```

### Manual Setup (Development)

```bash
# Terminal 1: Start Hardhat local node
cd hardhat && npm install && npx hardhat node

# Terminal 2: Deploy contracts
cd hardhat && npx hardhat run scripts/deploy.js --network localhost

# Terminal 3: Start IPFS daemon
ipfs daemon

# Terminal 4: Start checker service
cd python && pip install -r requirements.txt && uvicorn checker.checker:app --port 8000

# Terminal 5: Run simulation
cd python && python cli/simulate_lifecycle.py
```

## Architecture

### Tri-VM System

```
┌─────────────────────────────────────────────────────────────────┐
│                    BotChain Tri-VM Architecture                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
│  │   EVM VM    │  │   SVM VM    │  │   AI Agent Sidecar VM   │ │
│  │ (Contracts) │  │ (Programs)  │  │    (Model Execution)    │ │
│  │             │  │             │  │                         │ │
│  │ - BOT Token │  │ - State     │  │ - Parent log inherit    │ │
│  │ - Marriage  │  │   accounts  │  │ - Fine-tune models      │ │
│  │   License   │  │ - Cross-VM  │  │ - IPFS manifests        │ │
│  │ - HTLC Swap │  │   calls     │  │ - Checker validation    │ │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘ │
│         │                │                     │               │
│         └────────────────┼─────────────────────┘               │
│                          │                                     │
│              ┌───────────▼───────────┐                         │
│              │   Canonical Ledger    │                         │
│              │   (Atomic Commits)    │                         │
│              └───────────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

### HTLC Atomic Swap Flow

```
Bitcoin Network                    BotChain
     │                                 │
     │  1. Alice creates HTLC          │
     │     H(secret), timeout=48 blocks│
     ▼                                 │
┌─────────┐                           │
│BTC HTLC │                           │
│ locked  │                           │
└─────────┘                           │
     │                                 │
     │  2. Bob sees BTC locked         │
     │                                 ▼
     │                          ┌─────────┐
     │                          │BOT HTLC │
     │                          │ locked  │
     │                          └─────────┘
     │                                 │
     │  3. Alice reveals secret        │
     │     on BotChain, claims BOT     │
     │                                 ▼
     │                          ┌─────────┐
     │                          │ Alice   │
     │                          │gets BOT │
     │                          └─────────┘
     │                                 │
     │  4. Bob uses secret to          │
     │     claim BTC                   │
     ▼                                 │
┌─────────┐                           │
│  Bob    │                           │
│gets BTC │                           │
└─────────┘                           │
```

### Agent Lifecycle

1. **Compilation**: Source artifacts processed through ethical compiler
2. **Checking**: Artifacts validated by checker service
3. **Minting**: Parents (Adam/Eve) mint child via MarriageLicense contract
4. **Training**: Child model fine-tuned on parent logs
5. **Registration**: Training results recorded on-chain

## Project Structure

```
botchain-tri-vm-genesis/
├── docs/root/README.md
├── docker-compose.yml
├── Makefile
├── .env.example
├── hardhat/
│   ├── contracts/
│   │   ├── BOT.sol
│   │   ├── MarriageLicense.sol
│   │   └── AtomicSwapAdapter.sol
│   ├── scripts/
│   │   └── deploy.js
│   ├── test/
│   │   ├── BOT.test.js
│   │   ├── MarriageLicense.test.js
│   │   └── AtomicSwap.test.js
│   ├── hardhat.config.js
│   └── package.json
├── python/
│   ├── compiler/
│   │   ├── compiler.py
│   │   ├── keygen.sh
│   │   └── commandments.json
│   ├── checker/
│   │   └── checker.py
│   ├── trainer/
│   │   └── trainer.py
│   ├── cli/
│   │   └── simulate_lifecycle.py
│   ├── utils/
│   │   ├── ipfs_client.py
│   │   └── web3_client.py
│   ├── tests/
│   │   ├── test_compiler.py
│   │   ├── test_checker.py
│   │   └── test_integration.py
│   └── requirements.txt
├── infra/
│   └── docker/
│       ├── Dockerfile.hardhat
│       ├── Dockerfile.python
│       └── Dockerfile.bitcoin
├── scripts/
│   └── setup.sh
├── docs/
│   └── architecture.md
└── ci/
    └── .github/
        └── workflows/
            └── ci.yml
```

## Smart Contracts

### BOT Token (ERC-20)
- Standard OpenZeppelin ERC-20
- Owner-controlled minting
- Faucet function for testing

### MarriageLicense (ERC-721)
- Requires compiler-signed manifest
- Requires checker signature
- BOT fee for child creation
- Stores parent IDs, artifact CID, training data CID, model CID
- Child training registration

### AtomicSwapAdapter (HTLC)
- Lock tokens with hashlock + timelock
- Claim with preimage reveal
- Refund after timeout expiry
- Compatible with Bitcoin HTLC pattern

## Security Features

- EIP-191/EIP-1271 signature verification
- Reentrancy guards on all state-changing functions
- Checker sandboxing (AST parsing only, no execution)
- Quarantine flow: `revokeChild()` for harmful models
- Multi-sig controlled emergency functions

## Environment Variables

Copy `.env.example` to `.env` and configure:

```env
# Network
HARDHAT_RPC_URL=http://localhost:8545
IPFS_API_URL=http://localhost:5001

# Bitcoin Regtest
BITCOIN_RPC_URL=http://localhost:18443
BITCOIN_RPC_USER=bitcoin
BITCOIN_RPC_PASS=bitcoin

# Keys (auto-generated if not present)
COMPILER_PRIVATE_KEY=
CHECKER_PRIVATE_KEY=

# Contract Addresses (set after deployment)
BOT_TOKEN_ADDRESS=
MARRIAGE_LICENSE_ADDRESS=
ATOMIC_SWAP_ADDRESS=
```

## Testing

```bash
# Run Hardhat unit tests
make test-contracts

# Run Python unit tests
make test-python

# Run integration tests
make test-integration

# Run all tests
make test
```

## License

MIT
