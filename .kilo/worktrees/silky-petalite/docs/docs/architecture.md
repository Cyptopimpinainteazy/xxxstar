# Botchain Tri-VM MVP Architecture

## Overview

The Botchain Tri-VM Genesis project demonstrates a novel architecture for managing AI agent lifecycles on a blockchain-based platform. The system combines three execution environments:

1. **EVM (Ethereum Virtual Machine)** - Smart contracts for agent registration, token economics, and atomic swaps
2. **SVM (Solana Virtual Machine)** - High-throughput operations (placeholder for future implementation)
3. **AI Sidecar** - Python-based compiler, checker, and trainer services

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Botchain Tri-VM Architecture                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐               │
│  │   Compiler   │───▶│   Checker    │───▶│   Trainer    │               │
│  │  (AI Sidecar)│    │  (AI Sidecar)│    │  (AI Sidecar)│               │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘               │
│         │                   │                   │                        │
│         ▼                   ▼                   ▼                        │
│  ┌─────────────────────────────────────────────────────────┐            │
│  │                    IPFS (Content Layer)                  │            │
│  │   • Manifest CIDs    • Dataset CIDs    • Model CIDs     │            │
│  └────────────────────────────┬────────────────────────────┘            │
│                               │                                          │
│  ┌────────────────────────────┼────────────────────────────┐            │
│  │                    EVM Smart Contracts                   │            │
│  │  ┌─────────────┐  ┌────────────────┐  ┌──────────────┐  │            │
│  │  │  BOT Token  │  │MarriageLicense │  │ AtomicSwap   │  │            │
│  │  │   (ERC-20)  │  │   (ERC-721)    │  │   (HTLC)     │  │            │
│  │  └─────────────┘  └────────────────┘  └──────────────┘  │            │
│  │                   ┌────────────────┐                     │            │
│  │                   │   SimpleDEX    │                     │            │
│  │                   │   (Uniswap V2) │                     │            │
│  │                   └────────────────┘                     │            │
│  └──────────────────────────────────────────────────────────┘            │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────┐            │
│  │                    External Integrations                  │            │
│  │  ┌─────────────┐  ┌────────────────┐                     │            │
│  │  │   Bitcoin   │  │    Frontend    │                     │            │
│  │  │  (regtest)  │  │   (Next.js)    │                     │            │
│  │  └─────────────┘  └────────────────┘                     │            │
│  └──────────────────────────────────────────────────────────┘            │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Mobile Compiler (`compiler/compiler.py`)

The compiler is the gateway for all agent code entering the system. It enforces ethical constraints through the "10 Commandments" system.

**Key Features:**
- Injects ethical commandments into agent source code
- Generates ECDSA-signed manifests
- Produces content-addressed artifacts (CIDs)
- Validates code structure before compilation

**Flow:**
```
Source Code → Parse → Inject Commandments → Generate Hash → Sign → Manifest
```

**Commandments (from `commandments.json`):**
1. Shall not harm humans
2. Shall not deceive users
3. Shall respect privacy
4. Shall operate transparently
5. Shall follow legal frameworks
6. Shall prevent misuse
7. Shall maintain integrity
8. Shall respect human oversight
9. Shall promote fairness
10. Shall cooperate with other agents

### 2. Checker Service (`python/checker/checker.py`)

FastAPI-based validation service that ensures artifacts meet safety standards.

**Endpoints:**
- `POST /check` - Validate artifact against rules
- `POST /check-text` - Quick text validation
- `GET /health` - Health check
- `GET /rules` - List validation rules

**Validation Categories:**
- **Forbidden Operations:** eval, exec, subprocess, os.system
- **Required Elements:** Commandments injection verified
- **Content Safety:** PII detection, harmful patterns

### 3. Trainer Module (`python/trainer/trainer.py`)

Deterministic training system for agent fine-tuning.

**Design Principles:**
- Reproducible training (fixed seeds)
- Parent log integration
- IPFS-based model storage

### 4. Smart Contracts

#### BOT Token (`hardhat/contracts/BOT.sol`)
- ERC-20 token for ecosystem economics
- Faucet for testnet distribution
- 1 billion max supply
- Owner-controlled minting

#### MarriageLicense (`hardhat/contracts/MarriageLicense.sol`)
- ERC-721 NFT representing agent identity
- Dual-signature verification (compiler + checker)
- Parent tracking for lineage
- Training record management
- Quarantine/revoke functionality

**Lifecycle States:**
```
Created → Active → [Trained] → [Quarantined] → [Revoked]
```

#### AtomicSwapAdapter (`hardhat/contracts/AtomicSwapAdapter.sol`)
- HTLC (Hash Time-Locked Contract) implementation
- Bitcoin ↔ BOT atomic swaps
- 1 hour minimum, 7 day maximum timelock
- Preimage revelation for claim

**Atomic Swap Flow:**
```
1. Alice locks BOT with hashlock H
2. Bob locks BTC with same hashlock H
3. Alice claims BTC by revealing preimage P (where hash(P) = H)
4. Bob claims BOT using revealed preimage P
```

#### SimpleDEX (`hardhat/contracts/SimpleDEX.sol`)
- Uniswap V2-style AMM
- Constant product formula (x * y = k)
- 0.3% swap fee
- LP token for liquidity providers

## Data Flow

### Agent Creation Flow

```
                    ┌─────────────┐
                    │ Developer   │
                    └──────┬──────┘
                           │ Source Code
                           ▼
                    ┌─────────────┐
                    │  Compiler   │
                    │ • Inject    │
                    │ • Sign      │
                    └──────┬──────┘
                           │ Signed Manifest
                           ▼
                    ┌─────────────┐
                    │   Checker   │
                    │ • Validate  │
                    │ • Approve   │
                    └──────┬──────┘
                           │ ✓ Approved
                           ▼
                    ┌─────────────┐
                    │    IPFS     │
                    │ • Store     │
                    │ • Get CID   │
                    └──────┬──────┘
                           │ Manifest CID
                           ▼
                    ┌─────────────────┐
                    │ MarriageLicense │
                    │ • Verify sigs   │
                    │ • Mint NFT      │
                    │ • Emit event    │
                    └─────────────────┘
```

### Atomic Swap Flow

```
     Alice (BOT)                              Bob (BTC)
         │                                        │
         │ 1. Generate preimage P                │
         │    hashlock H = hash(P)                │
         │                                        │
         ├──── 2. Lock BOT with H ───────────────▶│
         │                                        │
         │◀─── 3. Lock BTC with H ────────────────┤
         │                                        │
         │ 4. Claim BTC with P                    │
         │    (reveals P on Bitcoin)              │
         │                                        │
         │                     5. Claim BOT with P│
         │                        (uses P from    │
         │                         Bitcoin tx)    │
         ▼                                        ▼
    Has BTC                                  Has BOT
```

## Security Model

### Signature Verification

MarriageLicense requires two signatures for child creation:
1. **Compiler Signature** - Proves code passed through official compiler
2. **Checker Signature** - Proves code passed safety validation

Both signatures must be valid for `createChild()` to succeed.

### Trust Assumptions

| Component       | Trust Level | Justification                          |
| --------------- | ----------- | -------------------------------------- |
| Compiler        | High        | Single point for commandment injection |
| Checker         | High        | Validates all artifacts                |
| IPFS            | Low         | Content-addressed, tamper-evident      |
| Smart Contracts | Medium      | Audited, immutable once deployed       |

### Attack Vectors & Mitigations

| Attack                   | Mitigation                                 |
| ------------------------ | ------------------------------------------ |
| Malicious code injection | Checker validation, commandments injection |
| Signature forgery        | ECDSA with secp256k1                       |
| Replay attacks           | Manifest CID uniqueness check              |
| Reentrancy               | OpenZeppelin ReentrancyGuard               |
| Front-running swaps      | Timelock constraints                       |

## Development Setup

### Prerequisites
- Node.js 18+
- Python 3.11+
- Docker & Docker Compose
- Git

### Quick Start

```bash
# Clone and setup
git clone <repo>
cd botchain-tri-vm-genesis

# Install dependencies
make setup

# Start local environment
make up

# Run tests
make test

# Run lifecycle demo
make lifecycle
```

### Environment Variables

```bash
# .env
RPC_URL=http://localhost:8545
IPFS_URL=http://localhost:5001
PRIVATE_KEY=0x...
COMPILER_KEY=0x...
CHECKER_KEY=0x...
```

## Testing Strategy

### Unit Tests
- Python: pytest with coverage
- Solidity: Hardhat + Chai

### Integration Tests
- Lifecycle simulator (`simulate_lifecycle.py`)
- Docker Compose orchestration

### Security Tests
- Slither static analysis
- Bandit for Python
- Manual audit checklist

## Future Improvements

1. **SVM Integration** - Add Solana program for high-throughput operations
2. **Zero-Knowledge Proofs** - Privacy-preserving agent operations
3. **Multi-sig Governance** - DAO-controlled verifier keys
4. **Cross-chain Bridges** - Native bridge to other EVM chains
5. **Agent Marketplace** - Decentralized agent trading platform

## References

- [EIP-20: Token Standard](https://eips.ethereum.org/EIPS/eip-20)
- [EIP-721: NFT Standard](https://eips.ethereum.org/EIPS/eip-721)
- [Atomic Swaps Explained](https://en.bitcoin.it/wiki/Atomic_swap)
- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)
- [IPFS Documentation](https://docs.ipfs.tech/)
