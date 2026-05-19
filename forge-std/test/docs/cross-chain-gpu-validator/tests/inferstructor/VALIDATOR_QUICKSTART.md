# Validator Quick Start Guide

## 🚀 Get Started in 3 Minutes

### Step 1: Register Your Validator

```bash
curl -X POST http://localhost:7001/api/validators/register \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "solana",
    "email": "your-validator@example.com",
    "sla_tier": "pro"
  }'
```

**Response:**
```json
{
  "success": true,
  "credentials": {
    "validator_id": "solana_a3f5e7c9b1d2",
    "chain": "solana",
    "api_key": "infra_xxxxxxxxxxxxx",
    "api_secret": "yyyyyyyyyyyy",
    "sla_tier": "pro",
    "max_tps": 1000000,
    "bridge_endpoint": "http://localhost:9999",
    "toll_booth_endpoint": "http://localhost:7000"
  }
}
```

⚠️ **SAVE YOUR API SECRET!** It's only shown once.

### Step 2: Test Connection

```bash
# Save your credentials
export INFRA_API_KEY="infra_xxxxxxxxxxxxx"
export INFRA_API_SECRET="yyyyyyyyyyyy"

# Test health endpoint
curl -H "X-API-Key: $INFRA_API_KEY" \
  http://localhost:9999/health
```

### Step 3: Send Your First Transaction

#### Option A: Using curl

```bash
curl -X POST http://localhost:9999/accelerate \
  -H "X-API-Key: $INFRA_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "tx_hash": "abc123",
    "tx_data": "48656c6c6f20496e66656e737472756374696f72",
    "chain": "solana"
  }'
```

#### Option B: Using the Go Adapter

```bash
cd "TPS TESTING/inferstructor"

# Build
go build -o tps_adapter tps_inferstructor_adapter.go

# Run test
./tps_adapter \
  --target-tps 500000 \
  --duration 60 \
  --bridge http://localhost:9999
```

#### Option C: Using Python

```python
import requests

API_KEY = "infra_xxxxxxxxxxxxx"

response = requests.post(
    "http://localhost:9999/accelerate",
    headers={"X-API-Key": API_KEY},
    json={
        "tx_hash": "abc123",
        "tx_data": "48656c6c6f",
        "chain": "solana"
    }
)

print(response.json())
```

## 📊 Monitor Your Usage

```bash
# Login to get JWT token
curl -X POST http://localhost:7001/api/validators/login \
  -H "Content-Type: application/json" \
  -d "{
    \"api_key\": \"$INFRA_API_KEY\",
    \"api_secret\": \"$INFRA_API_SECRET\"
  }"

# Save token
export JWT_TOKEN="eyJhbGciOiJIUzI1NiIs..."

# Get your stats
curl -H "Authorization: Bearer $JWT_TOKEN" \
  http://localhost:7001/api/validators/stats
```

**Response:**
```json
{
  "validator_id": "solana_a3f5e7c9b1d2",
  "chain": "solana",
  "sla_tier": "pro",
  "max_tps": 1000000,
  "usage": {
    "total_requests": 12500,
    "total_tx": 500000,
    "last_used": 1707753600.123
  },
  "status": "enabled"
}
```

## 🎯 SLA Tiers

| Tier | Max TPS | Latency | Priority | Cost/M TX |
|------|---------|---------|----------|-----------|
| **Basic** | 100K | <5ms | Standard | $10 |
| **Pro** | 1M | <1ms | High | $50 |
| **Enterprise** | Unlimited | <500μs | Instant | $200 |

## 🧪 Run Performance Test

```bash
cd cross-chain-gpu-validator/tests/inferstructor

# Quick 1-minute test
./run_300x_test.sh --duration 1m

# Full test with your credentials
API_KEY=$INFRA_API_KEY ./run_300x_test.sh --duration 10m
```

## 📈 Expected Results

### Solana Validator
- **Native TPS:** ~65,000
- **With Inferstructor:** ~19,500,000
- **Speedup:** **300×** ✅

### Ethereum L2
- **Native TPS:** ~4,000
- **With Inferstructor:** ~1,200,000
- **Speedup:** **300×** ✅

## 🔧 Integration Examples

### Solana Validator

```typescript
// your-validator.ts
import { Connection } from '@solana/web3.js';

const INFRA_ENDPOINT = 'http://localhost:9999';
const API_KEY = process.env.INFRA_API_KEY;

async function accelerateTransaction(tx: Transaction) {
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
  
  return response.json();
}
```

### Ethereum Validator

```javascript
// your-validator.js
const Web3 = require('web3');
const axios = require('axios');

const INFRA_ENDPOINT = 'http://localhost:9999';
const API_KEY = process.env.INFRA_API_KEY;

async function accelerateTransaction(txHash, txData) {
  const response = await axios.post(
    `${INFRA_ENDPOINT}/accelerate`,
    {
      tx_hash: txHash,
      tx_data: txData,
      chain: 'ethereum',
    },
    {
      headers: { 'X-API-Key': API_KEY }
    }
  );
  
  return response.data;
}
```

### Generic Go Client

```go
package main

import (
    "bytes"
    "encoding/json"
    "net/http"
)

const (
    InfraEndpoint = "http://localhost:9999"
    APIKey        = "infra_xxxxx" // From registration
)

type AccelRequest struct {
    TxHash string `json:"tx_hash"`
    TxData string `json:"tx_data"`
    Chain  string `json:"chain"`
}

func accelerate(txHash, txData, chain string) (map[string]interface{}, error) {
    req := AccelRequest{
        TxHash: txHash,
        TxData: txData,
        Chain:  chain,
    }
    
    body, _ := json.Marshal(req)
    
    httpReq, _ := http.NewRequest("POST", InfraEndpoint+"/accelerate", bytes.NewBuffer(body))
    httpReq.Header.Set("X-API-Key", APIKey)
    httpReq.Header.Set("Content-Type", "application/json")
    
    client := &http.Client{}
    resp, err := client.Do(httpReq)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    
    var result map[string]interface{}
    json.NewDecoder(resp.Body).Decode(&result)
    
    return result, nil
}
```

## 🛡️ Fallback Strategy

**Always implement native fallback:**

```javascript
async function sendTransaction(tx) {
  try {
    // Try acceleration first
    const result = await accelerateTransaction(tx);
    if (result.success) {
      return result;
    }
  } catch (error) {
    console.warn('Acceleration failed, using native:', error);
  }
  
  // Fallback to native validation
  return await sendNatively(tx);
}
```

## 📞 API Endpoints

### Validator Registry (Port 7001)
- `POST /api/validators/register` - Register new validator
- `POST /api/validators/login` - Get JWT token
- `GET /api/validators/validate` - Validate JWT token
- `GET /api/validators/stats` - Get usage statistics

### TPS Bridge (Port 9999)
- `POST /accelerate` - Accelerate single transaction
- `POST /accelerate/batch` - Accelerate batch of transactions
- `GET /stats` - Get current stats
- `GET /health` - Health check

### Metrics Dashboard (Port 8080)
- `GET /` - Real-time dashboard UI
- `GET /api/current` - Current metrics
- `GET /api/history` - Historical data

## 🐛 Troubleshooting

### "Invalid API key"
```bash
# Verify your key is correct
echo $INFRA_API_KEY

# Re-login to get new JWT token
curl -X POST http://localhost:7001/api/validators/login \
  -d '{"api_key":"...", "api_secret":"..."}'
```

### "Rate limit exceeded"
Your SLA tier has a max TPS limit. Upgrade:
- Basic: 100K TPS → Pro: 1M TPS
- Contact for Enterprise (unlimited)

### "Connection refused"
```bash
# Check services are running
curl http://localhost:7001/health  # Registry
curl http://localhost:9999/health  # Bridge
curl http://localhost:8080          # Dashboard

# Start services if needed
cd cross-chain-gpu-validator/tests/inferstructor
./run_300x_test.sh --phase start-services
```

## 🔐 Security Best Practices

1. **Never commit API secrets**
```bash
# Add to .env (gitignored)
echo "INFRA_API_KEY=infra_xxx" >> .env
echo "INFRA_API_SECRET=yyy" >> .env
```

2. **Rotate keys regularly**
```bash
# Contact support to rotate keys
```

3. **Use HTTPS in production**
```bash
# Production endpoint
https://inferstructor.x3.network
```

4. **Validate responses**
```javascript
if (response.result_hash !== expectedHash) {
  throw new Error('Hash mismatch - falling back to native');
}
```

## 📚 Next Steps

1. ✅ Register validator → Get API key
2. ✅ Test connection → Verify working
3. ✅ Send first transaction → See acceleration
4. ✅ Run performance test → Measure speedup
5. ✅ Integrate fallback → Production ready
6. ✅ Monitor dashboard → Track metrics

## 🎓 Learn More

- **Full Test Plan:** [INFERSTRUCTOR_300X_TEST_PLAN.md](../../docs/INFERSTRUCTOR_300X_TEST_PLAN.md)
- **Architecture:** [README.md](../../../root/README.md)
- **Authentication:** [AUTHENTICATION_SETUP.md](../../../runbooks/getting-started/AUTHENTICATION_SETUP.md)
- **Quick Reference:** [QUICKREF.md](QUICKREF.md)

## 💬 Support

- **Issues:** GitHub issues
- **Email:** support@x3.network
- **Dashboard:** http://localhost:8080
- **Status:** http://status.x3.network

---

**Ready to go 300× faster?** 🚀

Register now: `POST http://localhost:7001/api/validators/register`
