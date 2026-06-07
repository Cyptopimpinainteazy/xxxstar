# Disaster Recovery Runbook

**Document Version:** 1.0  
**Date:** April 26, 2026  
**Status:** 🚨 CRITICAL OPERATIONAL DOCUMENT  
**Target Audience:** Incident response team, validators, core team

---

## Executive Summary

This runbook provides **battle-tested procedures** for recovering from catastrophic failures in the X3 ATOMIC STAR mainnet. Every second counts during incidents - this document is designed for rapid execution under pressure.

**Incident Types Covered:**
1. Chain Halt (network stops producing blocks)
2. Consensus Failure (multiple competing forks)
3. State Corruption (invalid state transitions)
4. Bridge Exploit (unauthorized asset transfers)
5. Validator Mass Offline (>33% validators down)
6. Network Partition (validator set split)
7. Governance Attack (malicious proposals)

**Average Recovery Times:**
- Chain Halt: 15-60 minutes
- Consensus Failure: 1-4 hours
- State Corruption: 4-24 hours (depends on severity)
- Bridge Exploit: IMMEDIATE (halt within 5 minutes)
- Validator Mass Offline: 30 minutes - 2 hours
- Network Partition: 1-3 hours
- Governance Attack: 2-48 hours (depends on proposal)

---

## Incident Classification

### Severity Levels

| Level | Name | Definition | Response Time | Escalation |
|-------|------|------------|---------------|------------|
| **S0** | Catastrophic | Chain halted, assets at risk, or exploits in progress | **<5 min** | CEO + CTO + All validators |
| **S1** | Critical | Degraded service, potential asset risk, or security vulnerability | **<15 min** | CTO + Incident Commander + On-call team |
| **S2** | Major | Service disruption, no immediate asset risk | **<30 min** | Incident Commander + Ops team |
| **S3** | Minor | Performance degradation, no service impact | **<2 hours** | Ops team only |

### Incident Types

#### S0 - CATASTROPHIC

**Chain Halt:**
- No blocks produced for >5 minutes
- No finality for >10 minutes
- All validators reporting errors

**Active Exploit:**
- Bridge replay attack detected
- Canonical supply violation in progress
- Double-spend confirmed
- Unauthorized minting detected

**State Corruption:**
- Merkle root mismatch between validators
- Canonical supply != sum(all_ledgers)
- Invalid state transition accepted

#### S1 - CRITICAL

**Consensus Degradation:**
- Competing forks (>5 blocks divergence)
- Finality lag >50 blocks
- <66% validators agreeing on best block

**Validator Mass Offline:**
- >33% of validators offline
- <5 active validators (below minimum)

**Security Vulnerability:**
- Zero-day exploit published
- S0/S1 blocker discovered in production code

#### S2 - MAJOR

**Performance Degradation:**
- Block time >10 seconds (normal: 6s)
- Transaction queue backing up (>10k pending)
- Validator uptime <95%

**Network Issues:**
- Peer connectivity degraded
- Regional outage affecting validators

#### S3 - MINOR

**Isolated Issues:**
- Single validator offline
- Transaction pool spam
- RPC endpoint degraded

---

## Response Team Structure

### Incident Commander (IC)

**Role:** Single point of authority during incidents  
**Responsibilities:**
- Declare incident severity
- Coordinate all response activities
- Make go/no-go decisions (rollback, pause, etc.)
- Communicate with stakeholders
- Sign off on incident closure

**On-Call Rotation:**
| Week | Primary IC | Backup IC |
|------|-----------|-----------|
| Week 1 | [Name 1] | [Name 2] |
| Week 2 | [Name 2] | [Name 3] |
| Week 3 | [Name 3] | [Name 1] |

**Contact:** incidents@x3.network | +1-XXX-EMERGENCY

### Response Roles

| Role | Responsibilities | Contact |
|------|------------------|---------|
| **Technical Lead** | Root cause analysis, fix implementation | dev@x3.network |
| **Communications Lead** | Public updates, social media, support tickets | comms@x3.network |
| **Security Analyst** | Exploit analysis, vulnerability assessment | security@x3.network |
| **Validator Coordinator** | Coordinate validator actions, collect status | validators@x3.network |
| **Legal Counsel** | Regulatory implications, user communications | legal@x3.network |

---

## Incident Response Playbooks

## 1. CHAIN HALT RECOVERY

### Symptoms
- No new blocks for >5 minutes
- All validators showing "Idle" in telemetry
- Block explorer frozen
- RPC endpoints returning stale data

### Immediate Actions (0-5 minutes)

**Step 1: Declare Incident**
```bash
# Incident Commander sends to all channels:
🚨 INCIDENT DECLARED: S0 - CHAIN HALT
Time: [ISO 8601 timestamp]
Issue: Network stopped producing blocks
IC: [Your Name]
War room: [Discord/Zoom link]
```

**Step 2: Validator Status Check**
```bash
# All validators: Report status immediately
# Template:
Validator: [Name]
Node status: [RUNNING/STOPPED/ERROR]
Peers: [Count]
Best block: #[Number]
Finalized: #[Number]
Errors: [Last 10 lines of logs]
```

**Step 3: Collect Diagnostic Data**

Each validator runs:
```bash
# System health
uptime
df -h
free -h
top -bn1 | head -20

# Node status
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_health"}' \
  http://localhost:9944

# Recent logs
tail -n 500 /var/log/x3-validator.log | grep -E "ERROR|WARN|FATAL"

# Peer info
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_peers"}' \
  http://localhost:9944
```

### Root Cause Analysis (5-15 minutes)

**Common Causes:**

1. **>33% Validators Offline**
   - Check validator count: Should be ≥7/10 for BFT consensus
   - If <7 online: This is the root cause

2. **Database Corruption**
   - Check logs for "Database error" or "State read failed"
   - If found: Database issue confirmed

3. **Network Partition**
   - Check if validators see different peer sets
   - If east coast only sees east coast: Network partition confirmed

4. **Consensus Bug**
   - Check logs for "BABE" or "GRANDPA" errors
   - If found: Consensus implementation bug

5. **Resource Exhaustion**
   - Check disk space (>90% = problem)
   - Check memory (OOM killer active = problem)

### Recovery Procedures

#### Scenario A: Validators Offline (Most Common)

```bash
# If <7 validators online, wake up offline validators

# Incident Commander coordinates:
1. Contact all offline validators (phone/telegram)
2. Each offline validator:
   - Restart node: systemctl restart x3-validator
   - Report status in 2 minutes
3. Once 7+ validators online:
   - Network should auto-recover
   - Monitor for block production resumption

# Expected recovery time: 5-15 minutes
```

#### Scenario B: Database Corruption

```bash
# Affected validators only:

# 1. Stop node
systemctl stop x3-validator

# 2. Backup corrupted DB (forensics)
tar -czf /backup/corrupted-db-$(date +%s).tar.gz /var/lib/x3-data/chains/

# 3. Wipe database
rm -rf /var/lib/x3-data/chains/*/db/

# 4. Restart node (will sync from peers)
systemctl start x3-validator

# 5. Monitor sync progress
journalctl -u x3-validator -f | grep "Syncing"

# Expected recovery time: 30-60 minutes (full sync)
```

#### Scenario C: Network Partition

```bash
# Identify partition:
# - Group A validators only see Group A peers
# - Group B validators only see Group B peers

# Resolution:
# 1. Incident Commander identifies "majority partition"
#    (group with most validators)

# 2. Minority partition validators:
#    - Stop nodes
#    - Clear peer database
#    - Restart with explicit bootnodes

# Example for minority validator:
systemctl stop x3-validator

# Clear peers
rm -rf /var/lib/x3-data/chains/*/network/

# Restart with bootnodes
x3-chain-node \
  --base-path /var/lib/x3-data \
  --chain x3-mainnet-raw.json \
  --validator \
  --bootnodes /ip4/[MAJORITY_VALIDATOR_IP]/tcp/30333/p2p/[PEER_ID] \
  --bootnodes /ip4/[ANOTHER_MAJORITY_IP]/tcp/30333/p2p/[PEER_ID]

# Expected recovery time: 15-45 minutes
```

#### Scenario D: Consensus Bug (CRITICAL)

```bash
# If consensus bug confirmed:
# 1. HALT ALL VALIDATORS IMMEDIATELY

# Incident Commander broadcasts:
🚨 HALT ORDER: All validators stop nodes NOW
systemctl stop x3-validator

# 2. Core team analysis (emergency patch required)
# 3. Deploy hotfix
# 4. Coordinate restart (see GENESIS_CEREMONY_RUNBOOK.md)

# Expected recovery time: 1-4 hours (depends on patch complexity)
```

### Verification Checklist

After recovery, verify:
- [ ] All validators online (10/10)
- [ ] Blocks producing (1 block every 6 seconds)
- [ ] Finality progressing (<10 block lag)
- [ ] No errors in validator logs (last 100 lines)
- [ ] Telemetry showing all validators
- [ ] Block explorer updated
- [ ] RPC endpoints responsive
- [ ] Transaction pool processing

---

## 2. CONSENSUS FAILURE RECOVERY

### Symptoms
- Multiple competing forks (>5 blocks divergent)
- Validators disagree on best block
- Finality stalled (no new finalized blocks)
- "Conflicting headers" errors in logs

### Immediate Actions (0-5 minutes)

**Step 1: Declare Incident**
```bash
🚨 INCIDENT DECLARED: S1 - CONSENSUS FAILURE
Time: [ISO 8601 timestamp]
Issue: Multiple competing forks detected
IC: [Your Name]
War room: [Discord/Zoom link]
```

**Step 2: Identify Forks**

Each validator reports:
```bash
# Best block
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock"}' \
  http://localhost:9944 | jq '.result.block.header'

# Finalized block
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getFinalizedHead"}' \
  http://localhost:9944 | jq -r '.result'
```

**Create Fork Map:**
```
Fork A: Validators 1, 3, 5, 7 (4 validators)
  Best: #12045 (0xabc123...)
  Finalized: #12000

Fork B: Validators 2, 4, 6 (3 validators)  
  Best: #12042 (0xdef456...)
  Finalized: #12000

Fork C: Validators 8, 9, 10 (3 validators)
  Best: #12044 (0x789ghi...)
  Finalized: #12000
```

### Recovery Procedure

**Step 1: Identify Canonical Fork**

Canonical fork = fork with:
1. Most validators (if >66% on one fork → that's canonical)
2. If no majority: Choose longest chain
3. If tied: Choose fork with lowest block hash (lexicographically)

**Step 2: Restart Minority Validators on Canonical Fork**

```bash
# Minority validators (e.g., Fork B and C):

# 1. Stop node
systemctl stop x3-validator

# 2. Export keys (CRITICAL - don't lose these)
cp -r /var/lib/x3-data/chains/*/keystore /backup/keystore-$(date +%s)

# 3. Wipe chain data (keeps keystore)
rm -rf /var/lib/x3-data/chains/*/db/
rm -rf /var/lib/x3-data/chains/*/network/

# 4. Restart node (will sync from majority)
systemctl start x3-validator

# 5. Monitor sync to canonical fork
journalctl -u x3-validator -f | grep "best:"
# Should sync to Fork A's best block
```

**Step 3: Wait for Finality Recovery**

Once all validators on same fork:
- GRANDPA should resume finalizing blocks
- Monitor finality lag: should decrease to <10 blocks within 30 minutes

### Verification Checklist

- [ ] All validators on same best block hash
- [ ] Finality progressing (finalized block increasing)
- [ ] No more "Conflicting headers" errors
- [ ] Validator agreement >66% (ideally 100%)

---

## 3. STATE CORRUPTION RECOVERY

### Symptoms
- Canonical supply mismatch (total_supply != sum(all_ledgers))
- Merkle root mismatch between validators
- Invalid state transitions accepted
- "State verification failed" errors

### Immediate Actions (0-5 minutes)

**Step 1: HALT NETWORK IMMEDIATELY**

```bash
🚨 EMERGENCY HALT: S0 - STATE CORRUPTION
All validators: STOP NODES NOW
systemctl stop x3-validator
```

**This is a CRITICAL incident - DO NOT allow network to continue with corrupted state.**

**Step 2: Identify Corruption Point**

```bash
# Core team analyzes:
# 1. Last known good block (where state was valid)
# 2. First corrupted block (where state became invalid)

# Check canonical supply at various blocks:
x3-chain-node export-state \
  --chain x3-mainnet-raw.json \
  --base-path /var/lib/x3-data \
  --block [BLOCK_HASH] \
  | jq '.balances.balances | map(.[1] | tonumber) | add'

# Compare to expected supply:
# Expected: [GENESIS_SUPPLY + INFLATION - BURNS]
```

**Step 3: Determine Recovery Strategy**

**Option A: Rollback to Last Good Block**
- **Use when:** Corruption is recent (<1000 blocks)
- **Procedure:** Export state at last good block → new genesis
- **User impact:** Lost transactions after rollback point
- **Time:** 2-4 hours

**Option B: State Repair (Advanced)**
- **Use when:** Corruption is old (>1000 blocks), rollback too disruptive
- **Procedure:** Manually fix corrupted state + re-execute blocks
- **User impact:** Minimal (if successful)
- **Time:** 12-48 hours
- **Risk:** HIGH (easy to introduce new bugs)

**Option C: Snapshot + Compensation**
- **Use when:** Corruption is catastrophic, no clean recovery
- **Procedure:** Snapshot all balances → compensate affected users → new chain
- **User impact:** HIGH (all users affected)
- **Time:** 1-2 weeks
- **Last resort only**

### Recovery Procedure (Option A: Rollback - Most Common)

**Step 1: Core Team Creates New Genesis**

```bash
# 1. Export state at last good block
LAST_GOOD_BLOCK="0x[hash_of_last_good_block]"

x3-chain-node export-state \
  --chain x3-mainnet-raw.json \
  --base-path /var/lib/x3-data \
  --block $LAST_GOOD_BLOCK \
  > state-export-rollback.json

# 2. Create new chain spec with exported state
# (Manual process - edit chain-spec-rollback.json)

# 3. Convert to raw format
x3-chain-node build-spec \
  --chain chain-spec-rollback.json \
  --raw \
  > x3-mainnet-rollback-raw.json

# 4. Calculate new genesis hash
NEW_GENESIS=$(x3-chain-node build-spec --chain x3-mainnet-rollback-raw.json 2>&1 | grep "Genesis Hash" | awk '{print $3}')

echo "New Genesis Hash: $NEW_GENESIS"
```

**Step 2: Distribute to All Validators**

```bash
# Core team sends:
Subject: URGENT: Chain Rollback - Action Required

Attached: x3-mainnet-rollback-raw.json

SITUATION:
State corruption detected at block #[NUM]. Rolling back to block #[LAST_GOOD].

INSTRUCTIONS:
1. Download attached chain spec
2. Backup your validator keys:
   tar -czf ~/x3-keys-backup.tar.gz /var/lib/x3-data/chains/*/keystore
3. Wipe chain data:
   rm -rf /var/lib/x3-data/chains/
4. Replace chain spec:
   mkdir -p /var/lib/x3-data/chains/x3_mainnet/
   cp x3-mainnet-rollback-raw.json /var/lib/x3-data/chains/x3_mainnet/chain_spec.json
5. Wait for coordinated restart: [TIMESTAMP]

ROLLBACK DETAILS:
- Last good block: #[NUM] (0x[hash])
- New genesis hash: [NEW_GENESIS]
- Blocks lost: [COUNT]
- Coordinated restart: [ISO 8601 timestamp]

Questions: Reply to this email or join #emergency-response
```

**Step 3: Coordinated Restart**

Follow GENESIS_CEREMONY_RUNBOOK.md launch sequence with new chain spec.

**Step 4: Post-Rollback Verification**

```bash
# After restart, verify:

# 1. Canonical supply matches expected
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_properties"}' \
  http://localhost:9944

# 2. All validator balances correct
# 3. No double-spends possible
# 4. Bridge balances match

# Run comprehensive audit:
./launch-gates/comprehensive-mainnet-readiness.sh
```

### User Communication

**Immediate (within 1 hour of halt):**
```
🚨 NETWORK HALT NOTICE

X3 ATOMIC STAR network halted due to state corruption.

WHAT HAPPENED:
A state corruption was detected at block #[NUM]. To preserve asset safety, we immediately halted the network.

WHAT'S BEING DONE:
We're rolling back to the last verified good state at block #[LAST_GOOD].

YOUR ASSETS:
✅ SAFE - All balances preserved at block #[LAST_GOOD]

TIMELINE:
- Current: Network halted
- +2 hours: Rollback prepared
- +4 hours: Network restart (target)

IMPACT:
- Transactions after block #[LAST_GOOD] will be reversed
- Users affected: [NUMBER]
- Total value affected: [AMOUNT]

NEXT UPDATE: 30 minutes

Status: https://status.x3.network
```

**Follow-up (24 hours after recovery):**
```
✅ NETWORK RECOVERY COMPLETE

X3 ATOMIC STAR is fully operational following state corruption rollback.

RECOVERY SUMMARY:
- Network halted: [TIME]
- Rollback completed: [TIME]
- Network restarted: [TIME]
- Total downtime: [DURATION]

WHAT WAS LOST:
- [NUMBER] transactions reversed
- Block range: #[START] to #[END]
- Affected users: [NUMBER]

COMPENSATION:
Affected users will receive [DETAILS].

TECHNICAL ROOT CAUSE:
[Brief explanation - full post-mortem in 72 hours]

PREVENTATIVE MEASURES:
1. [Measure 1]
2. [Measure 2]

Thank you for your patience.
```

---

## 4. BRIDGE EXPLOIT RECOVERY

### Symptoms
- Unauthorized asset transfers from bridge
- Replay attacks detected
- Bridge balance mismatch with source chain
- "Bridge security violation" errors

### CRITICAL: This is an ACTIVE EXPLOIT scenario

**Response Time: <5 MINUTES**

### Immediate Actions (0-5 minutes)

**Step 1: PAUSE BRIDGE IMMEDIATELY**

```bash
# Incident Commander or any core team member can execute:

# Call bridge pause extrinsic (requires sudo or governance)
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "author_submitExtrinsic",
    "params": ["0x...PAUSE_BRIDGE_CALL_DATA"],
    "id": 1
  }'

# If sudo not configured (mainnet shouldn't have sudo):
# Use emergency governance proposal with fast-track
```

**🚨 IF BRIDGE CANNOT BE PAUSED:**
- **HALT THE ENTIRE NETWORK** (better to stop chain than allow drain)
- Execute network halt procedure from Playbook #1

**Step 2: Quantify Damage**

```bash
# Identify exploit transactions:
# 1. Query bridge pallet events
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_events"}' \
  http://localhost:9944 \
  | jq '.result[] | select(.event.section == "bridge")'

# 2. Calculate stolen amount
# Sum all unauthorized transfers

# 3. Identify affected users
# List all users who lost assets

# Example output:
TOTAL_STOLEN="500000 X3"
AFFECTED_USERS=15
ATTACKER_ADDRESS="5Ck5SL..."
```

**Step 3: Identify Exploit Vector**

Common bridge exploits:
1. **Replay Attack:** Same message_id processed twice
2. **Signature Forgery:** Invalid signature accepted
3. **Nonce Manipulation:** Out-of-order message processing
4. **Amount Manipulation:** Modified transfer amount
5. **Source Chain Mismatch:** Message from wrong chain accepted

```bash
# Analyze exploit transaction:
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "chain_getBlock", "params": ["0x[EXPLOIT_BLOCK_HASH]"]}' \
  http://localhost:9944 \
  | jq '.result.block.extrinsics[] | select(.method.section == "bridge")'

# Look for:
# - Duplicate message_id (replay attack)
# - Invalid signature (signature forgery)
# - Non-sequential nonce (nonce manipulation)
```

### Recovery Procedure

**Step 1: Deploy Emergency Patch**

```bash
# Core team:
# 1. Identify vulnerability in code
# 2. Write patch (add validation check)
# 3. Test on fork
# 4. Deploy via runtime upgrade

# Example patch (pseudo-code):
fn process_bridge_message(msg: BridgeMessage) -> Result<()> {
    // NEW: Check message_id not already processed
    ensure!(!ProcessedMessages::contains_key(msg.id), "Replay attack");
    
    // NEW: Verify signature with proper public key
    ensure!(msg.verify_signature(&expected_pubkey), "Invalid signature");
    
    // Existing logic...
    ProcessedMessages::insert(msg.id, ());
    Ok(())
}
```

**Step 2: Rollback Decision**

**Rollback if:**
- Exploit is ongoing (patch not stopping it)
- Stolen amount >$100k
- >10 users affected
- Exploit is <2 hours old

**Don't rollback if:**
- Exploit stopped by patch
- Stolen amount <$10k
- Compensation is cheaper than rollback
- Exploit is >24 hours old (too disruptive to rollback)

**If rollback → Follow Playbook #3 (State Corruption Recovery)**

**Step 3: Compensation (if NOT rolling back)**

```bash
# Create compensation proposal:

# 1. Snapshot affected users + amounts
affected_users = [
    ("5Ck5SL...", 10000), # User 1 lost 10k X3
    ("5Dk9XH...", 5000),  # User 2 lost 5k X3
    ...
]

# 2. Calculate compensation source:
# - Treasury funds
# - Insurance fund
# - Core team allocation

# 3. Execute compensation transfers (requires governance)
for (user, amount) in affected_users:
    governance.propose(balances.transfer(user, amount))

# 4. Publicly document compensation
```

### Post-Exploit Actions

**Within 24 hours:**
- [ ] Publish incident report
- [ ] Notify all users (even if not affected)
- [ ] Coordinate with exchanges (may need to pause deposits/withdrawals)
- [ ] Report to law enforcement (if amount >$1M)
- [ ] Engage external security auditor for emergency review

**Within 1 week:**
- [ ] Full post-mortem published
- [ ] Bounty program update (reward reporter if responsible disclosure)
- [ ] Code audit focused on exploit area
- [ ] Enhanced monitoring deployed

---

## 5. VALIDATOR MASS OFFLINE RECOVERY

### Symptoms
- >33% of validators offline (can't reach BFT threshold)
- Chain degraded but still producing blocks
- Risk of chain halt imminent

### Immediate Actions

**Step 1: Contact Offline Validators**

```bash
# Validator Coordinator sends emergency page:

EMERGENCY: Validator Mass Offline Event
Time: [ISO 8601]
Status: [X]/10 validators offline
Action: Investigate + restart ASAP

Offline validators:
- Validator 3: [Contact info]
- Validator 7: [Contact info]
- Validator 9: [Contact info]

All offline validators: Check status and report in 5 minutes.
```

**Step 2: Identify Root Cause**

Common causes:
1. **Regional Outage:** All offline validators in same datacenter/region
2. **DDoS Attack:** Coordinated attack on validator IPs
3. **Software Bug:** Bug causing validators to crash
4. **Coordinated Attack:** Malicious intent to halt chain

```bash
# Each offline validator investigates:

# System status
systemctl status x3-validator

# Recent logs
journalctl -u x3-validator --since "10 minutes ago"

# Network connectivity
ping 8.8.8.8
traceroute [BOOTNODE_IP]

# Resource usage
df -h
free -h
```

### Recovery Strategies

**Scenario A: Regional Outage**
```bash
# If all offline validators in same region:
# 1. Coordinate with datacenter provider
# 2. Migrate validators to backup regions (if available)
# 3. Or wait for regional outage resolution

# Expected time: 30 minutes - 4 hours (depends on provider)
```

**Scenario B: DDoS Attack**
```bash
# If validators being DDoS'd:
# 1. Enable DDoS protection (Cloudflare, AWS Shield)
# 2. Change validator IPs (if possible)
# 3. Use VPN/proxy for validator connections

# Emergency IP change:
systemctl stop x3-validator
# Change server IP in DNS
# Update firewall rules
systemctl start x3-validator

# Expected time: 1-2 hours
```

**Scenario C: Software Bug**
```bash
# If validators crashing due to bug:
# 1. Collect crash dumps from all affected validators
# 2. Core team analyzes crash
# 3. Deploy hotfix
# 4. Coordinate restart

# Expected time: 2-6 hours (depends on bug complexity)
```

---

## 6. NETWORK PARTITION RECOVERY

### Symptoms
- Validators split into groups that can't communicate
- Multiple competing chains
- Conflicting telemetry data

### Recovery Procedure

**Step 1: Identify Partition Topology**

```bash
# Each validator reports peer list:
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method": "system_peers"}' \
  http://localhost:9944 \
  | jq -r '.result[].peerId'

# Create partition map:
Partition A: Validators 1,2,3,4,5 (can see each other)
Partition B: Validators 6,7,8,9,10 (can see each other)
No cross-partition connectivity
```

**Step 2: Identify Root Cause**

Common causes:
1. **Network Infrastructure:** BGP routing issue, ISP outage
2. **Firewall Rules:** Misconfigured firewall blocking cross-region
3. **NAT Issues:** Port forwarding broken

**Step 3: Restore Connectivity**

```bash
# Test direct connectivity:
nc -zv [OTHER_PARTITION_VALIDATOR_IP] 30333

# If fails:
# 1. Check firewall rules
# 2. Verify port 30333 open
# 3. Test with traceroute

# Fix firewall (example):
sudo ufw allow 30333/tcp
sudo ufw reload
```

**Step 4: Force Peering**

```bash
# Validators in minority partition manually connect to majority:
curl -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "system_addReservedPeer",
    "params": ["/ip4/[MAJORITY_VALIDATOR_IP]/tcp/30333/p2p/[PEER_ID]"],
    "id": 1
  }'
```

---

## Communication Protocols

### Internal Alerts (Slack/Discord)

**S0 Incident:**
```
@channel 🚨 S0 INCIDENT DECLARED 🚨
Type: [Chain Halt / Exploit / State Corruption]
Time: [ISO 8601]
IC: @[Incident Commander Name]
War Room: [Zoom link]
Status Page: https://status.x3.network

ALL HANDS REQUIRED - Join war room immediately.
```

**S1/S2 Incident:**
```
@ops-team ⚠️ S1 INCIDENT
Type: [Consensus Failure / Validator Offline]
Time: [ISO 8601]
IC: @[Incident Commander Name]
Thread: [Discord thread link]

On-call team: Investigate and report status in 15 min.
```

### Public Status Updates

**Update Frequency:**
- S0: Every 30 minutes until resolved
- S1: Every 1 hour
- S2: Every 4 hours

**Template:**
```
🔄 INCIDENT UPDATE [HH:MM UTC]

STATUS: [Investigating / Identified / Fixing / Monitoring / Resolved]

WHAT'S HAPPENING:
[Brief non-technical explanation]

CURRENT IMPACT:
- Network: [Operational / Degraded / Down]
- Transactions: [Processing / Delayed / Paused]
- Your funds: [Safe / At risk - explain]

WHAT WE'RE DOING:
[Actions being taken]

NEXT UPDATE: [Time]

Full details: https://status.x3.network/incidents/[ID]
```

---

## Post-Incident Procedures

### Post-Mortem Template

**Due:** Within 72 hours of incident resolution

```markdown
# Incident Post-Mortem: [TITLE]

**Incident ID:** INC-[YYYYMMDD]-[SEQ]  
**Date:** [ISO 8601]  
**Severity:** [S0/S1/S2]  
**Duration:** [Start] to [End] ([Duration])  
**Impact:** [Brief summary]  

## Summary
[2-3 sentence summary of what happened]

## Timeline
All times in UTC.

| Time | Event | Actor |
|------|-------|-------|
| 12:00 | Incident detected | Monitoring system |
| 12:05 | IC declared S0 incident | [Name] |
| 12:15 | Root cause identified | [Name] |
| 12:30 | Fix deployed | [Name] |
| 13:00 | Service restored | [Name] |
| 13:30 | Monitoring confirmed recovery | [Name] |

## Root Cause
[Detailed technical explanation of what caused the incident]

## Resolution
[What was done to resolve the incident]

## User Impact
- Users affected: [Number]
- Transactions lost/delayed: [Number]
- Financial impact: $[Amount]
- Downtime: [Duration]

## What Went Well
1. [Success 1]
2. [Success 2]

## What Went Wrong
1. [Failure 1]
2. [Failure 2]

## Action Items
| ID | Action | Owner | Due Date | Status |
|----|--------|-------|----------|--------|
| AI-1 | [Action] | [Name] | [Date] | Open |
| AI-2 | [Action] | [Name] | [Date] | Open |

## Lessons Learned
1. [Lesson 1]
2. [Lesson 2]

---
*Prepared by: [Name]*  
*Reviewed by: [Names]*  
*Approved by: [IC + CTO signatures]*
```

---

## Appendix: Emergency Contacts

### Core Team 24/7 On-Call

| Role | Primary | Backup | Phone | Telegram |
|------|---------|--------|-------|----------|
| Incident Commander | [Name 1] | [Name 2] | +1-XXX | @xxx |
| Technical Lead | [Name 3] | [Name 4] | +1-XXX | @xxx |
| Security Lead | [Name 5] | [Name 6] | +1-XXX | @xxx |

### External Escalation

| Service | Contact | Phone | Email | SLA |
|---------|---------|-------|-------|-----|
| AWS Support | Enterprise Support | +1-XXX | aws@x3.network | 15 min |
| Cloudflare | Priority Support | +1-XXX | cf@x3.network | 30 min |
| Security Auditor | Trail of Bits | +1-XXX | emergency@trailofbits.com | 2 hours |
| Legal Counsel | [Firm] | +1-XXX | legal@firm.com | 4 hours |

---

**END OF RUNBOOK**

*This document is classified as: INTERNAL*  
*Last updated: April 26, 2026*  
*Version: 1.0*  
*Owner: X3 Operations Team*
