# Phase 13f Quick Reference Guide

**Purpose:** Fast lookup during launch (T-0h to T+7d)  
**Audience:** All team members  
**Usage:** Print this. Keep it visible during launch operations.

---

## 1. OPERATOR'S QUICK-START (1 Page)

### Pre-Launch Checklist (Print & Post)

**T-48h Before Launch:**
- [ ] Confirm relayer binary compiled: `ls -la target/release/x3-relayer`
- [ ] Verify config file: `cat relayer-config-mainnet.yaml | grep -i "rpc\|runtime"`
- [ ] Test relayer locally: `./target/release/x3-relayer --validate-config`
- [ ] Confirm all RPC providers responding: `curl -s [provider-url] | jq .result`

**T-24h Before Launch:**
- [ ] All team members on call schedule
- [ ] Monitoring dashboards loaded (Grafana open in browser)
- [ ] PagerDuty escalation matrix confirmed
- [ ] Slack #mainnet-launch channel created and everyone added

**T-4h Before Launch:**
- [ ] All systems health check green
- [ ] VPN/SSH access tested for all team members
- [ ] Incident response playbooks printed and distributed
- [ ] Final go/no-go decision: [GO / NO-GO]

### Launch Execution Checklist (T-0h)

**T-30m to T-0m:**
- [ ] Start relayer service: `sudo systemctl start x3-relayer`
- [ ] Verify startup: `sudo systemctl status x3-relayer`
- [ ] Check logs: `tail -f /var/log/x3-relayer/relayer.log`
- [ ] Confirm blocks polling: Look for "Block X polled" in logs

**T-0m to T+1h:**
- [ ] Monitor Grafana dashboard (Bridge Activity)
- [ ] Check proof submissions starting (MAINNET_PERFORMANCE_BASELINE.md targets)
- [ ] Review logs for errors: `grep ERROR /var/log/x3-relayer/relayer.log`
- [ ] Post hourly status to Slack

**T+1h to T+24h:**
- [ ] Continue hourly monitoring
- [ ] Check performance metrics every 4 hours
- [ ] If issues → follow incident playbook (see Section 2)
- [ ] Daily status report at T+24h

---

## 2. INCIDENT RESPONSE QUICK REFERENCE

### "Something Is Wrong" Decision Tree

```
Is the relayer service running?
├─ NO → Go to INCIDENT #1: Relayer Crashed
└─ YES → Are blocks being polled?
         ├─ NO → Go to INCIDENT #2: RPC Provider Down
         └─ YES → Are proofs being submitted?
                  ├─ NO → Go to INCIDENT #3: Proof Submission Failed
                  └─ YES → Is memory/CPU abnormal?
                           ├─ YES → Go to INCIDENT #4: Resource Issue
                           └─ NO → Check incident response doc
```

### Quick Incident Summary Table

| Incident | Detection | First Action | Recovery Time | Docs |
|----------|-----------|--------------|----------------|------|
| **Relayer Crashed** | Service down | Restart: `sudo systemctl start x3-relayer` | < 2 min | MAINNET_INCIDENT_RESPONSE.md § 1.1 |
| **RPC Down** | No blocks polling | Check provider status, failover to backup | < 5 min | RPC_FAILOVER_PROCEDURES.md § 2 |
| **Proof Fail** | No proofs submitted | Check relayer logs, verify X3 runtime | < 10 min | MAINNET_INCIDENT_RESPONSE.md § 3.1 |
| **Bridge Paused** | Relayer paused | Check X3 runtime for pause, escalate | < 5 min | MAINNET_INCIDENT_RESPONSE.md § 2.1 |
| **Memory Leak** | Memory > 80% | Restart relayer, check for issues | < 3 min | GPU_VALIDATOR_TROUBLESHOOTING.md § 4 |
| **Network Issue** | High latency | Check network, restart if needed | < 10 min | RPC_FAILOVER_PROCEDURES.md § 3 |

### Fastest Response Commands

**Relayer Status:**
```bash
sudo systemctl status x3-relayer
sudo journalctl -u x3-relayer -n 50 --no-pager
tail -f /var/log/x3-relayer/relayer.log
```

**Check RPC Health:**
```bash
# EVM
curl -X POST https://eth-mainnet.alchemyapi.io/v2/[KEY] \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Solana
curl -X POST https://api.mainnet-beta.solana.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getSlot","params":[],"id":1}'
```

**Check System Resources:**
```bash
free -h
top -bn1 | head -20
df -h
```

**Restart Relayer (Last Resort):**
```bash
sudo systemctl restart x3-relayer
sleep 5
sudo systemctl status x3-relayer
```

---

## 3. ESCALATION LADDER

**LEVEL 1 — Operator (On Scene)**
- Detect issue
- Follow quick reference checklist
- Try basic recovery steps
- Notify Level 2 if unresolved after 5 minutes

**LEVEL 2 — Incident Commander**
- Called at 5-min mark
- Declare severity level (LOW/MEDIUM/HIGH/CRITICAL)
- Activate incident response playbook
- Continue escalation if unresolved after 10 minutes

**LEVEL 3 — Engineering Lead**
- Called if unresolved after 10 minutes
- Review logs and metrics
- Make technical decisions
- Escalate to VP Eng if critical

**LEVEL 4 — VP Engineering**
- Called only for CRITICAL incidents
- Strategic decisions (rollback, pause, etc.)
- Executive communication
- Board notification if needed

---

## 4. EMERGENCY CONTACTS (Keep Printed)

| Role | Owner | Primary Channel | Secondary Channel | Chat |
|------|-------|-----------------|-------------------|------|
| Launch Director | Support Desk | support@x3-chain.io | rpc-support@x3.chain | https://discord.gg/x3-chain |
| Incident Commander | RPC Operations | rpc-support@x3.chain | support@x3-chain.io | https://discord.gg/x3-chain |
| Relayer Operator | Relayer Engineering | rpc-support@x3.chain | support@x3-chain.io | https://discord.gg/x3-chain |
| RPC Manager | RPC Operations | rpc-support@x3.chain | support@x3-chain.io | https://discord.gg/x3-chain |
| Infrastructure Lead | Validator Support | staking-support@x3.chain | rpc-support@x3.chain | https://discord.gg/x3-chain |
| VP Engineering | Escalation Alias | rpc-support@x3.chain | support@x3-chain.io | https://discord.gg/x3-chain |
| On-Call Backup | Validator Support | staking-support@x3.chain | support@x3-chain.io | https://discord.gg/x3-chain |

**Escalation Channel:** rpc-support@x3.chain  
**War Room:** https://discord.gg/x3-chain  
**Status Channel:** https://discord.gg/x3-chain

---

## 5. T-0h TO T+24h EXECUTION CHECKLIST

### Hour 0 (T-0h to T+1h) — CRITICAL

- [ ] **T-30m:** Final pre-launch verification
  - [ ] All systems green
  - [ ] Team in position
  - [ ] Monitoring ready
- [ ] **T-0m:** Start relayer
  - [ ] Service starts: `sudo systemctl start x3-relayer`
  - [ ] Status healthy: `sudo systemctl status x3-relayer`
  - [ ] Logs normal (no errors)
- [ ] **T+5m:** First blocks polling
  - [ ] Relayer polling EVM: check logs
  - [ ] Relayer polling SVM: check logs
- [ ] **T+10m:** First proofs submitted
  - [ ] Check MAINNET_PERFORMANCE_BASELINE.md targets
  - [ ] Proofs in queue
- [ ] **T+30m:** Monitoring healthy
  - [ ] Grafana dashboard updating
  - [ ] No alerts firing
  - [ ] Slack status post
- [ ] **T+1h:** Report success
  - [ ] All systems nominal
  - [ ] Post status update

### Hours 1-6 (T+1h to T+6h) — WATCHFUL

- [ ] Hourly monitoring (every hour)
  - [ ] Check Grafana metrics
  - [ ] Review logs for warnings
  - [ ] Post Slack status
- [ ] Performance tracking
  - [ ] Blocks polled: [Target: 4-5/min EVM, 8-10/min SVM]
  - [ ] Proofs submitted: [Target: 2-4/min]
  - [ ] Latency: [Target: 5-30s proof, 60-180s EVM, 10-50s SVM]

### Hours 6-24 (T+6h to T+24h) — STEADY

- [ ] 4-hourly monitoring
  - [ ] Check metrics every 4 hours
  - [ ] Review logs for errors
  - [ ] Post status every 6 hours
- [ ] Performance baseline establishment
  - [ ] Record throughput metrics
  - [ ] Record latency metrics
  - [ ] Record resource utilization
- [ ] Daily checklist (T+24h)
  - [ ] All systems still green
  - [ ] No critical incidents
  - [ ] Performance baseline stable
  - [ ] Write daily status report

---

## 6. DOCUMENT QUICK LINKS

**For [Issue Type] → Go To [Document]**

| Need to... | Go to... | Section |
|-----------|----------|---------|
| Execute launch | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2 (Hour-by-Hour) |
| Handle incident | MAINNET_INCIDENT_RESPONSE.md | § 1-8 (Playbooks) |
| Failover RPC | RPC_FAILOVER_PROCEDURES.md | § 2-3 |
| Manage validators | VALIDATOR_OPERATIONS.md | § 1-4 |
| Check performance | MAINNET_PERFORMANCE_BASELINE.md | § 2 (Targets) |
| Fix GPU issues | GPU_VALIDATOR_TROUBLESHOOTING.md | § 1-4 |
| Find documentation | PHASE_13F_MASTER_INDEX.md | § 1-4 |
| Navigate documents | PHASE_13F_CROSS_REFERENCE_GUIDE.md | (next section) |

---

**Print this guide. Post it in your war room.**

**Last Updated:** April 21, 2026  
**Version:** 1.0
