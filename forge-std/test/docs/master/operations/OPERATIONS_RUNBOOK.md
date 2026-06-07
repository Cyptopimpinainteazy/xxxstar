# Phase 5 Operations Runbook

## Quick Reference

### Health Check
```bash
./scripts/health-check.sh
```

### View Logs
```bash
docker-compose logs -f jury-anchorer
docker-compose logs -f blockchain-node
docker-compose logs -f jury-service
```

### Restart Service
```bash
docker-compose restart jury-anchorer
```

### Emergency: Rollback
```bash
./scripts/rollback.sh
```

---

## Daily Tasks

### Morning Starting Procedures (8:00 AM)

1. **Check System Status** (5 min)
   ```bash
   ./scripts/health-check.sh
   ```
   - All services running? ✓
   - Database connected? ✓
   - No critical alerts? ✓

2. **Review Overnight Logs** (10 min)
   ```bash
   docker-compose logs jury-anchorer --since 12h | grep ERROR
   ```
   - Any errors overnight? 
   - Any failed anchoring operations?
   - Any database issues?

3. **Check Metrics** (5 min)
   - Open: http://localhost:3000 (Grafana)
   - Check dashboard: "Phase 5 Jury Anchoring"
   - Review metrics:
     - Anchor latency (target: <5s)
     - Success rate (target: >99%)
     - Decision count (should increase)

4. **Notify Team** (2 min)
   - Post in #operations: "✓ Jury Phase 5 operational"
   - Or alert if issues found

### Hourly Monitoring (During Business Hours)

1. **Every hour during 9 AM - 6 PM:**
   ```bash
   ./scripts/health-check.sh && echo "OK at $(date)"
   ```

2. **Watch for alerts:**
   - Monitoring dashboard
   - Email notifications
   - Slack bot (if configured)

3. **If errors:** Escalate to on-call engineer

### End of Day Shutdown (6:00 PM)

1. **Final health check**
   ```bash
   ./scripts/health-check.sh
   ```

2. **Backup database**
   ```bash
   docker exec postgres pg_dump -U jury_admin jury_db > backups/daily_$(date +%Y%m%d).sql
   ```

3. **Export metrics**
   ```bash
   curl http://localhost:9090/api/v1/query_range?query=up > metrics/daily_$(date +%Y%m%d).json
   ```

4. **Create daily report**
   - Anchors processed: ___
   - Success rate: ___
   - Incidents: ___
   - Status: ✓ or ⚠️

---

## Common Procedures

### Add New Jury Authority

```bash
# 1. Get new authority address
NEW_AUTHORITY="0x..."

# 2. Update environment
echo "JURY_AUTHORITY=$NEW_AUTHORITY" >> .env.production

# 3. Restart services
docker-compose restart jury-anchorer

# 4. Verify
./scripts/health-check.sh
```

### Check Pending Anchoring Operations

```bash
# Connect to database
docker exec postgres psql -U jury_admin jury_db

# Query pending anchors
SELECT * FROM jury_decisions WHERE status = 'pending' ORDER BY created_at DESC;

# Check error logs for the session
SELECT * FROM errors WHERE session_id = 'SESSION_ID' ORDER BY created_at DESC;
```

### Manual Anchor Verification

```bash
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "query.atlasJuryAnchor.juryDecisions",
    "params": ["SESSION_ID"],
    "id": 1
  }'
```

### Scale Anchoring Service

```bash
# Increase thread count in environment
echo "ANCHORER_THREADS=4" >> .env.production

# Restart service
docker-compose restart jury-anchorer

# Monitor performance
watch -n 5 'docker stats jury-anchorer'
```

---

## Troubleshooting

### Problem: High Anchor Latency (>10s)

**Symptoms:**
- Anchor operations taking >10 seconds
- Users report slow dashboard updates

**Diagnosis:**
```bash
# Check blockchain node performance
curl http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'

# Check database query performance
docker exec postgres psql -U jury_admin jury_db -c "
  SELECT mean, count FROM pg_stat_statements 
  WHERE query LIKE '%jury%' 
  ORDER BY mean DESC;"
```

**Solution:**
1. If blockchain node is slow: `docker restart blockchain-node`
2. If database is slow: Add index on `created_at` column
3. If service is slow: Scale to multiple workers

### Problem: Anchor Verification Fails

**Symptoms:**
- Frontend shows "Verification Failed"
- Hash mismatch errors in logs

**Diagnosis:**
```bash
# 1. Check the on-chain hash
curl http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"query.atlasJuryAnchor.juryDecisions","params":["SESSION_ID"],"id":1}'

# 2. Check the local hash
docker exec jury-anchorer python -c "
  import hashlib
  import json
  # Compute hash for session
  session_data = {...}
  h = hashlib.sha256(json.dumps(session_data, sort_keys=True).encode()).hexdigest()
  print(f'Local: {h}')
  print(f'On-chain: ...')
"
```

**Solution:**
1. If hashes match: Timing issue, retry
2. If hashes don't match: Data corruption, investigate
3. Last resort: Anchor again with verification

### Problem: Database Connection Lost

**Symptoms:**
- "Connection refused" errors
- Jury service crashes

**Diagnosis:**
```bash
# Check if database is running
docker ps | grep postgres

# Check database logs
docker logs postgres | tail -50

# Test connection
docker exec postgres psql -U jury_admin -d jury_db -c "SELECT 1;"
```

**Solution:**
```bash
# 1. If database isn't running
docker-compose up -d postgres

# 2. If database won't start
docker-compose down
docker volume rm x3_postgres_data
docker-compose up -d

# 3. Restore from backup
psql -U jury_admin jury_db < backups/latest.sql
```

### Problem: Memory Leak / OOM Killer

**Symptoms:**
- Service crashes with OOM
- Memory usage keeps growing

**Diagnosis:**
```bash
# Monitor memory
docker stats jury-anchorer

# Check logs for warnings
docker logs jury-anchorer | grep -i memory
```

**Solution:**
```bash
# 1. Restart service
docker restart jury-anchorer

# 2. If recurring: Scale to multiple instances
docker-compose up -d --scale jury-anchorer=2

# 3. Review code for memory leaks
```

---

## Monitoring Alerts

### Critical Alerts (Page On-Call)
- Jury service down (no heartbeat for 5 min)
- Blockchain node sync ratio <90%
- Database connection failures
- Anchor failure rate >5%

### Warning Alerts (Create Ticket)
- High latency (anchor >10s)
- Memory usage >80%
- Disk usage >85%
- Error rate >1%

### Info Alerts (Log Only)
- Service restarts
- Configuration changes
- Backup completion
- Daily metrics summary

---

## Incident Response

### Severity 1: Service Down

1. **Immediately:**
   ```bash
   ./scripts/health-check.sh
   ```

2. **Diagnosis (5 min):**
   - Which service is down?
   - What happened? (check logs)
   - When did it start? (check metrics)

3. **Mitigation (10 min):**
   ```bash
   docker-compose restart [service]
   ./scripts/health-check.sh
   ```

4. **If restart doesn't work:**
   ```bash
   ./scripts/rollback.sh
   ```

5. **Communication:**
   - Slack: #incidents channel
   - Status page: Mark as "Investigating"
   - Executives: Get approval for rollback

### Severity 2: Degraded Performance

1. **Diagnose (5 min):**
   - Which metric is bad?
   - How long has it been bad?
   - Affecting users?

2. **Scale resources (5 min):**
   ```bash
   docker-compose up -d --scale jury-anchorer=2
   ```

3. **Monitor (10 min):**
   ```bash
   watch -n 5 ./scripts/health-check.sh
   ```

4. **If not improving in 15 min:** Escalate to Severity 1

### Severity 3: Non-Critical Issue

1. **Create ticket**
2. **Schedule fix for next sprint**
3. **Monitor trending**

---

## Backup & Recovery

### Daily Backups (Automated)

```bash
# Runs at 2 AM daily
0 2 * * * /path/to/backup.sh
```

### Manual Backup

```bash
docker exec postgres pg_dump -U jury_admin jury_db | gzip > backup.sql.gz
```

### Recovery from Backup

```bash
# 1. Restore database
gunzip < backup.sql.gz | docker exec -i postgres psql -U jury_admin jury_db

# 2. Verify data
docker exec postgres psql -U jury_admin jury_db -c "SELECT COUNT(*) FROM jury_decisions;"

# 3. Restart services
docker-compose restart
```

---

## Maintenance Windows

### Weekly (Sunday 2 AM - 3 AM)

```bash
# 1. Stop services gracefully
docker-compose stop

# 2. Run backups
./scripts/backup.sh

# 3. Update container images
docker-compose pull

# 4. Run migrations
docker-compose run --rm jury-service python -m alembic upgrade head

# 5. Start services
docker-compose up -d

# 6. Verify
./scripts/health-check.sh
```

### Quarterly (First Sunday of quarter)

```bash
# Full system test and optimization
./scripts/full-system-check.sh
```

---

## Escalation Contacts

- **Level 1 (Duty):** Check documentation, restart services
- **Level 2 (On-Call):** Page if not resolved in 15 min
- **Level 3 (Manager):** Page if not resolved in 1 hour or Severity 1
- **Level 4 (CTO):** Severity 1 + customer impact

---

## Success Metrics

Track these daily:

| Metric | Target | Actual |
|--------|--------|--------|
| Uptime | >99.9% | ___ |
| Anchor Success Rate | >99% | ___ |
| Avg Anchor Latency | <5s | ___ |
| Decision Count | Trending ↑ | ___ |
| Error Rate | <0.1% | ___ |

---

## Quick Commands

```bash
# Deploy
./scripts/deploy.sh production

# Health check
./scripts/health-check.sh

# Logs
docker-compose logs -f

# Restart
docker-compose restart jury-anchorer

# Backup
docker exec postgres pg_dump -U jury_admin jury_db > backup.sql

# Rollback
./scripts/rollback.sh

# Scale
docker-compose up -d --scale jury-anchorer=3
```

---

**Last Updated:** 2026-02-08  
**Next Review:** 2026-02-15
