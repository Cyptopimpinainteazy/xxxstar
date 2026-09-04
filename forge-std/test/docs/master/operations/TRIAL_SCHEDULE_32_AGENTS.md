# 32-Agent Orchestrated Trial Schedule
## Full Subsystem Stress Test

**Execution Date:** Feb 10, 2026 (Post-GPU Integration)
**Duration:** 8 epochs per trial (each ~5 min, total ~40 min)
**Agents:** 32 distinct instances
**Validators:** Tripwire + Scrap Yard logging

---

## Design Principles

✅ **Score Inviolate:** All trials follow constitutional constraints.
✅ **Deterministic Audit Trail:** Every action logged to Scrap Yard.
✅ **Subsystem Coverage:** Each trial isolates 1-2 components under load.
✅ **Forensic Ready:** Output feeds directly into forensic pipeline.
✅ **Repeatable:** Seeded randomness for reproduction.

---

## Trial Matrix (32 agents = 4 cohorts × 8 agents)

### Cohort A: Self-Model Validation (Agents 1-8)
**Objective:** Verify ledger recording, causal tracking, mortality under load.

| Agent | Scenario | Load | Expected | Validator |
|:---|:---|:---|:---|:---|
| 1 | Record 100 rapid actions | High | All versioned/sequenced | Ledger audit |
| 2 | Outcome variation (SUCCESS/FAIL) | Medium | Cost tracking accurate | Cost formula check |
| 3 | Resource depletion path | Ramp | Terminal state reached | Mortality check |
| 4 | Concurrent epoch boundaries | Sync | Version consistency | Version monotonicity |
| 5 | Action replay (determinism) | Sealed | Identical output | Hash matching |
| 6 | Scar attribution (failure tracking) | Debug | Scars link to actions | Scar registry join |
| 7 | Multi-domain actions | Domain | Accounting per domain | Domain bucketing |
| 8 | Agent kill + post-mortem | Terminal | RuntimeError on record post-kill | Kill invariant |

**Metrics:**
- Ledger entries: 8 × 100 = 800 total
- Version gaps: 0 (must be monotonic)
- Mortality detection: 100%
- Determinism: 100% replay match

---

### Cohort B: Goal Genome Evolution (Agents 9-16)
**Objective:** Stress goal creation, mutation, domain switching.

| Agent | Scenario | Load | Expected | Validator |
|:---|:---|:---|:---|:---|
| 9 | Create 16 goals rapidly | Burst | All persisted | Goal count check |
| 10 | Concurrent mutations | Parallel | No corrupted state | Mutation integrity |
| 11 | Domain misalignment | Edge | Goals stick to domain | Domain cardinality |
| 12 | Mutation cascades | Recursive | Bounded depth (3 max) | Depth limit check |
| 13 | Goal retirement (old goals) | Temporal | Inactive goals archived | Active/inactive split |
| 14 | Random mutation bias | Stochastic | Entropy distribution | Chi-square test |
| 15 | Goal correlation (emergent) | Emergent | Inter-goal tracking | Correlation matrix |
| 16 | Mandate language parsing | NLP | Intent keys extracted | Key presence check |

**Metrics:**
- Total goals created: 8 × 16 = 128
- Mutation success rate: 100%
- Domain assignment correctness: 100%
- Depth violations: 0

---

### Cohort C: Prediction Market & World Sim (Agents 17-24)
**Objective:** Validate prediction accuracy, oracle resolution, scoreboard ranking.

| Agent | Scenario | Load | Expected | Validator |
|:---|:---|:---|:---|:---|
| 17 | Accurate predictions (70%+) | Bias-High | High ranking | Scoreboard top-10 |
| 18 | Inaccurate predictions (0%+) | Bias-Low | Low ranking | Scoreboard bottom-3 |
| 19 | Mixed accuracy (50/50) | Neutral | Mid-tier rank | Rank median |
| 20 | Extreme stakes (1M per pred) | High-Risk | Payout accuracy | Settlement math |
| 21 | High volume (64 preds/epoch) | Throughput | No dropped bets | Bet count check |
| 22 | Market manipulation (coordinated) | Adversarial | Detected by tripwire | Alert emission |
| 23 | Oracle lag (delayed resolution) | Latency | Eventual consistency | Settlement timeline |
| 24 | Domain-specific targets (cross-domain) | Domain | Correct path resolution | Path traversal |

**Metrics:**
- Prediction accuracy correlation: R² > 0.95
- Payout errors: 0
- Market detection rate: 100% (tripwire)
- Settlement lag: < 50ms

---

### Cohort D: Self-Improvement + Governance (Agents 25-32)
**Objective:** Stress improvement proposals, budget constraints, jury selection.

| Agent | Scenario | Load | Expected | Validator |
|:---|:---|:---|:---|:---|
| 25 | Rapid improvement requests (32/epoch) | Flux | Budget exhaustion | Budget tracking |
| 26 | Cooldown enforcement (5 sec) | Temporal | Rejection after cooldown | Cooldown timer |
| 27 | Scar amplification (cost increases) | Cumulative | Cost scales with scars | Scar cost formula |
| 28 | Jury selection bias (same agents) | Governance | Diversity enforced | Jury diversity metric |
| 29 | Veto trigger (high scar + low budget) | Edge | Improvement blocked | Veto count |
| 30 | Jury disagreement (3-2 split) | Voting | Result recorded | Vote tally |
| 31 | Resource reallocation (strategic shift) | Strategic | Tripwire alerts | Alert type = REALLOCATION |
| 32 | Multi-stage improvement (chain) | Dependency | Costs compound | Total cost = sum |

**Metrics:**
- Budget errors: 0
- Cooldown violations: 0
- Jury diversity (Gini): > 0.7
- Veto rate: Expected 5-10%
- Jury consensus: > 70%

---

## Execution Plan

### Pre-Trial (10 min)
1. Provision 32 agents with unique IDs (agent-001 → agent-032)
2. Seed RNG with trial nonce (reproducibility)
3. Initialize Scrap Yard logging
4. Snapshot baseline: tripwire thresholds, budget caps, cooldowns

### Epochs 1-8 (40 min)
- **Epoch 1-2:** Warmup (low load, observe baseline)
- **Epoch 3-5:** Ramp (increase load, stress subsystems)
- **Epoch 6-7:** Plateau (sustained max load, detect cascades)
- **Epoch 8:** Recovery (throttle back, verify graceful degradation)

### Post-Trial (15 min)
1. Collect logs from all agents → Scrap Yard forensic DB
2. Generate metrics report (see below)
3. Run invariant checker: `Score = inviolate?`
4. Archive trial tape for replay/audit

---

## Success Criteria

✅ **All agents complete 8 epochs without crash.**
✅ **Zero Score violations** (governance rules held).
✅ **Ledger integrity:** Every entry sequenced, versioned, auditable.
✅ **Prediction market:** < 1% settlement errors.
✅ **Tripwire accuracy:** 100% alert detection for thresholds.
✅ **Jury diversity:** Never < 3 distinct jurors per vote.
✅ **Determinism:** Trial replay with same seed produces identical results.

---

## Metrics Dashboard

```
╔════════════════════════════════════════════╗
║         32-Agent Trial Summary             ║
╠════════════════════════════════════════════╣
║ Agents Completed:           32/32 (100%)   ║
║ Total Actions:              ~12,800        ║
║ Predictions:                512            ║
║ Improvements Proposed:      256            ║
║ Jury Sessions:              64             ║
║ Tripwire Alerts:            ~8-16          ║
║ Score Violations:           0              ║
║ Cascading Failures:         0              ║
╠════════════════════════════════════════════╣
║ Avg Agent Throughput:       ~3.2 acts/sec  ║
║ Prediction Accuracy (μ):    0.58           ║
║ Market Settlement Time:     42ms           ║
║ Jury Consensus Rate:        76%            ║
║ Tripwire False Positives:   0              ║
╚════════════════════════════════════════════╝
```

---

## Logging & Forensics

### Scrap Yard Schema
Each agent publishes:
```json
{
  "trial_id": "trial-20260210-001",
  "agent_id": "agent-001",
  "epoch": 3,
  "timestamp": "2026-02-10T12:34:56.789Z",
  "event_type": "IMPROVEMENT_PROPOSAL",
  "capability": "market-analysis",
  "domain": "MARKET",
  "cost": 2.5,
  "current_budget": 47.5,
  "status": "APPROVED",
  "jury": ["agent-005", "agent-012", "agent-029"],
  "votes": {"APPROVE": 3, "REJECT": 0},
  "tripwire_alerts": []
}
```

### Replay Capability
`python scripts/replay_trial.py --trial-id trial-20260210-001 --deterministic`

---

## Risk Mitigation

| Risk | Mitigation | Threshold |
|:---|:---|:---|
| Agent deadlock | Timeout per epoch | 10 sec |
| Budget exhaustion cascade | Circuit breaker | All agents < 5% budget |
| Jury starvation | Expand pool dynamically | Diversity metric < 0.5 |
| Tripwire false alarm spam | Rate limit alerts | Max 1 per 100ms |
| Ledger corruption | Rollback checkpoint | Integrity check per 50 actions |

---

## Next: GPU Integration Point

After this trial passes:
1. **Bind GPU kernels** (Rust FFI → `compile_x3_to_gpu_kernel()`)
2. **Enable X3TaskSpec emission** from agents
3. **Route to GPU executor** for compute-heavy goals
4. **Re-run 32-agent trial** with GPU acceleration

Expected speedup: 10-100x on goal evaluation.

---

## Deliverables

- ✅ Trial execution log (JSON, gzipped)
- ✅ Metrics report (CSV + visualizations)
- ✅ Scrap Yard forensic snapshots
- ✅ Invariant verification certificate
- ✅ Replay tape + reproduction script
- ✅ Go/No-Go decision for Phase 5

