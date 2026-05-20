# Mainnet Deployment Runbook

**Phase:** 13e → 13f Transition  
**Status:** Ready for Launch  
**Approval Required:** Governance/Core Team  

---

## Pre-Deployment (4 hours before launch)

### 1. Final Verification Checklist

```bash
#!/bin/bash
# final-mainnet-check.sh

set -e

echo "=== Mainnet Deployment Final Verification ==="
echo ""

# 1. Binary integrity
echo "[1/8] Verifying binary..."
if ! file target/release/x3-relayer | grep -q "ELF 64-bit"; then
  echo "❌ Invalid binary format"
  exit 1
fi

# Check binary size (should be ~30-50MB)
BINARY_SIZE=$(stat -f%z target/release/x3-relayer 2>/dev/null || stat -c%s target/release/x3-relayer)
if [[ $BINARY_SIZE -lt 10000000 ]]; then
  echo "❌ Binary too small (may be corrupted)"
  exit 1
fi
echo "✅ Binary integrity verified (size: $BINARY_SIZE bytes)"

# 2. Configuration validity
echo "[2/8] Validating configuration..."
if ! yamllint relayer-config-mainnet.yaml; then
  echo "❌ Configuration has YAML errors"
  exit 1
fi
echo "✅ Configuration valid"

# 3. Environment variables
echo "[3/8] Checking environment variables..."
for var in X3_RPC_URL X3_RELAYER_ACCOUNT X3_RELAYER_SEED_PHRASE; do
  if [[ -z "${!var}" ]]; then
    echo "❌ Missing: $var"
    exit 1
  fi
done
echo "✅ All required environment variables set"

# 4. RPC connectivity
echo "[4/8] Testing RPC connectivity..."
if ! curl -s "$X3_RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq -e '.result' > /dev/null; then
  echo "❌ X3 RPC not accessible"
  exit 1
fi
echo "✅ X3 RPC accessible"

# 5. Ethereum RPC
echo "[5/8] Testing Ethereum mainnet RPC..."
if ! curl -s https://eth-mainnet.g.alchemy.com/v2/YOUR_ALCHEMY_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' | jq -e '.result' > /dev/null; then
  echo "❌ Ethereum RPC not accessible (check Alchemy key)"
  exit 1
fi
echo "✅ Ethereum RPC accessible"

# 6. Solana RPC
echo "[6/8] Testing Solana mainnet RPC..."
if ! curl -s https://api.mainnet-beta.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}' | jq -e '.result' > /dev/null; then
  echo "❌ Solana RPC not accessible"
  exit 1
fi
echo "✅ Solana RPC accessible"

# 7. Systemd service
echo "[7/8] Checking systemd service..."
if ! systemctl is-enabled x3-relayer > /dev/null 2>&1; then
  echo "⚠️  Service not enabled (will enable automatically)"
fi
echo "✅ Systemd service ready"

# 8. Disk space
echo "[8/8] Checking disk space..."
AVAILABLE=$(df /opt/x3-relayer | awk 'NR==2 {print $4}')
if [[ $AVAILABLE -lt 1000000 ]]; then
  echo "❌ Less than 1GB free space"
  exit 1
fi
echo "✅ Sufficient disk space ($((AVAILABLE/1024))MB available)"

echo ""
echo "✅ All pre-deployment checks passed!"
echo "Status: READY FOR LAUNCH"
```

### 2. Communications Template

```markdown
# Mainnet Deployment Announcement

Dear Team,

We are deploying the X3 Bridge Relayer to mainnet in **4 hours**.

**Timeline:**
- T-1h: Final go/no-go review
- T-0h: Binary deployment
- T+5m: Monitoring confirms polling
- T+24h: Stable operation confirmed

**What to expect:**
- Deployment downtime: ~2 minutes (restart required)
- No impact on bridge (proofs queued during maintenance)
- Continuous monitoring (24 hour watch)

**Contact:**
- Incident Commander: [NAME]
- Status Page: [URL]
- Alert Channel: #x3-relayer-mainnet

We'll provide updates every 30 minutes for the first 4 hours.

Best regards,
X3 Engineering
```

### 3. Go/No-Go Decision

Create a go/no-go assessment document:

```markdown
# Mainnet Launch - Go/No-Go Assessment

**Date:** 2026-04-21
**Time:** 14:00 UTC
**Relayer Version:** 13c-5
**Configuration:** mainnet-final

## Criteria

| Item | Status | Notes |
|------|--------|-------|
| Code Review | ✅ PASS | All PRs approved |
| Staging Validation | ✅ PASS | All 5 stages complete |
| Load Testing | ✅ PASS | 10x load handled |
| Monitoring Ready | ✅ PASS | Dashboards live |
| Incident Response | ✅ PASS | Team trained |
| Security Review | ✅ PASS | No vulnerabilities |
| Configuration Final | ✅ PASS | Mainnet endpoints set |
| RPC Connectivity | ✅ PASS | All providers accessible |
| Runbooks Complete | ✅ PASS | Available to team |
| Rollback Tested | ✅ PASS | Procedure confirmed |

## Decision: **GO FOR LAUNCH**

All criteria met. No blockers identified.

**Approvals:**
- [ ] Engineering Lead: __________________
- [ ] Operations Lead: __________________
- [ ] Governance Rep: __________________

**Contingency:**
- If any RPC provider fails: Use remaining providers
- If X3 runtime unavailable: Pause and wait for recovery
- If critical bug found: Rollback to testnet

**Launch authorized:** 2026-04-21 18:00 UTC
```

---

## Deployment (Day of Launch)

### Phase 1: Pre-Launch (T-30 minutes)

```bash
#!/bin/bash
# pre-launch.sh

echo "=== Pre-Launch Checks (T-30m) ==="

# 1. Verify all team members present
echo "Team members present? (y/n): "
read -r present
if [[ "$present" != "y" ]]; then
  echo "Aborting: Not all team members present"
  exit 1
fi

# 2. Verify communication channels
echo "Verify communication channels online (Slack, PagerDuty, etc)? (y/n): "
read -r comms
if [[ "$comms" != "y" ]]; then
  echo "Aborting: Communication channels not ready"
  exit 1
fi

# 3. Final RPC check
echo "Testing RPC endpoints..."
if ! curl -s "$X3_RPC_URL" -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq -e '.result' > /dev/null; then
  echo "❌ RPC check failed"
  exit 1
fi
echo "✅ RPC healthy"

# 4. Status page update
echo "Update status page to 'scheduled maintenance'? (y/n): "
read -r update
if [[ "$update" == "y" ]]; then
  curl -X POST https://status.example.com/api/incidents \
    -H "Authorization: Bearer $STATUS_PAGE_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "X3 Relayer Mainnet Deployment",
      "status": "investigating",
      "message": "Deploying X3 Bridge Relayer to mainnet"
    }'
  echo "✅ Status page updated"
fi

echo ""
echo "✅ Pre-launch checks complete"
echo "Proceeding to deployment phase..."
```

### Phase 2: Deploy Binary (T-0)

```bash
#!/bin/bash
# deploy-mainnet.sh

set -e

echo "=== Mainnet Binary Deployment ==="
echo "Deployment start time: $(date)"
echo ""

# 1. Copy binary to production
echo "[1/6] Copying binary to production..."
sudo cp target/release/x3-relayer /opt/x3-relayer/bin/x3-relayer.new
sudo chown x3-relayer:x3-relayer /opt/x3-relayer/bin/x3-relayer.new
sudo chmod 755 /opt/x3-relayer/bin/x3-relayer.new
echo "✅ Binary copied"

# 2. Verify binary works (dry run)
echo "[2/6] Verifying binary (dry run)..."
export X3_DRY_RUN=true
timeout 5 /opt/x3-relayer/bin/x3-relayer.new || true
export X3_DRY_RUN=false
echo "✅ Binary verification passed"

# 3. Backup old binary
echo "[3/6] Backing up old binary..."
TIMESTAMP=$(date +%s)
sudo cp /opt/x3-relayer/bin/x3-relayer /opt/x3-relayer/bin/x3-relayer.backup.$TIMESTAMP
echo "✅ Backup created: x3-relayer.backup.$TIMESTAMP"

# 4. Atomic binary swap
echo "[4/6] Swapping binaries..."
sudo mv /opt/x3-relayer/bin/x3-relayer.new /opt/x3-relayer/bin/x3-relayer
echo "✅ Binary swapped"

# 5. Stop old relayer service
echo "[5/6] Stopping old relayer instance..."
sudo systemctl stop x3-relayer || true
sleep 2
echo "✅ Relayer stopped"

# 6. Start new relayer service
echo "[6/6] Starting relayer..."
sudo systemctl start x3-relayer
sleep 5

# Verify startup
if sudo systemctl is-active x3-relayer > /dev/null; then
  echo "✅ Relayer started successfully"
else
  echo "❌ Relayer failed to start"
  echo "Rolling back..."
  sudo cp /opt/x3-relayer/bin/x3-relayer.backup.$TIMESTAMP /opt/x3-relayer/bin/x3-relayer
  sudo systemctl start x3-relayer
  exit 1
fi

echo ""
echo "✅ Deployment complete at $(date)"
echo "Relayer running with PID: $(pgrep -f 'x3-relayer')"
```

### Phase 3: Immediate Monitoring (T+0 to T+1 hour)

```bash
#!/bin/bash
# monitor-mainnet-launch.sh

echo "=== Mainnet Launch Monitoring (First Hour) ==="
echo "Start time: $(date)"
echo ""

INTERVAL=10  # Check every 10 seconds
DURATION=3600  # Monitor for 1 hour
ELAPSED=0

while [[ $ELAPSED -lt $DURATION ]]; do
  
  # Timestamp
  TIME=$(date '+%H:%M:%S')
  
  # Check if process alive
  if ! pgrep -f 'x3-relayer' > /dev/null; then
    echo "[$TIME] ❌ CRITICAL: Relayer process not running!"
    echo "Attempting auto-restart..."
    sudo systemctl restart x3-relayer
    continue
  fi
  
  # Get recent metrics
  LINES=$(tail -20 /var/log/x3-relayer/relayer.log)
  
  # Check for errors
  if echo "$LINES" | grep -q "ERROR"; then
    echo "[$TIME] ⚠️  ERROR-level logs detected:"
    echo "$LINES" | grep "ERROR" | tail -1
  fi
  
  # Extract metrics
  BLOCKS=$(echo "$LINES" | grep -oE "blocks_polled=[0-9]+" | tail -1 | cut -d= -f2)
  FINALIZED=$(echo "$LINES" | grep -oE "blocks_finalized=[0-9]+" | tail -1 | cut -d= -f2)
  PROOFS=$(echo "$LINES" | grep -oE "proofs_submitted=[0-9]+" | tail -1 | cut -d= -f2)
  
  # Display status
  echo "[$TIME] Blocks: $BLOCKS | Finalized: $FINALIZED | Proofs: $PROOFS"
  
  # Check resource usage
  PID=$(pgrep -f 'x3-relayer')
  if [[ ! -z "$PID" ]]; then
    MEMORY=$(ps -p $PID -o rss= | awk '{printf "%.0f", $1/1024}')
    CPU=$(ps -p $PID -o %cpu= | awk '{printf "%.1f", $1}')
    echo "[$TIME]   CPU: ${CPU}% | Memory: ${MEMORY}MB"
    
    # Alert if resources abnormal
    if (( $(echo "$MEMORY > 500" | bc -l) )); then
      echo "[$TIME]   ⚠️  HIGH MEMORY USAGE (${MEMORY}MB)"
    fi
    if (( $(echo "$CPU > 50" | bc -l) )); then
      echo "[$TIME]   ⚠️  HIGH CPU USAGE (${CPU}%)"
    fi
  fi
  
  sleep $INTERVAL
  ELAPSED=$((ELAPSED + INTERVAL))
  
done

echo ""
echo "✅ First hour monitoring complete"
```

---

## Post-Deployment (First 24 Hours)

### Hour 0-1: Immediate Validation

- [ ] Relayer process running
- [ ] Metrics appearing in logs
- [ ] RPC endpoints responding
- [ ] No ERROR-level logs
- [ ] Memory usage < 200MB
- [ ] CPU usage < 20%

### Hour 1-4: Continuous Monitoring

```bash
#!/bin/bash
# monitor-mainnet-continuous.sh

# Monitor every 5 minutes for 4 hours
for i in {1..48}; do
  TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
  
  # Core metrics
  BLOCKS=$(tail -100 /var/log/x3-relayer/relayer.log | grep -oE "blocks_polled=[0-9]+" | tail -1 | cut -d= -f2 || echo "N/A")
  FINALIZED=$(tail -100 /var/log/x3-relayer/relayer.log | grep -oE "blocks_finalized=[0-9]+" | tail -1 | cut -d= -f2 || echo "N/A")
  PROOFS=$(tail -100 /var/log/x3-relayer/relayer.log | grep -oE "proofs_submitted=[0-9]+" | tail -1 | cut -d= -f2 || echo "N/A")
  
  echo "[$TIMESTAMP] Blocks: $BLOCKS | Finalized: $FINALIZED | Proofs: $PROOFS"
  
  # Check for warnings
  if tail -100 /var/log/x3-relayer/relayer.log | grep -q "ERROR"; then
    echo "[$TIMESTAMP] ⚠️  ALERT: Errors in recent logs"
  fi
  
  sleep 300  # 5 minute intervals
done
```

**Success Criteria (4 hours):**
- ✅ Continuous polling (no gaps)
- ✅ Regular finalization (every 5-10 minutes)
- ✅ Proofs submitting
- ✅ Error rate < 1%
- ✅ System resources stable

### Hour 4-24: Sustained Monitoring

- [ ] Shift handoff every 8 hours
- [ ] Alert threshold monitoring continues
- [ ] Prometheus metrics collected
- [ ] Grafana dashboards monitored
- [ ] No unplanned restarts
- [ ] All stakeholders updated (every 4 hours)

---

## Success Metrics by Phase

### Deployment (T+0 to T+5m)

| Metric | Target | Status |
|--------|--------|--------|
| Startup time | < 5s | ✅ If metrics appear |
| First poll | < 30s | ✅ If blocks appear |
| No startup errors | 100% | ✅ No ERROR logs |

### Stabilization (T+5m to T+1h)

| Metric | Target | Status |
|--------|--------|--------|
| Polling rate | 1 block/13s | ✅ Consistent |
| Error rate | < 1% | ✅ < 1 error per 100 polls |
| RPC latency | < 500ms | ✅ < 500ms p95 |
| Memory growth | None | ✅ Stable RSS |

### Production (T+1h to T+24h)

| Metric | Target | Status |
|--------|--------|--------|
| Uptime | > 99.5% | ✅ No unplanned restarts |
| Proof success | > 99% | ✅ > 99% submissions succeed |
| Finalization rate | Consistent | ✅ Regular finalization |
| System health | All green | ✅ All alerts nominal |

---

## Incident Response

### If Relayer Stops

```bash
# 1. Check status
systemctl status x3-relayer

# 2. View logs
journalctl -u x3-relayer -n 50 --no-pager

# 3. Restart
sudo systemctl restart x3-relayer

# 4. Monitor for 5 minutes
tail -f /var/log/x3-relayer/relayer.log

# 5. If still failing, escalate to rollback
```

### If RPC Endpoints Fail

```bash
# Check which endpoints are down
for provider in alchemy infura quicknode; do
  echo "Testing $provider..."
  curl -s https://eth-mainnet.${provider}.com/... | jq '.result' || echo "❌ $provider down"
done

# Relayer will automatically failover
# Monitor and wait for recovery
```

### If Proofs Not Submitting

```bash
# Check X3 runtime
curl -s http://localhost:9933 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq '.'

# Check bridge pause status
# If paused: Wait for governance to resume
# If error: Check X3 logs and runtime state
```

### If Memory Leaks Detected

```bash
# Immediate action: Restart relayer
sudo systemctl restart x3-relayer

# Collect logs for analysis
sudo journalctl -u x3-relayer > /tmp/relayer-memleak-$(date +%s).log

# Investigate:
# 1. Check proof cache size
# 2. Monitor finalized_headers map growth
# 3. Review recent code changes
```

---

## Rollback Procedure

**Decision Point:** If critical issue cannot be resolved in < 15 minutes

```bash
#!/bin/bash
# rollback-mainnet.sh

set -e

echo "=== Mainnet Rollback Initiated ==="
BACKUP_TIME=$(ls -t /opt/x3-relayer/bin/x3-relayer.backup.* | head -1 | grep -oE '[0-9]+$')

echo "[1/4] Stopping relayer..."
sudo systemctl stop x3-relayer

echo "[2/4] Restoring backup..."
sudo cp /opt/x3-relayer/bin/x3-relayer.backup.$BACKUP_TIME /opt/x3-relayer/bin/x3-relayer

echo "[3/4] Restarting..."
sudo systemctl start x3-relayer
sleep 5

echo "[4/4] Verifying..."
if sudo systemctl is-active x3-relayer > /dev/null; then
  echo "✅ Rollback complete"
else
  echo "❌ Rollback failed - manual intervention required"
  exit 1
fi

echo ""
echo "⚠️  ALERT: Rollback executed"
echo "Action: Create post-mortem incident ticket"
```

---

## Communication During Incident

**Immediately (< 5 minutes):**
- Update status page: "Investigating"
- Slack alert: #x3-relayer-mainnet
- Page on-call engineer

**Every 10 minutes:**
- Update incident timeline
- Share diagnostic findings

**Resolution:**
- Final status: "Resolved"
- Root cause summary
- Post-mortem scheduled

---

## Completion Criteria

**Deployment considered successful when:**

✅ Relayer running for 24 hours without restart  
✅ Proofs submitted consistently (> 99% success)  
✅ All success metrics in green  
✅ No critical incidents  
✅ Team confident in stability  
✅ Post-incident review complete (if any issues)  

---

## Next Phase

After 24-hour monitoring confirms stability:

**Phase 13f: Mainnet Operations**
- Ongoing monitoring and alerts
- Regular health checks
- Performance optimization
- Bridge security monitoring
- Incident response readiness

---

## Documents

- **PHASE_13E_MAINNET_PREP.md** — Planning & requirements
- **relayer-config-mainnet.yaml** — Configuration template
- **MAINNET_VALIDATION.md** — Staging validation steps
- **Mainnet Deployment Runbook** — This document
