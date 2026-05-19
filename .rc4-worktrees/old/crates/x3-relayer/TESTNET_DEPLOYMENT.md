# X3 Bridge Relayer - Testnet Deployment Guide

**Phase 13d: Testnet Go-Live**  
**Status:** Ready for deployment  
**Target:** Sepolia (EVM) ↔ Solana Testnet (SVM)

---

## Pre-Deployment Checklist

### Prerequisites
- [x] Rust toolchain 1.35+ installed
- [ ] Access to X3 testnet node (RPC at `http://localhost:9933`)
- [ ] Relayer account funded on X3 testnet
- [ ] Infura API key for Sepolia access
- [x] Git repository cloned with Phase 13c code

### Environment Setup
- [ ] Verify X3 testnet node is running: `curl -s http://localhost:9933 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'`
- [ ] Verify Sepolia RPC is accessible: `curl -s https://sepolia.infura.io/v3/<KEY> -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'`
- [x] Verify Solana testnet RPC: `curl -s https://api.testnet.solana.com -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}'`

---

## Deployment Steps

### Step 1: Build Release Binary

```bash
cd /home/lojak/Desktop/x3-chain-master
cargo build --package x3-relayer --release 2>&1 | tail -10
```

**Expected Output:**
```
Finished release [optimized] profile [unoptimized + debuginfo] target(s) in X.XXs
```

### Step 2: Prepare Configuration

Create testnet-specific config with actual endpoints:

```bash
# Copy template
cp crates/relayer/relayer-config.testnet.yaml crates/relayer/relayer-config.deployment.yaml

# Edit with your endpoints (use environment variables for secrets)
export X3_RPC_URL="http://localhost:9933"
export X3_RELAYER_ACCOUNT="5GrwvaEF5zXb26Fz9rcQkEvVkd7FcWI4twpBD6CFPhxGwwQ"
export X3_LOG_LEVEL="debug"
export X3_CONFIG_PATH="crates/relayer/relayer-config.deployment.yaml"
```

### Step 3: Update Configuration File

Edit `crates/relayer/relayer-config.deployment.yaml`:

```yaml
evm_chains:
  - name: "Sepolia Testnet"
    chain_id: 11155111
    x3_domain_id: 200
    rpc_endpoint: "https://sepolia.infura.io/v3/YOUR_INFURA_KEY_HERE"  # ← INSERT KEY
    state_root_contract: "0x0000000000000000000000000000000000000000"
    finality_threshold: 12
    block_poll_interval_ms: 13000
    max_concurrent_requests: 5

svm_clusters:
  - name: "Solana Testnet"
    cluster_name: "testnet"
    x3_domain_id: 501
    rpc_endpoint: "https://api.testnet.solana.com"
    finality_threshold: 32
    slot_poll_interval_ms: 15000
    max_concurrent_requests: 5
```

### Step 4: Run Relayer Service

```bash
# Terminal 1: Run relayer with debug logging
export X3_LOG_LEVEL="debug"
export X3_RPC_URL="http://localhost:9933"
export X3_CONFIG_PATH="crates/relayer/relayer-config.deployment.yaml"

./target/release/x3-relayer
```

**Expected Output (first 30 seconds):**
```
[TIMESTAMP] INFO  [x3_relayer] Relayer service starting...
[TIMESTAMP] INFO  [x3_relayer] Loaded configuration from relayer-config.deployment.yaml
[TIMESTAMP] INFO  [x3_relayer] Configuration validation passed
[TIMESTAMP] INFO  [x3_relayer] EVM header watcher initialized for Sepolia
[TIMESTAMP] INFO  [x3_relayer] SVM header watcher initialized for Solana testnet
[TIMESTAMP] INFO  [x3_relayer] RPC submitter initialized
[TIMESTAMP] INFO  [x3_relayer] Starting relay loop
[TIMESTAMP] DEBUG [x3_relayer] Polling EVM headers (Sepolia)...
[TIMESTAMP] DEBUG [x3_relayer] Polling SVM slots (Solana testnet)...
[TIMESTAMP] DEBUG [x3_relayer] Checking finality...
[TIMESTAMP] DEBUG [x3_relayer] Processing finalized headers...
```

### Step 5: Monitor Relay Loop (in separate terminal)

```bash
# Terminal 2: Watch logs in real-time
tail -f relayer.log | grep -E "Submitted|finalized|error|warn"

# Or extract key metrics every 10 seconds
while true; do
  echo "=== $(date) ==="
  tail -30 relayer.log | grep -E "blocks_polled|blocks_finalized|proofs_submitted"
  sleep 10
done
```

### Step 6: Validate Cross-Chain Proof Submission

```bash
# Terminal 3: Query X3 runtime for submitted proofs
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "state_getStorage",
    "params": ["0x...", "0x..."],
    "id": 1
  }' | jq '.result'

# Or check Sepolia for state root updates
curl -s https://sepolia.infura.io/v3/YOUR_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_getStorageAt",
    "params": ["0x0000000000000000000000000000000000000000", "0x0", "latest"],
    "id": 1
  }' | jq '.result'
```

---

## Performance Monitoring

### Key Metrics to Track

| Metric | Expected | Alert Threshold |
|--------|----------|-----------------|
| Blocks Polled (EVM) | 1 per 13s | < 1 per 30s |
| Slots Polled (SVM) | 1 per 15s | < 1 per 30s |
| Blocks Finalized (EVM) | 1 per 13-156s | 0 for 10 min |
| Slots Finalized (SVM) | 1 per 15-192s | 0 for 10 min |
| Proofs Submitted | 1 per 1-5 min | 0 for 10 min |
| Submission Failures | 0 (ideally) | > 3 consecutive |
| Poll Failures | < 1 per 100 | > 5 per 100 |
| Pause Events | 0 (unless governance) | Any unexpected |

### Log Levels

- **debug**: Detailed trace of every operation (verbose, use for troubleshooting)
- **info**: Normal operations (polled blocks, finalized headers, submitted proofs)
- **warn**: Recoverable errors (retry attempts, submission failures)
- **error**: Critical failures (RPC down, config invalid)

### Metrics Export Format

```
[HH:MM:SS.mmm] INFO  blocks_polled=42 blocks_finalized=3 proofs_submitted=3
[HH:MM:SS.mmm] INFO  evm_head=12456789 svm_head=234567890
[HH:MM:SS.mmm] INFO  pending_submissions=0 proof_cache_size=3 uptime_secs=3600
[HH:MM:SS.mmm] WARN  submission_failure: {reason} (retry_count=1)
```

---

## Troubleshooting

### Issue: "RPC endpoint not accessible"

```bash
# Verify endpoint
curl -v https://sepolia.infura.io/v3/YOUR_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'

# Check for rate limiting (add retry-after header handling)
```

### Issue: "No blocks/slots being polled"

```bash
# Check watchers initialized
grep "initialized for\|Polling" relayer.log | head -20

# Verify block poll interval isn't too aggressive
# Sepolia: 13s, Solana: 15s minimum recommended
```

### Issue: "Proofs stuck in submission"

```bash
# Check if bridge is paused by governance
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"bridge_isPaused","params":[],"id":1}' | jq '.result'

# Check relay nonce
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_accountNextIndex","params":["5GrwvaEF..."],"id":1}' | jq '.result'
```

### Issue: "High retry backoff (8s+)"

```bash
# Check RPC availability
time curl -s https://sepolia.infura.io/v3/YOUR_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Reduce max_retries or retry_backoff_ms if RPC is reliable
```

---

## Success Criteria

Relay loop is operating successfully when:

1. ✅ **Logs show continuous polling**
   - "Polling EVM headers (Sepolia)..." appears every 13s
   - "Polling SVM slots (Solana testnet)..." appears every 15s

2. ✅ **Blocks/slots are finalized**
   - "blocks_finalized" counter increments periodically
   - Finalization happens 13-156s after polling (EVM) or 15-192s (SVM)

3. ✅ **Proofs are submitted**
   - "Submitted EVM proof" or "Submitted SVM proof" in logs
   - proofs_submitted counter increments
   - Submission successful on first attempt or after exponential backoff

4. ✅ **No critical errors**
   - No ERROR-level log entries (only WARN for transient failures)
   - RPC failures trigger retry logic, not loop termination
   - Individual chain failures don't halt other chains

5. ✅ **Graceful shutdown**
   - SIGINT (Ctrl+C) triggers "Shutdown signal received"
   - All pending operations complete before exit
   - Clean shutdown within 5 seconds

---

## Performance Baseline

**Expected Performance on Testnet:**

```
Configuration:
  - Sepolia poll interval: 13 seconds
  - Solana poll interval: 15 seconds
  - Finality threshold: 12 blocks (EVM), 32 slots (SVM)
  - Max concurrent requests: 5 (EVM), unlimited (SVM)
  
Sustained Throughput:
  - EVM: ~1 finalized block per 2-3 minutes (finality ~156-312s)
  - SVM: ~1 finalized slot per 2-3 minutes (finality ~192-300s)
  - Proof submission rate: ~1-2 proofs per 3-5 minutes
  
Resource Usage:
  - CPU: 2-5% (Ryzen 9, 16 cores)
  - Memory: 50-100 MB RSS
  - Network: 10-20 KB/s bandwidth
  - RPC calls: ~6 per relay loop iteration
```

---

## Deployment Checklist

After relayer is running for 10 minutes:

- [ ] Logs show continuous polling (no ERROR level)
- [ ] blocks_polled incrementing (every 13s for EVM, 15s for SVM)
- [ ] blocks_finalized incrementing (every 1-5 minutes)
- [ ] proofs_submitted incrementing (every 1-5 minutes)
- [ ] No RPC timeout failures in logs
- [ ] Pause events = 0 (or expected governance pauses only)
- [ ] Poll failures < 1% of attempts
- [ ] Nonce incrementing correctly with submissions

---

## Next Steps

Once testnet deployment is stable:

1. **Phase 13e:** Mainnet Preparation
   - Deploy to mainnet testnet (if different)
   - Validate with real mainnet-equivalent endpoints
   - Performance testing under load

2. **Phase 13f:** Mainnet Go-Live
   - Deploy to mainnet with same relayer code
   - Monitor 24/7 for first week
   - Establish on-call rotation

3. **Ongoing Monitoring**
   - Set up Prometheus metrics export
   - Create Grafana dashboards
   - Establish alerting thresholds
   - Document runbook for incident response

---

## Support & Debugging

For issues during deployment, collect:
- Last 100 lines of logs: `tail -100 relayer.log`
- Config file (sanitized): `cat relayer-config.deployment.yaml`
- Environment variables: `env | grep X3_`
- RPC endpoint health: curl to each endpoint
- X3 runtime status: `curl http://localhost:9933`

**Expected time to stable state: 10-30 minutes**
