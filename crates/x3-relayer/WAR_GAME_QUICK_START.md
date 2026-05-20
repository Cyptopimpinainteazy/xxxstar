# 🚀 WAR GAME QUICK-START GUIDE

**Exercise Date:** April 28, 2026  
**Time:** 14:00-16:30 UTC (2.5 hours)  
**Objective:** Validate RPC failover procedures under multiple provider failures  
**Owner:** Infrastructure Lead

---

## 60-Second Pre-Exercise Briefing

**Scenario:** Your two primary RPC providers go down simultaneously. You have 60 seconds to detect it, 90 seconds to escalate, and 10 minutes to recover normal operations.

**Success:** Blocks polling resumes using backup providers. Team communicates clearly. No panic.

**Rules:**
- This is a safe exercise — staging environment only
- No judgment for mistakes
- Focus on *procedures*, not speed
- Capture learnings for next exercises

---

## Materials Needed (Gather Before Exercise)

### For Each Team Member

| Role | Materials | Location |
|------|-----------|----------|
| **Incident Commander** | Escalation matrix, incident template | PHASE_13F_QUICK_REFERENCE_GUIDE.md § 3 |
| **RPC Manager** | RPC status commands, failover decision tree | RPC_FAILOVER_PROCEDURES.md § 2 |
| **Relayer Operator** | Relayer status commands, logs location | PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md § 4 |
| **Network Operator** | Monitoring dashboards, alert thresholds | MAINNET_PERFORMANCE_BASELINE.md |
| **Launch Director** | Timeline tracker, team contact list | PHASE_13F_QUICK_REFERENCE_GUIDE.md § 4 |
| **Note-Taker** | Event log template (below), pen | This document |

### Infrastructure Setup

**Confirm These Working (Test Day Before):**
- [ ] Staging relayer running normally (blocks polling every 10 blocks)
- [ ] Grafana dashboard showing RPC health
- [ ] Slack #x3-launch-verification channel created
- [ ] Terminal access to relayer logs
- [ ] Grafana RPC Health panel accessible
- [ ] Command shortcuts created:
  ```bash
  # RPC Health Status
  curl -s http://localhost:9090/metrics | grep rpc_request
  
  # Relayer Status
  tail -50 /var/log/x3-relayer/relayer.log | grep -i "blocks\|rpc\|error"
  
  # Latest Block Polled
  grep "latest_block" /var/log/x3-relayer/relayer.log | tail -1
  ```

---

## Exercise Timeline (2.5 Hours)

### T-15 minutes: Pre-Exercise Setup (14:00 UTC)

**What to do:**
1. All participants join Slack channel #x3-launch-verification
2. Quick tech check (all can see Grafana, access logs, Slack working)
3. Infrastructure Lead checks staging environment status:
   ```bash
   # Confirm relayer healthy
   ps aux | grep x3-relayer
   # Should show: active, running, CPU <10%, Memory <500MB
   
   # Confirm blocks polling
   tail -20 /var/log/x3-relayer/relayer.log
   # Should show: "Polled blocks" recently
   ```
4. Launch Director confirms all 5 team members present
5. Brief kickoff: "In 15 minutes, we'll simulate RPC failure. Your job is to detect it, escalate, and recover."

**Expected State:**
- ✅ Staging relayer healthy and polling blocks normally
- ✅ All dashboards displaying correctly
- ✅ All team members ready
- ✅ Communication channel open

---

### T+0 minutes: Exercise Starts (14:15 UTC)

**Incident Injection (Infrastructure Lead executes):**

Infrastructure Lead announces in Slack:
```
🚨 EXERCISE START 🚨
"Alchemy and Infura RPC providers just became unreachable. 
Relayer can only reach QuickNode (Ethereum) and Triton (Solana).
Both teams - check your RPC health status now."
```

**What Each Team Member Does Immediately:**

**RPC Manager:**
- [ ] Run RPC health check command
- [ ] Observe in Grafana: 2 of 3 providers showing RED
- [ ] Post in Slack: "Alchemy and Infura down, QuickNode only"
- [ ] **CRITICAL:** Note the exact time (log it)

**Relayer Operator:**
- [ ] Check relayer logs: `tail -50 /var/log/x3-relayer/relayer.log`
- [ ] Look for error patterns (connection refused, timeouts)
- [ ] Post in Slack: "Seeing RPC errors in logs"
- [ ] DO NOT restart relayer yet

**Network Operator:**
- [ ] Check Grafana RPC Health panel
- [ ] Confirm visual indication of failed providers
- [ ] Monitor alert status (should be firing soon)
- [ ] Post in Slack: "Alerts firing on RPC health"

**Note-Taker:**
- [ ] Log the exact time RPC failure announced
- [ ] Record what each team member observes
- [ ] Track all actions and times

**Launch Director:**
- [ ] Observe team response
- [ ] Time from incident to first escalation
- [ ] Check: Are people panicking or following procedure?

---

### T+60 seconds: Detection Complete (14:16 UTC)

**Target: First Escalation**

**Incident Commander:**
- [ ] By now, RPC Manager should have reported failure
- [ ] IC decides: "Failover needed"
- [ ] Posts in Slack: "Activating RPC failover procedure"
- [ ] Consults RPC_FAILOVER_PROCEDURES.md § 2 for exact steps

**Success Criteria:**
- ✅ RPC failure detected and reported within 60 seconds
- ✅ Incident Commander has taken command
- ✅ Slack notification sent (or alert firing)
- ✅ Decision made: "Proceed to failover"

**If You're Behind Schedule:**
- It's OK — this is data to learn from
- Keep going, don't reset
- Note in post-mortem: "Detection took 2 minutes"
- Address detection timing in next iteration

---

### T+90 seconds: Failover Decision (14:17 UTC)

**Target: Failover Execution Initiated**

**RPC Manager Executes Failover:**
1. Pulls up RPC_FAILOVER_PROCEDURES.md § 2 (Manual Failover Steps)
2. Executes configuration change:
   ```bash
   # Check current config
   cat /etc/x3-relayer/config.yaml | grep -A 5 rpc_providers
   
   # Update to use backup providers
   # (In real scenario: edit config, reload)
   echo "✅ RPC config updated to backup providers"
   ```
3. Posts in Slack: "RPC failover executed - monitoring for recovery"
4. **Note the exact time**

**Relayer Operator:**
- [ ] Continue monitoring logs
- [ ] Watch for new RPC connection attempts
- [ ] Should see: "Connected to QuickNode" and "Connected to Triton"
- [ ] If not: Report to Incident Commander immediately

**Success Criteria:**
- ✅ Failover decision made within 90 seconds of incident
- ✅ RPC configuration updated
- ✅ Team awaiting recovery signals

---

### T+2 minutes: Recovery Monitoring (14:17 UTC)

**Target: First Sign of Recovery**

**Network Operator:**
- [ ] Watch Grafana RPC Health panel
- [ ] Should see: Status changing from RED to YELLOW (recovering)
- [ ] Post in Slack: "RPC providers health improving"

**Relayer Operator:**
- [ ] Check logs: `tail -20 /var/log/x3-relayer/relayer.log`
- [ ] Should see: "Successfully polled blocks from QuickNode/Triton"
- [ ] Post in Slack: "Blocks polling resumed from backup providers"

**Incident Commander:**
- [ ] Update status: "Failover successful, monitoring recovery"
- [ ] Relayer is now running on backup providers

**Success Criteria:**
- ✅ Relayer successfully using backup providers
- ✅ Block polling resumed
- ✅ No errors in logs

---

### T+5 minutes: Stabilization (14:20 UTC)

**Target: Normal Operations on Backup Providers**

**All Teams:**
- [ ] Confirm all metrics green (backup providers only, but stable)
- [ ] No active alerts
- [ ] Blocks polling every 10 blocks normally
- [ ] Proofs should start submitting again

**Incident Commander:**
- [ ] Update status: "Stabilized on backup providers"
- [ ] Decision: "Monitor for primary provider recovery"

**Success Criteria:**
- ✅ System stable on backup providers
- ✅ No errors or unusual activity
- ✅ Metrics show normal operation

---

### T+10 minutes: Primary Recovery (14:25 UTC)

**Target: Primary Providers Back Online**

**Infrastructure Lead Announces in Slack:**
```
"Primary RPC providers are back online:
- Alchemy: Responding normally
- Infura: Responding normally
Decide when to failback."
```

**RPC Manager Decides:**
- [ ] Check both providers health: "Both show YELLOW → GREEN"
- [ ] Option A: Failback now (or wait for further stability)
- [ ] Posts: "Recommend failback - primary providers stable"

**Incident Commander:**
- [ ] Decision: "Proceed with failback to primary"
- [ ] Executes failback configuration change
- [ ] Posts: "Failback initiated"

**Relayer Operator:**
- [ ] Monitor logs during failback
- [ ] Should see smooth transition back to primary providers
- [ ] Confirm: "Primary providers active, backup on standby"

**Success Criteria:**
- ✅ Failback decision made intelligently (not too quick, not too slow)
- ✅ Failback execution smooth
- ✅ Back to original provider configuration
- ✅ No service interruption during failback

---

### T+15 minutes: Exercise Complete (14:30 UTC)

**Incident Commander:**
- [ ] Posts in Slack: "🎉 Exercise complete. Normal operations restored."
- [ ] Gathers post-mortem feedback from team

---

## 30-Minute Post-Exercise Debrief (14:30-15:00 UTC)

**What Went Well:**
- Each person shares one thing they did well
- Notes in post-mortem

**What Could Improve:**
- Identify timing issues
- Note any confusion or unclear procedures
- Document procedure gaps

**Key Metrics to Discuss:**

| Metric | Target | Actual | Notes |
|--------|--------|--------|-------|
| **Detection Time** | <60s | ___ | From incident to first alert |
| **Escalation Time** | <90s | ___ | To failover decision |
| **Failover Execution** | <2min | ___ | Configuration change + recovery |
| **Recovery Time** | <10min | ___ | Back to normal operations |
| **Team Communication** | Clear | ___ | Was everyone on same page? |
| **Procedure Clarity** | >90% | ___ | Did procedures match reality? |

**Action Items (If Any):**
- Document who, what, deadline
- Priority: Critical vs. Nice-to-Have

---

## Expected Results

### If Exercise Succeeds ✅
- All time targets met or close
- Team confident in procedures
- No critical gaps identified
- Procedures validated as written
- **Next Step:** Move to Team Rehearsal (May 2)

### If Issues Found ⚠️
- **Critical** (e.g., failover doesn't work):
  - [ ] Document in TEST_RESULTS_TRACKER.md
  - [ ] Fix before next exercise
  - [ ] Re-test failover change
  - [ ] Proceed to Team Rehearsal after fix
- **High Priority** (e.g., timing off by 30 sec):
  - [ ] Document learnings
  - [ ] Update procedures if needed
  - [ ] Proceed to Team Rehearsal
- **Medium Priority** (e.g., communication could be clearer):
  - [ ] Note for Phase 14 improvements
  - [ ] Proceed to Team Rehearsal

---

## Documentation During Exercise

**Note-Taker Uses This Format:**

```
EVENT LOG: War Game Exercise - April 28, 2026

T+0 (14:15): Infrastructure announces RPC failure
  - RPC Manager sees providers RED in Grafana
  - Relayer Operator confirms errors in logs

T+60s (14:16): First escalation
  - Incident Commander takes command
  - RPC Manager reports: "Alchemy + Infura down, QuickNode only"

T+90s (14:17): Failover decision
  - RPC Manager executes: "Config updated to backup providers"
  - [Note exact time here: ___]

T+2min (14:17): Recovery monitoring
  - Relayer Operator: "Blocks polling resumed"
  - Network Operator: "RPC health YELLOW, improving"

T+5min (14:20): Stabilization
  - All systems: GREEN, stable on backup providers
  - Incident Commander: "Stable, monitoring for primary recovery"

T+10min (14:25): Primary recovery
  - Infrastructure: "Alchemy + Infura both GREEN"
  - RPC Manager: "Failback ready"
  - Incident Commander: "Executing failback"

T+15min (14:30): Exercise complete
  - Primary providers active, backup on standby
  - Normal operations restored
```

---

## Command Reference Sheet (Print This)

**RPC Health Status:**
```bash
curl -s http://localhost:9090/metrics | grep rpc_
# Look for: rpc_latency_ms, rpc_failures
```

**Relayer Status:**
```bash
ps aux | grep x3-relayer
# Should show: running, low CPU, reasonable memory
```

**Block Polling:**
```bash
tail -20 /var/log/x3-relayer/relayer.log | grep -i "polled\|block"
# Should see recent entries, no errors
```

**Failover Trigger (Manual):**
```bash
# Edit relayer config to change RPC provider order
sudo nano /etc/x3-relayer/config.yaml
# Change primary → backup in provider list
# Restart: sudo systemctl restart x3-relayer
```

**Monitoring Dashboard:**
- Grafana RPC Health: http://localhost:3000/d/rpc-health
- Grafana Bridge Activity: http://localhost:3000/d/bridge-activity

---

## Team Contacts (Tape This to War Room Wall)

| Role | Name | Phone | Slack |
|------|------|-------|-------|
| **Incident Commander** | _______ | ___________ | @_______ |
| **RPC Manager** | _______ | ___________ | @_______ |
| **Relayer Operator** | _______ | ___________ | @_______ |
| **Network Operator** | _______ | ___________ | @_______ |
| **Launch Director** | _______ | ___________ | @_______ |
| **VP Engineering** | _______ | ___________ | @_______ |

---

## Day-Before Checklist

**Thursday, April 27 (Day Before Exercise)**

- [ ] Test staging relayer (confirm healthy state)
- [ ] Test Grafana RPC Health dashboard
- [ ] Confirm all team members can access logs
- [ ] Confirm Slack channel working
- [ ] Run quick 5-minute dry-run with infra team only
- [ ] Send reminder message to team: "Exercise tomorrow 14:00 UTC"
- [ ] Print this document for war room
- [ ] Print PHASE_13F_QUICK_REFERENCE_GUIDE.md § 3 (Escalation Ladder)
- [ ] Print RPC_FAILOVER_PROCEDURES.md § 2 (Failover Steps)
- [ ] Confirm all participants have access to all needed documents

---

## Success Declaration

**After Exercise Completion:**

```
WAR GAME EXERCISE: SUCCESSFUL ✅

✅ Detection: < 60 seconds
✅ Escalation: < 90 seconds  
✅ Failover Execution: < 2 minutes
✅ Recovery: < 10 minutes
✅ Team Communication: Clear and confident
✅ Procedures: Validated

Next Exercise: Team Rehearsal - May 2, 2026
Status: READY TO PROCEED
```

---

**Document Version:** 1.0  
**Last Updated:** April 21, 2026  
**Exercise Date:** April 28, 2026  
**Owner:** Infrastructure Lead  
**Status:** READY TO EXECUTE
