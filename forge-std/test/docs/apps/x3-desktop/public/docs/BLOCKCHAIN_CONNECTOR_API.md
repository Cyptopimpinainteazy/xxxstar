# Blockchain Connector — API Reference

## Overview

The Blockchain Connector exposes three integration surfaces:

| Protocol | Endpoint | Use Case |
|---|---|---|
| **REST** | `POST /api/v1/connectors/*` | CRUD, queries, test runs |
| **WebSocket** | `ws://host:port/ws` | Real-time block/tx events |
| **gRPC** | `host:50051` | High-throughput programmatic access |

All endpoints require an API key via `X-Api-Key` header or `apiKey` query parameter.

---

## REST API

### Connectors

#### Create a connector

```
POST /api/v1/connectors
Content-Type: application/json

{
  "chainId": "ethereum-mainnet",
  "rpcEndpoints": ["https://eth.llamarpc.com"],
  "options": {
    "retryAttempts": 3,
    "timeoutMs": 10000,
    "enableMetrics": true
  }
}
```

**Response** `201 Created`
```json
{
  "id": "conn_a1b2c3d4",
  "chainId": "ethereum-mainnet",
  "status": "connected",
  "connectedAt": "2025-02-06T18:30:00Z",
  "metrics": {
    "latencyMs": 42,
    "requestsTotal": 0,
    "errorsTotal": 0
  }
}
```

#### List connectors

```
GET /api/v1/connectors
```

Returns an array of all active connectors with current status and metrics.

#### Get connector by ID

```
GET /api/v1/connectors/:id
```

#### Disconnect

```
DELETE /api/v1/connectors/:id
```

---

### Chain Registry

#### List all chains

```
GET /api/v1/chains
```

Returns the full chain registry (40+ chains) with metadata:

```json
{
  "chains": [
    {
      "id": "ethereum-mainnet",
      "name": "Ethereum",
      "family": "evm",
      "chainIdNumeric": 1,
      "nativeCurrency": { "symbol": "ETH", "decimals": 18 },
      "gpuAccelerated": true,
      "rpcEndpoints": ["https://eth.llamarpc.com"],
      "explorerUrl": "https://etherscan.io",
      "status": "active"
    }
  ]
}
```

#### Get chain by ID

```
GET /api/v1/chains/:chainId
```

---

### Blocks & Transactions

#### Get latest block

```
GET /api/v1/connectors/:id/blocks/latest
```

```json
{
  "number": 19000000,
  "hash": "0xabc...",
  "parentHash": "0xdef...",
  "timestamp": 1706140800,
  "transactions": 182,
  "gasUsed": "12500000",
  "gasLimit": "30000000"
}
```

#### Get block by number or hash

```
GET /api/v1/connectors/:id/blocks/:numberOrHash
```

#### Get transaction

```
GET /api/v1/connectors/:id/transactions/:hash
```

---

### Validators (where supported)

```
GET /api/v1/connectors/:id/validators
```

Returns validator set with stake, uptime, and liveness data. Available on
PoS chains (Ethereum, Solana, Cosmos, Polkadot, NEAR).

---

### Metrics

```
GET /api/v1/connectors/:id/metrics
```

```json
{
  "connectorId": "conn_a1b2c3d4",
  "chainId": "ethereum-mainnet",
  "uptime": 99.97,
  "latency": {
    "p50": 38,
    "p90": 72,
    "p99": 145
  },
  "requests": {
    "total": 14832,
    "success": 14819,
    "errors": 13,
    "errorRate": 0.088
  },
  "since": "2025-02-06T18:30:00Z"
}
```

---

### Test Harness

#### Run a test profile

```
POST /api/v1/tests/run
Content-Type: application/json

{
  "connectorId": "conn_a1b2c3d4",
  "profile": "full-suite",
  "options": {
    "gpuBenchmark": true,
    "durationOverrideSec": 300
  }
}
```

**Profiles:** `latency`, `throughput`, `reorg-simulation`, `edge-cases`,
`validator-health`, `gpu-benchmark`, `pool-performance`, `full-suite`

#### Get test results

```
GET /api/v1/tests/:testRunId
```

```json
{
  "id": "test_xyz789",
  "profile": "full-suite",
  "status": "completed",
  "grade": "A+",
  "passRate": 98.5,
  "results": {
    "latency": { "grade": "A+", "p50": 38, "p90": 72, "p99": 145 },
    "throughput": { "grade": "A", "sustainedTps": 487, "peakTps": 612 },
    "gpu": { "grade": "A+", "sha256OpsPerSec": 10100000, "keccakOpsPerSec": 45700000 }
  },
  "completedAt": "2025-02-06T18:35:00Z"
}
```

---

## WebSocket API

### Connect

```
ws://host:port/ws?apiKey=YOUR_KEY
```

### Subscribe to events

After connecting, send JSON frames to subscribe:

```json
{
  "action": "subscribe",
  "connectorId": "conn_a1b2c3d4",
  "events": ["newBlock", "newTransaction", "reorg", "validatorSlash"]
}
```

### Event frames

**New block:**
```json
{
  "event": "newBlock",
  "connectorId": "conn_a1b2c3d4",
  "data": {
    "number": 19000001,
    "hash": "0x...",
    "timestamp": 1706140812,
    "txCount": 195
  }
}
```

**Reorg detected:**
```json
{
  "event": "reorg",
  "connectorId": "conn_a1b2c3d4",
  "data": {
    "depth": 2,
    "oldHead": "0xabc...",
    "newHead": "0xdef...",
    "affectedBlocks": [19000000, 18999999]
  }
}
```

### Unsubscribe

```json
{
  "action": "unsubscribe",
  "connectorId": "conn_a1b2c3d4",
  "events": ["newTransaction"]
}
```

---

## TypeScript SDK

### Installation

```bash
npm install @x3-chain/blockchain-connector
```

### Quick Start

```typescript
import { ConnectorManager, ChainRegistry } from "@x3-chain/blockchain-connector";

// Browse available chains
const chains = ChainRegistry.listChains();
const ethChain = ChainRegistry.getChain("ethereum-mainnet");

// Create a manager and connect
const manager = new ConnectorManager();
const connector = await manager.connect("ethereum-mainnet", {
  rpcEndpoints: ["https://eth.llamarpc.com"],
  enableMetrics: true,
});

// Query data
const block = await connector.getLatestBlock();
console.log(`Latest block: ${block.number}`);

// Subscribe to events
connector.subscribe("newBlock", (block) => {
  console.log(`New block #${block.number} with ${block.txCount} txs`);
});

// Run benchmarks
const results = await manager.runTest(connector.id, "gpu-benchmark");
console.log(`Grade: ${results.grade}`);

// Clean up
await manager.disconnectAll();
```

### Key Types

```typescript
interface ChainConfig {
  id: string;
  name: string;
  family: "evm" | "bitcoin" | "solana" | "cosmos" | "substrate" | "near" | "generic";
  chainIdNumeric?: number;
  nativeCurrency: { symbol: string; decimals: number };
  gpuAccelerated: boolean;
  rpcEndpoints: string[];
}

interface ConnectorMetrics {
  latencyMs: number;
  requestsTotal: number;
  errorsTotal: number;
  errorRate: number;
  uptime: number;
}

interface TestResult {
  id: string;
  profile: TestProfile;
  grade: "A+" | "A" | "B" | "C" | "D" | "F";
  passRate: number;
  results: Record<string, CategoryResult>;
}
```

---

## Error Codes

| Code | Meaning |
|---|---|
| `CHAIN_NOT_FOUND` | Chain ID not in registry |
| `CONNECTOR_EXISTS` | Duplicate connector for same chain |
| `RPC_TIMEOUT` | All RPC endpoints timed out |
| `RPC_ERROR` | Upstream RPC returned an error |
| `AUTH_FAILED` | Invalid or missing API key |
| `RATE_LIMITED` | Tier request limit exceeded |
| `INVALID_PROFILE` | Unknown test profile name |
| `GPU_UNAVAILABLE` | GPU benchmark requested but no CUDA device found |
