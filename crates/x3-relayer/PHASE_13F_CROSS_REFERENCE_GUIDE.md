# Phase 13f Cross-Reference Guide

**Purpose:** Find what you need across all 14 Phase 13f documents  
**Audience:** All team members  
**Usage:** Search by topic → find all relevant documents → get specific section numbers

---

## Quick Navigation by Role

### If you are the **Launch Director**
- Overall coordination: PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md § 1-2
- Risk assessment: PHASE_13F_STAKEHOLDER_SUMMARY.md § 5
- Go/no-go decision: PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md § 2.1
- Daily reporting: PHASE_13F_DAILY_STATUS_TEMPLATE.md

### If you are the **Incident Commander**
- Incident playbooks: MAINNET_INCIDENT_RESPONSE.md § 1-8
- Escalation matrix: PHASE_13F_MASTER_INDEX.md § 5
- Communications: PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md § 4
- Decision tree: PHASE_13F_QUICK_REFERENCE_GUIDE.md § 2
- Infrastructure validation: PHASE_13F_INFRASTRUCTURE_VALIDATION.md § 1-5

### If you are the **Relayer Operator**
- Launch execution: PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md § 2
- Performance baseline: MAINNET_PERFORMANCE_BASELINE.md § 1-2
- Incident response: MAINNET_INCIDENT_RESPONSE.md § 1.1 (Relayer Crash)
- Quick commands: PHASE_13F_QUICK_REFERENCE_GUIDE.md § 2

### If you are the **RPC Manager**
- RPC failover: RPC_FAILOVER_PROCEDURES.md § 1-3
- Provider health: RPC_FAILOVER_PROCEDURES.md § 3.2
- Incident response: MAINNET_INCIDENT_RESPONSE.md § 1.2 (RPC Down)
- Performance baseline: MAINNET_PERFORMANCE_BASELINE.md § 2.1-2.2

### If you are the **Network Operator**
- Infrastructure validation: PHASE_13F_INFRASTRUCTURE_VALIDATION.md
- Monitoring setup: PHASE_13F_INFRASTRUCTURE_VALIDATION.md § 3
- Performance baseline: MAINNET_PERFORMANCE_BASELINE.md
- Launch execution: PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md

### If you are the **Validator Lead**
- Validator operations: VALIDATOR_OPERATIONS.md § 1-4
- GPU troubleshooting: GPU_VALIDATOR_TROUBLESHOOTING.md § 1-4
- Performance baseline: MAINNET_PERFORMANCE_BASELINE.md § 2.3
- Incident response: MAINNET_INCIDENT_RESPONSE.md § 1.5 (X3 Runtime Error)

### If you are the **Communications Lead**
- Launch announcement: PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md § 1-2
- Crisis communication: PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md § 4-5
- Daily status: PHASE_13F_DAILY_STATUS_TEMPLATE.md
- Stakeholder summary: PHASE_13F_STAKEHOLDER_SUMMARY.md

---

## Topic-to-Document Matrix

### Launch Planning & Execution

| Topic | Document | Section | Pages |
|-------|----------|---------|-------|
| **Hour-by-hour launch procedures** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2 | p. 10-25 |
| **T-48h to T-24h prep** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.1 | p. 10-12 |
| **T-24h to T-4h pre-launch** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.2 | p. 12-14 |
| **T-4h to T-0h final prep** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.3 | p. 14-16 |
| **T-0h execution** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.4 | p. 16-18 |
| **T+0h to T+24h operations** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.5 | p. 18-21 |
| **Go/no-go decision** | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.1 & § 3 | p. 10, 25 |
| **Execution checklist** | PHASE_13F_QUICK_REFERENCE_GUIDE.md | § 5 | p. 5 |

### Incident Response

| Topic | Document | Section |
|-------|----------|---------|
| **Incident #1: Relayer Crash** | MAINNET_INCIDENT_RESPONSE.md | § 1.1 |
| **Incident #2: Single RPC Down** | MAINNET_INCIDENT_RESPONSE.md | § 1.2 |
| **Incident #3: Multiple RPC Down** | MAINNET_INCIDENT_RESPONSE.md | § 1.3 |
| **Incident #4: Bridge Paused** | MAINNET_INCIDENT_RESPONSE.md | § 1.4 |
| **Incident #5: X3 Runtime Error** | MAINNET_INCIDENT_RESPONSE.md | § 1.5 |
| **Incident #6: Proof Submission Failure** | MAINNET_INCIDENT_RESPONSE.md | § 1.6 |
| **Incident #7: Memory Leak** | MAINNET_INCIDENT_RESPONSE.md | § 1.7 |
| **Incident #8: Network Partition** | MAINNET_INCIDENT_RESPONSE.md | § 1.8 |
| **Escalation matrix** | PHASE_13F_MASTER_INDEX.md | § 5 |
| **Decision tree** | MAINNET_INCIDENT_RESPONSE.md | § Appendix A |
| **Quick incident summary** | PHASE_13F_QUICK_REFERENCE_GUIDE.md | § 2 |

### RPC Provider Management

| Topic | Document | Section |
|-------|----------|---------|
| **RPC provider architecture** | RPC_FAILOVER_PROCEDURES.md | § 1 |
| **Ethereum RPC setup** | RPC_FAILOVER_PROCEDURES.md | § 1.1 |
| **Solana RPC setup** | RPC_FAILOVER_PROCEDURES.md | § 1.2 |
| **X3 Runtime setup** | RPC_FAILOVER_PROCEDURES.md | § 1.3 |
| **Automatic failover** | RPC_FAILOVER_PROCEDURES.md | § 2.1 |
| **Manual failover** | RPC_FAILOVER_PROCEDURES.md | § 2.2 |
| **Health checking** | RPC_FAILOVER_PROCEDURES.md | § 3 |
| **Quota management** | RPC_FAILOVER_PROCEDURES.md | § 4 |
| **Testing failover** | RPC_FAILOVER_PROCEDURES.md | § 5 |
| **Failover quick checklist** | PHASE_13F_QUICK_REFERENCE_GUIDE.md | § 2 |

### Validator Operations

| Topic | Document | Section |
|-------|----------|---------|
| **Hardware requirements** | VALIDATOR_OPERATIONS.md | § 1 |
| **Adding validators** | VALIDATOR_OPERATIONS.md | § 2.1 |
| **Removing validators** | VALIDATOR_OPERATIONS.md | § 2.2 |
| **Key rotation** | VALIDATOR_OPERATIONS.md | § 3 |
| **Slashing recovery** | VALIDATOR_OPERATIONS.md | § 4 |
| **Rewards management** | VALIDATOR_OPERATIONS.md | § 5 |
| **Health monitoring** | VALIDATOR_OPERATIONS.md | § 6 |

### GPU Troubleshooting

| Topic | Document | Section |
|-------|----------|---------|
| **GPU detection** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 1 |
| **CUDA initialization** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 2 |
| **CUDA error diagnosis** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 3 |
| **Out-of-memory errors** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 3.1 |
| **Compute capability errors** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 3.2 |
| **Thermal throttling** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 4 |
| **Memory leak detection** | GPU_VALIDATOR_TROUBLESHOOTING.md | § 5 |
| **Health check script** | GPU_VALIDATOR_TROUBLESHOOTING.md | § Appendix A |

### Performance & Monitoring

| Topic | Document | Section |
|-------|----------|---------|
| **Performance baselines** | MAINNET_PERFORMANCE_BASELINE.md | § 2 |
| **Throughput targets** | MAINNET_PERFORMANCE_BASELINE.md | § 2.1 |
| **Latency targets** | MAINNET_PERFORMANCE_BASELINE.md | § 2.2 |
| **Resource targets** | MAINNET_PERFORMANCE_BASELINE.md | § 2.3 |
| **Grafana dashboard setup** | MAINNET_PERFORMANCE_BASELINE.md | § 3 |
| **Dashboard creation** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3.2 |
| **Alert rules** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3.3 |

### Infrastructure Validation

| Topic | Document | Section |
|-------|----------|---------|
| **Relayer service** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 1 |
| **RPC providers health** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 2 |
| **X3 runtime** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 2.3 |
| **Prometheus setup** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3.1 |
| **Grafana setup** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3.2 |
| **Alerting setup** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3.3-3.5 |
| **Security hardening** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 4 |
| **Disaster recovery** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 5 |
| **Sign-off checklist** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 6 |

### Stakeholder Communication

| Topic | Document | Section |
|-------|----------|---------|
| **Executive summary** | PHASE_13F_STAKEHOLDER_SUMMARY.md | § 1 |
| **Completion status** | PHASE_13F_STAKEHOLDER_SUMMARY.md | § 2 |
| **Risk mitigation** | PHASE_13F_STAKEHOLDER_SUMMARY.md | § 5 |
| **Success criteria** | PHASE_13F_STAKEHOLDER_SUMMARY.md | § 6 |
| **Briefing slides** | PHASE_13F_STAKEHOLDER_BRIEFING_SLIDES.md | § 1-13 |
| **Internal announcement** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 1 |
| **Partner announcement** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 2 |
| **Press announcement** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 3 |
| **Crisis communication** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 4 |
| **All-clear announcement** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 5 |
| **Daily status template** | PHASE_13F_DAILY_STATUS_TEMPLATE.md | § 1 |
| **Communication channels** | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § Appendix A |

### Verification & Testing

| Topic | Document | Section |
|-------|----------|---------|
| **War game exercise** | PHASE_13F_VERIFICATION_EXERCISES.md | § 1 |
| **Team rehearsal** | PHASE_13F_VERIFICATION_EXERCISES.md | § 2 |
| **Infrastructure validation** | PHASE_13F_VERIFICATION_EXERCISES.md | § 3 |
| **Failover testing** | PHASE_13F_VERIFICATION_EXERCISES.md | § 4 |
| **Validation checklist** | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § All |
| **Test results tracker** | PHASE_13F_TEST_RESULTS_TRACKER.md | § All |

### Post-Launch

| Topic | Document | Section |
|-------|----------|---------|
| **Retrospective preparation** | PHASE_13F_POSTLAUNCH_RETROSPECTIVE.md | § 1 |
| **Retrospective workshop** | PHASE_13F_POSTLAUNCH_RETROSPECTIVE.md | § 2 |
| **Post-retrospective actions** | PHASE_13F_POSTLAUNCH_RETROSPECTIVE.md | § 3 |

### Navigation

| Topic | Document | Section |
|-------|----------|---------|
| **Master index** | PHASE_13F_MASTER_INDEX.md | All |
| **Decision tree** | PHASE_13F_MASTER_INDEX.md | § 1 |
| **Document dependencies** | PHASE_13F_MASTER_INDEX.md | § 2 |
| **Success criteria** | PHASE_13F_MASTER_INDEX.md | § 3 |
| **Cross-reference guide** | PHASE_13F_CROSS_REFERENCE_GUIDE.md | (this file) |
| **Quick reference** | PHASE_13F_QUICK_REFERENCE_GUIDE.md | All |

---

## "I Need to..." → "Go To..." Lookup

### Launch Execution

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| What happens at T-48h? | Confirm state & notify stakeholders | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.1 |
| What happens at T-24h? | Final communications to validators | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.2 |
| What happens at T-4h? | Final go/no-go decision | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.3 |
| What do I do at T-0m? | Start relayer service | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md | § 2.4 |
| What should I monitor? | Check dashboard every hour | MAINNET_PERFORMANCE_BASELINE.md | § 2 |
| How do I know if we're successful? | See success criteria | PHASE_13F_STAKEHOLDER_SUMMARY.md | § 6 |

### Incidents

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| Relayer just crashed, what do I do? | Follow playbook #1 | MAINNET_INCIDENT_RESPONSE.md | § 1.1 |
| RPC provider is down, what do I do? | Follow playbook #2 & RPC failover | MAINNET_INCIDENT_RESPONSE.md § 1.2 + RPC_FAILOVER_PROCEDURES.md | § 2 |
| Bridge is paused, what do I do? | Follow playbook #4 | MAINNET_INCIDENT_RESPONSE.md | § 1.4 |
| Memory is leaking, what do I do? | Follow playbook #7 | MAINNET_INCIDENT_RESPONSE.md | § 1.7 |
| Who do I escalate to? | See escalation matrix | PHASE_13F_MASTER_INDEX.md | § 5 |
| How do I communicate an incident? | Use crisis template | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 4 |

### RPC Issues

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| How do RPC failovers work? | Automatic failover procedure | RPC_FAILOVER_PROCEDURES.md | § 2 |
| Can I manually switch providers? | Yes, follow manual failover | RPC_FAILOVER_PROCEDURES.md | § 2.2 |
| How do I check RPC health? | Run health check commands | RPC_FAILOVER_PROCEDURES.md | § 3 |
| What if multiple RPC providers fail? | Follow cascade failover | RPC_FAILOVER_PROCEDURES.md | § 2.1 |
| Can I test failover without affecting production? | Yes, use staging environment | RPC_FAILOVER_PROCEDURES.md | § 5 |

### Validator Issues

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| How do I add a new validator? | Follow validator operations | VALIDATOR_OPERATIONS.md | § 2.1 |
| How do I rotate validator keys? | Follow key rotation procedure | VALIDATOR_OPERATIONS.md | § 3 |
| What if a validator gets slashed? | Follow recovery procedure | VALIDATOR_OPERATIONS.md | § 4 |
| How do I check validator health? | Monitor metrics + logs | VALIDATOR_OPERATIONS.md | § 6 |
| GPU is not initializing, what do I do? | Check GPU troubleshooting | GPU_VALIDATOR_TROUBLESHOOTING.md | § 1-2 |

### Monitoring & Performance

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| What throughput should we expect? | 4-5 blocks/min EVM, 8-10 SVM | MAINNET_PERFORMANCE_BASELINE.md | § 2.1 |
| What latency should we expect? | 5-30s proofs, 60-180s EVM | MAINNET_PERFORMANCE_BASELINE.md | § 2.2 |
| How do I set up Grafana? | Follow dashboard setup | MAINNET_PERFORMANCE_BASELINE.md | § 3 |
| Which dashboard should I watch? | Bridge Activity dashboard | MAINNET_PERFORMANCE_BASELINE.md | § 3 |
| What metrics should I track? | Throughput, latency, resources | PHASE_13F_QUICK_REFERENCE_GUIDE.md | § 5 |

### Infrastructure

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| Is the relayer ready for launch? | Check validation checklist | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 1 |
| Are all RPC providers healthy? | Check provider health tests | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 2 |
| Is monitoring configured? | Check monitoring setup | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 3 |
| Is disaster recovery tested? | Check recovery test results | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 5 |
| Can we restore from backup? | Follow restore procedure | PHASE_13F_INFRASTRUCTURE_VALIDATION.md | § 5.2 |

### Communications

| Question | Answer | Document | Section |
|----------|--------|----------|---------|
| What should I tell the team at T-48h? | Use internal announcement | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 1 |
| What should I tell partners? | Use partner announcement | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 2 |
| What should I tell the press? | Use press announcement | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 3 |
| How do I report daily progress? | Use daily status template | PHASE_13F_DAILY_STATUS_TEMPLATE.md | § 1 |
| What if there's an incident? | Use crisis communication template | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 4 |
| How do I announce we're safe? | Use all-clear template | PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md | § 5 |

---

## Document Relationships Map

```
PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md
  ├─ references → MAINNET_INCIDENT_RESPONSE.md (§ 2.6)
  ├─ references → RPC_FAILOVER_PROCEDURES.md (§ 2.3)
  ├─ references → VALIDATOR_OPERATIONS.md (§ 2.5)
  ├─ references → MAINNET_PERFORMANCE_BASELINE.md (§ 2.4)
  ├─ references → GPU_VALIDATOR_TROUBLESHOOTING.md (§ 2.7)
  ├─ references → PHASE_13F_MASTER_INDEX.md (§ 1)
  └─ references → PHASE_13F_DAILY_STATUS_TEMPLATE.md (§ 2.5)

MAINNET_INCIDENT_RESPONSE.md
  ├─ references → RPC_FAILOVER_PROCEDURES.md (in playbook #2)
  ├─ references → VALIDATOR_OPERATIONS.md (in playbook #5)
  ├─ references → GPU_VALIDATOR_TROUBLESHOOTING.md (in playbook #7)
  ├─ references → PHASE_13F_MASTER_INDEX.md (escalation matrix)
  └─ references → PHASE_13F_LAUNCH_ANNOUNCEMENT_TEMPLATES.md (crisis template)

PHASE_13F_MASTER_INDEX.md
  ├─ links to → All 6 operational documents
  └─ links to → All 14 Phase 13f documents

PHASE_13F_INFRASTRUCTURE_VALIDATION.md
  ├─ supports → PHASE_13F_VERIFICATION_EXERCISES.md (§ 3)
  └─ checks → PHASE_13F_TEST_RESULTS_TRACKER.md
```

---

## Quick Facts

**Total Phase 13f Documents:** 14
- Operational: 6
- Navigation: 1
- Communication: 4
- Post-Launch: 1
- Verification: 3

**Total Lines of Documentation:** 14,730+
- Operational procedures: 2,250+ lines
- Incident response: 8+ playbooks
- RPC failover: Complete architecture
- Validator operations: Complete lifecycle
- Performance baseline: All metrics defined
- GPU troubleshooting: 5 key areas covered

**Total Page Count (estimated):** ~40 pages (printed)

**Estimated Read Time:**
- Quick Reference: 5-10 minutes
- Master Index: 10 minutes
- Full operational suite: 2-3 hours
- All documents: 6-8 hours

---

**Last Updated:** April 21, 2026  
**Version:** 1.0  
**Status:** Complete & Cross-Referenced
