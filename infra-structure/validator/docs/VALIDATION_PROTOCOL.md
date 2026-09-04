# Infenstructior Validation Protocol v1.0

## Purpose

This protocol defines the formal validation requirements for validator
fallback protection in the X3/Infenstructior system. It ensures that
all fallback mechanisms are tested, verified, and auditable before
mainnet deployment.

---

## 1. Scope

This protocol applies to all validators using X3 as a GPU-accelerated
superhighway. It covers:

- Process supervision (Watchdog)
- Hot standby failover (StandbyManager)
- Multi-region cluster failover (ClusterCoordinator)
- Health scoring and lane orchestration
- Signer lock and fencing tokens
- Degraded mode operation

---

## 2. Validation Levels

### Level 1 — Unit Validation
Each component is tested in isolation with mocked dependencies.

**Required for:** All tiers (Bronze, Silver, Gold, Platinum)

**Gates:**
- [ ] All unit tests pass
- [ ] Code coverage ≥ 80%
- [ ] No TODO/FIXME in production code
- [ ] All public APIs documented

### Level 2 — Integration Validation
Components are tested together with real (or containerized) dependencies.

**Required for:** Silver, Gold, Platinum

**Gates:**
- [ ] Watchdog + StandbyManager integration passes
- [ ] StandbyManager + ClusterCoordinator integration passes
- [ ] SignerLock works across processes
- [ ] Redis-based heartbeat system works
- [ ] Lane failover triggers correctly

### Level 3 — System Validation
Full system test with real validator binary.

**Required for:** Gold, Platinum

**Gates:**
- [ ] Primary crash → standby promotion works end-to-end
- [ ] Network partition → leader election works
- [ ] Split-brain → resolution works
- [ ] Degraded mode → CPU-only operation works
- [ ] Recovery after failover works

### Level 4 — Adversarial Validation
System is tested under adversarial conditions.

**Required for:** Platinum

**Gates:**
- [ ] Lane flood test passes
- [ ] GPU crash test passes
- [ ] Determinism attack test passes
- [ ] Network chaos test passes
- [ ] All adversarial scenarios pass without data loss

---

## 3. Invariants

### INV-FALLBACK-001: Process Supervision
```
Given: Validator process is running under Watchdog
When:  Process crashes (SIGKILL)
Then:  Watchdog restarts within 60s
       Restart event is logged
       Exponential backoff is respected
```

### INV-FALLBACK-002: Hot Standby Failover
```
Given: Primary and Standby are running
When:  Primary becomes unreachable for > 15s
Then:  Standby acquires SignerLock
       Standby promotes to primary
       No double-signing occurs
       Failover completes within 20s
```

### INV-FALLBACK-003: Cluster Leader Election
```
Given: Cluster has N nodes (N ≥ 3)
When:  Leader becomes unreachable
Then:  Election is triggered within 15s
       New leader is elected with quorum (≥ 51%)
       SignerLock is transferred to new leader
       No split-brain persists > 30s
```

### INV-FALLBACK-004: Split-Brain Resolution
```
Given: Two nodes claim leadership
When:  Split-brain is detected
Then:  Leader with highest term wins
       False leader steps down
       Cluster returns to STABLE state
```

### INV-FALLBACK-005: Fencing Token Monotonicity
```
Given: SignerLock is acquired
When:  Lock is released and re-acquired
Then:  Fencing token is monotonically increasing
       Stale token cannot sign
```

### INV-FALLBACK-006: Degraded Mode Liveness
```
Given: All GPU lanes are unavailable
When:  Degraded mode is activated
Then:  CPU-only lane serves requests
       No signing occurs in degraded mode
       Recovery is attempted every 30s
```

### INV-FALLBACK-007: Memory Limit Enforcement
```
Given: Validator exceeds memory limit
When:  MemoryMonitor detects violation
Then:  Process is killed with SIGKILL
       Watchdog logs the event
       Process is restarted (if within limits)
```

### INV-FALLBACK-008: Health Score Composite
```
Given: Health daemon is running
When:  Any health component degrades
Then:  Composite score reflects degradation
       Lane failover triggers at threshold
       Event is logged
```

---

## 4. Test Suites

### 4.1 Unit Tests

```bash
# Run all unit tests
cd infra-structure/validator
python -m pytest tests/test_resilience.py -v

# Run specific test classes
python -m pytest tests/test_resilience.py::TestWatchdog -v
python -m pytest tests/test_resilience.py::TestStandbyManager -v
python -m pytest tests/test_resilience.py::TestClusterCoordinator -v
```

### 4.2 Integration Tests

```bash
# Run integration tests (requires Redis)
python -m pytest tests/test_resilience.py::TestWatchdogIntegration -v
python -m pytest tests/test_resilience.py::TestStandbyIntegration -v
python -m pytest tests/test_resilience.py::TestClusterIntegration -v
```

### 4.3 Adversarial Tests

```bash
# Run adversarial stress tests
python tests/adversarial/test_lane_flood.py
python tests/adversarial/test_gpu_crash.py
python tests/adversarial/test_determinism_attack.py
python tests/adversarial/test_network_chaos.py
```

### 4.4 Full Validation Suite

```bash
# Run all validation levels
python scripts/run_validation.py --level 4
```

---

## 5. Validation Report

Each validation run produces a report:

```json
{
  "protocol_version": "1.0",
  "timestamp": "2026-06-10T00:00:00Z",
  "tier": "gold",
  "validation_level": 3,
  "results": {
    "unit": {"pass": true, "tests": 45, "failures": 0},
    "integration": {"pass": true, "tests": 12, "failures": 0},
    "system": {"pass": true, "tests": 6, "failures": 0},
    "adversarial": {"pass": null, "tests": 0, "failures": 0}
  },
  "invariants": {
    "INV-FALLBACK-001": "pass",
    "INV-FALLBACK-002": "pass",
    "INV-FALLBACK-003": "pass",
    "INV-FALLBACK-004": "pass",
    "INV-FALLBACK-005": "pass",
    "INV-FALLBACK-006": "pass",
    "INV-FALLBACK-007": "pass",
    "INV-FALLBACK-008": "pass"
  },
  "overall": "PASS"
}
```

---

## 6. Sign-off Requirements

### Bronze
- [ ] Level 1 validation passes
- [ ] All INV-FALLBACK-001, 007, 008 pass

### Silver
- [ ] Level 1 + Level 2 validation passes
- [ ] All INV-FALLBACK-001 through 008 pass
- [ ] Failover time < 15s verified

### Gold
- [ ] Level 1 + Level 2 + Level 3 validation passes
- [ ] All INV-FALLBACK-001 through 008 pass
- [ ] Cluster election verified with ≥ 3 nodes
- [ ] Split-brain resolution verified

### Platinum
- [ ] Level 1 + Level 2 + Level 3 + Level 4 validation passes
- [ ] All INV-FALLBACK-001 through 008 pass
- [ ] Adversarial scenarios pass
- [ ] Global topology verified
- [ ] GPU kernel attestation verified

---

## 7. Continuous Validation

Validation is run automatically on:
- Every PR that touches `infra-structure/validator/`
- Every release candidate
- Weekly full validation suite

Results are posted to:
- GitHub Actions summary
- #validator-releases Slack channel
- `docs/reports/VALIDATION_STATUS.md`

---

## 8. Exception Process

If an invariant cannot be satisfied:

1. File an exception request in `docs/exceptions/`
2. Describe the invariant, the failure, and the mitigation
3. Get approval from:
   - Bronze: Lead developer
   - Silver: Lead developer + Tech lead
   - Gold: Lead developer + Tech lead + Security
   - Platinum: Full engineering consensus
4. Exception expires after 30 days
5. Exception must be resolved before next release

---

## Appendix A: Validation Scripts

```bash
# Quick validation (Level 1)
python scripts/validate.py --level 1

# Full validation (Level 4)
python scripts/validate.py --level 4 --tier platinum

# Validate specific invariant
python scripts/validate.py --invariant INV-FALLBACK-002
```

## Appendix B: Environment Requirements

| Validation Level | Redis | GPU | Multi-Machine | Duration |
|-----------------|-------|-----|---------------|----------|
| 1 (Unit)        | No    | No  | No            | < 1 min  |
| 2 (Integration) | Yes   | No  | No            | < 5 min  |
| 3 (System)      | Yes   | Yes | No            | < 30 min |
| 4 (Adversarial) | Yes   | Yes | Yes           | < 2 hr   |
