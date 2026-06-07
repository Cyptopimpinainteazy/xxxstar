# X3 Bridge Relayer - Testnet Validation Guide

**Phase 13d: Testnet Go-Live - Validation Steps**

This guide validates that the relayer service is operating correctly on testnet. Follow these steps after the relayer has been running for at least 10 minutes.

---

## Pre-Deployment Validation

### 1. Verify Dependencies

```bash
# Check Rust installation
rustc --version
# Expected: rustc 1.X.X or higher

# Check cargo
cargo --version
# Expected: cargo 1.X.X or higher

# Check git
git --version
# Expected: git 2.X.X or higher
```

### 2. Verify RPC Endpoints

#### X3 Testnet Node
```bash
# Check X3 node health
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"system_health",
    "params":[],
    "id":1
  }' | jq '.'

# Expected response:
# {
#   "jsonrpc": "2.0",
#   "result": {
#     "peers": 0,
#     "isSyncing": false,
#     "shouldHavePeers": false
#   },
#   "id": 1
# }
```

#### Sepolia RPC
```bash
# Replace YOUR_INFURA_KEY with actual key
INFURA_KEY="your_key_here"

curl -s "https://sepolia.infura.io/v3/${INFURA_KEY}" \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"eth_chainId",
    "params":[],
    "id":1
  }' | jq '.'

# Expected response:
# {
#   "jsonrpc": "2.0",
#   "result": "0xaa36a7",  ← Sepolia chain ID
#   "id": 1
# }
```

#### Solana Testnet RPC
```bash
curl -s https://api.testnet.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"getHealth",
    "params":[],
    "id":1
  }' | jq '.'

# Expected response:
# {
#   "jsonrpc": "2.0",
#   "result": "ok",
#   "id": 1
# }
```

---

## Deployment Execution

### Step 1: Start Relayer Service

```bash
cd /home/lojak/Desktop/x3-chain-master/crates/relayer

# Run deployment script with your Infura key
./deploy-testnet.sh your_infura_api_key

# Expected output (first 30 seconds):
# [INFO] Checking prerequisites...
# [SUCCESS] X3 testnet node is accessible
# [SUCCESS] Sepolia RPC is accessible
# [SUCCESS] Solana testnet RPC is accessible
# [SUCCESS] Prerequisites check complete
# [INFO] Building relayer binary (release mode)...
# [SUCCESS] Relayer binary built
# [INFO] Creating deployment configuration...
# [SUCCESS] Configuration created
# [INFO] Setting up environment variables...
# [SUCCESS] Environment variables set
# [INFO] Starting relayer service...
# [SUCCESS] Relayer service starting...
#
# [2026-04-21T12:34:56.789Z] INFO [x3_relayer] Starting relay loop
# [2026-04-21T12:34:56.890Z] DEBUG [x3_relayer] Polling EVM headers...
# [2026-04-21T12:34:56.991Z] DEBUG [x3_relayer] Polling SVM slots...
```

### Step 2: Open Monitoring Terminal

In a **separate terminal**, start the monitoring script:

```bash
cd /home/lojak/Desktop/x3-chain-master/crates/relayer

# Start real-time monitoring
./monitor-relayer.sh relayer.log

# Expected output (updates every 5 seconds):
# ╔════════════════════════════════════════════════════════════════╗
# ║  X3 Bridge Relayer - Testnet Monitoring                       ║
# ║  Real-time Health Check                                       ║
# ╚════════════════════════════════════════════════════════════════╝
# 
# Status: ACTIVE | Uptime: 00:02:15 | Last Update: 12:37:11
# 
# ── Core Metrics ──
#   Blocks Polled:              12 (Δ  2)
#   Blocks Finalized:            0 (Δ  0)
#   Proofs Submitted:            0 (Δ  0)
#   Proofs Failed:               0 (Δ  0)
```

---

## Post-Deployment Validation

### Validation Period: 10-30 Minutes

Monitor the relayer for the following success criteria:

#### ✅ Validation Checklist

**0-2 Minutes (Startup)**
- [ ] Relayer starts without errors
- [ ] Configuration loads successfully
- [ ] Status shows "ACTIVE"
- [ ] No ERROR-level log entries
- [ ] Logs show "Polling EVM headers" and "Polling SVM slots"

**2-10 Minutes (Initial Operation)**
- [ ] `blocks_polled` counter increments
  - EVM: every ~13 seconds (Sepolia block time)
  - SVM: every ~15 seconds (Solana slot time)
- [ ] No RPC connection errors in logs
- [ ] No configuration errors in logs
- [ ] Poll failure rate < 5% (check with: `grep "ERROR" relayer.log | wc -l`)

**10-20 Minutes (Finality Onset)**
- [ ] `blocks_finalized` counter starts incrementing
  - EVM: should see finalized blocks (after 12 block confirmations = ~156s)
  - SVM: should see finalized slots (after 32 slot confirmations = ~192s)
- [ ] `proofs_submitted` counter increments
  - Proofs should be submitted 1-5 minutes after blocks finalize
- [ ] No submission failures in logs
- [ ] Relay loop continues operating smoothly

**20-30 Minutes (Steady State)**
- [ ] Consistent polling (blocks_polled increments regularly)
- [ ] Regular finalization (blocks_finalized increments every 2-5 minutes)
- [ ] Regular submission (proofs_submitted increments every 3-10 minutes)
- [ ] Error rate remains < 5%
- [ ] No WARN or ERROR level entries except transient RPC timeouts

---

## Validation Commands

### 1. Check Relay Loop Status

```bash
# In the relayer terminal, watch the latest logs
tail -20 relayer.log | grep -E "Polling|finalized|Submitted"

# Expected pattern (repeating):
# [TIME] DEBUG Polling EVM headers...
# [TIME] DEBUG Polling SVM slots...
# [TIME] DEBUG Checking finality...
# [TIME] DEBUG Processing finalized headers...
# [TIME] DEBUG blocks_polled=45 blocks_finalized=2 proofs_submitted=1
```

### 2. Verify Metrics Are Incrementing

```bash
# In a separate terminal, check metrics every 10 seconds
watch -n10 'tail -50 relayer.log | grep -oE "blocks_polled=|blocks_finalized=|proofs_submitted=" | tail -5'

# Expected: Numbers should be increasing
```

### 3. Check for Errors

```bash
# Count ERROR-level entries (should be very few)
grep "ERROR" relayer.log | wc -l
# Expected: 0 or very small number

# Count WARN-level entries (transient failures are normal)
grep "WARN" relayer.log | wc -l
# Expected: < 10 for 30 minute run

# View actual errors
grep "ERROR" relayer.log | head -5
```

### 4. Validate Proof Submission

```bash
# Search for successful submissions
grep -i "Submitted" relayer.log | head -10

# Expected output:
# [TIME] DEBUG Submitted EVM proof: 0x1234567890abcdef...
# [TIME] DEBUG Submitted SVM proof: 0xfedcba0987654321...

# Check submission success rate
echo "Total proofs submitted: $(grep -c 'Submitted' relayer.log)"
echo "Total proofs failed: $(grep -c 'Failed to submit' relayer.log)"
```

### 5. Check RPC Performance

```bash
# Measure Sepolia RPC latency
time curl -s "https://sepolia.infura.io/v3/YOUR_KEY" \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null

# Expected: < 500ms response time

# Measure Solana RPC latency
time curl -s https://api.testnet.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}' > /dev/null

# Expected: < 500ms response time
```

### 6. Cross-Verify on Blockchain

#### Check X3 Runtime for Proof Records

```bash
# Query X3 runtime for submitted proofs
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"state_getStorage",
    "params":["0x...", "0x..."],
    "id":1
  }' | jq '.result'
```

#### Check Sepolia for Cross-VM Updates

```bash
# Get contract event logs (requires state root contract address)
curl -s "https://sepolia.infura.io/v3/YOUR_KEY" \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"eth_getLogs",
    "params":[{
      "address":"0x...",
      "topics":["0x..."],
      "fromBlock":"0x...",
      "toBlock":"latest"
    }],
    "id":1
  }' | jq '.result | length'
```

---

## Success Criteria Summary

### Green Light (Deployment Successful ✅)

The relayer deployment is **production-ready** when:

1. **Polling is consistent**
   - Blocks polled: > 1 per 13 seconds (EVM)
   - Slots polled: > 1 per 15 seconds (SVM)

2. **Finalization is occurring**
   - Blocks finalized: > 0 after 10 minutes
   - Consistent finalization rate after 15 minutes

3. **Submissions are successful**
   - Proofs submitted: > 0 after 20 minutes
   - Success rate: ≥ 95% (< 3 failures)

4. **Error rate is acceptable**
   - Poll failures: < 5% of attempts
   - No ERROR-level entries (or < 1)
   - Transient RPC timeouts handled gracefully

5. **System is stable**
   - No memory leaks (RSS memory stable)
   - No CPU spikes (steady 2-5% CPU)
   - Clean shutdown on SIGINT (Ctrl+C)

### Yellow Light (Investigation Needed ⚠️)

Review deployment if:

- Polling stops after 5 minutes → Check RPC endpoints
- No finalization after 20 minutes → Check finality thresholds
- Submission failures > 10% → Check X3 runtime availability
- Memory growing continuously → Check for resource leak
- CPU > 20% sustained → Check polling intervals

### Red Light (Deployment Failed ❌)

Do NOT proceed to mainnet if:

- Relayer crashes with ERROR
- Cannot connect to RPC endpoints
- All proofs fail to submit
- Continuous polling failures (> 50%)
- Configuration errors in logs

---

## Next Steps After Successful Validation

### If Validation PASSES (All Green Lights)

1. **Run extended validation (1-2 hours)**
   ```bash
   # Monitor relayer for 1-2 hours to ensure sustained stability
   # Watch for: consistent polling, periodic finalization, regular submissions
   ```

2. **Document baseline metrics**
   ```bash
   # Capture metrics at 10min, 30min, 60min
   tail -100 relayer.log | grep -oE "blocks_polled|blocks_finalized|proofs_submitted" > metrics-baseline.txt
   ```

3. **Create runbook**
   - Document startup procedure
   - List key monitoring metrics
   - Create alert thresholds
   - Plan incident response

4. **Proceed to Phase 13e (Mainnet Preparation)**

### If Validation FAILS (Red/Yellow Lights)

1. **Collect debugging information**
   ```bash
   # Full logs
   cp relayer.log relayer-debug.log
   
   # Configuration
   cat relayer-config.deployment.yaml
   
   # Environment
   env | grep X3_
   
   # RPC endpoint status
   # Test each endpoint manually
   ```

2. **Review troubleshooting section in TESTNET_DEPLOYMENT.md**

3. **Fix issues and retry deployment**

---

## Log Format Reference

The relayer outputs logs with the following format:

```
[YYYY-MM-DD HH:MM:SS.mmm] LEVEL [module] message | metric1=value1 metric2=value2
```

**Log Levels:**
- `DEBUG` - Detailed operation trace (polling, finality checks, submissions)
- `INFO` - Important state changes (startup, shutdown, pause/resume)
- `WARN` - Recoverable errors (RPC timeout, retry attempt)
- `ERROR` - Fatal failures (config invalid, RPC down, cannot recover)

**Key Log Patterns:**
```
# Healthy polling
DEBUG Polling EVM headers...
DEBUG Polling SVM slots...

# Healthy finality checking
DEBUG blocks_polled=45 blocks_finalized=2

# Healthy submission
DEBUG Submitted EVM proof: 0x...
DEBUG proofs_submitted=3 proofs_failed=0

# Issues requiring attention
WARN Failed to submit proof (retry_count=1)
ERROR RPC endpoint not responding
ERROR Configuration validation failed
```

---

## Expected Timeline

| Time | Event | Expected Behavior |
|------|-------|-------------------|
| 0-2m | Startup | Logs show initialization, status = ACTIVE |
| 2-5m | First polls | blocks_polled counter starts incrementing |
| 5-10m | Steady polling | Consistent 1 block per 13s (EVM), 1 slot per 15s (SVM) |
| 10-15m | First finalization | blocks_finalized increments (12 blocks = ~156s) |
| 15-20m | Proof acquisition | proofs_submitted starts incrementing |
| 20-30m | Steady state | Consistent metrics, low error rate |
| 30m+ | Production | Monitor for stability, memory usage, CPU |

---

## Support

For issues during validation:

1. Check TESTNET_DEPLOYMENT.md troubleshooting section
2. Review logs: `tail -100 relayer.log`
3. Test RPC endpoints manually
4. Verify configuration file syntax
5. Check environment variables: `env | grep X3_`

**Expected resolution time: 15-30 minutes**
