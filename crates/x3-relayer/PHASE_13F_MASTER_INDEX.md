# Phase 13f Master Index: Mainnet Launch Documentation Suite

**Document Version:** 1.0  
**Last Updated:** 2026-04-21  
**Status:** Master Control Document  
**Target Audience:** Launch Operations Team, On-Call Engineers, Network Operators

---

## Overview

This document is your **single entry point** for Phase 13f mainnet launch. It provides:
- Quick-reference decision tree (which doc for which scenario)
- Document dependency map and reading order
- Success criteria checklist (pre-launch, at-launch, post-launch)
- Incident escalation flowchart
- Cross-document scenario flows

**All 6 Phase 13f Documents:**
1. PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md — Hour-by-hour execution (T-48h to T+7d)
2. MAINNET_INCIDENT_RESPONSE.md — 8 incident playbooks with recovery procedures
3. RPC_FAILOVER_PROCEDURES.md — RPC provider failure handling and recovery
4. VALIDATOR_OPERATIONS.md — Validator lifecycle management (add/remove/rotate/recover)
5. MAINNET_PERFORMANCE_BASELINE.md — Expected performance, metrics, alerts
6. GPU_VALIDATOR_TROUBLESHOOTING.md — GPU initialization, CUDA errors, thermal issues

---

## Quick-Reference Decision Tree

**Use this when you need to know which document to consult:**

```
┌─ LAUNCH EXECUTION & TIMELINE ────────────────────────────────────────┐
│  "What's the schedule for launch?"                                    │
│  "What should we be doing in the next 4 hours?"                       │
│  "What's the T-48h to T+7d timeline?"                                 │
│                                                                        │
│  → PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md                                │
│    (Hour-by-hour procedures, checklists, team coordination)          │
└────────────────────────────────────────────────────────────────────────┘

┌─ SOMETHING IS BROKEN NOW (INCIDENT RESPONSE) ────────────────────────┐
│  "Relayer service crashed"                                            │
│  "RPC endpoints are down"                                             │
│  "Proofs are not being submitted"                                     │
│  "Memory is growing uncontrolled"                                     │
│  "Bridge is paused"                                                   │
│  "X3 runtime is returning errors"                                     │
│  "Network is partitioned"                                             │
│  "Consensus is degraded"                                              │
│                                                                        │
│  → MAINNET_INCIDENT_RESPONSE.md                                       │
│    (8 detailed playbooks: detection, root cause, recovery, escalation)│
└────────────────────────────────────────────────────────────────────────┘

┌─ RPC PROVIDER ISSUES ────────────────────────────────────────────────┐
│  "Primary RPC endpoint is slow"                                       │
│  "Need to switch to failover RPC"                                     │
│  "Multiple RPC providers are having issues"                           │
│  "Want to test failover without breaking production"                  │
│  "Need to set up automatic RPC failover"                              │
│                                                                        │
│  → RPC_FAILOVER_PROCEDURES.md                                         │
│    (Failover decision tree, manual/automatic procedures, testing)    │
└────────────────────────────────────────────────────────────────────────┘

┌─ VALIDATOR OPERATIONS ──────────────────────────────────────────────┐
│  "Need to add a new validator"                                        │
│  "Need to remove a validator"                                         │
│  "Need to rotate validator keys"                                      │
│  "Validator was slashed, how do we recover?"                          │
│  "How do we manage rewards?"                                          │
│  "Validator is not producing blocks"                                  │
│                                                                        │
│  → VALIDATOR_OPERATIONS.md                                            │
│    (Lifecycle: add/remove, key rotation, slashing recovery, rewards) │
└────────────────────────────────────────────────────────────────────────┘

┌─ PERFORMANCE MONITORING & BASELINES ────────────────────────────────┐
│  "What TPS should we be seeing?"                                      │
│  "Why is latency increasing?"                                         │
│  "How much CPU/memory should the relayer use?"                        │
│  "Detected performance regression, what's normal?"                    │
│  "Setting up monitoring and alerts"                                   │
│                                                                        │
│  → MAINNET_PERFORMANCE_BASELINE.md                                    │
│    (Expected TPS, latency, resource utilization, regression detection)│
└────────────────────────────────────────────────────────────────────────┘

┌─ GPU VALIDATOR ISSUES ──────────────────────────────────────────────┐
│  "GPU not detected by validator"                                      │
│  "CUDA out of memory error"                                           │
│  "GPU is thermally throttling"                                        │
│  "GPU memory is growing (possible leak)"                              │
│  "GPU performance degraded"                                           │
│  "GPU hardware failure suspected"                                     │
│                                                                        │
│  → GPU_VALIDATOR_TROUBLESHOOTING.md                                   │
│    (GPU detection, CUDA errors, thermal, memory, hardware failure)   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Document Dependency Map

### Reading Order (Recommended)

**Before Launch (T-48h to T-0h):**

```
1. PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md (start here)
   ↓
2. MAINNET_PERFORMANCE_BASELINE.md (understand what "good" looks like)
   ↓
3. RPC_FAILOVER_PROCEDURES.md (ensure RPC resilience is configured)
   ↓
4. VALIDATOR_OPERATIONS.md (understand validator procedures)
   ↓
5. GPU_VALIDATOR_TROUBLESHOOTING.md (GPU validators only)
   ↓
6. MAINNET_INCIDENT_RESPONSE.md (familiarize ops team with playbooks)
```

**During Launch (T-0h onwards):**
```
PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md (primary reference)
  ↓ (if incident occurs)
  → MAINNET_INCIDENT_RESPONSE.md
    ↓ (if RPC issue)
    → RPC_FAILOVER_PROCEDURES.md
    ↓ (if GPU issue)
    → GPU_VALIDATOR_TROUBLESHOOTING.md
    ↓ (if validator issue)
    → VALIDATOR_OPERATIONS.md
    ↓ (if performance concern)
    → MAINNET_PERFORMANCE_BASELINE.md
```

### Document Cross-References

| Document | References | Referenced By |
|----------|-----------|---|
| PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | All 5 others (for escalation) | Index (as primary) |
| MAINNET_INCIDENT_RESPONSE.md | RPC_FAILOVER, VALIDATOR_OPS, GPU_TROUBLESHOOTING, PERF_BASELINE | LAUNCH_RUNBOOK |
| RPC_FAILOVER_PROCEDURES.md | MAINNET_INCIDENT_RESPONSE (Incident #3) | LAUNCH_RUNBOOK, INCIDENT_RESPONSE |
| VALIDATOR_OPERATIONS.md | MAINNET_INCIDENT_RESPONSE (recovery procedures) | LAUNCH_RUNBOOK |
| MAINNET_PERFORMANCE_BASELINE.md | MAINNET_INCIDENT_RESPONSE (degradation detection) | LAUNCH_RUNBOOK |
| GPU_VALIDATOR_TROUBLESHOOTING.md | VALIDATOR_OPERATIONS (hardware context) | LAUNCH_RUNBOOK, INCIDENT_RESPONSE |

---

## Success Criteria Checklist

### Placeholder Substitution Gate (Blocking)

This gate is mandatory before launch execution. All gate-scoped launch docs below must have environment placeholders replaced with real values, or explicitly marked as intentionally illustrative.

Gate scope:
- PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md
- MAINNET_INCIDENT_RESPONSE.md
- RPC_FAILOVER_PROCEDURES.md
- VALIDATOR_OPERATIONS.md
- MAINNET_PERFORMANCE_BASELINE.md
- GPU_VALIDATOR_TROUBLESHOOTING.md
- PHASE_13F_MASTER_INDEX.md

Run this check:

```bash
bash scripts/check_phase13f_placeholders.sh
```

Blocking policy:
- T-24h gate fails if any unresolved placeholders are reported.
- T-4h gate fails if any unresolved placeholders are reported.
- T-30m go/no-go cannot proceed while this check is failing.

If a placeholder is intentionally left in a non-executable example, append `PHASE13F_PLACEHOLDER_OK` on that same line to suppress that specific finding.

### Pre-Launch Readiness (T-48h to T-0h)

**Configuration & Infrastructure:**
- [ ] Mainnet validator configuration reviewed and approved
- [ ] RPC endpoints verified (Ethereum: Alchemy/Infura/QuickNode, Solana: QuickNode/Helius/Triton)
- [ ] Systemd services configured and tested on staging
- [ ] Log rotation configured for 30-day retention
- [ ] Monitoring (Prometheus/Grafana) deployed and dashboards active
- [ ] Alert rules configured (see MAINNET_INCIDENT_RESPONSE.md Appendix)
- [ ] Backup procedures documented and tested

**Team & Communication:**
- [ ] Launch team roster established (ops, validators, engineers, comms)
- [ ] On-call rotation configured for T-48h to T+7d
- [ ] Communication channels ready (Slack, email, incident bridge)
- [ ] Escalation procedures documented and acknowledged
- [ ] Customer/stakeholder notification plan ready

**Validation & Testing:**
- [ ] 6-stage validation complete (see PHASE_13E_MAINNET_PREP.md)
- [ ] Staging deployment successful with all checks passing
- [ ] Failover procedures tested without production impact
- [ ] Incident playbooks reviewed by ops team (all 8 scenarios)
- [ ] Recovery procedures tested (rollback, incident response)

**Documentation Review:**
- [ ] All team members reviewed PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md
- [ ] Incident response team reviewed MAINNET_INCIDENT_RESPONSE.md
- [ ] RPC team reviewed RPC_FAILOVER_PROCEDURES.md
- [ ] Validator team reviewed VALIDATOR_OPERATIONS.md
- [ ] GPU validators reviewed GPU_VALIDATOR_TROUBLESHOOTING.md
- [ ] All team members understand MAINNET_PERFORMANCE_BASELINE.md

**Go/No-Go Criteria:**
```
LAUNCH APPROVED IF ALL OF:
✅ Infrastructure checks passing (T-4h gate)
✅ RPC failover tested and ready (T-4h gate)
✅ Team standing by (T-2h gate)
✅ Final go/no-go meeting held (T-30m gate)
✅ All stakeholders notified (T-0 gate)
```

---

### At-Launch Verification (T-0h to T+1h)

**System Health:**
- [ ] Relayer service started and running
- [ ] RPC endpoints responding normally
- [ ] First blocks being polled from both EVM and SVM
- [ ] First proofs being submitted to X3 runtime
- [ ] Monitoring showing expected metrics (see MAINNET_PERFORMANCE_BASELINE.md)
- [ ] No critical alerts firing
- [ ] Logs clean of errors

**Network Health:**
- [ ] X3 runtime producing blocks at expected rate
- [ ] Validator participation > 65%
- [ ] Network latency normal (< 500ms RPC response)
- [ ] No consensus issues reported

**Go-Live Sign-Off:**
```
LAUNCH SUCCESSFUL IF:
✅ Relayer healthy (polling 3+ blocks/min, submitting 1+ proofs/min)
✅ RPC endpoints responding (< 200ms latency)
✅ Network healthy (blocks produced, > 65% participation)
✅ No critical incidents in first hour
✅ Team standing by and monitoring
```

---

### Post-Launch Monitoring (T+1h to T+7d)

**T+1h to T+6h:**
- [ ] Relayer processing blocks at steady rate (4-5 blocks/min EVM, 8-10 SVM)
- [ ] Proofs submitting at expected rate (2-4 per minute)
- [ ] No incident escalations
- [ ] Team morale good, confidence high

**T+6h to T+24h:**
- [ ] 24-hour uptime achieved (0% downtime in first day)
- [ ] Performance baseline established (actual vs expected)
- [ ] No regressions from T+0h metrics
- [ ] Validator stakes settling at expected distribution
- [ ] Rewards flowing correctly

**T+24h to T+7d:**
- [ ] 7-day uptime achieved
- [ ] Performance stable and predictable
- [ ] All validators operating normally
- [ ] No slashing events (or only minor downtime slashing)
- [ ] Incident response procedures validated in practice (if any incidents occurred)

**Success Metrics (Target Ranges):**
| Metric | Target | Good | Warning | Alert |
|--------|--------|------|---------|-------|
| Blocks polled/min | 4-5 (EVM) | 3-6 | 2-3 | < 2 |
| Proofs/min | 2-4 | 1-6 | 0.5-1 | < 0.5 |
| Pending proofs | < 5 | < 10 | 10-20 | > 20 |
| CPU % | 50 | < 70 | 70-80 | > 80 |
| Memory MB | 250 | < 500 | 500-1000 | > 1000 |
| Uptime | 99.9% | > 99.5% | > 95% | < 95% |

---

## Incident Escalation Flowchart

```
┌─────────────────────────────────────────────┐
│ INCIDENT DETECTED (alert or user report)    │
└────────────┬────────────────────────────────┘
             │
             ↓
    ┌────────────────────────────────┐
    │ CLASSIFY INCIDENT              │
    │ (Severity: P1/P2/P3)           │
    └────────────┬───────────────────┘
                 │
    ┌────────────┴──────────────────────────────────────────────┐
    │                                                            │
    ↓                                                            ↓
P1: CRITICAL                                           P2: HIGH / P3: MEDIUM
(Revenue impact,                                       (Degraded perf,
 total outage)                                         minor incidents)
    │                                                            │
    ├─ Incident #1: Relayer Crash                              │
    │  → MAINNET_INCIDENT_RESPONSE.md: Incident #1             │
   │  → Escalate: rpc-support@x3.chain (< 5 min)              │
    │  → Team: On-call relayer engineer + ops lead             │
    │                                                            │
    ├─ Incident #3: Multiple RPC Providers Down                │
    │  → RPC_FAILOVER_PROCEDURES.md + MAINNET_INCIDENT_RESPONSE.md: #3
   │  → Escalate: rpc-support@x3.chain + provider portals (< 10 min) │
    │  → Team: RPC specialist + relayer engineer               │
    │                                                            │
    ├─ Incident #5: X3 Runtime Error                           │
    │  → MAINNET_INCIDENT_RESPONSE.md: Incident #5             │
   │  → Escalate: rpc-support@x3.chain (< 5 min)              │
    │  → Team: Runtime expert + bridge engineer                │
    │                                                            │
    ├─ Incident #8: Network Partition                          │
    │  → MAINNET_INCIDENT_RESPONSE.md: Incident #8             │
   │  → Escalate: rpc-support@x3.chain + support@x3-chain.io  │
    │  → Team: All hands on deck + external communication      │
    │                                                            │
    └─ Unknown P1 Issue                                          │
       → MAINNET_INCIDENT_RESPONSE.md: Appendix (decision tree)│
       → Escalate: rpc-support@x3.chain immediately            │
       → Team: War room (incident bridge + Slack)              │
                                                                │
                                                            ├─ Incident #2: Single RPC Down
                                                            │  → RPC_FAILOVER_PROCEDURES.md
                                                            │  → Escalate: rpc-support@x3.chain (< 15 min)
                                                            │  → Team: RPC specialist
                                                            │
                                                            ├─ Incident #4: Bridge Paused
                                                            │  → MAINNET_INCIDENT_RESPONSE.md: #4
                                                            │  → Escalate: support@x3-chain.io
                                                            │  → Team: Bridge operator
                                                            │
                                                            ├─ Incident #6: Proof Submission Fail
                                                            │  → MAINNET_INCIDENT_RESPONSE.md: #6
                                                            │  → Escalate: rpc-support@x3.chain
                                                            │  → Team: Relayer engineer
                                                            │
                                                            ├─ Incident #7: Memory Leak
                                                            │  → MAINNET_INCIDENT_RESPONSE.md: #7
                                                            │  → Escalate: rpc-support@x3.chain (< 30 min)
                                                            │  → Team: Relayer engineer
                                                            │
                                                            └─ Performance Degradation
                                                               → MAINNET_PERFORMANCE_BASELINE.md
                                                               → MAINNET_INCIDENT_RESPONSE.md
                                                               → Escalate: support@x3-chain.io (< 30 min)
                                                               → Team: Performance specialist
    │
    ↓
┌──────────────────────────────────────────────────────┐
│ EXECUTE INCIDENT PLAYBOOK (see document specified)  │
│ • Detection confirmation                             │
│ • Root cause analysis                                │
│ • Immediate recovery steps                           │
│ • Verification procedures                            │
│ • Communication to stakeholders                      │
└──────────────────┬───────────────────────────────────┘
                   │
                   ↓
        ┌─────────────────────────┐
        │ INCIDENT RESOLVED?      │
        └────────────┬────────────┘
             No      │      Yes
             ├───────┼───────┤
             ↓       ↓       ↓
        Escalate  Pause   Close
        (next     (wait   (post-
         level)   5 min)  mortem)
```

---

## Cross-Document Scenario Flows

### Scenario 1: RPC Provider Fails at T+2h

**Discovery:** Alert fires - "Ethereum RPC latency > 1s for 5 minutes"

**Flow:**
```
1. Check RPC_FAILOVER_PROCEDURES.md:
   - Is this temporary latency or actual provider failure?
   - Check: Section 2 "Detecting RPC Failure"
   - Run: Multi-provider health check script

2. If confirmed provider down:
   - RPC_FAILOVER_PROCEDURES.md: Section 3 "Manual Failover"
   - Switch to next provider in chain
   - Verify recovery: < 200ms latency again

3. If multiple providers affected:
   - MAINNET_INCIDENT_RESPONSE.md: Incident #3 "Multiple RPC Providers Down"
   - Escalate: RPC provider team
   - Run: Diagnosis procedures
   - Execute: Recovery steps

4. Monitor recovery:
   - MAINNET_PERFORMANCE_BASELINE.md: Expected latency targets
   - Verify proofs still submitting at normal rate (2-4/min)
   - Check CPU/memory not spiking due to retry logic

5. Document & debrief:
   - PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md: Communication template
   - Send stakeholder update
   - Schedule post-incident review
```

---

### Scenario 2: GPU Validator Crashes at T+4h

**Discovery:** Alert - "GPU validator offline, not producing blocks"

**Flow:**
```
1. Check GPU_VALIDATOR_TROUBLESHOOTING.md:
   - Section 1: GPU Detection Issues
   - Run: 5-minute health check script
   - Determine: Is GPU detected? Is CUDA working?

2. If GPU initialization failure:
   - GPU_VALIDATOR_TROUBLESHOOTING.md: Section 1 recovery steps
   - Check driver, CUDA toolkit
   - May require system restart

3. If CUDA out of memory:
   - GPU_VALIDATOR_TROUBLESHOOTING.md: Section 4
   - Run: Memory diagnostics
   - Options:
     a) Reduce cache size (Section 4: recovery steps)
     b) Restart validator to clear memory
     c) Switch to another GPU if available

4. If hardware failure suspected:
   - GPU_VALIDATOR_TROUBLESHOOTING.md: Section 6 "Hardware Failure"
   - Run: Diagnostic tests
   - Document ECC errors
   - VALIDATOR_OPERATIONS.md: Remove validator gracefully
   - Order replacement GPU

5. Meanwhile, ensure network health:
   - Other validators still producing blocks?
   - Check MAINNET_PERFORMANCE_BASELINE.md: Expected block production
   - If consensus degraded, escalate to runtime team

6. Communicate:
   - PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md: Communication template
   - Notify validator operators of GPU issue
   - Share ETA for recovery
```

---

### Scenario 3: Proof Submission Rate Drops to 0/min at T+6h

**Discovery:** Alert - "Proofs submitted in last 5 minutes: 0"

**Flow:**
```
1. Diagnose which component failed:
   - Are blocks still being polled?
   - Are proofs being calculated?
   - Are proofs being submitted?

   MAINNET_INCIDENT_RESPONSE.md: Incident #6 (Proof Submission Failure)
   - Run: Diagnosis procedures (Section 1)

2. Branch on root cause:

   A) Blocks not being polled:
      → RPC_FAILOVER_PROCEDURES.md: Check RPC health
      → If RPC down, follow Scenario 1 above

   B) Proofs being calculated but not submitted:
      → MAINNET_INCIDENT_RESPONSE.md: Incident #6
      → Check: Nonce mismatch? Account locked?
      → Recovery: Section 2 recovery procedures

   C) Relayer service crashed:
      → MAINNET_INCIDENT_RESPONSE.md: Incident #1 (Relayer Crash)
      → Run: Service restart with health checks

3. Verify recovery:
   - MAINNET_PERFORMANCE_BASELINE.md: Expected proofs/min (2-4)
   - Monitor for 5+ minutes to confirm steady state
   - Check for cascading failures

4. If unresolved after 15 minutes:
   - MAINNET_INCIDENT_RESPONSE.md: Escalation procedures
   - Escalate to VP Engineering
   - Investigate logs for CUDA errors, RPC issues, memory problems
   - Consider rollback to last known good state

5. Communicate:
   - PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md: Communication template
   - Every 5 minutes update stakeholders
   - Include ETA for resolution
```

---

### Scenario 4: Performance Degradation (Slow But Working) at T+18h

**Discovery:** Relayer is working but blocks/min dropping from 5 → 2.5

**Flow:**
```
1. Characterize degradation:
   MAINNET_PERFORMANCE_BASELINE.md: Section 5 "Regression Detection"
   - Run: Automated regression detector
   - Calculate: % deviation from baseline
   - Severity: 50% drop = significant regression

2. Identify which component:
   A) Block polling rate down:
      → RPC_FAILOVER_PROCEDURES.md: Check RPC health/latency
      → MAINNET_PERFORMANCE_BASELINE.md: Expected latency (100ms)
      → If latency > 500ms, escalate to RPC team

   B) Proof calculation slow:
      → MAINNET_PERFORMANCE_BASELINE.md: CPU/Memory check
      → GPU_VALIDATOR_TROUBLESHOOTING.md: GPU performance (if applicable)
      → Check for memory leaks or resource exhaustion

   C) Proof submission bottleneck:
      → MAINNET_INCIDENT_RESPONSE.md: Incident #6 (retry logic?)
      → Check pending proofs count (should be < 5)
      → If high: submission is backing up, check RPC

3. Recovery strategy:
   - If RPC issue: RPC_FAILOVER_PROCEDURES.md
   - If resource exhaustion: Restart relayer service
   - If software bug: Escalate to engineering team

4. Establish new baseline:
   - MAINNET_PERFORMANCE_BASELINE.md: Baseline Establishment
   - Don't change config yet if stable (even if degraded)
   - Monitor for 1+ hours to see if stabilizes
   - If continues degrading: escalate

5. Communicate:
   - PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md: Communication template
   - Update: "Investigating performance degradation, still operational"
   - Provide ETA for analysis
```

---

## Team Contact Matrix

| Role | Document Reference | On-Call Window | Escalation Path |
|------|---|---|---|
| **Relayer Operations** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | T-48h to T+7d | → rpc-support@x3.chain |
| **RPC Provider Specialist** | RPC_FAILOVER_PROCEDURES.md | T-24h to T+48h | → rpc-support@x3.chain |
| **Validator Operator** | VALIDATOR_OPERATIONS.md | T-24h to T+7d | → staking-support@x3.chain |
| **GPU Specialist** | GPU_VALIDATOR_TROUBLESHOOTING.md | T-24h to T+48h | → rpc-support@x3.chain |
| **Incident Commander** | MAINNET_INCIDENT_RESPONSE.md | T-0h to T+7d (primary) | → rpc-support@x3.chain |
| **Performance SRE** | MAINNET_PERFORMANCE_BASELINE.md | T-4h to T+7d | → support@x3-chain.io |
| **Communications Lead** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | T-24h to T+7d | → https://discord.gg/x3-chain |

---

## Quick Links (In Emergency)

**Relayer crashed?**
→ MAINNET_INCIDENT_RESPONSE.md: Incident #1

**RPC down?**
→ RPC_FAILOVER_PROCEDURES.md: Section 2-3

**Validator issue?**
→ VALIDATOR_OPERATIONS.md: Relevant section

**GPU problem?**
→ GPU_VALIDATOR_TROUBLESHOOTING.md: Section matching symptom

**Proofs not submitting?**
→ MAINNET_INCIDENT_RESPONSE.md: Incident #6

**Performance degrading?**
→ MAINNET_PERFORMANCE_BASELINE.md: Section 5

**What time is it in launch schedule?**
→ PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md: Hour-by-hour section

**Multi-issue escalation?**
→ MAINNET_INCIDENT_RESPONSE.md: Appendix (decision tree)

---

## Document Checklist for Team

Print this checklist. Each team member should acknowledge:

```
PHASE 13f LAUNCH TEAM CERTIFICATIONS

Name: ________________________     Role: ____________________

I have read and understand:

☐ PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md (T-48h to T+7d timeline)
☐ MAINNET_INCIDENT_RESPONSE.md (incident playbooks)
☐ MAINNET_PERFORMANCE_BASELINE.md (expected metrics)
☐ RPC_FAILOVER_PROCEDURES.md (RPC resilience procedures)
  [RPC specialists only]
☐ VALIDATOR_OPERATIONS.md (validator lifecycle)
  [Validator operators only]
☐ GPU_VALIDATOR_TROUBLESHOOTING.md (GPU-specific troubleshooting)
  [GPU validators only]

I understand:
☐ My role during launch
☐ My escalation path for incidents
☐ The success criteria for launch
☐ How to use this master index to find information

Signature: ________________________     Date: _________________

Acknowledged by: ________________________
                 (Team Lead/VP Engineering)
```

---

## Document Version Control

| Document | Version | Last Updated | Status |
|----------|---------|---|---|
| PHASE_13F_MASTER_INDEX.md | 1.0 | 2026-04-21 | Ready |
| PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | 1.0 | 2026-04-21 | Ready |
| MAINNET_INCIDENT_RESPONSE.md | 1.0 | 2026-04-21 | Ready |
| RPC_FAILOVER_PROCEDURES.md | 1.0 | 2026-04-21 | Ready |
| VALIDATOR_OPERATIONS.md | 1.0 | 2026-04-21 | Ready |
| MAINNET_PERFORMANCE_BASELINE.md | 1.0 | 2026-04-21 | Ready |
| GPU_VALIDATOR_TROUBLESHOOTING.md | 1.0 | 2026-04-21 | Ready |

---

## Feedback & Updates

**Is something unclear in this index?**
- Check which specific document covers that scenario
- Read that document's relevant section
- If still unclear, escalate to author

**Need to update these docs?**
- Update the specific document
- Update version number
- Update this master index to reflect change
- Notify team of updates via email + Slack

---

## Success Criteria: Master Index

This master index is successful if:

✅ Team member can find the right document in < 1 minute  
✅ Each scenario in the decision tree has clear ownership  
✅ All 6 documents are cross-linked and reference each other  
✅ Incident flowchart covers all critical paths  
✅ Success criteria are measurable and verifiable  
✅ Team members acknowledge understanding (via checklist)  

---

**This is your launch control center. Print it. Bookmark it. Know it.**

**Questions during launch? Start here. Then drill into the specific document.**

---

**Phase 13f: Ready for Mainnet Launch** ✅

