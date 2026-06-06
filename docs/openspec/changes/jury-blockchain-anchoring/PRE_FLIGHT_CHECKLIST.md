# Phase 5 Pre-Flight Checklist

Use this checklist 24 hours before and immediately before deployment.

---

## 📋 PRE-DEPLOYMENT CHECKLIST (24 Hours Before)

### Code Quality & Testing (Must All Pass)

- [ ] **Rust Pallet Tests**
  ```bash
  cd pallets/x3-jury-anchor && cargo test --release
  ```
  Expected: 8/8 tests passing
  Person: _________ Date: _________ Time: _________

- [ ] **Python Integration Tests**
  ```bash
  pytest tests/test_jury_anchoring.py -v
  ```
  Expected: 13/13 tests passing
  Person: _________ Date: _________ Time: _________

- [ ] **TypeScript Compilation**
  ```bash
  npm run build --workspace packages/blockchain-adapter
  ```
  Expected: No compilation errors
  Person: _________ Date: _________ Time: _________

- [ ] **Code Quality Checks**
  ```bash
  cargo fmt --all && cargo clippy --all-targets
  npm run lint
  python -m mypy
  ```
  Expected: No warnings
  Person: _________ Date: _________ Time: _________

- [ ] **Security Scan**
  ```bash
  cargo audit
  npm audit
  python -m pip-audit
  ```
  Expected: No critical vulnerabilities
  Person: _________ Date: _________ Time: _________

### Infrastructure Verification

- [ ] **Docker Images Built**
  ```bash
  docker images | grep jury
  ```
  Expected: All images present with correct version tags
  Person: _________ Date: _________ Time: _________

- [ ] **Environment Files Created**
  ```bash
  ls -la .env.staging .env.production
  ```
  Expected: Both files exist with all required variables
  Verify: All secrets are NOT in version control
  Person: _________ Date: _________ Time: _________

- [ ] **Database Migrations Ready**
  ```bash
  ls -la alembic/versions/
  ```
  Expected: Latest migration is for Phase 5
  Person: _________ Date: _________ Time: _________

- [ ] **Monitoring Configured**
  - [ ] Prometheus config updated
  - [ ] Grafana dashboards exported
  - [ ] Alert rules configured
  Person: _________ Date: _________ Time: _________

### Documentation Verification

- [ ] **Deployment Guide Current**
  - [ ] Steps match actual scripts
  - [ ] Timings are accurate
  - [ ] Emergency contacts listed
  Person: _________ Date: _________ Time: _________

- [ ] **Operations Runbook Complete**
  - [ ] Common issues documented
  - [ ] Troubleshooting procedures tested
  - [ ] Rollback procedure verified
  Person: _________ Date: _________ Time: _________

- [ ] **Team Communications Ready**
  - [ ] Announcement drafted
  - [ ] Status update templates prepared
  - [ ] Escalation contacts verified
  Person: _________ Date: _________ Time: _________

### Stakeholder Sign-Offs

- [ ] **Engineering Review Complete**
  - [ ] Code reviewed by 2+ engineers
  - [ ] Architecture approved
  - [ ] Test coverage acceptable
  Signed by: _________________ Date: _________

- [ ] **Security Review Complete**
  - [ ] Threat model reviewed
  - [ ] No critical vulnerabilities
  - [ ] Data handling approved
  Signed by: _________________ Date: _________

- [ ] **Operations Review Complete**
  - [ ] Runbook tested
  - [ ] Monitoring ready
  - [ ] Capacity sufficient
  Signed by: _________________ Date: _________

- [ ] **Product Approval**
  - [ ] Feature meets spec
  - [ ] User impact communicated
  - [ ] Go-live approved
  Signed by: _________________ Date: _________

---

## 🚀 DEPLOYMENT DAY CHECKLIST (Before Deployment)

### 2 Hours Before (T-2:00)

**Notification Phase**

- [ ] Send pre-deployment announcement
  ```bash
  # Post to #jury-engineering, #jury-status
  ```
  Sent by: _________________ Time: _________

- [ ] Notify on-call team
  - [ ] Slack message sent
  - [ ] On-call engineer confirmed ready
  Person: _________________ Time: _________

- [ ] Verify runbook accessible to team
  Person: _________________ Time: _________

### 1 Hour Before (T-1:00)

**Pre-Staging Phase**

- [ ] Create database backup
  ```bash
  pg_dump jury_db > backups/pre_phase5_$(date +%Y%m%d_%H%M%S).sql
  ```
  Verified by: _________________ Time: _________

- [ ] Stop accepting new jury decisions
  ```bash
  curl -X POST http://localhost:8080/admin/pause
  ```
  Confirmed by: _________________ Time: _________

- [ ] Wait 5 minutes for in-flight decisions to complete
  Time: _________ to _________

- [ ] Verify no active transactions
  ```bash
  psql jury_db -c "SELECT * FROM xact_locks WHERE NOT granted;"
  ```
  Result: _________________ Person: _________

- [ ] Check disk space
  ```bash
  df -h | grep -E "/" | head -3
  ```
  Sufficient (>30%): [ ] Person: _________

- [ ] Verify backup location accessible
  Person: _________________ Time: _________

### 30 Minutes Before (T-0:30)

**Final Verification Phase**

- [ ] Review deployment script
  ```bash
  cat scripts/deploy-phase5.sh | head -50
  ```
  Reviewed by: _________________ Time: _________

- [ ] Verify all environment variables set
  ```bash
  env | grep -E "JURY|RPC|DB" | sort
  ```
  Correct: [ ] Person: _________

- [ ] Test health check script works
  ```bash
  ./scripts/health-check-phase5.sh --dry-run
  ```
  Passed: [ ] Person: _________

- [ ] Verify rollback script available
  ```bash
  ls -la scripts/rollback-phase5.sh
  ```
  Verified: [ ] Person: _________

- [ ] Announce final 30-minute warning
  Sent to: ______________ Time: _________

### Deployment Time (T-0:00)

**Execution Phase**

- [ ] **START TIME: __________ UTC**

- [ ] Execute deployment script
  ```bash
  ./scripts/deploy-phase5.sh production cpu 2>&1 | tee deploy_$(date +%Y%m%d_%H%M%S).log
  ```
  Started by: _________________ Time: _________

- [ ] **T+5 min:** Update: Services launching
  Updated by: _________________ Time: _________

- [ ] **T+10 min:** Update: Services online
  Updated by: _________________ Time: _________

- [ ] **T+15 min:** RPC Node Health Check
  ```bash
  curl -s http://localhost:9944 -X POST \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}'
  ```
  Status: ✓ Synced / ⏳ Syncing / ✗ Failed
  Person: _________ Time: _________

- [ ] **T+20 min:** Run Health Checks
  ```bash
  ./scripts/health-check-phase5.sh
  ```
  Result: ✅ PASS / ⚠️ WARN / ❌ FAIL
  Person: _________ Time: _________

- [ ] **T+25 min:** Run Integration Tests
  ```bash
  pytest tests/test_jury_anchoring.py::TestJuryAnchoringEndToEnd -v
  ```
  Result: ✅ PASS / ❌ FAIL
  Person: _________ Time: _________

- [ ] **T+30 min:** Announce: Phase 5 Online
  Announced to: ______________ Time: _________

### Post-Deployment Monitoring (T+30 to T+3:30)

**During this 3-hour period, maintain constant vigilance:**

- [ ] **Deploy Successful Announced** (T+30 min)
  Announced by: _________________ Time: _________

- [ ] **Monitor Every 15 Minutes:**
  
  **T+45 min:**
  - RPC latency: _________ (target: <1s)
  - Anchor latency (p95): _________ (target: <5s)
  - Success rate: _________ (target: >99%)
  Checked by: _________________ Time: _________

  **T+1:00** (same metrics)
  Checked by: _________________ Time: _________

  **T+1:15** (same metrics)
  Checked by: _________________ Time: _________

  **T+1:30** (same metrics)
  Checked by: _________________ Time: _________

  **T+2:00** (same metrics)
  Checked by: _________________ Time: _________

  **T+2:30** (same metrics)
  Checked by: _________________ Time: _________

  **T+3:00** (same metrics)
  Checked by: _________________ Time: _________

- [ ] **Monitor Logs for Errors**
  ```bash
  docker-compose logs --tail=100 | grep ERROR
  ```
  Errors found: [ ] None [ ] 1-2 (acceptable) [ ] >2 (investigate)
  Checked by: _________________ Time: _________

- [ ] **No Critical Alerts Triggered**
  Monitoring dashboard: ___________________
  Checked by: _________________ Time: _________

### Final Sign-Off (T+3:30)

- [ ] **DEPLOYMENT SUCCESSFUL DECLARED** (T+3:30)

  All metrics nominal: ✅
  No incidents: ✅
  Support team ready: ✅
  
  Declared by: _________________ Time: _________

- [ ] **Enable New Decisions**
  ```bash
  curl -X POST http://localhost:8080/admin/resume
  ```
  Confirmed by: _________________ Time: _________

- [ ] **Announce Success to All Stakeholders**
  Announced by: _________________ Time: _________

- [ ] **Begin Weekly Monitoring**
  Assigned to: _________________ Time: _________

- [ ] **Schedule Post-Mortem** (if any issues)
  Scheduled for: _________________ Time: _________

---

## 🚨 EMERGENCY ROLLBACK CHECKLIST

Use this ONLY if critical issues occur during deployment.

- [ ] **DECLARE INCIDENT**
  Declared by: _________________ Time: _________
  Severity: [ ] P1 [ ] P2 [ ] P3

- [ ] **STOP ACCEPTING DECISIONS**
  ```bash
  curl -X POST http://localhost:8080/admin/pause
  ```
  Executed by: _________________ Time: _________

- [ ] **NOTIFY ALL STAKEHOLDERS**
  Notified at: _________ UTC
  Notification method: _________________ Time: _________

- [ ] **EXECUTE ROLLBACK**
  ```bash
  ./scripts/rollback-phase5.sh
  ```
  Executed by: _________________ Time: _________

- [ ] **VERIFY ROLLBACK SUCCESSFUL**
  ```bash
  ./scripts/health-check-phase5.sh
  ```
  Status: ✅ PASSED / ❌ FAILED
  Verified by: _________________ Time: _________

- [ ] **ANNOUNCE ROLLBACK**
  Announced to: ______________ Time: _________

- [ ] **SCHEDULE INCIDENT POST-MORTEM**
  Scheduled for: _________________ Time: _________

---

## 📞 CONTACT INFORMATION

| Role | Name | Phone | Slack |
|------|------|-------|-------|
| Deployment Lead | _____________ | _____________ | _____________ |
| On-Call Engineer | _____________ | _____________ | _____________ |
| On-Call Manager | _____________ | _____________ | _____________ |
| VP Engineering | _____________ | _____________ | _____________ |
| VP Operations | _____________ | _____________ | _____________ |

**Escalation Path:**
1. First 15 min: Deployment Lead
2. After 15 min: On-Call Manager
3. After 30 min: VP Engineering & VP Operations

---

## 📝 NOTES & OBSERVATIONS

```

[Deployment lead will use this section to document any issues, 
decisions, or observations during the deployment process]

T+0:00 - Started
_________________________________________________________________

T+0:15 - RPC sync status
_________________________________________________________________

T+0:30 - Services online
_________________________________________________________________

T+1:00 - First hour complete
_________________________________________________________________

T+2:00 - Halfway through monitoring
_________________________________________________________________

T+3:00 - Final hour beginning
_________________________________________________________________

T+3:30 - Deployment complete
_________________________________________________________________

Post-Deployment Notes:
_________________________________________________________________

```

---

**DEPLOYMENT COMPLETE**

- [ ] All checkboxes above completed: ✅
- [ ] No critical issues: ✅
- [ ] All metrics nominal: ✅
- [ ] Stakeholders notified: ✅

**Signed Off By:**

Deployment Lead: _________________________ Time: _________

Operations Director: _________________________ Time: _________

Chief Technology Officer: _________________________ Time: _________

---

**Document prepared and approved:**

Date: 2026-02-08  
Template version: 1.0  
Last reviewed: 2026-02-08  
Next review: 2026-03-08
