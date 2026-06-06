# Phase 13f: Mainnet Launch Runbook

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Execution Draft (Operator-Ready After Environment Substitution)  
**Target Audience:** Launch Team, Stakeholders, On-Call Operators

---

## Overview

This runbook provides **complete hour-by-hour guidance** for X3 mainnet launch, extending from T-48 hours through T+1 week post-launch. It integrates with existing Phase 13e documentation and provides Phase 13f-specific execution procedures.

## Current Reality

This document is procedurally complete for planning and rehearsal, but several values remain environment-specific and must be replaced with real production endpoints, credentials, and hostnames before launch execution. It should be treated as a launch template plus operator checklist, not evidence that launch prerequisites are already satisfied.

## Verified

The timeline and checklists are present from T-48h through post-launch stabilization, and the document is cross-linked to deployment, incident response, and RPC failover procedures in the same relayer documentation set.

## Gaps / Risks

Command examples include placeholder variables and example URLs that are not safe for direct production execution without substitution and rehearsal. Team contact rosters, escalation phone bridges, and final go/no-go authority mappings must be confirmed in the active on-call roster before launch day.

## Release Impact

This runbook reduces execution risk for public testnet and mainnet launch windows, but incomplete substitution of environment values can still create launch delays or false-negative health checks.

## Next Required Work

Complete environment substitution, run one tabletop simulation, and run one dry-run rehearsal with the actual launch team and final binary before declaring this runbook execution-ready.

### Quick Reference Timeline
- **T-48h to T-24h:** Readiness assessment and preparation
- **T-24h to T-4h:** Final verification and team positioning
- **T-4h to T-30m:** Pre-deployment procedures (see MAINNET_DEPLOYMENT_RUNBOOK.md)
- **T-30m to T+24h:** Deployment and immediate monitoring (see MAINNET_DEPLOYMENT_RUNBOOK.md)
- **T+24h to T+7d:** Post-launch validation and stabilization

### Document Map
- **PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md** ← You are here (complete timeline)
- **MAINNET_DEPLOYMENT_RUNBOOK.md** (T-30m to T+24h execution procedures)
- **MAINNET_INCIDENT_RESPONSE.md** (Incident playbooks and escalation)
- **RPC_FAILOVER_PROCEDURES.md** (RPC failover and provider management)
- **VALIDATOR_OPERATIONS.md** (Adding/removing validators, key rotation)
- **MAINNET_PERFORMANCE_BASELINE.md** (Expected performance targets)
- **GPU_VALIDATOR_TROUBLESHOOTING.md** (GPU-specific issues and recovery)

### When to Escalate to Other Documents

**Something broken during launch?**
→ Jump to **MAINNET_INCIDENT_RESPONSE.md** (8 detailed incident playbooks)

**RPC provider issues?**
→ See **RPC_FAILOVER_PROCEDURES.md** (failover and provider management)

**Validator not producing blocks?**
→ See **VALIDATOR_OPERATIONS.md** (validator health and recovery)

**GPU validator problems?**
→ See **GPU_VALIDATOR_TROUBLESHOOTING.md** (GPU detection, CUDA, thermal issues)

**Performance degrading?**
→ See **MAINNET_PERFORMANCE_BASELINE.md** (expected metrics, regression detection)

---

## Section 1: Pre-Launch Week (T-48h to T-24h)

### T-48h: Launch Readiness Assessment

**Participants:** Engineering Lead, DevOps Lead, Product Lead

**Duration:** 1-2 hours

#### 1.1 Code and Binary Verification
```bash
# Verify release binary exists and is correct
cd /home/lojak/Desktop/x3-chain-master
cargo build --release --locked 2>&1 | tail -20

# Verify binary signature
sha256sum target/release/x3-relayer
# Compare against: X3_RELEASE_BINARY_HASH.txt

# Run smoke tests
cargo test --release --package x3-relayer -- --nocapture 2>&1 | tail -50
```

**Success Criteria:**
- [ ] Binary builds successfully
- [ ] Hash matches documented release hash
- [x] All tests pass (33/33)
- [ ] Binary size reasonable (~50-150MB)

#### 1.2 Configuration Template Review
```bash
# Verify mainnet config template
ls -lh crates/relayer/relayer-config-mainnet.yaml
wc -l crates/relayer/relayer-config-mainnet.yaml

# Check configuration syntax
cargo run --release -- validate-config crates/relayer/relayer-config-mainnet.yaml 2>&1
```

**Success Criteria:**
- [ ] Config file exists and is readable
- [ ] Config passes validation script
- [ ] All RPC endpoints listed
- [ ] Finality thresholds documented
- [ ] Polling intervals reasonable

#### 1.3 RPC Provider Verification
```bash
# Test connectivity to all primary RPC endpoints
echo "Testing Ethereum (Alchemy)..."
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' 2>&1

echo "Testing Solana (QuickNode)..."
curl -X POST https://api.quicknode.com/solana/mainnet/$QUICKNODE_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}' 2>&1

echo "Testing X3 Runtime..."
# Test connection to X3 mainnet RPC endpoint
```

**Success Criteria:**
- [ ] Ethereum endpoint responds to `eth_blockNumber`
- [ ] Solana endpoint responds to `getSlot`
- [ ] X3 Runtime endpoint responds
- [ ] All responses < 500ms
- [ ] No authentication errors

#### 1.4 Systemd Service Preparation
```bash
# Check systemd service template
cat /etc/systemd/system/x3-relayer.service

# Verify directories exist
sudo mkdir -p /var/log/x3-relayer /var/lib/x3-relayer
sudo chown x3-relayer:x3-relayer /var/log/x3-relayer /var/lib/x3-relayer
sudo chmod 750 /var/log/x3-relayer /var/lib/x3-relayer

# Verify log rotation config
cat /etc/logrotate.d/x3-relayer
```

**Success Criteria:**
- [ ] Service file created and valid
- [ ] Directories created with proper permissions
- [ ] Log rotation configured
- [ ] User account exists

#### 1.5 Environment Variable Inventory
Create `launch-secrets-inventory.txt` (not committed to repo):
```
X3_LOG_LEVEL=debug
X3_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY
X3_RELAYER_ACCOUNT=0x1234567890ABCDEF...
X3_RELAYER_SEED_PHRASE=*** (stored in secure vault)
X3_CONFIG_PATH=/etc/x3-relayer/mainnet.yaml
X3_PROMETHEUS_PORT=9090
X3_SENTRY_DSN=https://...
```

**Success Criteria:**
- [ ] All 5 critical variables documented
- [ ] All values obtained from secure sources
- [ ] Secrets stored in vault, not in config files
- [ ] Backup copy of secrets in secure location (safely encrypted)

#### 1.6 Readiness Decision Point

| Component | Status | Risk Level | Owner |
|-----------|--------|-----------|-------|
| Code | ✅ Tested | 🟢 Low | Engineering |
| Configuration | ✅ Validated | 🟢 Low | DevOps |
| RPC Endpoints | ✅ Connected | 🟢 Low | DevOps |
| Infrastructure | ✅ Ready | 🟢 Low | DevOps |
| Secrets | ✅ Secured | 🟢 Low | Security |
| Team | ✅ Briefed | 🟢 Low | Product |

**Go/No-Go Decision:** PROCEED if all components are ✅ and risk is 🟢

---

### T-24h: Final Verification and Team Briefing

**Participants:** Full Launch Team

**Duration:** 1-2 hours

#### 2.1 Final Binary and Code Verification
```bash
# Last-minute code verification
cd /home/lojak/Desktop/x3-chain-master
git log --oneline -5
git status  # Should be clean

# Re-verify binary
cargo build --release --locked 2>&1 | grep -i "finished\|error"
sha256sum target/release/x3-relayer

# Final test run
cargo test --release --package x3-relayer -- --nocapture --test-threads 1 2>&1 | tail -20
```

#### 2.2 Configuration Dry-Run
```bash
# Load and validate mainnet config
cargo run --release -- \
  --config crates/relayer/relayer-config-mainnet.yaml \
  --dry-run 2>&1 | head -50

# Check for warnings or deprecations
cargo check --release 2>&1 | grep -i "warning\|deprecated"
```

#### 2.3 Pre-Launch Checklist Meeting

**Duration:** 30 minutes

**Agenda:**
1. Review go/no-go status from T-48h
2. Present any changes since T-48h
3. Discuss known risks and mitigations
4. Review communication plan
5. Confirm on-call schedules
6. Review incident response procedures

**Meeting Output:**
- [ ] Launch decision confirmed: **GO**
- [ ] On-call schedule posted (T-24h to T+48h)
- [ ] Escalation numbers confirmed
- [ ] Stakeholder notification list verified
- [ ] Incident response contacts briefed

#### 2.4 Stakeholder Communication Template

**Send this email:**
```
Subject: X3 Mainnet Launch - Final Confirmation (T-24h)

Status: ✅ GO FOR LAUNCH
Timeline: [ISO 8601 timestamp for T-0]
Expected Duration: 2-4 hours for full stability

Key Milestones:
  T-30m: Final verification procedures
  T-0:   Relayer service starts
  T+1h:  Initial stability checks
  T+24h: Extended monitoring and validation

On-Call Contact:
  Lead: support@x3-chain.io
  DevOps: rpc-support@x3.chain
  Engineering: rpc-support@x3.chain

Stakeholder Updates:
  Every 30 minutes during T-30m to T+2h
  Hourly during T+2h to T+24h
  Daily thereafter until T+1w

Expected Metrics:
  - Blocks polled: 1+ per 5 seconds (testnet)
  - Proofs submitted: 10-50 per hour
  - Error rate: < 0.1%

If you experience any issues:
  1. Check https://discord.gg/x3-chain for live operator updates
  2. Email support@x3-chain.io with details
  3. Escalate critical issues to rpc-support@x3.chain

Thank you for your patience!
```

#### 2.5 Team Positioning

**Create Incident Response Team Assignment Table:**

| Role | Owner | Contact Channel | Timezone | On-Call |
|------|------|-----------------|----------|---------|
| Launch Lead | Support Desk | support@x3-chain.io | PT | T-24h to T+48h |
| DevOps Lead | RPC Operations | rpc-support@x3.chain | PT | T-24h to T+48h |
| Engineering Lead | Relayer Engineering | rpc-support@x3.chain | PT | T-24h to T+48h |
| Secondary On-Call | Validator Support | staking-support@x3.chain | ET | T-24h to T+48h |
| Comms Lead | Community Team | https://discord.gg/x3-chain | PT | T-0 to T+6h |

---

## Section 2: Final Preparation (T-24h to T-4h)

### T-24h to T-12h: Document and Infrastructure Verification

**Duration:** 2-4 hours

#### 3.1 Final Documentation Review

```bash
# Verify all launch documents are up to date
for doc in PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md \
           MAINNET_DEPLOYMENT_RUNBOOK.md \
           MAINNET_INCIDENT_RESPONSE.md \
           RPC_FAILOVER_PROCEDURES.md; do
  echo "Checking $doc..."
  wc -l "crates/relayer/$doc"
  grep -c "##" "crates/relayer/$doc" 2>/dev/null || echo "  (reference doc)"
done
```

**Verification Checklist:**
- [ ] PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md exists and is complete
- [ ] MAINNET_DEPLOYMENT_RUNBOOK.md sections accessible
- [ ] MAINNET_INCIDENT_RESPONSE.md covers all 8+ scenarios
- [ ] RPC_FAILOVER_PROCEDURES.md is detailed and actionable
- [ ] All documents cross-referenced correctly
- [ ] No broken links or missing references

#### 3.2 Infrastructure Final Check

```bash
# Verify disk space
df -h /var/log/x3-relayer /var/lib/x3-relayer /

# Check system resources
free -h  # Memory
nproc    # CPU cores

# Verify network connectivity
ping -c 3 eth-mainnet.g.alchemy.com
ping -c 3 api.mainnet.solana.com
ping -c 3 [x3-runtime-endpoint]

# Check DNS resolution
nslookup eth-mainnet.g.alchemy.com
nslookup api.mainnet.solana.com
```

**Success Criteria:**
- [ ] At least 50GB free disk space
- [ ] At least 16GB free memory
- [ ] At least 4 CPU cores available
- [ ] All endpoints reachable
- [ ] No DNS resolution issues

#### 3.3 Monitoring System Readiness

```bash
# Verify Prometheus is running
curl -s http://localhost:9090/-/healthy | head -5

# Verify Grafana is accessible
curl -s http://localhost:3000/api/health

# Create dashboard for launch monitoring
# (Dashboard should show: blocks_polled, proofs_submitted, errors)
```

**Success Criteria:**
- [ ] Prometheus scraping metrics
- [ ] Grafana dashboards accessible
- [ ] Alerts configured for key metrics
- [ ] Alert channels tested (email, Slack, PagerDuty)

#### 3.4 Backup and Recovery Verification

```bash
# Create snapshot of current production state
sudo mkdir -p /backup/x3-relayer-pre-launch
sudo cp -r /var/lib/x3-relayer /backup/x3-relayer-pre-launch/
sudo cp /etc/x3-relayer/mainnet.yaml /backup/x3-relayer-pre-launch/

# Verify rollback binary is available
ls -lh target/release/x3-relayer
cp target/release/x3-relayer /backup/x3-relayer-pre-launch/

# Document configuration for rollback
git log --oneline -1 > /backup/x3-relayer-pre-launch/ROLLBACK_INFO.txt
date >> /backup/x3-relayer-pre-launch/ROLLBACK_INFO.txt
```

**Success Criteria:**
- [ ] Pre-launch state backed up
- [ ] Rollback binary stored
- [ ] Rollback procedure documented
- [ ] Recovery tested (restore from backup)

### T-12h to T-4h: Final RPC Validation and Network Tests

**Duration:** 1-2 hours

#### 4.1 RPC Endpoint Health Check

```bash
# Run comprehensive RPC validation
cargo run --release -- \
  --config crates/relayer/relayer-config-mainnet.yaml \
  --validate-rpc-health \
  --timeout 30 2>&1
```

**Test Each Provider:**

**Ethereum (Primary: Alchemy, Secondary: Infura, Tertiary: QuickNode)**
```bash
# Latest block number
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' 2>&1

# Current gas price
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_gasPrice","params":[],"id":1}' 2>&1

# Recent block headers (finality check)
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' 2>&1
```

**Solana (Primary: QuickNode, Secondary: Helius, Tertiary: Triton)**
```bash
# Latest slot
curl -X POST https://api.quicknode.com/solana/mainnet/$QUICKNODE_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}' 2>&1

# Cluster info
curl -X POST https://api.quicknode.com/solana/mainnet/$QUICKNODE_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getClusterNodes","params":[],"id":1}' 2>&1

# Recent blockhash
curl -X POST https://api.quicknode.com/solana/mainnet/$QUICKNODE_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getLatestBlockhash","params":[],"id":1}' 2>&1
```

**Success Criteria:**
- [ ] All primary RPC endpoints responding
- [ ] All secondary RPC endpoints responding (for failover validation)
- [ ] All tertiary RPC endpoints responding (for degraded mode)
- [ ] Response times < 500ms
- [ ] No auth errors or rate limiting
- [ ] Block/slot numbers current (< 30 seconds old)

#### 4.2 Latency Baseline Establishment

Record latencies for comparison during launch:

```bash
# Run 10 requests to each endpoint and record latencies
for i in {1..10}; do
  time curl -s -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":'$i'}' \
    > /dev/null 2>&1
done

# Calculate baseline
# Expected: p50 < 200ms, p99 < 500ms
```

**Expected Baselines (Record these):**

| Endpoint | P50 Latency | P95 Latency | P99 Latency |
|----------|-------------|-------------|-------------|
| Alchemy | ___ ms | ___ ms | ___ ms |
| Infura | ___ ms | ___ ms | ___ ms |
| QuickNode (EVM) | ___ ms | ___ ms | ___ ms |
| QuickNode (SOL) | ___ ms | ___ ms | ___ ms |
| Helius | ___ ms | ___ ms | ___ ms |
| X3 Runtime | ___ ms | ___ ms | ___ ms |

#### 4.3 Network Partition Simulation (Optional but Recommended)

**Warning:** Only do this if you have a staging environment separate from production.

```bash
# Simulate RPC endpoint becoming unavailable
# (Only in staging - DO NOT do on production network!)

# 1. Stop Alchemy endpoint access (via firewall rule)
# 2. Verify relayer failsover to Infura
# 3. Stop Infura endpoint access
# 4. Verify relayer failovers to QuickNode
# 5. Restore connectivity and verify recovery

# See RPC_FAILOVER_PROCEDURES.md for detailed testing procedures
```

#### 4.4 Pre-Launch Team Standup

**Duration:** 15-30 minutes

**Agenda:**
1. Report RPC health status
2. Report baseline latencies
3. Confirm on-call assignments
4. Review incident response procedures
5. Confirm communication channels (Slack, Email, Phone)
6. Final weather check (no unforeseen issues)

**Decision Point:**
- **GO**: Proceed with launch as planned
- **HOLD**: Delay launch by 2 hours
- **ABORT**: Postpone launch pending investigation

**Expected Decision:** GO (assuming all checks pass)

---

## Section 3: Launch Day Preparation (T-4h to T-30m)

For detailed T-4h to T+24h procedures, see: **MAINNET_DEPLOYMENT_RUNBOOK.md**

**Summary Timeline:**
- **T-4h:** Pre-deployment procedures begin
- **T-3h:** Final verification checklist
- **T-2h:** Binary verification and staging
- **T-1h:** Configuration validation and systemd preparation
- **T-30m:** Final go/no-go decision and stakeholder notification

---

## Section 4: Post-Launch Validation (T+24h to T+7d)

### T+24h: Extended System Validation

**Duration:** 2-4 hours

#### 5.1 Relayer Health Status

```bash
# Check relayer logs for errors
sudo tail -500 /var/log/x3-relayer/relayer.log | grep -i "error\|warn" | tail -20

# Verify metrics are being exported
curl -s http://localhost:9090/api/v1/query?query=up{job%3D%22x3-relayer%22}

# Check key metrics over the last 24 hours
curl -s "http://localhost:9090/api/v1/query_range?query=blocks_polled&start=$(($(date +%s)-86400))&end=$(date +%s)&step=300"

# Verify no memory leaks
free -h
ps aux | grep x3-relayer | grep -v grep
```

**Success Criteria:**
- [ ] Relayer service running continuously
- [ ] No crashes or restarts in last 24 hours
- [ ] Memory usage stable (not growing)
- [ ] CPU usage < 50% average
- [ ] No error rate spikes

#### 5.2 Proof Submission Validation

```bash
# Check proofs submitted in last 24 hours
curl -s "http://localhost:9090/api/v1/query?query=increase(proofs_submitted_total[24h])"

# Expected: 100-1000 proofs depending on activity
# If < 50: Investigate (possibly not enough bridge activity)
# If > 2000: Investigate (possibly duplicate submissions)

# Check proof success rate
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_failed_total[24h])"

# Expected: < 0.1% failure rate
```

**Success Criteria:**
- [ ] Proofs being submitted regularly
- [ ] Success rate > 99%
- [ ] No duplicate proof submissions
- [ ] All proofs confirmed on-chain

#### 5.3 RPC Provider Monitoring

```bash
# Check RPC failure count
curl -s "http://localhost:9090/api/v1/query?query=increase(rpc_failures_total[24h])"

# Expected: < 5 failures in 24 hours
# If > 10: Investigate provider issues and consider failover

# Verify failover procedures are working
curl -s "http://localhost:9090/api/v1/query?query=rpc_failover_count"

# Expected: 0 (no failovers needed)
# If > 0: Verify failover worked correctly
```

**Success Criteria:**
- [ ] < 5 RPC failures in 24 hours
- [ ] No provider outages lasting > 5 minutes
- [ ] Failover procedures working (if tested)
- [ ] Primary provider maintains 99%+ uptime

#### 5.4 Stakeholder Communication

Send daily status updates to stakeholders:

```
Subject: X3 Mainnet Launch - 24-Hour Status Report ✅

Timeline: [Launch timestamp + 24h]
Status: STABLE ✅

Metrics Summary:
  Uptime: 99.5%
  Blocks Polled: >= 17280 in last 24h
  Proofs Submitted: 240-1200 (success rate: 99.9%)
  RPC Provider: Healthy (5 failures, all < 1s)

Key Achievements:
  ✅ Relayer running stably for 24 hours
  ✅ All proofs submitting successfully
  ✅ No critical incidents
  ✅ Performance within expected ranges

Upcoming Milestones:
  T+48h: Extended validation
  T+72h: Performance analysis
  T+7d: Mainnet launch complete ✅

On-Call Status:
  Continue monitoring through T+7d
  Escalate any anomalies to rpc-support@x3.chain

Questions or concerns?
  Contact: support@x3-chain.io or rpc-support@x3.chain
```

### T+2d to T+7d: Post-Launch Monitoring and Stabilization

#### 5.5 Daily Monitoring Schedule

**Every 24 hours (or every 8 hours if in critical monitoring phase):**

```bash
# Generate daily health report
cat << 'EOF' > /tmp/daily-health-check.sh
#!/bin/bash

echo "=== X3 Relayer Daily Health Check ==="
echo "Timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

echo "1. Service Status"
systemctl status x3-relayer --no-pager | head -10
echo

echo "2. Recent Errors (Last 24h)"
sudo journalctl -u x3-relayer --since "24 hours ago" | grep -i "error" | wc -l
echo

echo "3. Memory Usage"
ps aux | grep x3-relayer | grep -v grep | awk '{print $6}' | numfmt --to=iec
echo

echo "4. Proof Metrics (Last 24h)"
curl -s "http://localhost:9090/api/v1/query?query=increase(proofs_submitted_total[24h])"
echo

echo "5. RPC Failures (Last 24h)"
curl -s "http://localhost:9090/api/v1/query?query=increase(rpc_failures_total[24h])"
EOF

chmod +x /tmp/daily-health-check.sh
bash /tmp/daily-health-check.sh
```

#### 5.6 Performance Baseline Validation

Compare against expected baselines from MAINNET_PERFORMANCE_BASELINE.md:

| Metric | Expected | Actual | Status |
|--------|----------|--------|--------|
| TPS | 50-100 | ___ | ✅/⚠️/❌ |
| Latency (p99) | < 500ms | ___ | ✅/⚠️/❌ |
| Relayer Latency | < 1s | ___ | ✅/⚠️/❌ |
| Memory Usage | < 500MB | ___ | ✅/⚠️/❌ |
| CPU Usage | < 50% | ___ | ✅/⚠️/❌ |
| Uptime | > 99% | ___ | ✅/⚠️/❌ |

#### 5.7 Known Issues and Mitigations

**Issue #1: Occasional RPC Timeouts**
- **Expected Frequency:** 1-2 per day
- **Duration:** < 1 second
- **Impact:** Relayer automatically retries
- **Mitigation:** Monitor for increase in frequency
- **Escalation:** If > 5 per hour, contact RPC provider

**Issue #2: Proof Submission Delays**
- **Expected:** < 1 second in normal operation
- **During Congestion:** < 10 seconds
- **Mitigation:** Exponential backoff with max retries
- **Escalation:** If > 100 proofs delayed for > 1 hour

**Issue #3: Memory Gradual Growth**
- **Normal Pattern:** Stable ± 5% over 7 days
- **Warning Level:** Growing > 10% per day
- **Critical Level:** Growing > 50% per day
- **Mitigation:** Restart relayer service
- **Investigation:** Check for memory leaks (see GPU_VALIDATOR_TROUBLESHOOTING.md)

#### 5.8 Post-Launch Success Criteria

Mark launch as **SUCCESSFUL** when:

- [ ] Relayer has run for 7 days with > 99% uptime
- [ ] > 1000 proofs submitted with > 99% success rate
- [ ] No critical incidents requiring rollback
- [ ] RPC providers stable with < 10 failures total
- [ ] Memory and CPU usage stable
- [ ] No security incidents or vulnerability reports
- [ ] All stakeholders satisfied with performance
- [ ] Documentation complete and accurate

**Launch Declaration:**
```
On [DATE], X3 mainnet launched successfully.

Final Statistics:
  - Uptime: 99.X%
  - Proofs Submitted: X,XXX
  - Success Rate: 99.X%
  - Critical Incidents: 0

Status: PRODUCTION READY ✅
```

---

## Section 5: Incident Response During Launch

For comprehensive incident playbooks, see: **MAINNET_INCIDENT_RESPONSE.md**

### Quick Escalation Matrix

| Severity | Response Time | Owner | Escalation |
|----------|---------------|-------|-----------|
| 🟢 Low | < 1 hour | On-Call Eng | Dev Lead |
| 🟡 Medium | < 15 minutes | On-Call Eng | Engineering Lead |
| 🔴 Critical | Immediate | Launch Lead | VP Engineering |
| 🟣 Emergency | Immediate | All Hands | CEO Notification |

### Common Incidents During Launch

**Incident:** Relayer service crashes
- See: MAINNET_INCIDENT_RESPONSE.md → Incident #1
- Escalation: Contact Engineering Lead immediately
- Expected resolution: < 15 minutes
- Comms: Notify stakeholders every 5 minutes

**Incident:** RPC provider down (single)
- See: MAINNET_INCIDENT_RESPONSE.md → Incident #2
- Response: Automatic failover to backup provider
- Escalation: Contact DevOps if failover fails
- Comms: Notify stakeholders if downtime > 1 minute

**Incident:** Multiple RPC providers down
- See: MAINNET_INCIDENT_RESPONSE.md → Incident #3
- Response: See RPC_FAILOVER_PROCEDURES.md
- Escalation: Contact RPC providers + Engineering Lead
- Comms: Immediate stakeholder notification

**Incident:** Bridge paused
- See: MAINNET_INCIDENT_RESPONSE.md → Incident #4
- Response: Relayer queues proofs, waits for resume
- Escalation: Contact Governance Team
- Comms: Stakeholder notification with resume timeline

For 8+ additional incidents and detailed playbooks, see **MAINNET_INCIDENT_RESPONSE.md**.

---

## Section 6: Rollback Procedures

### When to Rollback

Rollback to previous version if:
- [ ] Critical bug found in relayer code
- [ ] RPC connectivity permanently lost
- [ ] Bridge contract halted due to security issue
- [ ] Consensus failure preventing finalization
- [ ] Cascading failures affecting > 10 proofs

**Do NOT rollback for:**
- Single RPC timeout (will self-correct)
- Brief memory spike (will stabilize)
- Occasional proof failure (within acceptable range)
- Slow proof submission (adjust parameters instead)

### Rollback Steps

See **MAINNET_DEPLOYMENT_RUNBOOK.md** → Rollback Procedures

**Quick Summary:**
1. Stop relayer service: `sudo systemctl stop x3-relayer`
2. Restore previous binary: `cp /backup/x3-relayer-pre-launch/x3-relayer target/release/`
3. Clear state (optional): `sudo rm -rf /var/lib/x3-relayer/headers.db`
4. Restart service: `sudo systemctl start x3-relayer`
5. Verify: `sudo journalctl -u x3-relayer -f`

**Decision Point:** Rollback decision made by: **Launch Lead + Engineering Lead**

**Comms Template:**
```
Subject: X3 Mainnet - Rollback to Previous Version

We have initiated a rollback to ensure network stability.

Timeline:
  Rollback initiated: [timestamp]
  Previous version deployed: [version]
  Expected recovery time: 15 minutes

What happened:
  [Brief explanation of issue]

What we're doing:
  [Explanation of rollback procedure]

Next steps:
  - We'll investigate the issue
  - We'll prepare a fixed version
  - We'll re-launch when ready

Estimated re-launch: [time]

Thank you for your patience.
```

---

## Section 7: Documentation and Runbooks

### Complete Documentation Map

```
Phase 13 Delivery
├── PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md
│   ├── T-48h to T-24h: Readiness & Preparation
│   ├── T-24h to T-4h: Final Verification
│   ├── T-4h to T-30m: Pre-Deployment (references MAINNET_DEPLOYMENT_RUNBOOK)
│   ├── T-30m to T+24h: Deployment (references MAINNET_DEPLOYMENT_RUNBOOK)
│   └── T+24h to T+7d: Post-Launch Validation
│
├── MAINNET_DEPLOYMENT_RUNBOOK.md (Phase 13e)
│   ├── T-4h: Pre-deployment checklist
│   ├── T-30m to T+0: Deployment procedures
│   ├── T+0 to T+24h: Monitoring procedures
│   ├── 4 basic incident scenarios
│   └── Rollback procedure
│
├── MAINNET_INCIDENT_RESPONSE.md
│   ├── 8+ comprehensive incident playbooks
│   ├── Detection procedures
│   ├── Recovery steps
│   └── Escalation procedures
│
├── RPC_FAILOVER_PROCEDURES.md
│   ├── Provider architecture
│   ├── Failover detection
│   ├── Manual & automatic failover
│   └── Testing procedures
│
├── VALIDATOR_OPERATIONS.md
│   ├── Adding validators
│   ├── Key rotation
│   ├── Slashing recovery
│   └── Rewards management
│
└── MAINNET_PERFORMANCE_BASELINE.md
    ├── Expected TPS
    ├── Latency targets
    ├── Resource utilization
    └── Capacity headroom
```

### How to Use This Runbook

**Before Launch (T-48h to T-0):**
1. Read sections 1 & 2 in order
2. Complete all checklists
3. Make go/no-go decisions
4. Brief your team

**During Launch (T-0 to T+24h):**
1. Reference MAINNET_DEPLOYMENT_RUNBOOK.md
2. Use MAINNET_INCIDENT_RESPONSE.md if issues arise
3. Update stakeholders hourly

**After Launch (T+24h to T+7d):**
1. Use section 4 for daily validation
2. Reference VALIDATOR_OPERATIONS.md if adding validators
3. Compare metrics against MAINNET_PERFORMANCE_BASELINE.md

---

## Section 8: Contact Information and Escalation

### Launch Team Contact Matrix

**Primary Launch Lead:**
- Owner: Support Desk
- Primary channel: support@x3-chain.io
- Secondary channel: rpc-support@x3.chain
- Availability: T-24h to T+48h

**DevOps Lead:**
- Owner: RPC Operations
- Primary channel: rpc-support@x3.chain
- Secondary channel: support@x3-chain.io
- Availability: T-24h to T+48h

**Engineering Lead:**
- Owner: Relayer Engineering
- Primary channel: rpc-support@x3.chain
- Secondary channel: support@x3-chain.io
- Availability: T-24h to T+48h

**VP Engineering (Escalation):**
- Escalation alias: rpc-support@x3.chain
- Executive notification path: support@x3-chain.io

**RPC Provider Primary Contacts:**

**Alchemy (Ethereum):**
- Contact path: Alchemy support portal ticket
- Support Portal: https://alchemy.com/support

**Infura (Ethereum Backup):**
- Contact path: Infura support portal ticket
- Support Portal: https://infura.io/support

**QuickNode (Both chains):**
- Contact path: QuickNode support portal ticket
- Support Portal: https://quicknode.com/support

**Stakeholder Notification List:**
- General support: support@x3-chain.io
- RPC operations: rpc-support@x3.chain
- Validator operations: staking-support@x3.chain
- Community announcements: https://discord.gg/x3-chain

### Escalation Decision Tree

```
Problem Occurs
    ↓
Is it critical? (> 50% uptime loss, security issue)
    ├─ YES → Call Launch Lead immediately
    │         ├─ Assess impact
    │         ├─ Activate incident response
    │         └─ Notify VP Engineering
    │
    └─ NO → Log issue + notify on-call engineer
            ├─ Can it wait?
            │  ├─ YES → Schedule for next check-in
            │  └─ NO → Escalate to Engineering Lead
            └─ Is it resolved?
               ├─ YES → Update status
               └─ NO → Escalate to VP Engineering
```

---

## Appendix A: Pre-Launch Checklist (Full)

### T-48 Hours
- [ ] Code builds successfully with no errors
- [x] All tests pass (33/33)
- [ ] Binary signature verified
- [ ] Configuration template valid
- [ ] All RPC endpoints reachable
- [ ] Systemd service prepared
- [ ] Environment variables documented
- [ ] Secrets secured in vault
- [ ] On-call schedule finalized
- [ ] Team briefed on timeline

### T-24 Hours
- [ ] Binary verified (re-check)
- [ ] Configuration dry-run successful
- [ ] Pre-launch meeting completed
- [ ] Go/no-go decision: **GO**
- [ ] Stakeholders notified
- [ ] On-call team in position
- [ ] Incident response briefed
- [ ] Documentation reviewed
- [ ] Backup systems tested
- [ ] Rollback procedure verified

### T-12 Hours
- [ ] All documents complete and cross-referenced
- [ ] Infrastructure final check passed
- [ ] Monitoring systems operational
- [ ] Pre-launch state backed up
- [ ] Rollback binary stored
- [ ] RPC endpoints all healthy
- [ ] Latency baselines recorded
- [ ] Network partition test completed (if applicable)
- [ ] Team standup completed
- [ ] Final go/no-go: **GO**

### T-4 Hours to T-30m
- [ ] See MAINNET_DEPLOYMENT_RUNBOOK.md

---

## Appendix B: Post-Launch Checklist (T+24h to T+7d)

- [ ] Relayer uptime > 99%
- [ ] Proofs submitted > 100
- [ ] Proof success rate > 99%
- [ ] Memory usage stable
- [ ] CPU usage < 50% average
- [ ] No critical errors in logs
- [ ] RPC endpoints stable
- [ ] Failover procedures working (if tested)
- [ ] Stakeholder updates sent daily
- [ ] No security incidents
- [ ] Performance baselines met
- [ ] Documentation accurate and complete
- [ ] Day-7 success criteria met → **LAUNCH SUCCESSFUL** ✅

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-21 | Initial Phase 13f launch runbook |

---

**Next Steps:**
1. Print this runbook and bind with MAINNET_DEPLOYMENT_RUNBOOK.md
2. Distribute to launch team
3. Schedule T-48h readiness review
4. Begin execution at T-48h

**Questions or feedback?** Contact: rpc-support@x3.chain
