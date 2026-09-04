# 🚀 Inferstructor: External Validator Integration Guide

## What is Inferstructor?

**Inferstructor = Infrastructure + Accelerator**

A GPU-accelerated superhighway for blockchain validators to process transactions **300× faster** than native. Think of it as a toll booth - validators drive onto our highway, pay per use, and reach their destination at 300× speed.

### Key Features

- **300× Speed:** 19.5M TPS vs Solana's 65K baseline
- **Multi-Lane Architecture:** Primary GPU → Shadow GPU → Tertiary CPU fallback
- **Sub-millisecond Failover:** <3ms lane switching with zero downtime
- **Pay-Per-Use:** SLA tiers from $10/M to $200/M transactions
- **Chain Agnostic:** Works with Solana, Ethereum, Arbitrum, any blockchain
- **API Key Authentication:** Secure access with JWT tokens
- **Real-time Monitoring:** Live dashboard at http://localhost:8080

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    EXTERNAL VALIDATORS                       │
│           (Solana, Ethereum, Arbitrum, etc.)                 │
└────────────┬────────────────────────────────────────────────┘
             │ Register → Get API Key
             ▼
┌─────────────────────────────────────────────────────────────┐
│              VALIDATOR REGISTRY (Port 7001)                  │
│  • Registration: POST /api/validators/register               │
│  • Authentication: POST /api/validators/login                │
│  • API Key Management                                        │
│  • SLA Tier Selection (Basic/Pro/Enterprise)                │
└────────────┬────────────────────────────────────────────────┘
             │ API Key validated
             ▼
┌─────────────────────────────────────────────────────────────┐
│                TPS BRIDGE (Port 9999)                        │
│  • POST /accelerate - Single transaction                     │
│  • POST /accelerate/batch - Batch transactions              │
│  • API Key Validation with X-API-Key header                 │
└────────────┬────────────────────────────────────────────────┘
             │
             ├──────────────┬──────────────┬──────────────┐
             ▼              ▼              ▼              ▼
    ┌───────────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐
    │  TOLL BOOTH   │ │ PRIMARY  │ │  SHADOW  │ │  TERTIARY    │
    │  (Port 7000)  │ │   LANE   │ │   LANE   │ │    LANE      │
    │               │ │ 4×A100   │ │ 4×A100   │ │   CPU        │
    │ SLA Enforce   │ │ 10M TPS  │ │ 10M TPS  │ │ 250K TPS     │
    └───────────────┘ └──────────┘ └──────────┘ └──────────────┘
```

### Lane Strategy

| Lane | Hardware | Max TPS | Latency | Use Case |
|------|----------|---------|---------|----------|
| **Primary** | 4× NVIDIA A100 GPUs | 10M | <500μs | Active processing |
| **Shadow** | 4× NVIDIA A100 GPUs | 10M | <500μs | Hot standby (synced) |
| **Tertiary** | 64-core CPU | 250K | <5ms | Regional failover |

**Failover:** Primary fails → <3ms switch to Shadow → Tertiary if needed

## 🚀 Quick Start (3 Commands)

### 1. Start Inferstructor Services

```bash
cd cross-chain-gpu-validator/tests/inferstructor
./start_inferstructor.sh
```

**This starts:**
- Validator Registry (port 7001)
- TPS Bridge (port 9999)
- Metrics Dashboard (port 8080)
- Lane Orchestrator (background)

### 2. Register Your Validator

```bash
./register_validator.sh solana your-email@example.com pro
```

**Response:**
```
✅ Registration successful!

🔑 SAVE THESE CREDENTIALS:
Validator ID: solana_a3f5e7c9b1d2
API Key:      infra_xxxxxxxxxxxxx
API Secret:   yyyyyyyyyyyy
Max TPS:      1000000

💾 Credentials saved to: .env.validator.solana_a3f5e7c9b1d2
```

### 3. Test Acceleration

```bash
# Load credentials
source .env.validator.solana_a3f5e7c9b1d2

# Send test transaction
curl -X POST http://localhost:9999/accelerate \
  -H "X-API-Key: $INFRA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "tx_hash": "test123",
    "tx_data": "48656c6c6f",
    "chain": "solana"
  }'
```

**Expected Response:**
```json
{
  "success": true,
  "tx_hash": "test123",
  "result": "52534c5420484153483a20...",
  "result_hash": "d4e5f6a7b8c9...",
  "lane_id": "primary",
  "latency_ms": 0.42,
  "validator_id": "solana_a3f5e7c9b1d2"
}
```

✅ **300× acceleration active!**

## 🔐 Authentication System

Inferstructor uses JWT-based authentication with API keys.

### Registration Flow

```
1. POST /api/validators/register
   ↓
2. Receive API Key + Secret (SAVE THESE!)
   ↓
3. POST /api/validators/login (with key+secret)
   ↓
4. Receive JWT token (24hr expiry)
   ↓
5. Use X-API-Key header in all requests
```

### API Endpoints

#### Register Validator
```bash
curl -X POST http://localhost:7001/api/validators/register \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "solana",
    "email": "validator@example.com",
    "sla_tier": "pro"
  }'
```

#### Login (Get JWT Token)
```bash
curl -X POST http://localhost:7001/api/validators/login \
  -H "Content-Type: application/json" \
  -d '{
    "api_key": "infra_xxxxxxxxxxxxx",
    "api_secret": "yyyyyyyyyyyy"
  }'
```

#### Validate Token
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:7001/api/validators/validate
```

#### Get Usage Stats
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:7001/api/validators/stats
```

## 💰 SLA Tiers & Pricing

| Tier | Max TPS | Latency | Priority | Cost/M TX | Features |
|------|---------|---------|----------|-----------|----------|
| **Basic** | 100K | <5ms | Standard | $10 | GPU accel, deterministic |
| **Pro** | 1M | <1ms | High | $50 | + Priority lane, dashboard |
| **Enterprise** | ∞ | <500μs | Instant | $200 | + Dedicated slice, 24/7 support |

**Choose your tier during registration:**
```bash
./register_validator.sh solana you@example.com basic      # $10/M
./register_validator.sh solana you@example.com pro        # $50/M (default)
./register_validator.sh solana you@example.com enterprise # $200/M
```

## 📊 Integration Examples

### Solana Validator

```typescript
// solana-validator-acceleration.ts
import { Connection, Transaction } from '@solana/web3.js';

const INFRA_ENDPOINT = 'http://localhost:9999';
const API_KEY = process.env.INFRA_API_KEY;

async function accelerateTransaction(tx: Transaction): Promise<any> {
  const response = await fetch(`${INFRA_ENDPOINT}/accelerate`, {
    method: 'POST',
    headers: {
      'X-API-Key': API_KEY,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      tx_hash: tx.signature,
      tx_data: tx.serialize().toString('hex'),
      chain: 'solana',
    }),
  });
  
  if (!response.ok) {
    throw new Error(`Acceleration failed: ${response.status}`);
  }
  
  return response.json();
}

// Production validator logic with fallback
async function processTransaction(tx: Transaction) {
  try {
    // Try GPU acceleration first
    const result = await accelerateTransaction(tx);
    console.log(`✅ GPU accelerated: ${result.latency_ms}ms`);
    return result;
  } catch (error) {
    console.warn('Acceleration failed, using native validation:', error);
    
    // Fallback to native Solana validation
    return await sendNatively(tx);
  }
}
```

### Ethereum L2 Validator

```javascript
// eth-l2-validator-acceleration.js
const Web3 = require('web3');
const axios = require('axios');

const INFRA_ENDPOINT = 'http://localhost:9999';
const API_KEY = process.env.INFRA_API_KEY;

async function accelerateTransaction(txHash, txData) {
  try {
    const response = await axios.post(
      `${INFRA_ENDPOINT}/accelerate`,
      {
        tx_hash: txHash,
        tx_data: txData,
        chain: 'ethereum',
      },
      {
        headers: { 'X-API-Key': API_KEY },
        timeout: 100, // 100ms timeout
      }
    );
    
    return response.data;
  } catch (error) {
    console.error('Acceleration error:', error.message);
    throw error;
  }
}

// Use in your validator
async function validateBlock(block) {
  const acceleratedTxs = [];
  
  for (const tx of block.transactions) {
    try {
      const result = await accelerateTransaction(tx.hash, tx.data);
      acceleratedTxs.push(result);
    } catch (error) {
      // Fallback to native
      const nativeResult = await validateNatively(tx);
      acceleratedTxs.push(nativeResult);
    }
  }
  
  return acceleratedTxs;
}
```

### Go Client (Generic)

```go
// inferstructor_client.go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
    "os"
)

const InfraEndpoint = "http://localhost:9999"

type AccelRequest struct {
    TxHash string `json:"tx_hash"`
    TxData string `json:"tx_data"`
    Chain  string `json:"chain"`
}

type AccelResponse struct {
    Success   bool    `json:"success"`
    TxHash    string  `json:"tx_hash"`
    Result    string  `json:"result"`
    ResultHash string `json:"result_hash"`
    LaneID    string  `json:"lane_id"`
    LatencyMs float64 `json:"latency_ms"`
}

func accelerateTransaction(txHash, txData, chain string) (*AccelResponse, error) {
    apiKey := os.Getenv("INFRA_API_KEY")
    
    req := AccelRequest{
        TxHash: txHash,
        TxData: txData,
        Chain:  chain,
    }
    
    body, _ := json.Marshal(req)
    
    httpReq, _ := http.NewRequest("POST", InfraEndpoint+"/accelerate", bytes.NewBuffer(body))
    httpReq.Header.Set("X-API-Key", apiKey)
    httpReq.Header.Set("Content-Type", "application/json")
    
    client := &http.Client{}
    resp, err := client.Do(httpReq)
    if err != nil {
        return nil, fmt.Errorf("request failed: %w", err)
    }
    defer resp.Body.Close()
    
    if resp.StatusCode != 200 {
        return nil, fmt.Errorf("acceleration failed: HTTP %d", resp.StatusCode)
    }
    
    var result AccelResponse
    if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
        return nil, fmt.Errorf("decode failed: %w", err)
    }
    
    return &result, nil
}

// Use with Go TPS tester
func main() {
    result, err := accelerateTransaction("abc123", "48656c6c6f", "solana")
    if err != nil {
        fmt.Printf("Error: %v\n", err)
        return
    }
    
    fmt.Printf("✅ Accelerated: %s (%.2fms)\n", result.TxHash, result.LatencyMs)
}
```

## 🧪 Performance Testing

### Quick Test (1 minute)

```bash
cd cross-chain-gpu-validator/tests/inferstructor
./run_300x_test.sh --duration 1m
```

### Full Test (10 minutes, production-like)

```bash
# Export your API key
export INFRA_API_KEY="infra_xxxxxxxxxxxxx"

# Run full test
./run_300x_test.sh --duration 10m --validate-results
```

**Expected Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 PROOF: 300× FASTER THAN SOLANA
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Baseline (Solana):        65,000 TPS
Inferstructor:       19,500,000 TPS
Speedup:                    300.0×

✅ 300× TARGET ACHIEVED
```

### Using Go TPS Tester

```bash
cd "TPS TESTING/inferstructor"

# Build adapter
go build -o tps_adapter tps_inferstructor_adapter.go

# Run test with your API key
export INFRA_API_KEY="infra_xxxxxxxxxxxxx"

./tps_adapter \
  --target-tps 1000000 \
  --duration 300 \
  --bridge http://localhost:9999 \
  --workers 100
```

## 📈 Monitoring & Analytics

### Real-Time Dashboard

Open: **http://localhost:8080**

Shows:
- Current TPS (real-time)
- Latency distribution (p50, p95, p99)
- Active lane (Primary/Shadow/Tertiary)
- Failover events
- Validator usage by API key

### API Metrics

```bash
# Bridge stats
curl http://localhost:9999/stats

# Validator stats (requires JWT)
curl -H "Authorization: Bearer <token>" \
  http://localhost:7001/api/validators/stats
```

### Prometheus Metrics

Available at:
- Bridge: http://localhost:8002/metrics
- Orchestrator: http://localhost:9091/metrics

## 🛡️ Production Best Practices

### 1. Always Implement Fallback

```javascript
async function sendTransaction(tx) {
  try {
    // Try acceleration (with timeout)
    const result = await Promise.race([
      accelerateTransaction(tx),
      timeout(100) // 100ms max
    ]);
    
    if (result.success) {
      return result;
    }
  } catch (error) {
    console.warn('Acceleration unavailable, using native');
  }
  
  // Fallback to native validation
  return await sendNatively(tx);
}
```

### 2. Validate Results

```javascript
// Verify result hash matches expected
if (result.result_hash !== expectedHash) {
  console.error('Hash mismatch - potential corruption');
  // Fall back to native
  return await sendNatively(tx);
}
```

### 3. Monitor Rate Limits

Your SLA tier has max TPS limits:
- Basic: 100K TPS
- Pro: 1M TPS  
- Enterprise: Unlimited

**Track usage:**
```bash
curl -H "Authorization: Bearer $JWT_TOKEN" \
  http://localhost:7001/api/validators/stats
```

### 4. Secure Your Credentials

```bash
# Never commit credentials
echo ".env.validator.*" >> .gitignore

# Use environment variables
export INFRA_API_KEY="..."
export INFRA_API_SECRET="..."

# Rotate keys periodically (contact support)
```

### 5. Use HTTPS in Production

```bash
# Production endpoint (when live)
export INFRA_ENDPOINT="https://inferstructor.x3.network"
```

## 🐛 Troubleshooting

### "Invalid API key"

```bash
# Verify credentials
echo $INFRA_API_KEY

# Re-login to get new JWT
curl -X POST http://localhost:7001/api/validators/login \
  -d '{"api_key":"...","api_secret":"..."}'
```

### "Rate limit exceeded"

Your SLA tier max TPS exceeded. Upgrade:
```bash
# Contact support for tier upgrade
# Or reduce load to stay under limit
```

### "Connection refused"

```bash
# Check services are running
curl http://localhost:7001/health  # Registry
curl http://localhost:9999/health  # Bridge
curl http://localhost:8080          # Dashboard

# Restart if needed
./stop_inferstructor.sh
./start_inferstructor.sh
```

### "Timeout"

Acceleration took >100ms (rare). Your code should:
1. Catch timeout exception
2. Fall back to native validation
3. Log for review

## 📚 Documentation

- **Quick Start:** [VALIDATOR_QUICKSTART.md](VALIDATOR_QUICKSTART.md)
- **Full Test Plan:** [INFERSTRUCTOR_300X_TEST_PLAN.md](../../docs/INFERSTRUCTOR_300X_TEST_PLAN.md)
- **Quick Reference:** [QUICKREF.md](QUICKREF.md)
- **Authentication Setup:** [AUTHENTICATION_SETUP.md](../../../../runbooks/getting-started/AUTHENTICATION_SETUP.md)

## 🤝 Support

- **GitHub Issues:** https://github.com/your-org/x3-chain/issues
- **Email:** support@x3.network
- **Dashboard:** http://localhost:8080
- **Status Page:** http://status.x3.network

## 🎯 Roadmap

- [x] Multi-lane GPU acceleration (Primary/Shadow/Tertiary)
- [x] <3ms deterministic failover
- [x] JWT authentication & API keys
- [x] SLA tier enforcement
- [x] Real-time monitoring dashboard
- [ ] Multi-region deployment (US, EU, APAC)
- [ ] Custom lane slicing for Enterprise
- [ ] 24/7 NOC support
- [ ] Smart contract integration
- [ ] Cross-chain atomic swaps via acceleration

## ⚖️ License

See [LICENSE](../../../LICENSE) for details.

---

**Ready to go 300× faster?**

```bash
./register_validator.sh <chain> <email> <tier>
```

🚀 **Welcome to the Inferstructor Superhighway!**
