# Phase 13e: Mainnet Preparation

**Status: Planning**  
**Prerequisite:** Phase 13d testnet validation PASSED  
**Duration:** 2-3 hours planning + 2-4 hours validation  

---

## Overview

Phase 13e prepares the X3 Bridge Relayer for mainnet deployment. This phase uses the **same tested code** from Phase 13c-13d but with production-equivalent configuration, endpoints, and deployment procedures.

### Key Differences from Testnet

| Aspect | Testnet (13d) | Mainnet (13e) |
|--------|---------------|---------------|
| **RPC Endpoints** | Sepolia, Solana testnet | Ethereum mainnet, Solana mainnet |
| **Configuration** | Single-operator testnet | Production multi-operator |
| **Monitoring** | Basic (5s refresh) | Advanced (Grafana, alerts) |
| **Validation** | 30 minutes | 4-8 hours on staging |
| **Rollback Plan** | Restart relayer | Promote standby instance |
| **SLA Uptime** | Not required | 99.5% (max 3.6h downtime/month) |
| **Chain Network** | Test networks | Live networks (real value) |
| **Proof Submission** | Test proofs | Live bridge updates |

---

## Phase 13e Scope

### 1. Mainnet Configuration
- [ ] Ethereum mainnet RPC setup (Alchemy, Infura, Quicknode)
- [ ] Solana mainnet cluster configuration
- [ ] Production-grade finality thresholds
- [ ] Rate limiting optimization for mainnet load
- [ ] Relay loop timing tuned for mainnet block times

### 2. Deployment Infrastructure
- [ ] Pre-signed deployment package (immutable binary)
- [ ] Systemd service for relayer (auto-restart, logging)
- [ ] Environment variable management (vaults/secrets)
- [ ] Log rotation and retention policy
- [ ] Database for metrics persistence

### 3. Monitoring & Alerting
- [ ] Prometheus metrics export
- [ ] Grafana dashboard (real-time monitoring)
- [ ] Alert thresholds (CPU, memory, latency, failure rate)
- [ ] PagerDuty integration for on-call rotations
- [ ] Health check endpoint (liveness, readiness)

### 4. Disaster Recovery
- [ ] Automated failover to standby relayer
- [ ] RPC endpoint failover (multiple providers)
- [ ] Graceful degradation (prefer stale data over no data)
- [ ] Restore procedures (state recovery, proof replay)
- [ ] Incident playbooks (RPC down, bridge paused, etc.)

### 5. Security Hardening
- [ ] Code review checklist (mainnet-specific)
- [ ] Secrets management (environment variables, vaults)
- [ ] Network isolation (firewalls, VPN)
- [ ] Rate limiting on RPC calls
- [ ] Proof validation before submission

### 6. Testing & Validation
- [ ] Staging environment (mainnet forks)
- [ ] Load testing (100x testnet throughput)
- [ ] Failure scenario testing (RPC down, network delay)
- [ ] Security audit (if applicable)
- [ ] 4-8 hour pre-launch validation on staging

### 7. Deployment Checklist
- [ ] Go/no-go decision criteria
- [ ] Deployment rollback plan
- [ ] Communication plan (status updates)
- [ ] Post-deployment monitoring (first 24 hours)
- [ ] Success criteria validation

---

## Mainnet Configuration Differences

### RPC Providers (Ethereum)

**Testnet (Sepolia via Infura):**
```yaml
rpc_endpoint: "https://sepolia.infura.io/v3/{KEY}"
chain_id: 11155111
finality_threshold: 12  # ~156 seconds
```

**Mainnet (Multiple providers for redundancy):**
```yaml
primary_rpc: "https://eth-mainnet.g.alchemy.com/v2/{KEY}"
fallback_rpc: "https://ethereum-rpc.publicnode.com"
backup_rpc: "https://rpc.ankr.com/eth"
chain_id: 1
finality_threshold: 64  # ~15 minutes (much higher for safety)
```

### RPC Providers (Solana)

**Testnet:**
```yaml
rpc_endpoint: "https://api.testnet.solana.com"
finality_threshold: 32  # ~12 seconds
```

**Mainnet:**
```yaml
primary_rpc: "https://api.mainnet-beta.solana.com"
fallback_rpc: "https://solana-rpc.publicnode.com"
backup_rpc: "https://rpc.ankr.com/solana"
finality_threshold: 128  # ~50 seconds (confirmed)
```

### Performance Parameters

**Testnet:**
- Block poll interval: 13000ms (1 per Sepolia block)
- Slot poll interval: 15000ms (1 per custom interval)
- Max concurrent requests: 5 (EVM), 20 (SVM)

**Mainnet:**
- Block poll interval: 13000ms (unchanged, 1 per block)
- Slot poll interval: 6000ms (faster Solana polling)
- Max concurrent requests: 10 (EVM), 50 (SVM) [load test to verify]
- Submission batch size: 5 (group proofs to reduce RPC calls)
- Submission timeout: 120s (double timeout for network variability)

---

## Deployment Infrastructure

### Systemd Service (/etc/systemd/system/x3-relayer.service)

```ini
[Unit]
Description=X3 Bridge Relayer - Mainnet
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=x3-relayer
WorkingDirectory=/opt/x3-relayer
Environment="PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin"
EnvironmentFile=/etc/x3-relayer/relayer.env
ExecStart=/opt/x3-relayer/bin/x3-relayer
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=x3-relayer

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/x3-relayer /var/lib/x3-relayer

[Install]
WantedBy=multi-user.target
```

### Log Rotation (/etc/logrotate.d/x3-relayer)

```
/var/log/x3-relayer/relayer.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0640 x3-relayer x3-relayer
    sharedscripts
    postrotate
        systemctl reload x3-relayer > /dev/null 2>&1 || true
    endscript
}
```

### Environment Variables Management

**Vault-based approach (recommended):**
```bash
# Use HashiCorp Vault or AWS Secrets Manager
vault kv get -format=json secret/x3-relayer/mainnet | jq '.data.data' | export $(jq -r 'to_entries[] | "\(.key)=\(.value)"' /dev/stdin)

# Or AWS Secrets Manager
aws secretsmanager get-secret-value --secret-id x3-relayer-mainnet --query SecretString --output text | jq -r 'to_entries[] | "export \(.key)=\(.value)"' | source /dev/stdin
```

---

## Monitoring & Alerting Strategy

### Prometheus Metrics Export

The relayer will expose metrics at `/metrics`:

```
# HELP x3_relayer_blocks_polled Total EVM blocks polled
# TYPE x3_relayer_blocks_polled counter
x3_relayer_blocks_polled{domain_id="200"} 45000

# HELP x3_relayer_blocks_finalized Total blocks reaching finality
# TYPE x3_relayer_blocks_finalized counter
x3_relayer_blocks_finalized{domain_id="200"} 3200

# HELP x3_relayer_proofs_submitted Total proofs submitted
# TYPE x3_relayer_proofs_submitted counter
x3_relayer_proofs_submitted 2100

# HELP x3_relayer_poll_failures Poll failures
# TYPE x3_relayer_poll_failures counter
x3_relayer_poll_failures{domain_id="200"} 12

# HELP x3_relayer_uptime_seconds Relayer uptime
# TYPE x3_relayer_uptime_seconds gauge
x3_relayer_uptime_seconds 86400
```

### Alert Thresholds

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| **Polling Stopped** | No blocks polled in 5 min | CRITICAL | Page on-call, check RPC |
| **Finalization Blocked** | No finalized blocks in 30 min | CRITICAL | Investigate bridge state |
| **High Submission Failure** | > 10% proofs fail | WARNING | Check X3 runtime |
| **RPC Latency High** | P95 latency > 5s | WARNING | Switch to fallback RPC |
| **Memory Leak** | RSS > 500MB | WARNING | Restart relayer |
| **CPU Spike** | > 50% for 5 min | INFO | Monitor, no action yet |

### Grafana Dashboard

**Key panels:**
1. Polling rate (blocks/hour, slots/hour)
2. Finalization rate (blocks/hour, slots/hour)
3. Proof submission success rate (%)
4. Submission latency (p50, p95, p99)
5. RPC endpoint performance (latency by provider)
6. Error rate over time
7. System resources (CPU, memory, network)
8. Uptime and availability

---

## Disaster Recovery & Failover

### RPC Endpoint Failover

```rust
// Pseudocode for RPC failover logic
async fn get_with_failover(method: &str) -> Result<Value> {
    let providers = vec![
        ("primary", PRIMARY_RPC),
        ("fallback", FALLBACK_RPC),
        ("backup", BACKUP_RPC),
    ];
    
    for (name, url) in providers {
        match call_rpc(url, method).await {
            Ok(response) => {
                log::info!("RPC call succeeded with {}", name);
                return Ok(response);
            }
            Err(e) => {
                log::warn!("RPC call failed with {}: {}", name, e);
                // Try next provider
                continue;
            }
        }
    }
    
    Err("All RPC providers exhausted".into())
}
```

### Relayer Instance Failover

**Setup (2 relayers running in parallel):**
1. Primary relayer (active, submits proofs)
2. Secondary relayer (standby, synchronized state)
3. Health check endpoint (every 30 seconds)
4. Automatic promotion to primary on failure

**State Synchronization:**
- Both relayers query same RPC endpoints
- Secondary reads proof cache from primary
- On primary failure, secondary promotes itself
- Proof submissions are idempotent (same nonce = same proof)

### Graceful Degradation

If bridge is paused:
1. Continue polling headers
2. Continue tracking finality
3. Queue proofs locally (up to 1000)
4. Resume submission when bridge reopens
5. No data loss, no duplicate submissions

---

## Security Hardening Checklist

### Code Review
- [ ] No hardcoded secrets or credentials
- [ ] All RPC calls validated (JSON schema)
- [ ] Proof structure validated before submission
- [ ] Integer overflow/underflow checks
- [ ] Null pointer checks (Rust mitigates most)

### Secrets Management
- [ ] RPC keys stored in vault (not config files)
- [ ] Relayer account seed phrase in vault (not disk)
- [ ] Key rotation procedure documented
- [ ] Audit logs for secret access

### Network Security
- [ ] Firewall rules (restrict outbound to RPC endpoints only)
- [ ] VPN/SSH tunneling for RPC calls (optional, latency trade-off)
- [ ] Rate limiting on RPC calls (respect provider limits)
- [ ] DDoS protection (if publicly exposed)

### Operational Security
- [ ] No SSH key reuse (unique per server)
- [ ] Bastion host for SSH access
- [ ] Audit logs of all manual interventions
- [ ] Change control process for mainnet changes
- [ ] Runbooks for common incidents

---

## Testing & Validation Plan

### Stage 1: Configuration Validation (30 min)
- [ ] YAML syntax valid
- [ ] All RPC endpoints accessible
- [ ] Finality thresholds appropriate for mainnet
- [ ] Rate limits not exceeded

### Stage 2: Staging Environment (2-4 hours)
- [ ] Deploy to mainnet fork (Hardhat, Foundry)
- [ ] Run relay loop for 1 hour
- [ ] Verify all metrics are updating
- [ ] Simulate RPC failures → verify failover
- [ ] Simulate bridge pause → verify graceful handling

### Stage 3: Load Testing (1 hour)
- [ ] Inject 100x testnet load (simulated)
- [ ] Verify CPU/memory under control
- [ ] Verify no RPC endpoint exhaustion
- [ ] Measure latency under load (p95, p99)

### Stage 4: Security Validation (30 min)
- [ ] Secrets properly masked in logs
- [ ] No credentials in debug output
- [ ] Rate limiting working
- [ ] Proof validation preventing invalid submissions

### Stage 5: Failure Scenario Testing (1 hour)
- [ ] Primary RPC down → failover works
- [ ] All RPC down → graceful pause
- [ ] Bridge paused → proofs queued
- [ ] Network delay (5s latency) → submission succeeds
- [ ] Out-of-sync node → recovery works

---

## Pre-Launch Validation Checklist

**4 hours before launch:**

- [ ] Code review complete (mainnet-specific)
- [ ] Staging validation passed (all 5 stages)
- [ ] Monitoring dashboards operational
- [ ] Alert channels verified (PagerDuty, Slack)
- [ ] Runbooks accessible and documented
- [ ] Rollback plan tested
- [ ] Communication plan ready

**1 hour before launch:**

- [ ] Go/no-go decision made (all approvals)
- [ ] Deployment checklist reviewed
- [ ] Team assembled and briefed
- [ ] Monitor tabs open and ready
- [ ] Incident commander assigned

**At launch:**

- [ ] Binary deployed to production
- [ ] Systemd service started
- [ ] Logs appear (no startup errors)
- [ ] Metrics begin flowing
- [ ] First blocks polled (within 2 minutes)
- [ ] Communication sent (status update)

**First hour post-launch:**

- [ ] Polling rate stable (1 block per 13s)
- [ ] Finality checks passing
- [ ] No ERROR-level log entries
- [ ] RPC endpoint performance nominal
- [ ] System resources healthy (CPU < 10%, memory < 200MB)

**First 24 hours post-launch:**

- [ ] Continuous monitoring (no gaps)
- [ ] First proofs submitted successfully
- [ ] Finalization metrics normal
- [ ] No unplanned interventions needed
- [ ] Incident commander shift handoff complete

---

## Success Criteria

### Immediate (First Hour)
✅ Relayer starts without errors  
✅ Begins polling blocks/slots  
✅ Metrics appear in Prometheus  
✅ No ERROR-level log entries  

### Short-term (First 24 Hours)
✅ Polling rate stable and consistent  
✅ Finality checks passing regularly  
✅ Proofs submitted without issues  
✅ Error rate < 1%  
✅ System resources stable  

### Long-term (First Week)
✅ Uptime > 99.5%  
✅ Proof submission success rate > 99%  
✅ No unplanned restarts  
✅ RPC failover tested and working  
✅ Monitoring alerts validated  

### Production Baseline
✅ SLA of 99.5% uptime maintained  
✅ < 5 minute MTTD (mean time to detect)  
✅ < 15 minute MTTR (mean time to recover)  
✅ Proof latency < 1 minute p95  
✅ Cross-chain state consistency maintained  

---

## Rollback Plan

**Rollback Trigger:**
- Critical bug discovered (proof loss, state corruption)
- Unrecoverable RPC failures (all providers down)
- Bridge state inconsistency detected
- Decision made to revert to testnet relayer

**Rollback Steps:**
1. Stop primary relayer (systemctl stop x3-relayer)
2. Verify standby has caught up with proof cache
3. Promote standby to primary
4. Restart with previous configuration
5. Validate metrics resumed
6. Communication: status update sent

**Rollback Time:** < 5 minutes

**Data Preservation:**
- All proofs submitted to X3 runtime (immutable)
- Metrics preserved in Prometheus (time series)
- Logs preserved for post-mortem analysis

---

## Communication Plan

### Pre-Launch
- [ ] Notify stakeholders (48 hours before)
- [ ] Status page: "Scheduled Maintenance"
- [ ] Technical details in incident channel

### At Launch
- [ ] Status: "Deployment in progress"
- [ ] Updated every 15 minutes
- [ ] Real-time metrics shared

### Post-Launch
- [ ] Status: "Monitoring" (first 24h)
- [ ] Updates if issues arise
- [ ] Final "Stable" status after 24h

### Incident (If Needed)
- [ ] Immediate notification
- [ ] Root cause summary
- [ ] Remediation steps
- [ ] ETA to resolution
- [ ] Post-mortem within 24h

---

## Next Steps

Phase 13e proceeds with:

1. **Configuration Template Creation** (30 min)
   - Mainnet-equivalent YAML with placeholders
   - RPC provider selection guidance
   - Finality threshold recommendations

2. **Infrastructure Codification** (1 hour)
   - Systemd service definition
   - Log rotation configuration
   - Monitoring setup (Prometheus, Grafana)

3. **Runbook Development** (1 hour)
   - Deployment runbook
   - Emergency procedures
   - Common issues troubleshooting

4. **Staging Validation** (2-4 hours)
   - Set up mainnet fork environment
   - Deploy relayer to staging
   - Run through validation checklist
   - Test failure scenarios

5. **Go/No-Go Review** (1 hour)
   - Review all validation results
   - Risk assessment
   - Final decision and approval

---

## Mainnet Go-Live Timeline

```
Hour 0: Configuration finalized
Hour 0-1: Final validation on staging
Hour 1: Go/no-go decision
Hour 1-1.5: Deploy to production
Hour 1.5-2.5: First hour monitoring
Hour 2.5-26.5: 24-hour monitoring
Hour 26.5: Success confirmation
```

**Estimated Time to Production:** 2.5-4 hours after Phase 13d succeeds

---

## Success Path

Phase 13d (✅ Testnet) → Phase 13e (📋 Planning) → Phase 13f (🚀 Mainnet Launch)

This phase ensures **zero-surprise mainnet deployment** through comprehensive planning and validation.
