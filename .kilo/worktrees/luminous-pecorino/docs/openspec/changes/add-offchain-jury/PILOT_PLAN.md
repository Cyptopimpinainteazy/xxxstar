# Jury Service Pilot Testing Plan

**Phase:** 4.1 - Run a small pilot session (staging)  
**Date:** 2026-02-08  
**Environment:** Staging (pre-production)  
**Duration:** 2-3 days  
**Participants:** 5-member pilot jury  

---

## Executive Summary

This pilot validates the jury system under realistic conditions with:
- **5-member staged jury** from diverse sections (2 engineering, 2 operations, 1 security)
- **2 concurrent jury sessions** simulating real workload
- **Continuous monitoring** of performance, correctness, and audit integrity
- **Telemetry collection** for optimization and bottleneck identification

---

## Pilot Objectives

### Primary Objectives
1. ✅ Validate commit-reveal voting protocol correctness
2. ✅ Verify quorum enforcement (66% majority + 3-member minimum)
3. ✅ Confirm section diversity constraints (75% cap per section)
4. ✅ Test audit logging accuracy and integrity verification
5. ✅ Validate on-chain anchor integration stubs

### Secondary Objectives
6. Performance: API latency < 500ms for 95th percentile
7. Database: Query times < 50ms for audit lookups
8. Reliability: 100% uptime during 48-hour pilot
9. Scalability: Support 10+ concurrent jury sessions
10. Security: No unauthorized access attempts in audit logs

---

## Pilot Jury Configuration

### Session 1: "Infrastructure Upgrade" (PASS scenario)
**Objective:** 5-member jury votes on infrastructure upgrade (should pass)

**Jury Composition:**
- INF-001 (Engineering A) - Section: Infrastructure
- INF-002 (Engineering B) - Section: Infrastructure  
- OPS-001 (Operations) - Section: Operations
- SEC-001 (Security) - Section: Security
- OPS-002 (Operations) - Section: Operations

**Section Distribution:**
- Infrastructure: 2/5 = 40% ✅ (within 75% cap)
- Operations: 2/5 = 40% ✅ (within 75% cap)
- Security: 1/5 = 20% ✅ (within 75% cap)

**Vote Prediction:**
- Expected YES votes: 4 (INF-001, INF-002, OPS-001, OPS-002)
- Expected NO votes: 1 (SEC-001)
- Expected result: PASS (4/5 = 80% > 66% threshold) ✅

**Timeouts:**
- Commit phase: 5 minutes
- Reveal phase: 10 minutes
- Total session: ~20 minutes

---

### Session 2: "Security Policy" (FAIL scenario)
**Objective:** 3-member jury votes on security policy (should fail)

**Jury Composition:**
- SEC-002 (Security A) - Section: Security
- SEC-003 (Security B) - Section: Security
- OPS-003 (Operations) - Section: Operations

**Section Distribution:**
- Security: 2/3 = 67% ✅ (within 75% cap, but noted)
- Operations: 1/3 = 33% ✅ (within 75% cap)

**Vote Prediction:**
- Expected YES votes: 1 (OPS-003)
- Expected NO votes: 2 (SEC-002, SEC-003)
- Expected result: FAIL (1/3 = 33% < 66% threshold) ✅

**Timeouts:**
- Commit phase: 5 minutes
- Reveal phase: 10 minutes
- Total session: ~20 minutes

---

## Test Scenarios

### Scenario A: Happy Path (Session 1)

```
Timeline:
├─ T+0:00   Create session (5 members, infrastructure upgrade task)
├─ T+0:10   INF-001 submits commit: SHA256(TRUE|nonce-001)
├─ T+0:15   INF-002 submits commit: SHA256(TRUE|nonce-002)
├─ T+0:20   OPS-001 submits commit: SHA256(TRUE|nonce-003)
├─ T+0:25   SEC-001 submits commit: SHA256(FALSE|nonce-004)
├─ T+0:30   OPS-002 submits commit: SHA256(TRUE|nonce-005)
├─ T+5:00   Commit deadline reached → Transition to reveal phase
├─ T+5:05   INF-001 reveals vote: (TRUE, nonce-001) ✓ commitment verified
├─ T+5:10   INF-002 reveals vote: (TRUE, nonce-002) ✓ commitment verified
├─ T+5:15   OPS-001 reveals vote: (TRUE, nonce-003) ✓ commitment verified
├─ T+5:20   SEC-001 reveals vote: (FALSE, nonce-004) ✓ commitment verified
├─ T+5:25   OPS-002 reveals vote: (TRUE, nonce-005) ✓ commitment verified
├─ T+5:30   Aggregate votes: 4 YES, 1 NO → PASS (80% > 66%)
└─ T+10:00  Session complete + audit log sealed
```

**Success Criteria:**
- ✅ All commits accepted without errors
- ✅ Reveal phase transition successful
- ✅ All reveals verified against commits
- ✅ Quorum check: 5 votes ≥ 3 members minimum ✓
- ✅ Majority check: 4/5 = 80% ≥ 66% ✓
- ✅ Final result: PASS (TRUE)
- ✅ Audit trail: 8+ events recorded (CREATE, 5xCOMMIT, REVEAL_PHASE_ADVANCED, votes revealed, VOTES_AGGREGATED, SESSION_COMPLETED)

### Scenario B: Quorum Failure

```
Timeline (simulated failure):
├─ T+0:00   Create session (5 members)
├─ T+0:10   Member commits made (3 members only, 2 no-shows)
├─ T+5:00   Commit deadline reached
├─ T+5:05   Only 3 reveals submitted
├─ T+10:00  Aggregate votes: 3/5 members < 3 minimum? 
            WAIT - minimum is "3 minimum" so 3/5 qualifies!
            3 YES, 0 NO → PASS (100%)
└─ T+10:00  Session complete
```

**Adjusted Scenario B: Non-Quorum Test**
Actually use 5-member jury but ensure result is < 66%:
- 2 YES, 3 NO → FAIL (40% < 66%) ✓

---

### Scenario C: Audit Integrity Verification

```
Test: Can we verify audit logs haven't been tampered with?

Actions:
1. Record SHA256 hash after session completion
2. Wait 1 hour
3. Recompute hash from audit_logs table
4. Compare hashes
5. Verify: Hashes match = logs untampered ✓
```

**Success Criteria:**
- ✅ Original hash == recomputed hash
- ✅ Tampering detection: If logs modified, hash mismatch detected
- ✅ All events present in audit trail

---

## Monitoring & Instrumentation

### Metrics to Collect

**Performance Metrics:**
```
- API endpoint latencies (p50, p95, p99)
  * POST /api/jury/session: target < 100ms
  * POST /api/jury/vote (commit): target < 50ms
  * POST /api/jury/vote (reveal): target < 50ms
  * GET /api/jury/session/{id}: target < 50ms

- Database query times
  * insert audit_log: target < 10ms
  * select audit_logs WHERE session_id: target < 20ms
  * verify_log_integrity: target < 100ms

- Container metrics
  * CPU usage % (target: < 50% average)
  * Memory usage MB (target: < 500MB)
  * Disk I/O mb/s (target: < 50 mb/s)
```

**Correctness Metrics:**
```
- Vote accuracy
  * Commits stored correctly: 100%
  * Reveals verified correctly: 100%
  * Aggregation calculation matches: 100%

- Audit logging
  * Events recorded per session: 8-10 expected
  * Event timestamp ordering: monotonic increasing
  * SHA256 hashes: deterministic and reproducible

- Quorum enforcement
  * Sessions properly rejected if < 3 members: test
  * Majority threshold: 66% enforced: test
- Section diversity
  * < 75% per section cap enforced: test
  * Distribution tracked accurately: verify
```

**Reliability Metrics:**
```
- Error rates
  * API errors: 0
  * Database errors: 0
  * Timeout events: 0

- Availability
  * Service uptime: 100% over 48 hours
  * Health check success rate: 100%

- Data integrity
  * Audit log consistency: verified
  * No missing events: confirmed
  * State machine correctness: tested
```

---

## Monitoring Setup

### Prometheus Scrapes

Enable Prometheus in docker-compose:
```bash
docker-compose --profile observability up -d jury-metrics
```

**Queries to Run:**

1. **API Latency (p95):**
   ```promql
   histogram_quantile(0.95, jury_api_request_duration_seconds)
   ```

2. **Active Sessions:**
   ```promql
   jury_session_active{environment="staging"}
   ```

3. **Database Connection Pool:**
   ```promql
   jury_db_pool_available
   ```

4. **Error Rate:**
   ```promql
   rate(jury_api_errors_total[5m])
   ```

### Log Aggregation

View real-time logs:
```bash
docker-compose logs -f jury-service | jq .

# Filter by session
docker-compose logs jury-service | jq 'select(.session_id=="...")'

# Filter by event type
docker-compose logs jury-service | jq 'select(.event_type=="VOTE_REVEALED")'
```

### Health Checks

Periodic health verification:
```bash
# Query API health
curl -s http://localhost:8000/health | jq .

# Query database
docker exec x3-jury-db psql -U jury_admin -d jury_audit \
  -c "SELECT COUNT(*) FROM jury_sessions;"

# Query audit logs
docker exec x3-jury-db psql -U jury_admin -d jury_audit \
  -c "SELECT COUNT(*) FROM audit_logs;"
```

---

## Pilot Execution Steps

### Day 1: Setup & Validation

**1. Deploy Staging Environment**
```bash
./deploy.sh staging cpu
# Verify all services healthy
docker-compose ps
```

**2. Verify Schema**
```bash
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c "\dt"
# Expected: 4 tables (audit_logs, jury_sessions, jury_votes, audit_log_seals)
```

**3. Run Pre-Flight Tests**
```bash
# Test database connectivity
curl -s http://localhost:8000/health | jq .

# Verify metrics collection
curl -s http://localhost:9090/api/v1/query?query=up | jq .
```

**4. Enable Monitoring**
```bash
# Start Prometheus
docker-compose --profile observability up -d jury-metrics

# Verify Prometheus scraping
curl http://localhost:9090/metrics
```

### Day 2: Pilot Execution

**1. Session 1: Infrastructure Upgrade (PASS)**
```bash
# Create piloted jury session
curl -X POST http://localhost:8000/api/jury/session \
  -H "Content-Type: application/json" \
  -d '{
    "task_ids": ["INFRA-UPGRADE-001"],
    "members": [
      {"agent_id": "INF-001", "section": "infrastructure", "is_on_chain": false},
      {"agent_id": "INF-002", "section": "infrastructure", "is_on_chain": false},
      {"agent_id": "OPS-001", "section": "operations", "is_on_chain": false},
      {"agent_id": "OPS-002", "section": "operations", "is_on_chain": false},
      {"agent_id": "SEC-001", "section": "security", "is_on_chain": false}
    ],
    "commit_deadline_seconds": 300,
    "reveal_deadline_seconds": 600
  }'
# Record session_id: SESSION-001
```

**2. Members Submit Commits**
```bash
# (Show examples - 5 commit requests similar to above)
# Each member submits SHA256(vote|nonce)
```

**3. Transition to Reveal**
```bash
curl -X POST http://localhost:8000/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "SESSION-001",
    "type": "advance"
  }'
```

**4. Members Reveal Votes**
```bash
# (Show examples - 5 reveal requests)
# Each member reveals actual vote and nonce (verified against commit)
```

**5. Aggregate Results**
```bash
curl -X POST http://localhost:8000/api/jury/vote \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "SESSION-001",
    "type": "aggregate"
  }'
# Expected result: {"yes": 4, "no": 1, "result": true}
```

**6. Verify Audit Trail**
```bash
curl http://localhost:8000/api/jury/session/SESSION-001
# Verify all events recorded in audit_trail
```

**7. Session 2: Security Policy (FAIL)**
```bash
# Repeat similar process with 3-member jury
# Expected result: 1 YES, 2 NO → FAIL
```

### Day 3: Analysis & Iteration

**1. Collect Metrics**
```bash
# Export Prometheus metrics
curl http://localhost:9090/api/v1/query_range \
  ?query=jury_api_request_duration_seconds \
  &start=... &end=... &step=60s | jq .

# Export database audit logs
docker exec x3-jury-db psql -U jury_admin -d jury_audit \
  -c "SELECT * FROM audit_logs ORDER BY timestamp;" \
  >> audit-logs-export.csv
```

**2. Analyze Results**
- API latencies achieved?
- Database performance acceptable?
- Audit trail complete and verifiable?
- No errors or timeouts?

**3. Document Findings**
- What worked well
- Bottlenecks identified
- Recommended optimizations
- Design changes needed

---

## Success Criteria

### Functional Correctness ✅
- [ ] Session 1 completes with PASS result (4/5 = 80%)
- [ ] Session 2 completes with FAIL result (1/3 = 33%)
- [ ] All 10 votes properly committed and revealed
- [ ] All audit events recorded (8+ per session minimum)
- [ ] Audit integrity verification succeeds (hash match)

### Performance ✅
- [ ] API response times < 100ms (p95)
- [ ] Database queries < 50ms (99th percentile)
- [ ] CPU usage < 50% average
- [ ] Memory usage < 500MB peak
- [ ] No timeouts during 48-hour window

### Reliability ✅
- [ ] 100% uptime across both sessions
- [ ] 0 API errors
- [ ] 0 database errors
- [ ] All health checks passing
- [ ] No unexpected service restarts

### Security ✅
- [ ] No unauthorized access attempts
- [ ] Audit logs tamper-evident
- [ ] SHA256 hashes verified
- [ ] Role-based access enforced
- [ ] No credential leaks in logs

---

## Deliverables

Upon completion:
1. ✅ Pilot test results summary
2. ✅ Performance metrics report
3. ✅ Audit log analysis
4. ✅ Recommendation document
5. ✅ Go/no-go decision for production

---

## References

- [USAGE.md](USAGE.md) - API examples
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
- [design.md](design.md) - Voting protocol specs
- [severity-taxonomy.md](../specs/orchestra-governance/severity-taxonomy.md) - Task classification

---

**Prepared by:** GitHub Copilot  
**Date:** 2026-02-08  
**Status:** Ready for Pilot Execution
