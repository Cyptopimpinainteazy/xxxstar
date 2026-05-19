# X3 Chain — `infra-structure/`

Unified bare-metal infrastructure combining the cross-chain GPU validator, blockchain TPS benchmarking, 62,500+ chain database, and the Inferstructor Dashboard.

## Directory Layout

```
infra-structure/
├── start-all.sh            ← Start everything (bare metal)
├── stop-all.sh             ← Stop everything
├── status.sh               ← Health check all services
├── install.sh              ← One-shot dependency install
│
├── dashboard/              ← Inferstructor Dashboard (React + Vite + Tauri)
│   ├── src/
│   │   ├── App.tsx
│   │   ├── api.ts          ← API client (registry, bridge, RPC, admin, chain-db)
│   │   └── components/
│   │       ├── Dashboard.tsx
│   │       ├── AdminDashboard.tsx
│   │       ├── ChainExplorer.tsx    ← NEW: 62.5k chain browser
│   │       ├── LoginPage.tsx
│   │       ├── RegisterPage.tsx
│   │       └── AdminLogin.tsx
│   └── .env                ← Service URLs
│
├── services/
│   ├── chain-db/           ← Chain Database REST API (port 7070)
│   │   └── server.js       ← Express + better-sqlite3, FTS5 search
│   ├── blockchain-tps/     ← TPS benchmarking service (port 3010)
│   │   └── server.js
│   └── cloudflare-tunnel/  ← Tunnel config for exposing services
│
├── db/
│   ├── schema.sql          ← Full schema (chains, rpc_endpoints, gpu_stats, metrics)
│   ├── chains.db           ← SQLite DB with 62,500 blockchains
│   └── seed/
│       └── seed_chains.py  ← Generates 60k+ chains from real + synthetic data
│
├── validator/              ← Cross-chain GPU Validator (Python)
│   ├── src/cross_chain_gpu_validator/
│   │   ├── chain_registry.py
│   │   ├── chain_adapter.py
│   │   ├── gpu/            ← CUDA/GPU verification kernels
│   │   ├── evm/            ← EVM chain support
│   │   ├── svm/            ← Solana VM support
│   │   ├── cosmos/         ← Cosmos/Tendermint support
│   │   └── substrate/      ← Polkadot/Substrate support
│   ├── benchmarks/
│   ├── tests/
│   └── pyproject.toml
│
├── config/
│   ├── mainnet-rpc-endpoints.toml
│   └── mcp-config.json
│
└── logs/                   ← Runtime logs (auto-created)
```

## Quick Start

```bash
# 1. Install all dependencies (node + python)
./install.sh

# 2. Start everything
./start-all.sh

# 3. Open the dashboard
# → http://localhost:5174
```

## Services

| Service | Port | Description |
|---------|------|-------------|
| **Chain DB API** | 7070 | REST API over 62,500 blockchains — search, filter, paginate, ecosystem stats |
| **Blockchain TPS** | 3010 | TPS benchmarking, company demos, RPC probing |
| **Dashboard** | 5174 | Inferstructor UI — validator management, GPU monitoring, chain explorer |
| **GPU Validator** | — | Python-based cross-chain GPU signature verification |

## Bare Metal Commands

```bash
./start-all.sh              # Start all services
./start-all.sh --no-ui      # Services only (no dashboard)
./start-all.sh --seed       # Re-seed chain DB then start
./stop-all.sh               # Stop all services
./status.sh                 # Health check
./install.sh                # Install all dependencies
```

## Chain Database

The chain DB contains **62,500 blockchains** across 6 ecosystems:

| Ecosystem | Count | % |
|-----------|-------|---|
| EVM | ~36,500 | 58% |
| Cosmos | ~7,400 | 12% |
| Substrate | ~6,200 | 10% |
| SVM | ~5,000 | 8% |
| Move | ~4,300 | 7% |
| Other | ~3,100 | 5% |

### API Endpoints

```
GET  /api/chains                    — Paginated listing (filters: ecosystem, chain_type, status)
GET  /api/chains/search?q=ethereum  — Full-text search (FTS5)
GET  /api/chains/:chainId           — Chain detail + RPC endpoints + GPU stats
GET  /api/chains/stats/overview     — Aggregate statistics
GET  /api/chains/stats/ecosystems   — Ecosystem breakdown
GET  /api/rpc/:chainId              — RPC endpoints for a chain
GET  /api/gpu-stats/:chainId        — GPU validation stats
GET  /health                        — Health check
```

### Re-seeding

```bash
python3 db/seed/seed_chains.py --db db/chains.db --count 62000
```

## Dashboard Features

- **Validator Registration & Login** — API key + JWT auth
- **Real-Time TPS Monitoring** — 2s refresh, time-range selector, peak tracking
- **GPU Lane Status** — 3-lane GPU utilization, memory, temperature
- **Admin Panel** — Service management, subscriber accounting, stress testing
- **Chain Explorer** — Browse/search 62,500 chains with ecosystem charts and GPU validation details
