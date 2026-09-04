# Phase 4.1: Run a Small Pilot Session - Execution Guide

**Status:** 🟢 Ready to Execute  
**Date:** 2026-02-08  
**Estimated Duration:** 2-3 days  
**Environment:** Staging  

---

## Quick Start (Fast Path)

```bash
# 1. Deploy staging environment
cd openspec/changes/add-offchain-jury
./deploy.sh staging cpu

# 2. Verify services are healthy
docker-compose ps
docker-compose logs -f jury-service

# 3. Run pilot scenarios (Scenario 1: PASS, Scenario 2: FAIL)
python3 pilot_executor.py --api-url http://localhost:8000 --scenario all

# 4. Analyze results
./analyze_pilot.sh

# 5. Review report
cat pilot-results/pilot-report-*.md
```

---

## Detailed Execution Plan

### Phase 4.1a: Staging Deployment (Day 1, ~30 minutes)

#### Step 1: Deploy Infrastructure

```bash
cd openspec/changes/add-offchain-jury

# Deploy with CPU-only (sufficient for pilot)
./deploy.sh staging cpu

# Expected output:
# ℹ Deploying Jury Service
# ℹ Environment: staging
# ℹ GPU Mode: cpu
# ✓ Docker and Docker Compose found
# ✓ Docker image built successfully
# ✓ Jury service deployed in staging mode
```

#### Step 2: Verify Services

```bash
# Check all services are running and healthy
docker-compose ps

# Expected output (all services should be "Up"):
# NAME              | STATUS           | PORTS
# x3-jury-db    | Up 2 min        | 5432/tcp
# x3-jury-cache | Up 2 min        | 6379/tcp
# jury-service     | Up 1 min        | 8000-8001/tcp

# Verify API health
curl http://localhost:8000/health

# Expected response:
# {
#   "status": "healthy",
#   "version": "1.0.0",
#   "timestamp": "2026-02-08T..."
# }
```

#### Step 3: Enable Monitoring (Optional)

```bash
# Start Prometheus for metrics collection
docker-compose --profile observability up -d jury-metrics

# Verify Prometheus is running
curl http://localhost:9090
# Access dashboard: http://localhost:9090

# Verify metrics are being collected
curl http://localhost:9090/api/v1/query?query=jury_session_total
```

#### Step 4: Database Validation

```bash
# Verify schema is initialized
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c "\dt"

# Expected tables:
expected_tables="audit_logs|jury_sessions|jury_votes|audit_log_seals"

# Check row counts (should be empty initially)
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT 'sessions=' || COUNT(*) FROM jury_sessions; \
   SELECT 'audit_logs=' || COUNT(*) FROM audit_logs;"
```

---

### Phase 4.1b: Pilot Scenario Execution (Day 2, ~1 hour active)

#### Scenario 1: Infrastructure Upgrade (PASS case)

**Purpose:** Validate happy path with 5-member jury voting to PASS

**Timeline:**

```
T+0:00   - Start Scenario 1
T+0:15   - 5 members submit commits
T+0:20   - Transition to reveal phase
T+0:10   - 5 members submit reveals
T+0:35   - Aggregate and verify result (expect: PASS)
T+1:00   - Audit trail verification
```

**Execution:**

```bash
# Run Scenario 1 only
python3 pilot_executor.py \
  --api-url http://localhost:8000 \
  --scenario 1

# Expected output:
# ============================================================
# SCENARIO: Infrastructure Upgrade (PASS)
# ============================================================
# ✅ API health check passed
# ✅ Session created: <session-id>
# ✅ Commitment submitted: INF-001
# ✅ Commitment submitted: INF-002
# ✅ Commitment submitted: OPS-001
# ✅ Commitment submitted: OPS-002
# ✅ Commitment submitted: SEC-001
# ✅ Transitioned to reveal phase
# ✅ Reveal submitted: INF-001 (True)
# ✅ Reveal submitted: INF-002 (True)
# ✅ Reveal submitted: OPS-001 (True)
# ✅ Reveal submitted: OPS-002 (True)
# ✅ Reveal submitted: SEC-001 (False)
# ✅ Votes aggregated
#    YES: 4
#    NO: 1
#    Result: True
# ✅ VERIFICATION PASSED: Result matches expected (True)
```

**Manual Verification (if needed):**

```bash
# Query the database directly to verify
docker exec x3-jury-db psql -U jury_admin -d jury_audit << EOF
-- Check session was created
SELECT COUNT(*) as sessions FROM jury_sessions;

-- Check all votes were recorded
SELECT COUNT(*) as total_votes FROM jury_votes;

-- Check audit events
SELECT COUNT(*) as audit_events FROM audit_logs;

-- Verify vote commits match reveals
SELECT 
  jv.member_id,
  jv.vote,
  jv.reveal_verified
FROM jury_votes jv
ORDER BY jv.member_id;

-- Check final result
SELECT 
  result_yes_votes,
  result_no_votes,
  result_final,
  state
FROM jury_sessions;
EOF
```

#### Scenario 2: Security Policy (FAIL case)

**Purpose:** Validate edge case where vote fails to reach quorum

**Timeline:**

```
T+0:00   - Start Scenario 2
T+0:15   - 3 members submit commits
T+0:20   - Transition to reveal phase
T+0:10   - 3 members submit reveals
T+0:35   - Aggregate and verify result (expect: FAIL)
T+1:00   - Audit trail verification
```

**Execution:**

```bash
# Run Scenario 2 only
python3 pilot_executor.py \
  --api-url http://localhost:8000 \
  --scenario 2

# Expected output:
# ============================================================
# SCENARIO: Security Policy (FAIL)
# ============================================================
# ✅ API health check passed
# ✅ Session created: <session-id>
# ✅ Commitment submitted: SEC-002
# ✅ Commitment submitted: SEC-003
# ✅ Commitment submitted: OPS-003
# ✅ Transitioned to reveal phase
# ✅ Reveal submitted: SEC-002 (False)
# ✅ Reveal submitted: SEC-003 (False)
# ✅ Reveal submitted: OPS-003 (True)
# ✅ Votes aggregated
#    YES: 1
#    NO: 2
#    Result: False
# ✅ VERIFICATION PASSED: Result matches expected (False)
```

#### Run Both Scenarios

```bash
# Run all scenarios in sequence
python3 pilot_executor.py \
  --api-url http://localhost:8000 \
  --scenario all

# This runs Scenario 1, waits 10 seconds, then Scenario 2
# Total execution time: ~2 minutes
```

---

### Phase 4.1c: Data Collection & Analysis (Day 2-3, ~1 hour)

#### Step 1: Enable Detailed Logging

```bash
# View all jury service logs
docker-compose logs -f jury-service --tail 100

# Export logs to file for analysis
docker-compose logs jury-service > pilot-results/service-logs.txt

# View database logs (PostgreSQL)
docker-compose logs jury-db | grep "ERROR\|WARN" > pilot-results/db-logs.txt
```

#### Step 2: Collect Comprehensive Metrics

```bash
# Export database statistics
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT * FROM session_analytics;" \
  > pilot-results/session-analytics.csv

# Export full audit trail
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT session_id, event_type, actor, timestamp, description FROM audit_logs \
   ORDER BY timestamp;" \
  > pilot-results/audit-trail-full.csv

# Export vote records
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT session_id, member_id, vote, reveal_verified, \
          commitment_timestamp, reveal_timestamp \
   FROM jury_votes \
   ORDER BY session_id, member_id;" \
  > pilot-results/votes-export.csv
```

#### Step 3: Run Analysis Script

```bash
# Generate comprehensive analysis report
./analyze_pilot.sh

# This will create:
# - pilot-results/pilot-report-YYYYMMDD_HHMMSS.md (main report)
# - pilot-results/metrics-YYYYMMDD_HHMMSS.csv (metrics data)
```

#### Step 4: Verify Audit Integrity

```bash
# Check SHA256 hashes for tamper detection
docker exec x3-jury-db psql -U jury_admin -d jury_audit << EOF
-- Verify integrity of audit log seals
SELECT 
  session_id,
  content_hash,
  last_verified_at,
  verification_count
FROM audit_log_seals
ORDER BY last_verified_at DESC;

-- Check if hashes match (test tampering detection)
-- In production, this would detect unauthorized modifications
EOF
```

---

## Validation Checklist

After running pilot scenarios, verify:

### ✅ Functional Correctness

- [ ] Scenario 1 completed with PASS result (4/5 = 80%)
- [ ] Scenario 2 completed with FAIL result (1/3 = 33%)
- [ ] All votes correctly committed and revealed
- [ ] Audit events properly recorded (8-10 per session)
- [ ] Audit log integrity verified (hash matches)

### ✅ Performance

- [ ] API responses < 100ms (p95)
- [ ] Database queries < 50ms (99th percentile)
- [ ] CPU usage < 50% average
- [ ] Memory usage < 500MB
- [ ] No timeout events

### ✅ Reliability

- [ ] 100% service uptime (no restarts)
- [ ] 0 API errors logged
- [ ] 0 database errors logged
- [ ] All health checks passed
- [ ] Data consistency verified

### ✅ Security

- [ ] No unauthorized access attempts
- [ ] Audit logs tamper-evident
- [ ] SHA256 hashes verified
- [ ] Role-based access enforced
- [ ] No credential leaks in logs

### ✅ Compliance

- [ ] All test scenarios documented
- [ ] Metrics collected and exported
- [ ] Report generated
- [ ] Recommendations documented

---

## Troubleshooting

### Scenario Fails to Start

```bash
# Check health first
curl http://localhost:8000/health

# Check database connectivity
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c "SELECT 1"

# View service logs
docker-compose logs jury-service --tail 50
```

### Votes Don't Verify

```bash
# Check if commitment hash is stored
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT member_id, commitment_hash FROM jury_votes WHERE reveal_verified = false;"

# Manually verify a commitment
python3 << 'PYTHON'
import hashlib
vote = "1"  # or "0" for False
nonce = "your-nonce-here"
computed_hash = hashlib.sha256(f"{vote}|{nonce}".encode()).hexdigest()
print(f"Computed hash: {computed_hash}")
PYTHON
```

### High Latency

```bash
# Check database load
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT query, mean_time FROM pg_stat_statements \
   ORDER BY mean_time DESC LIMIT 5;"

# Check connection pool
docker exec x3-jury-db psql -U jury_admin -d jury_audit -c \
  "SELECT count(*) FROM pg_stat_activity;"
```

### Service Crashes

```bash
# Check container logs for stack traces
docker-compose logs jury-service | tail -100

# Check if out of memory
docker stats jury-service

# Increase memory limit in docker-compose.yml if needed
```

---

## Output Files

After running pilot scenarios, expect these files:

```
openspec/changes/add-offchain-jury/
├── pilot-results/
│   ├── pilot-report-20260208_HHMMSS.md     (main analysis report)
│   ├── metrics-20260208_HHMMSS.csv          (metrics data)
│   ├── session-analytics.csv                (database export)
│   ├── audit-trail-full.csv                 (all audit events)
│   ├── votes-export.csv                     (vote records)
│   ├── service-logs.txt                     (application logs)
│   └── db-logs.txt                          (database warnings/errors)
```

---

## Report Sections

The generated `pilot-report-*.md` contains:

1. **Executive Summary** - High-level results
2. **Test Execution Summary** - Scenarios run, session counts
3. **Performance Analysis** - Latency, resource usage
4. **Audit Trail Analysis** - Event distribution, integrity
5. **Voting Pattern Analysis** - Vote distribution, outcomes
6. **Quorum & Diversity Analysis** - Jury composition verification
7. **Error Analysis** - Any issues encountered
8. **Recommendations** - Suggested improvements
9. **Conclusion** - Go/no-go decision

---

## Success Criteria

**Pilot is SUCCESSFUL if:**

✅ Both scenarios execute without errors  
✅ Vote aggregation produces correct results  
✅ All votes verified against commitments  
✅ Audit trail complete and integrity verified  
✅ API latencies acceptable (< 100ms p95)  
✅ No consistency errors in database  
✅ 100% uptime during test period  

**Pilot is MARGINAL if:**

⚠️ Minor performance issues but all tests pass  
⚠️ Database query optimization needed  
⚠️ Small log inconsistencies  

**Pilot FAILS if:**

❌ Scenario votes don't aggregate correctly  
❌ Audit logs incomplete or corrupted  
❌ Database inconsistencies detected  
❌ Repeated service crashes  
❌ Security audit log violations  

---

## Phase 4.2: Next Steps (Iteration)

After pilot completion and analysis:

1. **Review findings** - Study the analysis report
2. **Address issues** - If needed (Phase 4.2a)
3. **Collect telemetry** - From audit logs
4. **Design refinements** - Document changes
5. **Prepare for Phase 4.3** - Archive and production handoff

---

## References

- [PILOT_PLAN.md](PILOT_PLAN.md) - Detailed test plan
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
- [USAGE.md](USAGE.md) - API documentation
- [design.md](design.md) - Voting protocol specs

---

## Contacts & Support

For issues during pilot execution:
- Check [DEPLOYMENT.md](DEPLOYMENT.md) troubleshooting section
- Review service logs: `docker-compose logs -f jury-service`
- Query database directly for state inspection
- Create issue if blockers found

---

**Status:** 🟢 Ready for Pilot Execution  
**Estimated Duration:** 2-3 days  
**Expected Output:** Comprehensive analysis report with go/no-go recommendation  

