# Phase 5 Operations Runbook

Complete operational procedures for running Jury Blockchain Anchoring in production.

## Table of Contents

- [Startup Procedures](#startup-procedures)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)
- [Maintenance](#maintenance)
- [Incident Response](#incident-response)
- [Disaster Recovery](#disaster-recovery)

---

## Startup Procedures

### Cold Start (First Time)

```bash
# 1. Verify prerequisites
./scripts/health-check-phase5.sh

# 2. Create environment file
cp .env.example .env.production
# Edit .env.production with actual values

# 3. Build and start services
./scripts/deploy-phase5.sh production cpu

# 4. Wait for blockchain sync (5-10 minutes)
watch 'curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"system_health\",\"params\":[],\"id\":1}" | jq'

# 5. Run functional tests
pytest tests/test_jury_anchoring.py::TestJuryAnchoringEndToEnd -v

# 6. Verify anchoring works
python examples/example_jury_anchor_python.py
```

### Warm Start (Normal Startup)

```bash
# Start all services
docker-compose -f docker-compose.production.yml up -d

# Wait for readiness
sleep 30

# Quick health check
./scripts/health-check-phase5.sh --quick

# Verify no errors in logs
docker-compose logs --tail=50 jury-anchorer | grep ERROR
```

### Graceful Shutdown

```bash
# 1. Stop accepting new decisions
curl -X POST http://localhost:8080/admin/pause

# 2. Wait for in-flight anchors to complete
sleep 30

# 3. Stop services
docker-compose -f docker-compose.production.yml down

# 4. Backup database
pg_dump -h localhost jury_db > backup_$(date +%Y%m%d_%H%M%S).sql
```

---

## Monitoring

### Real-Time Metrics

```bash
# Watch decision anchor rate
watch -n 5 'curl -s http://localhost:9090/api/v1/query?query=jury_anchor_rate_5m'

# Monitor blockchain block height
watch -n 10 'curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"chain_getBlock\",\"params\":[],\"id\":1}" | jq'

# Check service latency
watch -n 5 'curl -s http://localhost:9090/api/v1/query?query=jury_anchor_latency_p95'
```

### Key Metrics to Monitor

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| Anchor Success Rate | >99% | <95% |
| Verification Latency (p95) | <200ms | >500ms |
| Anchor Latency (p95) | <5s | >10s |
| RPC Node CPU | <50% | >80% |
| Database Connections | <10 | >20 |
| Disk Usage | <50% | >80% |

### Prometheus Query Examples

```promql
# anchor success rate (5 minute window)
rate(jury_anchor_success_total[5m]) / rate(jury_anchor_attempts_total[5m])

# P95 anchor latency
histogram_quantile(0.95, jury_anchor_duration_seconds)

# Verification failures
rate(jury_verify_failures_total[5m])

# Database connection pool usage
jury_db_connections / jury_db_max_connections
```

---

## Troubleshooting

### Issue: Anchor Latency High (>10 seconds)

**Symptoms:** `jury_anchor_latency_p95 > 10s`

**Diagnosis:**
```bash
# 1. Check RPC node health
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' | jq

# 2. Check node sync status
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_syncState","params":[],"id":1}' | jq
```

**Resolution:**
- If node is not synced: Wait for sync completion
- If RPC is slow: Increase RPC thread pool size
- If database is slow: Check PostgreSQL CPU/memory usage

### Issue: Verification Hash Mismatch

**Symptoms:** `jury_verify_mismatches_total` increasing

**Diagnosis:**
```bash
# 1. Get recent decision
SESSION_ID="session-20260208-001"
curl -s http://localhost:8080/api/anchor/$SESSION_ID/data

# 2. Verify hash locally
python3 << 'EOF'
import json
import hashlib

# Get from service
data = {"session_id": "session-20260208-001", "result": "PASS"}

# Compute hash both ways
hash_v1 = hashlib.sha256(json.dumps(data).encode()).hexdigest()
print(f"Service hash: {hash_v1}")

# Compare with on-chain
EOF

# 3. Query blockchain
./scripts/verify_jury_decision.sh $SESSION_ID $HASH
```

**Resolution:**
- If hashes don't match: Check vote aggregation logic
- If it's time-dependent: Check system time sync
- If isolated incident: File issue with session details

### Issue: Anchorer Service Stuck

**Symptoms:** No new anchors for >5 minutes

**Diagnosis:**
```bash
# 1. Check service logs
docker-compose logs -f jury-anchorer --tail=100

# 2. Check for database locks
psql -h localhost jury_db -U jury_admin -c \
  "SELECT * FROM pg_locks WHERE NOT granted;"

# 3. Check RPC connectivity
curl -v http://localhost:9944

# 4. Check service status
curl http://localhost:8081/health
```

**Resolution:**
- If stuck on RPC: Restart blockchain node
- If database locked: Kill blocking query
- If service unresponsive: Restart anchorer

```bash
docker-compose restart jury-anchorer
```

### Issue: Database Connection Pool Exhausted

**Symptoms:** `jury_db_connections >= jury_db_max_connections`

**Diagnosis:**
```bash
psql -h localhost jury_db -U jury_admin -c \
  "SELECT datname, count(*) FROM pg_stat_activity GROUP BY datname;"
```

**Resolution:**
1. Identify slow queries:
```bash
psql -h localhost jury_db -U jury_admin -c \
  "SELECT query, query_start FROM pg_stat_activity WHERE state = 'active';"
```

2. Kill idle connections:
```bash
psql -h localhost jury_db -U jury_admin -c \
  "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
   WHERE datname = 'jury_db' AND state = 'idle';"
```

3. Scale connections in docker-compose:
```yaml
services:
  jury-service:
    environment:
      DB_POOL_SIZE: 20  # Increase from default
```

---

## Maintenance

### Daily Tasks

```bash
# Morning check
./scripts/health-check-phase5.sh

# Monitor logs for errors
docker-compose logs -f --tail=200 | grep ERROR

# Check disk space
df -h /var/lib/docker/volumes/*jury*

# Verify anchor rate
curl -s http://localhost:9090/api/v1/query?query=rate(jury_anchor_attempts_total[24h])
```

### Weekly Tasks

```bash
# Backup database
pg_dump jury_db > backups/jury_db_$(date +%Y%m%d).sql

# Verify backups
ls -lh backups/ | tail -10

# Review error logs
docker logs jury-service 2>&1 | grep ERROR | tail -20

# Check disk usage growth
du -sh /var/lib/docker/volumes/*jury*

# Test disaster recovery procedure (staging only)
# ./scripts/backup-restore-test.sh
```

### Monthly Tasks

```bash
# Review performance metrics
# (captured in Grafana yearly report)

# Update dependencies
cargo update --workspace
pip list --outdated

# Review security advisories
cargo audit
pip-audit

# Full backup
./scripts/backup-full.sh

# Test upgrades in staging
./scripts/test-upgrade-staging.sh
```

### Quarterly Tasks

```bash
# Full disaster recovery test
./scripts/test-full-disaster-recovery.sh

# Capacity planning review
# (based on growth metrics)

# Dependency updates
cargo update --aggressive
pip install --upgrade pip setuptools

# Performance optimization review
# (analyze Prometheus data)
```

---

## Incident Response

### P1 Incident: Service Down

**Definition:** No anchors for >5 minutes

**Response:**
1. Page on-call engineer immediately
2. Assess which component failed:
   - RPC: `curl http://localhost:9944`
   - Service: `curl http://localhost:8080/health`
   - Database: `psql -h localhost jury_db -c "SELECT 1"`
3. Implement immediate fix or rollback
4. Notify stakeholders
5. Run post-mortem within 24h

**Rollback:**
```bash
docker-compose down
git revert <commit-hash>
./scripts/deploy-phase5.sh production
```

### P2 Incident: High Latency (>10s)

**Definition:** Anchor latency increasing persistently

**Response:**
1. Identify bottleneck (see Troubleshooting)
2. Scale problematic component:
   - RPC thread pool: Restart with more threads
   - Database: Increase connection pool
   - Network: Check bandwidth
3. Monitor for recovery
4. Schedule permanent fix

### P3 Incident: Hash Mismatches

**Definition:** >1% verification failure rate

**Response:**
1. Investigate specific session
2. Determine root cause (voting logic vs. hashing)
3. Apply fix in staging first
4. Roll out to production
5. Reanchor affected decisions

---

## Disaster Recovery

### Complete Data Loss Recovery

**Time to Recovery: 30 minutes (with backup)**

```bash
# 1. Stop services
docker-compose down

# 2. Restore database from backup
# Pick most recent backup
BACKUP_FILE="backups/jury_db_20260208.sql"

# Create new database
createdb -h localhost jury_db

# Restore
psql -h localhost jury_db < $BACKUP_FILE

# 3. Rebuild blockchain from snapshot (if available)
./scripts/restore-blockchain-snapshot.sh

# 4. Restart services
docker-compose up -d

# 5. Verify
./scripts/health-check-phase5.sh
```

### Partial Data Loss Recovery

**If database is corrupted but blockchain is fine:**

```bash
# 1. Stop anchorer (keep blockchain running)
docker-compose stop jury-anchorer

# 2. Query blockchain for all decisions
python3 << 'EOF'
import requests

RPC_URL = "http://localhost:9944"

# Query all jury decisions from blockchain
result = requests.post(RPC_URL, json={
    "jsonrpc": "2.0",
    "method": "query.atlasJuryAnchor.juryDecisions",
    "params": [],
    "id": 1
}).json()

for decision in result.get("decisions", []):
    print(decision)
EOF

# 3. Rebuild database from blockchain
python3 scripts/rebuild-from-blockchain.py

# 4. Restart all services
docker-compose up -d
```

---

## Contact & Escalation

**On-Call Schedule:** See Slack #jury-oncall  
**Escalation:** Page @jury-lead if no response in 15min  
**Post-Mortems:** Every incident generates postmortem in #jury-incidents  
**Status Page:** https://status.x3.io/jury-phase5  

---

**Last Updated:** 2026-02-08  
**Maintained by:** X3 Operations Team
