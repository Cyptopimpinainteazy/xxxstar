# Mainnet Performance Baseline

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Ready for Production Use  
**Target Audience:** Operations Team, Network Engineers, SREs

---

## Overview

This document defines the **performance baseline and targets for X3 mainnet**. It covers:
- Expected throughput (TPS)
- Expected latency targets
- Resource utilization expectations
- Capacity headroom strategy
- Performance regression detection
- Success metrics by component

---

## Section 1: Throughput Targets

### Transaction Processing (TPS)

**Mainnet Expected Performance:**

| Component | Target | Acceptable Range | Alert Threshold |
|-----------|--------|------------------|-----------------|
| **Bridge Relayer** |
| Blocks polled per minute (EVM) | 4-5 | 3-6 | < 3 |
| Blocks polled per minute (SVM) | 8-10 | 6-12 | < 6 |
| Proofs submitted per minute | 2-4 | 1-6 | < 1 |
| Average proofs queued | < 5 | < 20 | > 20 |
| **X3 Runtime** |
| Transactions per second | 100+ | 80+ | < 80 |
| Blocks per second | 1 | 0.8-1.2 | < 0.8 |
| Block finality time | < 15 seconds | 10-20 sec | > 30 sec |

### Establishing Baseline

**Day 1 (Launch + 24 hours):**

```bash
#!/bin/bash

echo "=== Establishing Mainnet Baseline ==="

# Collect 24-hour baseline
BASELINE_START=$(date)
echo "Baseline collection started: $BASELINE_START"

# Sample metrics every 5 minutes
for i in {1..288}; do  # 288 samples = 24 hours
  TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
  
  # Collect metrics
  BLOCKS_POLLED=$(curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])" | jq '.data.result[0].value[1]')
  PROOFS_SUBMITTED=$(curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])" | jq '.data.result[0].value[1]')
  PENDING_PROOFS=$(curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count" | jq '.data.result[0].value[1]')
  
  echo "$TIMESTAMP | Blocks: $BLOCKS_POLLED/min | Proofs: $PROOFS_SUBMITTED/min | Pending: $PENDING_PROOFS"
  
  # Log to file
  echo "$TIMESTAMP,$BLOCKS_POLLED,$PROOFS_SUBMITTED,$PENDING_PROOFS" >> /var/log/baseline.csv
  
  sleep 300  # Wait 5 minutes
done

echo "Baseline collection complete at $(date)"

# Calculate averages
echo
echo "=== Baseline Summary ==="
BLOCKS_AVG=$(tail -n +2 /var/log/baseline.csv | cut -d',' -f2 | awk '{sum+=$1; n++} END {print sum/n}')
PROOFS_AVG=$(tail -n +2 /var/log/baseline.csv | cut -d',' -f3 | awk '{sum+=$1; n++} END {print sum/n}')
PENDING_AVG=$(tail -n +2 /var/log/baseline.csv | cut -d',' -f4 | awk '{sum+=$1; n++} END {print sum/n}')

echo "Average blocks polled per minute: $BLOCKS_AVG"
echo "Average proofs submitted per minute: $PROOFS_AVG"
echo "Average pending proofs: $PENDING_AVG"

# Store baseline
cat > /etc/x3-relayer/baseline.yml << EOF
baseline:
  collected_at: "$(date)"
  measurement_period: "24 hours"
  blocks_polled_per_minute: $BLOCKS_AVG
  proofs_submitted_per_minute: $PROOFS_AVG
  pending_proofs_average: $PENDING_AVG
EOF
```

### Related Documents

**For incidents involving performance:** See **MAINNET_INCIDENT_RESPONSE.md** (performance degradation scenarios)

**For RPC latency issues:** See **RPC_FAILOVER_PROCEDURES.md** (when RPC slowness requires failover)

**For validator-related performance:** See **VALIDATOR_OPERATIONS.md** (validator addition impact on metrics)

**For GPU performance:** See **GPU_VALIDATOR_TROUBLESHOOTING.md** (GPU issues affecting block production)

**For launch timeline:** See **PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md** (expected T+24h, T+7d performance milestones)

---

## Section 2: Latency Targets

### Transaction Confirmation Times

**Expected latencies from submission to finality:**

| Action | Target | Acceptable Range | Alert Threshold |
|--------|--------|------------------|-----------------|
| RPC request → response | < 100ms | < 200ms | > 500ms |
| Proof submission → on-chain | 5-30 sec | 1-60 sec | > 120 sec |
| Block proposed → finalized (EVM) | 60-180 sec | 30-300 sec | > 600 sec |
| Slot proposed → finalized (SVM) | 10-30 sec | 5-60 sec | > 120 sec |
| Bridge pause → resume | < 1 hour | < 4 hours | > 4 hours |

### Measuring Latency

```bash
#!/bin/bash

echo "=== Latency Measurement ==="

# Test RPC latency
for RPC in "alchemy" "infura" "quicknode"; do
  echo "Testing $RPC..."
  
  START=$(date +%s%N)
  curl -X POST "https://eth-mainnet-$RPC.example.com/api" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
    --max-time 5 > /dev/null 2>&1
  END=$(date +%s%N)
  
  LATENCY=$(( (END - START) / 1000000 ))
  echo "  Latency: ${LATENCY}ms"
  
  if [ $LATENCY -gt 500 ]; then
    echo "  ⚠️  WARNING: Latency above threshold"
  fi
done

# Test proof submission latency
echo
echo "Measuring proof submission latency..."

# Submit a proof and measure time to finality
SUBMIT_TIME=$(date +%s)
PROOF_HASH=$(curl -s http://localhost:9090/api/v1/metrics | grep "last_submitted_proof_hash")

# Wait for finality
sleep 60

FINAL_TIME=$(date +%s)
SUBMISSION_LATENCY=$(( FINAL_TIME - SUBMIT_TIME ))

echo "Proof submission latency: ${SUBMISSION_LATENCY}s"
if [ $SUBMISSION_LATENCY -gt 120 ]; then
  echo "⚠️  WARNING: Submission latency above threshold"
fi
```

---

## Section 3: Resource Utilization Expectations

### CPU Utilization

**Expected CPU usage during normal operation:**

```bash
#!/bin/bash

echo "=== CPU Utilization Baseline ==="

# Establish baseline
for i in {1..60}; do
  CPU_PCT=$(top -b -n 1 | grep "x3-relayer" | awk '{print $9}')
  echo "$(date): CPU: $CPU_PCT%"
  sleep 60
done | tee /tmp/cpu-baseline.log

# Calculate statistics
echo
echo "=== CPU Statistics ==="
USAGE=$(awk '{print $NF}' /tmp/cpu-baseline.log | tail -n +2)
AVG=$(echo "$USAGE" | awk '{sum+=$1; n++} END {print sum/n}')
MAX=$(echo "$USAGE" | sort -n | tail -1)
MIN=$(echo "$USAGE" | sort -n | head -1)

echo "Average CPU: ${AVG}%"
echo "Maximum CPU: ${MAX}%"
echo "Minimum CPU: ${MIN}%"

# Alert if CPU sustained above 80%
if (( $(echo "$AVG > 80" | bc -l) )); then
  echo "⚠️  WARNING: CPU utilization consistently above 80%"
fi
```

**Expected CPU by operation:**

| Operation | Typical CPU | Peak CPU | Alert Threshold |
|-----------|------------|----------|-----------------|
| Idle (waiting) | 1-3% | — | > 5% |
| Block polling | 3-5% | — | > 10% |
| Proof calculation | 5-10% | — | > 20% |
| Peak load (all operations) | 10-15% | 30% | > 50% |

### Memory Utilization

**Expected memory usage:**

```bash
#!/bin/bash

echo "=== Memory Utilization Baseline ==="

# Monitor for 1 hour
for i in {1..60}; do
  MEM_MB=$(ps aux | grep "x3-relayer" | grep -v grep | awk '{print $6}')
  echo "$(date): Memory: ${MEM_MB}MB"
  sleep 60
done | tee /tmp/mem-baseline.log

# Calculate statistics
echo
echo "=== Memory Statistics ==="
USAGE=$(awk '{print $NF}' /tmp/mem-baseline.log | sed 's/MB//' | tail -n +2)
AVG=$(echo "$USAGE" | awk '{sum+=$1; n++} END {print sum/n}')
MAX=$(echo "$USAGE" | sort -n | tail -1)

echo "Average Memory: ${AVG}MB"
echo "Peak Memory: ${MAX}MB"

# Alert if memory growing
for i in {1..10}; do
  CURRENT=$(ps aux | grep "x3-relayer" | grep -v grep | awk '{print $6}')
  echo "$CURRENT"
  sleep 60
done | tail -1 > /tmp/final-mem.txt
FINAL=$(cat /tmp/final-mem.txt)

GROWTH=$(echo "$FINAL - $MAX" | bc)
if [ $GROWTH -gt 100 ]; then
  echo "⚠️  WARNING: Memory growing ${GROWTH}MB in 10 minutes"
fi
```

**Expected memory by configuration:**

| Scenario | Typical Usage | Peak Usage | Alert Threshold |
|----------|---------------|-----------|-----------------|
| Idle (no blocks) | 100-150 MB | — | > 200 MB |
| Normal operation | 150-300 MB | — | > 500 MB |
| Cache at capacity | 300-500 MB | — | > 1000 MB |
| Memory leak detected | Growing | Growing | > 5% growth/day |

### Disk I/O

**Expected disk usage patterns:**

```bash
#!/bin/bash

echo "=== Disk I/O Baseline ==="

# Monitor disk I/O for 1 hour
DEVICE="sda"  # Or appropriate device

for i in {1..60}; do
  READS=$(iostat -x 1 2 | grep $DEVICE | tail -1 | awk '{print $4}')
  WRITES=$(iostat -x 1 2 | grep $DEVICE | tail -1 | awk '{print $6}')
  echo "$(date): Reads: ${READS}MB/s, Writes: ${WRITES}MB/s"
  sleep 60
done

# Monitor disk space usage
df -h /var/lib/x3-relayer

# Expected:
# - Minimal reads during polling (< 1 MB/s)
# - Minimal writes during normal operation (< 5 MB/s)
# - Occasional writes for state snapshots (10-50 MB/s, brief)
```

### Network Bandwidth

**Expected network usage:**

| Direction | Typical | Peak | Alert Threshold |
|-----------|---------|------|-----------------|
| Download (RPC requests) | 50-100 Mbps | 200 Mbps | > 500 Mbps |
| Upload (Proof submissions) | 10-20 Mbps | 50 Mbps | > 200 Mbps |
| P2P traffic (consensus) | 5-10 Mbps | 20 Mbps | > 100 Mbps |

---

## Section 4: Capacity Headroom Strategy

### Safe Operating Ranges

**Keep these safety margins:**

```yaml
# CPU: Keep usage under 70% sustained
# This allows for:
# - Temporary spikes (up to 90%)
# - Load from monitoring/logging (5-10%)
# - System processes (5-10%)
cpu_target: 50%
cpu_maximum: 70%
cpu_emergency: 90%

# Memory: Keep usage under 60% of available
# This allows for:
# - Temporary growth (up to 80%)
# - OS and other processes (10-20%)
# - Sudden cache needs (5-10%)
memory_target: 40%
memory_maximum: 60%
memory_emergency: 80%

# Disk: Keep usage under 70% of capacity
# This allows for:
# - Temporary growth (up to 85%)
# - Log rotation (10%)
# - Snapshots (5%)
disk_target: 50%
disk_maximum: 70%
disk_emergency: 85%

# Network: Keep sustained usage under 20% of available
# This allows for:
# - Temporary peaks (up to 50%)
# - Protocol overhead (10%)
# - Provider failover (10%)
network_target: 10%
network_maximum: 20%
network_emergency: 50%
```

### Headroom Monitoring

```bash
#!/bin/bash

echo "=== Capacity Headroom Check ==="

# CPU
CPU_USAGE=$(top -b -n 1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us//')
echo "CPU: ${CPU_USAGE}% (Target: 50%, Max: 70%)"
if (( $(echo "$CPU_USAGE > 70" | bc -l) )); then
  echo "  ⚠️  CPU above maximum"
fi

# Memory
TOTAL=$(free -g | awk 'NR==2 {print $2}')
USED=$(free -g | awk 'NR==2 {print $3}')
PCT=$(echo "scale=1; $USED * 100 / $TOTAL" | bc)
echo "Memory: ${PCT}% (${USED}GB/${TOTAL}GB, Target: 40%, Max: 60%)"
if (( $(echo "$PCT > 60" | bc -l) )); then
  echo "  ⚠️  Memory above maximum"
fi

# Disk
DISK_USED=$(df -h /var/lib/x3-relayer | awk 'NR==2 {print $5}' | sed 's/%//')
echo "Disk: ${DISK_USED}% (Target: 50%, Max: 70%)"
if [ $DISK_USED -gt 70 ]; then
  echo "  ⚠️  Disk above maximum"
fi

# Network
BANDWIDTH=$(iftop -n -t | grep "Total" | awk '{print $2}')
echo "Network: $BANDWIDTH (Target: 10%, Max: 20%)"

echo
echo "=== Capacity Status ==="
if [ $CPU_USAGE -lt 50 ] && [ $(echo "$PCT < 40" | bc -l) -eq 1 ] && [ $DISK_USED -lt 50 ]; then
  echo "✅ All metrics within target ranges"
elif [ $CPU_USAGE -lt 70 ] && [ $(echo "$PCT < 60" | bc -l) -eq 1 ] && [ $DISK_USED -lt 70 ]; then
  echo "⚠️  Some metrics approaching maximum"
else
  echo "🔴 One or more metrics above maximum - URGENT ACTION NEEDED"
fi
```

---

## Section 5: Performance Regression Detection

### Regression Signatures

**How to detect performance degradation:**

| Symptom | Normal Range | Degraded | Critical |
|---------|--------------|----------|----------|
| Block polling rate dropping | 4-5/min | < 3/min | < 1/min |
| Proof submission rate dropping | 2-4/min | < 1/min | 0/min |
| Pending proofs increasing | < 5 | > 20 | > 100 |
| Latency increasing | 100ms | 500ms+ | 2000ms+ |
| Memory growing continuously | Stable | > 5%/day | > 20%/day |

### Automated Regression Detection

```bash
#!/bin/bash

echo "=== Automated Regression Detector ==="

# Get current baseline
BASELINE_FILE="/etc/x3-relayer/baseline.yml"
BASELINE_BLOCKS=$(grep "blocks_polled_per_minute:" $BASELINE_FILE | awk '{print $2}')
BASELINE_PROOFS=$(grep "proofs_submitted_per_minute:" $BASELINE_FILE | awk '{print $2}')

# Measure current performance
CURRENT_BLOCKS=$(curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[5m])" | jq '.data.result[0].value[1]')
CURRENT_PROOFS=$(curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[5m])" | jq '.data.result[0].value[1]')

# Calculate deviation
BLOCK_DEVIATION=$(echo "scale=1; ($BASELINE_BLOCKS - $CURRENT_BLOCKS) / $BASELINE_BLOCKS * 100" | bc)
PROOF_DEVIATION=$(echo "scale=1; ($BASELINE_PROOFS - $CURRENT_PROOFS) / $BASELINE_PROOFS * 100" | bc)

echo "Block polling: ${CURRENT_BLOCKS}/min (baseline: ${BASELINE_BLOCKS}/min)"
echo "  Deviation: ${BLOCK_DEVIATION}%"

echo "Proof submission: ${CURRENT_PROOFS}/min (baseline: ${BASELINE_PROOFS}/min)"
echo "  Deviation: ${PROOF_DEVIATION}%"

# Alert on significant degradation
if (( $(echo "$BLOCK_DEVIATION > 20" | bc -l) )); then
  echo
  echo "🔴 REGRESSION DETECTED: Block polling down ${BLOCK_DEVIATION}%"
  echo "  Possible causes:"
  echo "  - RPC provider issues"
  echo "  - Network latency increased"
  echo "  - Resource exhaustion"
  echo "  - Code regression"
fi

if (( $(echo "$PROOF_DEVIATION > 20" | bc -l) )); then
  echo
  echo "🔴 REGRESSION DETECTED: Proof submission down ${PROOF_DEVIATION}%"
  echo "  Possible causes:"
  echo "  - RPC endpoint down"
  echo "  - Bridge runtime issues"
  echo "  - Account nonce problems"
fi
```

---

## Section 6: Success Metrics by Component

### Bridge Relayer Metrics

**Health indicators for relayer service:**

```bash
# Check all relayer health metrics
echo "=== Bridge Relayer Health Check ==="

# 1. Service status
echo "1. Service Status:"
systemctl status x3-relayer --no-pager | grep "Active:"

# 2. Block polling
BLOCKS_1M=$(curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])" | jq '.data.result[0].value[1]')
echo "2. Blocks polled (1min): $BLOCKS_1M blocks/min"
[ $(echo "$BLOCKS_1M > 2" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ❌ TOO LOW"

# 3. Proof submissions
PROOFS_1M=$(curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])" | jq '.data.result[0].value[1]')
echo "3. Proofs submitted (1min): $PROOFS_1M proofs/min"
[ $(echo "$PROOFS_1M > 0.5" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ❌ TOO LOW"

# 4. Pending proofs
PENDING=$(curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count" | jq '.data.result[0].value[1]')
echo "4. Pending proofs: $PENDING"
[ $(echo "$PENDING < 10" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ⚠️  BACKING UP"

# 5. Error rate
ERRORS=$(curl -s "http://localhost:9090/api/v1/query?query=rate(proof_failures_total[5m])" | jq '.data.result[0].value[1]')
echo "5. Error rate (5min): $ERRORS errors/sec"
[ $(echo "$ERRORS < 0.01" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ❌ TOO HIGH"
```

### X3 Runtime Metrics

**Health indicators for X3 consensus:**

```bash
# Check runtime health
echo "=== X3 Runtime Health Check ==="

# 1. Block production
BLOCKS=$(curl -s http://x3-runtime-rpc/consensus/block_rate | jq '.blocks_per_second')
echo "1. Block production: $BLOCKS blocks/sec"
[ $(echo "$BLOCKS > 0.8" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ❌ DEGRADED"

# 2. Validator participation
PARTICIPATION=$(curl -s http://x3-runtime-rpc/consensus/participation | jq '.percentage')
echo "2. Validator participation: $PARTICIPATION%"
[ $(echo "$PARTICIPATION > 65" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ⚠️  LOW"

# 3. Finality time
FINALITY=$(curl -s http://x3-runtime-rpc/consensus/finality_time | jq '.seconds')
echo "3. Finality time: ${FINALITY}s"
[ $(echo "$FINALITY < 30" | bc) -eq 1 ] && echo "   ✅ OK" || echo "   ⚠️  SLOW"

# 4. Network health
PEERS=$(curl -s http://x3-runtime-rpc/network/peers | jq '.count')
echo "4. Connected peers: $PEERS"
[ $PEERS -gt 10 ] && echo "   ✅ OK" || echo "   ⚠️  LOW"
```

---

## Section 7: Dashboard Configuration

### Grafana Dashboard Setup

```json
{
  "dashboard": {
    "title": "X3 Mainnet Performance Baseline",
    "panels": [
      {
        "title": "Block Polling Rate",
        "targets": [{"expr": "rate(blocks_polled_total[1m])"}],
        "thresholds": [
          {"value": 2, "color": "red"},
          {"value": 3, "color": "yellow"},
          {"value": 4, "color": "green"}
        ]
      },
      {
        "title": "Proof Submission Rate",
        "targets": [{"expr": "rate(proofs_submitted_total[1m])"}],
        "thresholds": [
          {"value": 0.5, "color": "red"},
          {"value": 1, "color": "yellow"},
          {"value": 2, "color": "green"}
        ]
      },
      {
        "title": "Pending Proofs",
        "targets": [{"expr": "pending_proofs_count"}],
        "thresholds": [
          {"value": 20, "color": "red"},
          {"value": 10, "color": "yellow"},
          {"value": 5, "color": "green"}
        ]
      },
      {
        "title": "CPU Usage",
        "targets": [{"expr": "process_cpu_seconds_total"}],
        "thresholds": [
          {"value": 70, "color": "red"},
          {"value": 50, "color": "yellow"},
          {"value": 30, "color": "green"}
        ]
      },
      {
        "title": "Memory Usage",
        "targets": [{"expr": "process_resident_memory_bytes"}],
        "thresholds": [
          {"value": 1000000000, "color": "red"},  # 1GB
          {"value": 600000000, "color": "yellow"},  # 600MB
          {"value": 300000000, "color": "green"}  # 300MB
        ]
      }
    ]
  }
}
```

---

## Appendix: Quick Reference

**Performance Summary:**

| Metric | Target | Good | Warning | Critical |
|--------|--------|------|---------|----------|
| Blocks/min | 4-5 | > 3 | 2-3 | < 2 |
| Proofs/min | 2-4 | > 1 | 0.5-1 | < 0.5 |
| Pending proofs | < 5 | < 10 | 10-20 | > 20 |
| CPU % | 50 | < 70 | 70-80 | > 80 |
| Memory MB | 250 | < 500 | 500-1000 | > 1000 |
| RPC latency ms | 100 | < 200 | 200-500 | > 500 |

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-21 | Initial performance baseline |

---

**Questions?** Contact: [sre-team-email]
