# Mainnet Incident Response Playbooks

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Operational Draft (Requires Team Drill and Contact Validation)  
**Target Audience:** On-Call Engineers, Launch Team, SREs

---

## Overview

This document provides **detailed incident response playbooks** for 8+ production scenarios that may occur during X3 mainnet launch and operations. Each playbook includes: detection procedures, root cause analysis, recovery steps, verification procedures, and escalation paths.

## Current Reality

The incident scenarios and response procedures are documented in depth, but execution quality still depends on live telemetry, tested runbook ownership, and current responder contact data. The procedures are usable now for drills and rehearsals.

## Verified

Eight major incident classes are covered with detection, diagnosis, and recovery flow, and the document links to adjacent launch and failover procedures for coordinated response.

## Gaps / Risks

Per-environment values, paging targets, and escalation contacts can drift over time and are not automatically validated by this document. If the on-call map is stale, response speed degrades even when the technical steps are correct.

## Release Impact

This document lowers mean time to recovery during public testnet and mainnet windows, but stale contact mappings or unpracticed handoffs can still cause prolonged incidents.

## Next Required Work

Run a multi-incident tabletop exercise, confirm current on-call routing and backup responders, then record any timing or ownership corrections directly in this file.

### Quick Reference

| Incident | Severity | Response Time | Expected Resolution |
|----------|----------|---------------|-------------------|
| Relayer Crash | 🔴 Critical | Immediate | < 15 min |
| Single RPC Down | 🟡 Medium | < 5 min | < 1 min (automatic failover) |
| Multiple RPC Down | 🔴 Critical | Immediate | 5-30 min |
| Bridge Paused | 🟡 Medium | < 5 min | 1 hour (governance dependent) |
| X3 Runtime Error | 🟡 Medium | < 10 min | 5-30 min |
| Proof Submission Fail | 🟡 Medium | < 5 min | 5-15 min |
| Memory Leak | 🟡 Medium | < 10 min | 5 min (restart) |
| Network Partition | 🔴 Critical | Immediate | 10-60 min |

### Related Documents

**For timeline context:** See **PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md** (what should be happening at T+Xh)

**For RPC-specific issues:** See **RPC_FAILOVER_PROCEDURES.md** (Incident #2-3 involve RPC failover)

**For validator recovery:** See **VALIDATOR_OPERATIONS.md** (slashing recovery, key rotation)

**For GPU issues:** See **GPU_VALIDATOR_TROUBLESHOOTING.md** (hardware failures in production)

**For performance concerns:** See **MAINNET_PERFORMANCE_BASELINE.md** (establishing what "normal" looks like)

---

## Incident #1: Relayer Service Crash

**See also:** PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md (timeline context), MAINNET_PERFORMANCE_BASELINE.md (expected metrics before crash)

### Detection

**Automatic Alerts:**
- Prometheus alert: `relayer_up == 0` for > 30 seconds
- Systemd alert: Service stopped
- Pagerduty: Critical alert triggered

**Manual Detection:**
```bash
# Check if service is running
sudo systemctl status x3-relayer

# Expected output:
# ● x3-relayer.service - X3 Bridge Relayer
#    Loaded: loaded
#    Active: active (running)  ← Should see this

# If not running, check logs immediately
sudo journalctl -u x3-relayer -n 50 --no-pager
```

**Symptoms:**
- Service not listed in `systemctl status`
- Prometheus metric `up{job="x3-relayer"}` = 0
- No new blocks in logs for > 1 minute
- Memory/CPU spikes in monitoring system

### Severity Assessment

- **CRITICAL** if: Crash during peak transaction volume or multiple successive crashes
- **MEDIUM** if: Single crash with automatic recovery available
- **LOW** if: Crash during maintenance window with no pending proofs

### Root Cause Analysis

#### Step 1: Examine Crash Logs (First 2 minutes)

```bash
# Get last 100 lines of systemd journal
sudo journalctl -u x3-relayer -n 100 --no-pager > /tmp/relayer-crash.log

# Look for panic/crash indicators
grep -i "panic\|segfault\|thread\|crashed\|abort" /tmp/relayer-crash.log

# Get stack trace if available
sudo journalctl -u x3-relayer --no-pager | tail -50
```

**Common Crash Patterns:**

Pattern A: Out of Memory
```
Error: Cannot allocate memory
Signal: SIGKILL (9)
Last message: Allocating proof_cache
```
→ **Action:** Increase system memory or reduce cache size

Pattern B: Panic in Code
```
thread 'tokio-runtime-worker' panicked at
'index out of bounds'
```
→ **Action:** Review recent code changes, escalate to Engineering

Pattern C: Stack Overflow
```
runtime error: stack overflow
Stack exhaustion in async handler
```
→ **Action:** Reduce recursion depth, simplify async chains

Pattern D: SEGFAULT
```
Signal: SIGSEGV (11)
Memory address: 0x0
```
→ **Action:** Memory corruption - escalate immediately

#### Step 2: Check System Resources (During Crash)

```bash
# Check memory pressure
free -h
# If available < 1GB: Memory pressure is likely cause

# Check disk space
df -h /var/lib/x3-relayer
# If < 100MB free: Disk full is likely cause

# Check CPU overload
top -b -n 1 | head -20
# If load > num_cores: CPU exhaustion is likely cause

# Check for file descriptor limits
ulimit -n
# Typical: 65536, if full processes can't start
```

#### Step 3: Examine Recent Changes

```bash
# Check if code was recently updated
git log --oneline -10

# Check git diff from last known good state
git diff HEAD~1 HEAD

# Check for warnings during compilation
cargo check 2>&1 | grep -i "warning\|deprecated"
```

#### Step 4: Check External Dependencies

```bash
# Verify RPC endpoints are reachable
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | .labels.job'

# Check if any RPC endpoints are blocking
curl -I https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY

# Verify system networking
netstat -an | grep :9090 | wc -l
```

### Recovery Steps

#### Immediate Recovery (< 5 minutes)

**Step 1: Attempt automatic service restart**
```bash
# Systemd is configured to restart on failure
# Service should recover automatically within 30 seconds

# Verify restart
sleep 5
sudo systemctl status x3-relayer

# If still not running, proceed to Step 2
```

**Step 2: Manual restart**
```bash
# If automatic restart failed
sudo systemctl restart x3-relayer

# Monitor startup
sudo journalctl -u x3-relayer -f
```

**Step 3: Check startup success**
```bash
# Should see these messages in order:
# "Loading configuration..."
# "Validating configuration..."
# "Connecting to RPC endpoints..."
# "Starting main relay loop..."
# "Ready for events" or similar

# Wait for 1 minute of successful operation
sleep 60
curl -s http://localhost:9090/api/v1/query?query=up{job%3D%22x3-relayer%22}
# Should return: {"value": [xxx, "1"]}
```

#### Intermediate Recovery (5-15 minutes)

**If restart fails, try soft reset:**
```bash
# Stop service
sudo systemctl stop x3-relayer

# Clear any stuck state
sudo rm -f /var/lib/x3-relayer/*.db.lock
sudo rm -f /var/run/x3-relayer.pid

# Restart
sudo systemctl start x3-relayer

# Monitor for 5 minutes
sudo journalctl -u x3-relayer -f &
sleep 300
```

#### Full Recovery (15-30 minutes)

**If soft reset fails, try full reset:**
```bash
# CAUTION: This clears internal state but not proofs

# Stop service
sudo systemctl stop x3-relayer

# Backup current state
sudo cp -r /var/lib/x3-relayer /var/lib/x3-relayer.backup.$(date +%s)

# Clear state
sudo rm -rf /var/lib/x3-relayer/*

# Restart service
sudo systemctl start x3-relayer

# Monitor carefully for 10 minutes
sudo journalctl -u x3-relayer -f
```

**Monitoring During Recovery:**
```bash
# Watch for these metrics to return
watch -n 5 'curl -s http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])'
# Should see blocks_polled > 0

watch -n 5 'curl -s http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])'
# Should see proofs_submitted > 0 (if bridge active)
```

### Verification

#### Is the relayer back online?

```bash
# Check 1: Service is running
sudo systemctl status x3-relayer | grep "Active: active"

# Check 2: Responding to requests
curl -s http://localhost:9090/-/healthy

# Check 3: Processing blocks
curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])" | grep "value"

# Check 4: No recent errors in logs
sudo journalctl -u x3-relayer -n 50 | grep -i "error" | tail -5
```

#### Expected behavior post-recovery:

- [ ] Service status shows "active (running)"
- [ ] HTTP health check responds 200 OK
- [ ] blocks_polled metric > 0 (increasing)
- [ ] No critical errors in logs
- [ ] Monitoring dashboard shows green

### Escalation

**If crash repeats:**
1. Contact Engineering Lead immediately
2. Stop relayer: `sudo systemctl stop x3-relayer`
3. Preserve logs: `sudo journalctl -u x3-relayer > /tmp/full-logs.txt`
4. Document timeline and error messages
5. Escalate to VP Engineering for code review

**If system resources exhausted:**
1. Contact DevOps Lead immediately
2. Provision additional resources
3. Re-enable service

### Communication Template

```
[Immediately upon detection]
Subject: X3 Relayer - Service Alert

Alert: Relayer service crashed at [timestamp]
Impact: No proofs submitting for 5-15 minutes
Status: Recovery in progress

We are:
1. Analyzing crash logs
2. Attempting service restart
3. Monitoring recovery

Updates: Every 5 minutes until resolved

[After recovery]
Subject: X3 Relayer - Service Recovered ✅

Alert: Service recovered at [timestamp]
Root cause: [description]
Duration: 12 minutes
Proofs submitted since recovery: 18

Root Cause Analysis:
[Explanation]

Prevention:
[What we'll do to prevent this]

Apologies for the interruption.
```

---

## Incident #2: Single RPC Provider Down

**See also:** RPC_FAILOVER_PROCEDURES.md (detailed failover procedures, manual steps, testing)

### Detection

**Automatic Alerts:**
- Alert: `rpc_provider_down{provider="alchemy"}` > 0
- Metrics: Provider response time spike or connection errors
- Logs: "Connection refused" or timeout errors

**Manual Detection:**
```bash
# Test specific provider
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  --max-time 5

# Expected: JSON response with block number
# If failed: Connection timeout or error response
```

### Root Cause Analysis

```bash
# Check 1: Provider status page
echo "Checking provider status..."
curl -s https://status.alchemy.com/ | grep -i "incident\|down\|degraded"

# Check 2: Network connectivity from relayer
sudo systemctl status x3-relayer | grep -A 10 "Status:"

# Check 3: Recent error logs
sudo journalctl -u x3-relayer -n 100 | grep -i "alchemy\|timeout\|connection"

# Check 4: RPC error type
sudo journalctl -u x3-relayer -n 20 | grep "error"
```

**Common Causes:**
- Provider maintenance (check status page)
- Provider experiencing traffic spike (temporary)
- Network connectivity issue (ISP, firewall)
- API key rate limit exceeded (check quota)
- API key expired or invalid (check dashboard)

### Recovery Steps

#### Automatic Failover (Usually 0-5 seconds)

Relayer is configured to automatically failover:
1. Alchemy (primary) → Infura (secondary) → QuickNode (tertiary)
2. Failover triggered after 3 consecutive failures
3. Proofs queued during failover, submitted when connection restored

**Verify failover occurred:**
```bash
# Check metrics
curl -s "http://localhost:9090/api/v1/query?query=rpc_failover_count"

# Should show failover count increased by 1

# Check which provider is active now
curl -s "http://localhost:9090/api/v1/query?query=active_rpc_provider"

# Should show: "infura" (secondary provider)
```

#### Manual Failover (If automatic fails)

```bash
# Step 1: Verify provider really is down
curl -I https://eth-mainnet.g.alchemy.com/health

# Step 2: Edit config to disable Alchemy temporarily
sudo nano /etc/x3-relayer/mainnet.yaml
# Find: eth_providers: [alchemy, infura, quicknode]
# Change to: eth_providers: [infura, quicknode]

# Step 3: Restart relayer
sudo systemctl restart x3-relayer

# Step 4: Verify using backup provider
sleep 10
curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])"
# Should show blocks still being polled
```

#### Restore Primary Provider

```bash
# When provider is back online (check status page)

# Step 1: Verify provider is responding again
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  --max-time 5

# Expected: Valid response with current block

# Step 2: Re-enable provider in config
sudo nano /etc/x3-relayer/mainnet.yaml
# Find: eth_providers: [infura, quicknode]
# Change back to: eth_providers: [alchemy, infura, quicknode]

# Step 3: Restart relayer
sudo systemctl restart x3-relayer

# Step 4: Verify primary provider is used
sleep 10
curl -s "http://localhost:9090/api/v1/query?query=active_rpc_provider"
# Should show: "alchemy" (primary provider)
```

### Verification

- [ ] Service still running and processing blocks
- [ ] No proofs were dropped
- [ ] Failover occurred automatically (or manually confirmed)
- [ ] Primary provider restored when available
- [ ] No error spikes in monitoring

### Communication Template

```
Subject: X3 Relayer - RPC Provider Issue (Auto-Failover Active)

Alert: Alchemy RPC provider became unavailable at [timestamp]
Impact: Minimal - Automatic failover to Infura activated
Status: ✅ OPERATING NORMALLY on backup provider

Details:
- Provider: Alchemy (Ethereum)
- Duration: 7 minutes
- Impact: 0 dropped proofs (automatic failover)
- Current provider: Infura
- Proofs submitted during incident: 21

Next Steps:
- Monitoring Alchemy recovery
- Will automatically restore when provider is healthy
- Automatic restoration prevents future interruption

No action needed. Services operating normally.
```

---

## Incident #3: Multiple RPC Providers Down

### Detection

```bash
# Alert: All primary RPC providers failing
# Metrics show: All eth_provider_up == 0

# Check all providers manually
for provider in "alchemy" "infura" "quicknode"; do
  echo "Testing $provider..."
  curl -s https://api-$provider.example.com/health --max-time 5
done
```

### Severity Assessment

**CRITICAL** - This blocks all proof submissions

### Root Cause Analysis

**Common Causes:**
1. **Network partition** - ISP/firewall issue
2. **Provider outage** - Multiple providers down simultaneously (very rare)
3. **Rate limiting** - API quota exceeded across all keys
4. **DNS failure** - Can't resolve provider hostnames
5. **Configuration error** - All API keys invalid

#### Diagnosis Steps

```bash
# Step 1: Check network connectivity
ping -c 3 eth-mainnet.g.alchemy.com
ping -c 3 infura.io
ping -c 3 quicknode.com
# If all fail: Network issue

# Step 2: Check DNS resolution
nslookup eth-mainnet.g.alchemy.com
nslookup api.infura.io
# If fails: DNS issue

# Step 3: Check provider status pages
echo "Alchemy: https://status.alchemy.com"
echo "Infura: https://status.infura.io"
echo "QuickNode: https://status.quicknode.com"
# Check for any listed incidents

# Step 4: Check API rate limits
curl -X POST https://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' -v 2>&1 | grep "X-RateLimit"

# Step 5: Verify API keys
grep -E "ALCHEMY_KEY|INFURA_KEY|QUICKNODE_KEY" ~/.bashrc /etc/environment
# Verify keys are not expired
```

### Recovery Steps

#### Immediate Actions (< 5 minutes)

**If Network Issue:**
```bash
# Try alternative DNS
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf.d/99-emergency
sudo systemctl restart systemd-resolved

# Retry connectivity
ping -c 3 eth-mainnet.g.alchemy.com
```

**If Rate Limit Issue:**
```bash
# Check current rate limit status
curl -s https://api.example.com/rate-limit-status

# Option 1: Wait for rate limit window to reset (usually < 1 hour)
echo "Rate limit hit. Waiting for reset... (typically < 1 hour)"
# Relayer will automatically retry

# Option 2: Increase rate limit quota
# Contact Alchemy/Infura support via:
# Alchemy: https://alchemy.com/support
# Infura: https://infura.io/support
```

**If API Key Issue:**
```bash
# Verify API keys are valid
curl -X POST https://eth-mainnet.g.alchemy.com/v2/INVALID_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' 2>&1
# Should show specific error (invalid key, etc.)

# Check key expiration dates in provider dashboard
# Renew if needed

# Update config with new keys
sudo nano /etc/x3-relayer/mainnet.yaml
# Update: eth_api_key, sol_api_key, etc.

# Restart relayer
sudo systemctl restart x3-relayer
```

#### Escalation (5-15 minutes)

If none of the above work:

```bash
# Contact primary provider (Alchemy)
# Phone: [If available]
# Email: support@alchemy.com
# Portal: https://alchemy.com/support

# Escalate to VP Engineering
# Phone: [VP Engineering phone]
# Notify: Full engineering team

# Preserve evidence
sudo journalctl -u x3-relayer > /tmp/incident-logs.txt
curl -s http://localhost:9090/api/v1/tsdb/query_range?query=rpc_failures > /tmp/metrics.json
```

### Verification

**Once connectivity restored:**
```bash
# Test all providers again
for provider in "alchemy" "infura" "quicknode"; do
  curl -s https://api-$provider.example.com/health --max-time 2 && echo "$provider: OK"
done

# Verify relayer recovering
curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])"
# Should see blocks_polled > 0

# Check for pending proofs
curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count"
# Should see any queued proofs being submitted
```

### Communication Template

```
🚨 INCIDENT: Multiple RPC Providers Unavailable

Timeline:
  [Time]: Multiple RPC providers became unavailable
  [Time]: Network diagnosis initiated
  [Time]: Root cause identified: [description]
  [Time]: Recovery actions taken
  [Time]: Connectivity restored

Impact:
  Duration: 18 minutes
  Proofs queued: 34 (will submit upon recovery)
  Network health: CRITICAL (now RECOVERING)

Status:
  Provider: [provider] - RECOVERING ✅
  Relayer: Processing queued proofs ✅
  Users: Impact should clear within 5 minutes

Next Steps:
  1. Relayer resuming proof submission
  2. Monitoring for stability
  3. Post-incident review tomorrow

Thank you for your patience.
```

---

## Incident #4: Bridge Paused (Governance Action)

### Detection

```bash
# Relayer will detect bridge pause automatically
# Logs will show:
# "Bridge paused: paused_reason=governance"
# "Queueing proofs: waiting for resume"

# Manual check
curl -s http://x3-runtime-rpc/state/bridge_status | jq '.status'
# Expected response: "paused"
```

### Expected Behavior

When bridge is paused, relayer **should**:
1. Continue polling blocks
2. Queue proofs instead of submitting
3. Wait for bridge resume signal
4. Resume submission when bridge re-enabled
5. Process queued proofs in order

### Root Cause Analysis

```bash
# Check pause reason
curl -s http://x3-runtime-rpc/state/bridge_pause_reason | jq '.reason'

# Likely reasons:
# - "governance": Governance pause (expected)
# - "emergency": Security pause
# - "maintenance": Planned maintenance

# Check pause initiator
curl -s http://x3-runtime-rpc/state/bridge_pause_initiator | jq '.initiator'

# Check expected resume time
curl -s http://x3-runtime-rpc/state/bridge_resume_eta | jq '.eta'
```

### Recovery Steps

**Relayer will automatically resume when bridge resumes. No manual action typically needed.**

#### If Manual Resume Needed:

```bash
# Step 1: Verify bridge is actually paused
curl -s http://x3-runtime-rpc/state/bridge_status | jq '.'

# Step 2: If governance pause, initiate governance action to resume
# (Requires governance approval - contact governance team)

# Step 3: Once resumed on-chain, relayer will detect automatically
# within 30 seconds (next polling cycle)

# Step 4: Verify resumption
sleep 35
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])"
# Should see proofs_submitted > 0
```

### Monitoring During Pause

```bash
# Check queue depth
curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count"

# Monitor blocks polled (should continue)
curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])"

# Monitor for resume signal
sudo journalctl -u x3-relayer -f | grep -i "bridge\|resume"
```

### Verification

- [ ] Bridge status changed to "active"
- [ ] Relayer detected resume within 30 seconds
- [ ] Queued proofs submitting
- [ ] No proofs lost
- [ ] Normal operation resumed

### Communication Template

```
Subject: X3 Bridge - Governance Pause (Expected)

Status: PAUSED ✅ (Governance Action)

Timeline:
  [Time]: Bridge paused via governance
  Reason: [description]
  Expected resume: [time]

Impact:
  - Relayer continuing to monitor blocks
  - Proofs queued for later submission
  - 0 proofs lost (queued for resubmission)

What we're doing:
  ✓ Relayer operating normally
  ✓ Proofs safely queued
  ✓ Monitoring for governance resume signal

Expected resume:
  [Time/Date] (pending governance action)
  Proofs will submit automatically when resumed

No action needed - system operating as expected.
```

---

## Incident #5: X3 Runtime Error / Proof Rejection

### Detection

```bash
# Logs will show rejection errors:
# "Proof rejected: InvalidProofHash"
# "Error submitting proof: RuntimePanic"
# "Extrinsic rejected: CannotUpdateState"

# Metrics will show:
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_failed_total[1m])"
# Sudden spike in failures

# Alerts:
# proof_failure_rate > 5% for > 2 minutes
```

### Severity Assessment

- **CRITICAL** if: All proofs rejected, bridge appears to be in error state
- **MEDIUM** if: Some proofs rejected, pattern consistent
- **LOW** if: Single proof rejection, transient error

### Root Cause Analysis

```bash
# Step 1: Get error details from logs
sudo journalctl -u x3-relayer -n 50 | grep -A 5 "Proof rejected\|Error submitting"

# Step 2: Check error type
# Common X3 Runtime errors:
# - InvalidProofHash: Proof calculation wrong
# - CannotUpdateState: State root mismatch
# - ProofTooOld: Proof expired (finality changed)
# - InsufficientFunds: Account needs funding
# - NonceError: Sequence error

# Step 3: Verify state is consistent
curl -s http://x3-runtime-rpc/state/bridge_state_root | jq '.state_root'
# Compare against relayer logs: "Calculated root: 0x..."

# Step 4: Check account status
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.'
# Verify: balance > 0, nonce is consistent

# Step 5: Verify block finality
curl -s http://x3-runtime-rpc/state/finality_status | jq '.evm_finality, .svm_finality'
# Verify: finality thresholds haven't changed
```

### Recovery Steps

#### Error Type: InvalidProofHash

```bash
# This indicates relayer is calculating proofs incorrectly

# Step 1: Verify block data is correct
# Check latest EVM block
curl -s http://eth-rpc/eth_blockNumber

# Check latest SVM slot
curl -s http://sol-rpc/getSlot

# Step 2: Verify proof calculation
sudo journalctl -u x3-relayer -n 20 | grep "Calculating proof"
# Should show block/slot numbers and calculated hash

# Step 3: Check for recent code changes
git log --oneline -5 | head -1
git diff HEAD~1 HEAD | grep -A 10 "calculate_proof"

# Step 4: If code issue found:
# Option A: Restart with current code (might be temporary)
sudo systemctl restart x3-relayer

# Option B: Rollback to previous version
# See MAINNET_DEPLOYMENT_RUNBOOK.md → Rollback Procedures
```

#### Error Type: CannotUpdateState

```bash
# This indicates state root mismatch

# Step 1: Verify bridge state
curl -s http://x3-runtime-rpc/state/bridge_state | jq '.root'

# Step 2: Compare with relayer's expected state
sudo journalctl -u x3-relayer -n 10 | grep "Expected root:"

# Step 3: If mismatch:
# Option A: Wait for bridge consensus (usually 5-30 seconds)
sleep 30
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])"
# Should resume submitting

# Option B: Clear relayer state and resync
sudo systemctl stop x3-relayer
sudo rm -rf /var/lib/x3-relayer/headers.db
sudo systemctl start x3-relayer
```

#### Error Type: ProofTooOld / Finality Changed

```bash
# Relayer might be using stale finality settings

# Step 1: Check current finality settings
cat /etc/x3-relayer/mainnet.yaml | grep -A 5 "finality"

# Step 2: Check actual network finality
curl -s http://x3-runtime-rpc/state/finality_config | jq '.evm_finality, .svm_finality'

# Step 3: If changed, update config
sudo nano /etc/x3-relayer/mainnet.yaml
# Update: evm_finality_blocks, svm_finality_slots

# Step 4: Restart relayer
sudo systemctl restart x3-relayer
```

#### Error Type: InsufficientFunds

```bash
# Relayer account needs funding

# Step 1: Check balance
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.balance'

# Step 2: Get funder account
FUNDER_ACCOUNT="0x..." # Usually governance multisig

# Step 3: Transfer funds
# (This requires access to governance account)
# Either:
# A: Governance initiates transfer
# B: Pre-funded account transfers
# C: Testnet faucet (if applicable)

# Step 4: Verify transfer
sleep 10
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.balance'

# Step 5: Resume relayer
sudo systemctl restart x3-relayer
```

### Verification

```bash
# Monitor for recovery
watch -n 5 'curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_failed_total[1m])"'

# Expected: failure_rate drops to near 0

# Check successful submission
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])"
# Should see proofs_submitted > 0

# Verify no proofs lost
curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count"
# Should be 0 or decreasing
```

### Escalation

If error persists:
1. **Contact X3 Runtime team** - May indicate protocol issue
2. **Contact Engineering Lead** - May indicate relayer bug
3. **Review recent governance changes** - Finality settings may have changed
4. **Check for configuration inconsistencies**

---

## Incident #6: Proof Submission Failure (Nonce/Account Issues)

### Detection

```bash
# Logs show:
# "Nonce mismatch: expected 123, got 124"
# "Account locked"
# "Proof submission timed out"

# Metrics:
curl -s "http://localhost:9090/api/v1/query?query=proofs_failed_total"
# Spike in failed submissions
```

### Root Cause Analysis

```bash
# Check account nonce
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.nonce'

# Compare with relayer's expected nonce
sudo journalctl -u x3-relayer -n 20 | grep "nonce"

# Check if account is locked
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.status'
# If status != "active": account is locked

# Check recent transactions
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT/recent_txs | jq '.'
```

### Recovery Steps

#### Nonce Mismatch

```bash
# Step 1: Verify on-chain nonce
ON_CHAIN_NONCE=$(curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.nonce')
echo "On-chain nonce: $ON_CHAIN_NONCE"

# Step 2: Reset relayer nonce tracking
sudo systemctl stop x3-relayer

# Step 3: Update relayer state
# Edit relayer state file to match on-chain nonce
sudo nano /var/lib/x3-relayer/nonce.state
# Update nonce to: $ON_CHAIN_NONCE

# Step 4: Restart
sudo systemctl start x3-relayer

# Step 5: Verify recovery
sleep 10
curl -s "http://localhost:9090/api/v1/query?query=rate(proofs_submitted_total[1m])"
```

#### Account Locked

```bash
# Step 1: Verify account is locked
curl -s http://x3-runtime-rpc/state/account/$X3_RELAYER_ACCOUNT | jq '.status'

# Step 2: Unlock account (requires governance or admin)
# Contact governance team to unlock

# Step 3: Once unlocked, restart relayer
sudo systemctl restart x3-relayer
```

---

## Incident #7: Memory Leak Detected

### Detection

```bash
# Alert: memory_usage_bytes continuously increasing

# Manual check:
watch -n 5 'ps aux | grep x3-relayer | grep -v grep | awk "{print \$6}"'

# Expected: Stable within ± 5%
# Concerning: Growing > 10% per day
# Critical: Growing > 50% per day
```

### Root Cause Analysis

```bash
# Step 1: Get memory usage trend
for i in {1..10}; do
  echo "Iteration $i: $(date +%H:%M:%S) - $(ps aux | grep x3-relayer | grep -v grep | awk '{print $6}') MB"
  sleep 60
done

# Step 2: Check for common leaks in relayer code
grep -n "Box<\|Arc<RwLock<\|Mutex<" crates/relayer/src/*.rs | grep -v "drop"

# Step 3: Enable memory profiling (if available)
# Rebuild with profiling enabled
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release

# Step 4: Run with memory profiler
valgrind --leak-check=full ./target/release/x3-relayer 2>&1 | head -100
```

### Recovery Steps

**Short-term (< 5 minutes):**
```bash
# Restart relayer service (memory will be released)
sudo systemctl restart x3-relayer

# Monitor for leak recurrence
watch -n 60 'ps aux | grep x3-relayer | grep -v grep | awk "{print \$6}"'
```

**Medium-term (within 1 day):**
```bash
# Schedule automatic daily restart
# Edit crontab
crontab -e

# Add line:
# 0 2 * * * systemctl restart x3-relayer  # Daily restart at 2 AM

# This prevents long-running memory leaks from affecting service
```

**Long-term (investigate root cause):**
```bash
# Engage engineering team
# Collect memory profiles
# Review recent code changes
# Fix root cause in next version
```

### Verification

After restart:
```bash
# Monitor memory for 10 minutes
watch -n 60 'free -h; echo "---"; ps aux | grep x3-relayer | grep -v grep | awk "{print \"Memory: \" $6 \" MB\"}"'

# Should remain stable (< 5% change over 10 minutes)
```

---

## Incident #8: Network Partition / Consensus Degradation

### Detection

```bash
# Multiple symptoms:
# - All RPC requests timing out
# - Network errors in logs
# - Blocks/slots not advancing
# - Bridge appears stuck

# Logs show:
# "Network error: connection timeout"
# "Unable to reach consensus"
# "No new blocks for > 60 seconds"

# Metrics:
curl -s "http://localhost:9090/api/v1/query?query=blocks_polled_total"
# Should be increasing, but will be flat during partition
```

### Severity Assessment

**CRITICAL** - This indicates blockchain consensus failure or network partition

### Root Cause Analysis

```bash
# Step 1: Check if network is up
ping -c 3 8.8.8.8  # Public DNS
ping -c 3 eth-mainnet.g.alchemy.com
ping -c 3 api.mainnet.solana.com

# Step 2: Check relayer connectivity
sudo ss -tunap | grep -i "relayer\|9090\|8080"

# Step 3: Check RPC endpoint status
curl -s https://status.alchemy.com  
curl -s https://status.solana.com
# Look for any listed incidents

# Step 4: Check blockchain consensus
# Query multiple RPC providers independently
curl -s http://eth-rpc-1/eth_blockNumber
curl -s http://eth-rpc-2/eth_blockNumber
curl -s http://eth-rpc-3/eth_blockNumber
# All should show same (or very similar) block number

# If different: Consensus failure or network fork
```

### Recovery Steps

#### If Local Network Issue:

```bash
# Restart network interfaces
sudo systemctl restart networking

# Check DNS
sudo systemctl restart systemd-resolved

# Verify connectivity restored
ping -c 3 eth-mainnet.g.alchemy.com
```

#### If Blockchain Consensus Failure:

This is **extremely rare** and **critical**.

```bash
# Step 1: Do NOT make any changes yet
# Step 2: Immediately escalate to VP Engineering + Blockchain team
# Step 3: Preserve all logs and metrics

sudo journalctl -u x3-relayer > /tmp/relayer-incident.log
curl -s http://localhost:9090/api/v1/tsdb > /tmp/metrics-incident.json

# Step 4: Wait for blockchain team guidance
# Step 5: Follow their instructions for recovery

# This may require:
# - Waiting for network consensus to recover
# - Hard fork (if required)
# - Full rollback (if required)
```

### Verification

Once network restored:
```bash
# Check RPC endpoints responding
curl -s http://eth-mainnet.g.alchemy.com/v2/$ALCHEMY_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Check relayer recovering
sleep 10
curl -s "http://localhost:9090/api/v1/query?query=rate(blocks_polled_total[1m])"
# Should see > 0 blocks being polled

# Check for queued proofs
curl -s "http://localhost:9090/api/v1/query?query=pending_proofs_count"
# Relayer should start submitting queued proofs
```

---

## Appendix: Alert Configuration

### Prometheus Alert Rules

```yaml
groups:
- name: x3_relayer_alerts
  rules:
  - alert: RelayerDown
    expr: up{job="x3-relayer"} == 0
    for: 30s
    annotations:
      severity: critical
      summary: "X3 Relayer service is down"

  - alert: ProofFailureRate
    expr: rate(proofs_failed_total[5m]) / rate(proofs_submitted_total[5m]) > 0.05
    for: 2m
    annotations:
      severity: warning
      summary: "Proof failure rate > 5%"

  - alert: RPCProviderDown
    expr: rpc_provider_up == 0
    for: 1m
    annotations:
      severity: warning
      summary: "RPC provider unavailable"

  - alert: MemoryUsageHigh
    expr: process_resident_memory_bytes / 1024 / 1024 > 1000
    for: 5m
    annotations:
      severity: warning
      summary: "Relayer memory usage > 1000 MB"

  - alert: NoBlocksPolled
    expr: rate(blocks_polled_total[5m]) == 0
    for: 2m
    annotations:
      severity: critical
      summary: "No blocks being polled"
```

### Alert Escalation

| Alert | Severity | Action | Escalation |
|-------|----------|--------|-----------|
| RelayerDown | 🔴 Critical | Page on-call engineer | Immediate (5 min) |
| ProofFailureRate | 🟡 Medium | Alert on-call engineer | 15 min |
| RPCProviderDown | 🟡 Medium | Alert on-call engineer | 10 min |
| MemoryUsageHigh | 🟡 Medium | Alert on-call engineer | 30 min |
| NoBlocksPolled | 🔴 Critical | Page on-call engineer | Immediate |

---

## Quick Reference: Decision Tree

```
Problem detected
  │
  ├─ Relayer not running?
  │  └─ See Incident #1: Relayer Crash
  │
  ├─ RPC endpoint down?
  │  ├─ Single provider? → See Incident #2
  │  └─ All providers? → See Incident #3
  │
  ├─ Bridge paused?
  │  └─ See Incident #4
  │
  ├─ Proofs being rejected?
  │  └─ See Incident #5
  │
  ├─ Proof submission failing?
  │  └─ See Incident #6
  │
  ├─ Memory growing?
  │  └─ See Incident #7
  │
  └─ Network/consensus issue?
     └─ See Incident #8
```

---

**Version History:**

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-04-21 | Initial incident response playbooks |

---

**Questions?** Contact: [engineering-lead-email]
