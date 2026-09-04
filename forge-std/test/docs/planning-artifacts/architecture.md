---
stepsCompleted: ["step-01-init"]
inputDocuments: ["_bmad-output/planning-artifacts/product-brief-x3-chain-master-2026-02-13.md", "_bmad-output/planning-artifacts/prd.md", "_bmad-output/planning-artifacts/ux-design-specification.md"]
workflowType: 'architecture'
project_name: 'x3-chain-master'
user_name: 'Lojak'
date: '2026-02-13'
---

# Architecture Decision Document - x3-chain-master

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

---

## Section 1: Executive Summary

### Project Overview
**X3 Chain** is a multi-chain decentralized computing and finance platform built on Substrate with:
- Dual-VM architecture (EVM + SVM compatibility)
- AI-driven intelligence systems (X3 Intelligence)
- Decentralized GPU computing marketplace
- Enterprise-grade autonomic infrastructure

### Architectural Goals
1. **Scalability**: Target 10,000+ TPS with sharded architecture
2. **Security**: Multi-layer validation, slashing mechanisms, TEE support
3. **Privacy**: Confidential computing via TEE enclaves
4. **Interoperability**: Cross-chain asset transfers, 100+ chain support

---

## Section 2: Technology Stack

### Runtime & Blockchain
| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Blockchain Framework** | Substrate (Rust) | Flexible runtime, proven scalability |
| **Smart Contracts** | WASM + Solidity | Multi-VM support |
| **EVM Compatibility** | Frontier/Pallet EVM | Ethereum ecosystem integration |
| **SVM Compatibility** | Pallets (Custom) | Solana-style execution model |
| **Consensus** | Proof of Stake + Grandpa | Fast finality, BFT security |

### Frontend & UI
| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Desktop App** | Tauri (Rust + React) | Native performance, cross-platform |
| **Web Frontend** | React + TypeScript | Component-based, mature ecosystem |
| **State Management** | Zustand/Jotai | Lightweight, performant |
| **Styling** | Tailwind CSS | Rapid development, consistent design |

### Backend & Infrastructure
| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Autonomic System** | Python (async) | Health monitoring, healing |
| **GPU Orchestration** | Docker + Kubernetes | Containerized compute jobs |
| **Database** | PostgreSQL + Redis | Relational + caching layer |
| **Message Queue** | Kafka/RabbitMQ | Event-driven communication |

---

## Section 3: System Architecture

### 3.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        FRONTEND LAYER                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │
│  │ Desktop  │  │   Web    │  │  Mobile  │  │   CLI Tool   │ │
│  │  (Tauri) │  │ (React)  │  │   (TBD)  │  │   (Rust)     │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘ │
└───────┼─────────────┼─────────────┼───────────────┼──────────┘
        │             │             │               │
        └─────────────┴──────┬──────┴───────────────┘
                             │
┌────────────────────────────┼───────────────────────────────────┐
│                    API GATEWAY LAYER                           │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │              GraphQL/REST API Gateway                     │ │
│  │         (Authentication, Rate Limiting, Caching)        │ │
│  └────────────────────────────┬─────────────────────────────┘ │
└───────────────────────────────┼─────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
┌───────▼────────┐  ┌────────▼────────┐  ┌────────▼────────┐
│  BLOCKCHAIN    │  │   APPLICATION    │  │  EXTERNAL       │
│  NODE LAYER    │  │   SERVICES      │  │  INTEGRATIONS   │
├────────────────┤  ├─────────────────┤  ├─────────────────┤
│ - Runtime      │  │ - DEX Engine    │  │ - Chain Adapters│
│ - Pallets      │  │ - Wallet Service │  │ - Price Feeds   │
│ - Consensus    │  │ - GPU Marketplace│  │ - Oracle Nodes  │
│ - RPC Endpoints│  │ - X3 Intelligence│  │ - Bridge Relays │
└────────────────┘  └──────────────────┘  └─────────────────┘
```

### 3.2 Core Modules

#### Blockchain Runtime Modules (Pallets)
| Pallet | Purpose | Priority |
|--------|---------|----------|
| `pallet-dex` | Decentralized exchange, AMM | P0 |
| `pallet-x3-intelligence` | AI agent registry, intents | P0 |
| `pallet-gpu-marketplace` | Compute job marketplace | P1 |
| `pallet-confidential` | TEE-based private execution | P2 |
| `pallet-staking` | Validator selection, rewards | P0 |
| `pallet-xcm` | Cross-chain message passing | P1 |

#### Application Services
| Service | Responsibility |
|---------|---------------|
| `gpu-swarm` | GPU job orchestration, sandbox management |
| `confidential-gpu` | TEE attestation, enclave management |
| `private-mempool` | Encrypted transaction mempool |
| `contention-predictor` | ML-based network congestion prediction |
| `autonomic` | Self-healing infrastructure, health monitoring |

---

## Section 4: Data Architecture

### 4.1 On-Chain Data
- **Account State**: Balances, nonces, storage
- **DEX State**: Order books, liquidity pools
- **GPU Marketplace**: Job bids, completions, attestations
- **X3 Intelligence**: Agent registry, intent storage

### 4.2 Off-Chain Data
- **User Profiles**: Encrypted JSON in IPFS
- **Cache Layer**: Redis for hot data (prices, orders)
- **Analytics**: PostgreSQL for historical data

### 4.3 Data Flow

```
User Action → Frontend → API Gateway → Blockchain Node
                                              │
                                              ▼
                                    [Consensus + Execution]
                                              │
                                              ▼
                                    [State Update + Events]
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    │                         │                         │
                    ▼                         ▼                         ▼
              [Indexing]               [Notifications]           [Analytics]
                  │                         │                         │
                  ▼                         ▼                         ▼
            [Substrate                     [WebSocket              [PostgreSQL
             Archive]                      Subscribers]              Storage]
```

---

## Section 5: Security Architecture

### 5.1 Multi-Layer Security

| Layer | Mechanisms |
|-------|------------|
| **Network** | TLS 1.3, mTLS between services, WAF |
| **Application** | Input validation, rate limiting, CSRF tokens |
| **Consensus** | Slashing for byzantine behavior, staking requirements |
| **Smart Contract** | Formal verification, upgradeable pause patterns |
| **Privacy** | TEE encryption, threshold signatures |

### 5.2 Key Management
- **Hot Wallet**: HSM-backed, daily limits
- **Cold Storage**: Multi-sig, geographic distribution
- **TEE Keys**: Threshold BLS, AMD SEV enclave

---

## Section 6: Scalability Architecture

### 6.1 Performance Targets
| Metric | Target |
|--------|--------|
| TPS | 10,000+ |
| Block Time | <2 seconds |
| Finality | <6 seconds |
| API Latency | <100ms (p95) |

### 6.2 Scaling Strategy

1. **Layer 1 Scaling**: Parachain/shard model for parallel execution
2. **Layer 2**: Rollups for high-frequency operations
3. **Caching**: Multi-layer Redis + CDN
4. **Database Sharding**: Horizontal scaling by user/shard

---

## Section 7: Integration Architecture

### 7.1 External Chain Adapters
- **Ethereum**: Via Frontier pallet
- **Solana**: Custom bridge pallet
- **Other Chains**: XCM-based parachain communication

### 7.2 API Integration Points

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Price Oracles │────▶│  X3 Chain   │◀────│  Wallet Providers │
│  (Chainlink)   │     │  (Core System)  │     │  (Multi-chain)   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        │                       │                       │
        ▼                       ▼                       ▼
   [Price Data]           [Core Logic]          [User Accounts]
```

---

## Section 8: Deployment Architecture

### 8.1 Environment Tiers

| Environment | Purpose | Infrastructure |
|-------------|---------|----------------|
| **Development** | Local testing | Docker Compose |
| **Staging** | Integration testing | Kubernetes (cloud) |
| **Production** | Live network | Multi-cloud cluster |

### 8.2 High Availability
- **Validator Nodes**: 3+ geographic regions,冗余
- **API Gateways**: Load balanced, auto-scaling
- **Database**: Primary-replica with automatic failover
- **Monitoring**: 24/7 alerting, PagerDuty integration

---

## Section 9: Component Directory Mapping

| Epic/Feature | Module/Directory | Team |
|--------------|------------------|------|
| Multi-Chain Wallet | `apps/x3-desktop/` | Frontend |
| DEX & Trading | `pallets/dex/` + `packages/atomic-swap-sdk/` | Blockchain |
| GPU Marketplace | `crates/gpu-swarm/` + `pallets/depin-marketplace/` | Backend |
| X3 Intelligence | `crates/x3-intelligence/` + `pallets/x3-intel/` | AI/ML |
| Confidential Computing | `crates/confidential-gpu/` | Security |
| Network/Validation | `pallets/staking/` + `pallets/xcm/` | Blockchain |

---

## Section 10: Open Questions & Decisions Needed

### 10.1 Architecture Decisions Pending

| Decision | Impact | Priority |
|----------|--------|----------|
| SVM Implementation Approach | Full compatibility vs. subset | High |
| TEE Technology Choice | Intel SGX vs. AMD SEV vs. Nitro | High |
| Database Scaling Strategy | Sharding approach | Medium |
| Mobile App Framework | React Native vs. Flutter vs. Native | Medium |

---

*Document Version: 1.0 (Architecture Init Complete)*
*Created: 2026-02-13*
*Author: Lojak*
*Workflow: BMAD Create Architecture - Step 1 Complete*
