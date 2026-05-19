# Phase 13e: Mainnet Validation Guide

**Status:** Pre-Launch Planning  
**Duration:** 2-4 hours (staging environment)  
**Go-Live Ready:** After validation passes  

---

## Mainnet Validation Overview

This guide validates the relayer configuration and deployment procedures before mainnet launch. Validation occurs on a **mainnet fork** (Hardhat/Foundry) to catch issues safely.

### Key Differences from Testnet Validation

| Aspect | Testnet | Mainnet |
|--------|---------|---------|
| **Duration** | 30 minutes | 2-4 hours |
| **Environment** | Live testnet | Mainnet fork (simulated) |
| **Real Money** | Test value | Simulated only |
| **Failure Impact** | Restart relayer | Review logs, understand issue |
| **Deployment** | Direct | Staged (fork first, then live) |
| **Success Criteria** | Start & poll | All stages complete + load tested |

---

## Stage 1: Configuration Validation (30 minutes)

### 1.1 File Syntax Check

```bash
#!/bin/bash
# Check YAML syntax
yamllint /path/to/relayer-config-mainnet.yaml

# Expected: Clean output (no errors)
# If errors: Fix YAML indentation and structure
```

### 1.2 RPC Provider Validation

```bash
# Test Ethereum mainnet RPC
curl -s https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' | jq '.'

# Expected response:
# {
#   "jsonrpc": "2.0",
#   "result": "0x1",  ← Mainnet chain ID
#   "id": 1
# }

# Test Solana mainnet RPC
curl -s https://api.mainnet-beta.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getClusterNodes","params":[],"id":1}' | jq '.'

# Expected: Array of cluster nodes
```

### 1.3 RPC Latency Baseline

Measure latency to each RPC provider:

```bash
#!/bin/bash
# Test latency to all configured providers
echo "=== RPC Latency Baseline (Mainnet) ==="

# Ethereum (Alchemy)
echo "Alchemy:"
for i in {1..5}; do
  time curl -s https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null
done

# Ethereum (Infura)
echo "Infura:"
for i in {1..5}; do
  time curl -s https://mainnet.infura.io/v3/YOUR_KEY \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null
done

# Ethereum (QuickNode)
echo "QuickNode:"
for i in {1..5}; do
  time curl -s https://mainnet.quicknode.pro/?token=YOUR_KEY \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' > /dev/null
done

# Solana (QuickNode)
echo "Solana (QuickNode):"
for i in {1..5}; do
  time curl -s https://mainnet.quicknode.pro/?token=YOUR_KEY \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}' > /dev/null
done
```

**Expected Latencies:**
- Alchemy: 100-300ms
- Infura: 150-400ms
- QuickNode: 200-500ms
- Public endpoints: > 500ms (not acceptable for production)

---

## Stage 2: Mainnet Fork Environment (1 hour)

### 2.1 Set Up Hardhat Fork

```bash
# Install Hardhat
npm install --save-dev hardhat @nomicfoundation/hardhat-toolbox

# Initialize project
npx hardhat

# Create hardhat.config.js with mainnet fork
cat > hardhat.config.js << 'EOF'
require("@nomicfoundation/hardhat-toolbox");

module.exports = {
  solidity: "0.8.19",
  networks: {
    hardhat: {
      forking: {
        enabled: process.env.FORKING === "true",
        url: `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_KEY}`,
        blockNumber: 18000000  // Recent block (adjust as needed)
      }
    }
  }
};
EOF

# Start Hardhat fork
FORKING=true ALCHEMY_KEY=YOUR_KEY npx hardhat node
```

This creates a local Ethereum mainnet fork at `http://localhost:8545`

### 2.2 Deploy State Root Contract (Mock)

```bash
# Deploy mock state root contract to fork
cat > contracts/MockStateRoot.sol << 'EOF'
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract MockStateRoot {
    mapping(uint256 => bytes32) public stateRoots;
    uint256 public lastBlockNumber;
    
    event StateRootUpdated(uint256 blockNumber, bytes32 stateRoot);
    
    function updateStateRoot(uint256 blockNumber, bytes32 stateRoot) external {
        stateRoots[blockNumber] = stateRoot;
        lastBlockNumber = blockNumber;
        emit StateRootUpdated(blockNumber, stateRoot);
    }
    
    function getStateRoot(uint256 blockNumber) external view returns (bytes32) {
        return stateRoots[blockNumber];
    }
}
EOF

# Compile and deploy
npx hardhat compile
npx hardhat run --network localhost scripts/deploy.js
```

### 2.3 Configure Relayer for Fork

```yaml
# relayer-config-fork.yaml (temporary, for testing only)

x3:
  rpc_url: "http://localhost:9999"  # Mock X3 node on fork
  relayer_account: "0x1234567890123456789012345678901234567890"

evm_chains:
  - name: "Ethereum Fork"
    chain_id: 1
    x3_domain_id: 200
    rpc_endpoint: "http://localhost:8545"  # Hardhat fork
    state_root_contract: "0xDEPLOYED_CONTRACT_ADDRESS"
    finality_threshold: 2  # Lower for faster testing
    block_poll_interval_ms: 5000
    max_concurrent_requests: 10

svm_clusters:
  - name: "Solana Mock"
    cluster_name: "localhost"
    x3_domain_id: 501
    rpc_endpoint: "http://localhost:8899"  # Solana local validator
    finality_threshold: 1  # Immediate finality for testing
    slot_poll_interval_ms: 1000
    max_concurrent_requests: 20

submission:
  batch_size: 1
  timeout_secs: 10
  max_retries: 3
  retry_backoff_ms: 100

logging:
  level: "debug"
  format: "default"
```

---

## Stage 3: Deploy Relayer to Staging (1 hour)

### 3.1 Build Release Binary

```bash
cd /home/lojak/Desktop/x3-chain-master
cargo build --package x3-relayer --release

# Binary location: target/release/x3-relayer
```

### 3.2 Start Relayer on Fork

```bash
# Terminal 1: Run Hardhat fork
FORKING=true ALCHEMY_KEY=YOUR_KEY npx hardhat node

# Terminal 2: Run relayer
export X3_CONFIG_PATH="relayer-config-fork.yaml"
export X3_LOG_LEVEL="debug"
./target/release/x3-relayer

# Expected output:
# [2026-04-21T14:30:00Z] INFO Starting relay loop
# [2026-04-21T14:30:00Z] DEBUG Polling EVM headers...
# [2026-04-21T14:30:00Z] DEBUG Polling SVM slots...
```

### 3.3 Monitor for 1 Hour

```bash
# Terminal 3: Monitor metrics
tail -f relayer.log | grep -E "blocks_polled|blocks_finalized|proofs_submitted"

# Expected progression (every 5-10 seconds):
# [TIME] DEBUG blocks_polled=10 blocks_finalized=0
# [TIME] DEBUG blocks_polled=20 blocks_finalized=0
# [TIME] DEBUG blocks_polled=30 blocks_finalized=2
# [TIME] DEBUG blocks_finalized=2 proofs_submitted=1
```

**Success Criteria (Staging):**
- ✅ Relayer starts without errors
- ✅ Polling continues for entire hour
- ✅ Finalization occurs (after finality_threshold blocks)
- ✅ Proofs submitted (after finalization)
- ✅ No ERROR-level logs

---

## Stage 4: Failure Scenario Testing (30 minutes)

### 4.1 Simulate RPC Failure

```bash
# While relayer is running, stop Hardhat fork (Ctrl+C)

# Expected behavior:
# [TIME] WARN Failed to poll EVM headers: connection refused
# [TIME] WARN Retrying with fallback RPC...
# [TIME] INFO Resumed polling with fallback

# Restart Hardhat fork
FORKING=true ALCHEMY_KEY=YOUR_KEY npx hardhat node
```

**Success:** Relayer resumes polling after RPC recovery

### 4.2 Simulate High Latency

```bash
# Inject network delay with tc (traffic control)
sudo tc qdisc add dev lo root netem delay 5000ms

# Run relayer - observe behavior
tail -f relayer.log | grep -E "latency|timeout|backoff"

# Expected: Increased backoff, eventual success
# [TIME] WARN High RPC latency (5000ms), retry in 2s
# [TIME] INFO Submission succeeded after 3 retries

# Remove delay
sudo tc qdisc del dev lo root
```

**Success:** Relayer handles latency gracefully

### 4.3 Simulate Bridge Pause

```bash
# Mock governance pause in state
# Call mock pause contract:
npx hardhat run scripts/pause-bridge.js --network localhost

# Expected relayer behavior:
# [TIME] WARN Bridge is paused, queuing proofs
# [TIME] DEBUG pending_submissions=5 (queued)

# Resume bridge
npx hardhat run scripts/resume-bridge.js --network localhost

# [TIME] INFO Bridge resumed, processing queued proofs
# [TIME] DEBUG proofs_submitted=5
```

**Success:** Relayer queues proofs during pause, resumes after

---

## Stage 5: Load Testing (30 minutes)

### 5.1 Generate Load

```bash
#!/bin/bash
# inject-mainnet-load.sh
# Simulate 10x testnet load

# Inject 10 mock blocks every second
for i in {1..300}; do
  # Call mock contract to generate events
  npx hardhat run --network localhost scripts/inject-blocks.js
  sleep 0.1  # 10 blocks per second
done

# Relayer should handle this gracefully
# Monitor:
tail -f relayer.log | grep -oE "blocks_polled=|blocks_finalized=|poll_failures="
```

**Expected with 10x load:**
- Polling continues (may have higher error rate)
- Finalization still occurs (slower)
- System resources manageable (< 30% CPU, < 500MB memory)

### 5.2 Monitor Resource Usage

```bash
# Terminal: Watch system resources
watch -n2 'top -p $(pgrep x3-relayer) -b | tail -3'

# Expected metrics:
# %CPU: 5-20% (higher than testnet due to load)
# %MEM: 100-200MB (should not grow continuously)
# RES: 150-250MB

# If memory grows indefinitely → memory leak detected
# If CPU > 50% → optimization needed
```

**Success:** System handles load without degradation

---

## Stage 6: Configuration Optimization (30 minutes)

Based on staging results, optimize for mainnet:

### 6.1 Finality Threshold Tuning

If **finalization is too slow:**
```yaml
evm_chains:
  - finality_threshold: 32  # Lower from 64 (less safety, faster)

svm_clusters:
  - finality_threshold: 64  # Lower from 128
```

If **finalization is inconsistent:**
```yaml
evm_chains:
  - finality_threshold: 128  # Higher from 64 (more safety, slower)

svm_clusters:
  - finality_threshold: 256  # Higher from 128
```

### 6.2 Polling Interval Tuning

If **polling is missing blocks:**
```yaml
evm_chains:
  - block_poll_interval_ms: 5000  # More frequent (from 13000)

svm_clusters:
  - slot_poll_interval_ms: 3000  # More frequent (from 6000)
```

If **RPC rate limited:**
```yaml
evm_chains:
  - block_poll_interval_ms: 30000  # Less frequent (from 13000)
  - max_concurrent_requests: 5  # Reduce from 10

svm_clusters:
  - slot_poll_interval_ms: 10000  # Less frequent (from 6000)
  - max_concurrent_requests: 20  # Reduce from 50
```

### 6.3 Submission Tuning

If **proofs submit too slowly:**
```yaml
submission:
  batch_size: 20  # More aggressive batching (from 5)
  max_retries: 3  # Faster feedback (from 5)
  retry_backoff_ms: 500  # Quicker retries (from 1000)
```

If **submission failures high:**
```yaml
submission:
  batch_size: 1  # Conservative, one at a time (from 5)
  max_retries: 10  # More persistent (from 5)
  retry_backoff_ms: 2000  # Slower retries (from 1000)
```

---

## Pre-Launch Validation Checklist

### 4 Hours Before Launch

- [ ] Configuration file reviewed and final
- [ ] Staging validation complete (all 5 stages)
- [ ] All RPC providers tested and working
- [ ] Monitoring dashboards configured
- [ ] Alert thresholds set
- [ ] Runbooks written and available
- [ ] Team briefed on deployment procedure
- [ ] Rollback plan documented and tested
- [ ] Go/no-go decision criteria defined

### 1 Hour Before Launch

- [ ] Binary built and checksummed
- [ ] Configuration staged on production servers
- [ ] Environment variables prepared (in Vault)
- [ ] Systemd service files ready
- [ ] Log rotation configured
- [ ] Incident commander assigned
- [ ] Communication channels verified
- [ ] Final go/no-go review completed

### At Launch Time

- [ ] All team members present
- [ ] Monitoring tabs open
- [ ] Relayer binary copied to production
- [ ] Configuration deployed
- [ ] Systemd service started
- [ ] First metrics appear (within 2 minutes)
- [ ] Status page updated
- [ ] Stakeholders notified

### First Hour Post-Launch

- [ ] Polling rate normal (1 block per 13s)
- [ ] No ERROR-level logs
- [ ] RPC latency acceptable (< 1s)
- [ ] Memory usage normal (< 200MB)
- [ ] CPU usage normal (< 20%)
- [ ] Continuous monitoring (no gaps)

### First 24 Hours Post-Launch

- [ ] Proofs successfully submitted
- [ ] Finalization metrics normal
- [ ] Error rate < 1%
- [ ] No unplanned interventions
- [ ] Team morale good, no exhaustion
- [ ] Shift handoff complete

---

## Success Criteria

### Staging Validation ✅
All 5 stages complete with:
- No configuration errors
- RPC providers responding
- Polling working
- Finalization occurring
- Proofs submitting
- Failure scenarios handled
- Load testing passed

### Go-Live Ready ✅
All of:
- ✅ Staging validation complete
- ✅ Production configuration final
- ✅ Monitoring dashboards live
- ✅ Alert thresholds configured
- ✅ Team trained and ready
- ✅ Rollback plan tested
- ✅ Go/no-go decision made

### First Week Success ✅
- ✅ Uptime > 99.5%
- ✅ Proof success rate > 99%
- ✅ No unplanned restarts
- ✅ No state inconsistencies
- ✅ RPC failover tested

---

## Mainnet Launch Timeline

```
T-4h: Final staging validation
T-1h: Go/no-go decision
T-0h: Start relayer
T+5m: First polls appear
T+10m: First finalization
T+30m: First proofs submitted
T+1h: Continuous operation confirmed
T+24h: Production stable
```

**Total Time: 24 hours to confirm stability**

---

## Troubleshooting During Launch

| Symptom | Probable Cause | Fix |
|---------|----------------|-----|
| No polling | RPC endpoint down | Check endpoint, switch to fallback |
| RPC errors | Rate limit exceeded | Reduce polling frequency |
| Slow finality | Finality threshold too high | Lower threshold or wait longer |
| Stuck proofs | Bridge paused | Check governance status |
| High latency | Network congestion | Use different RPC provider |
| Memory growth | Memory leak | Restart relayer (collect logs) |
| Submission fails | X3 runtime issue | Check X3 node health |

---

## Next Steps

1. ✅ Run all 5 stages on mainnet fork
2. ✅ Optimize configuration based on results
3. ✅ Final validation checklist
4. ✅ Get go/no-go approval
5. 🚀 **Phase 13f: Execute mainnet launch**

See `PHASE_13E_MAINNET_PREP.md` for complete mainnet preparation plan.
