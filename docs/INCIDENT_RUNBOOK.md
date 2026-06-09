# X3 Atomic Star — Incident Runbook

**Version:** 1.0
**Status:** Active
**Date:** 2026-06-09
**Scope:** Procedures for responding to critical incidents on X3 Atomic Star mainnet and public testnet

---

## 1. Purpose

This runbook provides step-by-step procedures for responding to critical incidents on the X3 Atomic Star chain. Every incident handler must follow these procedures. Deviation requires documented justification.

---

## 2. Incident Severity Levels

| Level | Name | Criteria | Response Time | Escalation |
|---|---|---|---|---|
| **SEV0** | Chain Halt / Invariant Break | Blockchain stopped producing blocks OR supply invariance violation detected | Immediate (<5 min) | All Tier 1 + multisig signers |
| **SEV1** | Security Breach | Confirmed exploit, bridge compromise, double-spend, or unauthorized mint | <30 min | Tier 1 + Security Lead |
| **SEV2** | Degraded Service | >1/3 validators offline, severe network partition, RPC outage | <1 hour | Tier 1 |
| **SEV3** | Anomaly | Unexpected but non-critical behavior (e.g., flash finality timeouts, performance degradation) | <24 hours | Infrastructure Lead |

---

## 3. General Incident Response Procedure

### 3.1 Detection

Incidents may be detected via:
- Monitoring alerts (Prometheus/Grafana)
- Node operator reports (Matrix / Signal)
- Community reports (Discord / GitHub issues)
- External security disclosures (security@x3atomicstar.io)

### 3.2 Triage (First 5 Minutes)

1. **Acknowledge:** First responder acknowledges the alert in the Signal emergency channel.
2. **Assess:** Determine severity level using the criteria in §2.
3. **Notify:** For SEV0/SEV1, immediately notify all Tier 1 contacts.
4. **Log:** Open an incident in the incident tracker with timestamp, severity, and initial assessment.

### 3.3 Containment

1. **SEV0 (Chain Halt):** Do NOT restart nodes until root cause is identified. Collect logs from all validator operators.
2. **SEV1 (Security Breach):**
   - If an exploit is in progress, multisig signers should evaluate whether to trigger the **emergency pause** extrinsic (if implemented).
   - Isolate affected infrastructure (e.g., take compromised RPC nodes offline).
   - Preserve all logs and chain state for forensics.
3. **SEV2 (Degraded Service):** Contact affected validator operators; restart/reconnect nodes as needed.
4. **SEV3 (Anomaly):** Log for investigation; no immediate action required.

### 3.4 Investigation

1. Assign an incident commander (typically the most senior Tier 1 contact available).
2. Collect diagnostic data:
   - Node logs (last 1000 lines minimum) from all validators.
   - Chain state at the block of the incident.
   - Relevant extrinsics and events from the incident block.
   - Network telemetry (latency, packet loss, peer counts).
3. Determine root cause.
4. Document findings in the incident tracker.

### 3.5 Resolution

1. **SEV0/SEV1:** Resolution must be approved by at least 2 Tier 1 contacts.
2. If a runtime upgrade is required:
   - Propose upgrade via governance (or emergency governance if enabled).
   - Coordinate validator operators to upgrade their nodes.
   - Monitor upgrade enactment.
3. Verify the chain has resumed normal operation:
   - Blocks are being produced and finalized.
   - Supply ledger invariants hold (`TotalIssuance == sum(all accounts)`).
   - All internal cross-VM routes are functional.

### 3.6 Post-Mortem

1. Within 72 hours of resolution, publish a post-mortem containing:
   - Timeline of the incident.
   - Root cause analysis.
   - Impact assessment (funds at risk, downtime, affected users).
   - Actions taken to resolve.
   - Preventative measures and follow-up tasks.
2. File follow-up tasks as GitHub issues with the `incident-followup` label.
3. Update this runbook if the incident revealed gaps in procedure.

---

## 4. Scenario-Specific Procedures

### 4.1 Chain Halt (No Blocks Produced)

**Symptoms:** Block height stopped incrementing; all RPC nodes report same latest block.

**Procedure:**
1. Confirm halt: Query ≥3 independent RPC nodes for `system_number()`.
2. Check validator logs for:
   - BABE slot errors ("cannot claim slot" / "no block produced in slot").
   - GRANDPA finality errors ("round timed out" / "no pre-commits").
   - Networking errors ("no peers" / "peer count = 0").
3. Check validator status: `curl http://<node>:9933` → `isSyncing`, `peers`, `bestNumber`.
4. If cause is network partition: Re-establish connectivity; chain should auto-recover.
5. If cause is consensus failure (e.g., >1/3 validators crashed):
   - Restart crashed validators.
   - Chain will resume finalizing once >2/3 are online.
6. If cause is unknown or a runtime bug:
   - Do NOT restart nodes immediately — preserve state for debugging.
   - Escalate to Tier 1 contacts.
   - Consider emergency runtime upgrade via governance.

### 4.2 Supply Invariant Violation

**Symptoms:** `TotalIssuance != Σ(all account balances)` (detected by supply-ledger pallet).

**Procedure:**
1. **IMMEDIATE:** The supply-ledger pallet will reject the offending extrinsic. The chain should still produce blocks with the invariant intact for non-offending extrinsics.
2. Identify the extrinsic that attempted to break the invariant (check `system_events()` for `SupplyLedger::InvariantViolated`).
3. Trace the offending operation back to its origin (cross-VM transfer, mint, burn).
4. Determine if this was:
   - A bug (implementation error) → Fix and upgrade.
   - An exploit attempt → Trigger security incident (SEV1).
   - A false positive (invariant check is too strict) → Assess and adjust.
5. Do NOT disable the invariant check — it is the last line of defense.

### 4.3 Bridge Compromise

**Symptoms:** Unauthorized cross-chain messages, unexpected mint/burn events, relayers reporting anomalies.

**Procedure:**
1. Check `ExternalBridgesEnabled` in chain state — if still `false` at genesis, external bridges are not yet active; this is a non-issue.
2. If external bridges are enabled and a compromise is suspected:
   - The on-chain multisig should call the `pause_bridge` extrinsic immediately.
   - Identify the affected bridge route and pause only that route if possible.
   - Audit all cross-chain messages since the suspected compromise time.
   - Notify the other chain's validators/operators.
3. Recovery requires a runtime upgrade to patch the vulnerability before bridges are re-enabled.

### 4.4 Validator Equivocation

**Symptoms:** `Offences::Offence` event with `BabeEquivocation` or `GrandpaEquivocation`.

**Procedure:**
1. Verify the offence report is valid (the runtime validates equivocation proofs before slashing).
2. Identify the equivocating validator.
3. The validator is automatically slashed per the staking pallet's offence handler.
4. Contact the validator operator to investigate how the equivocation occurred (misconfiguration, key leak, or malicious action).
5. If the slash was false (bug in equivocation detection): This is a SEV1 — escalate immediately.

### 4.5 Large-Scale Validator Outage (>1/3 Offline)

**Symptoms:** Finality stalled; chain still producing blocks but not finalizing.

**Procedure:**
1. Contact all known validator operators to determine cause.
2. If cause is infrastructure (e.g., cloud provider outage): Wait for recovery or assist operators in migrating.
3. If cause is a coordinated attack: The chain will stall on finality but blocks will continue being produced (safety is maintained, liveness is lost temporarily).
4. Recovery: As validators come back online, GRANDPA will resume finalizing blocks. No chain restart needed.

---

## 5. Emergency Governance Actions

### 5.1 Emergency Pause (Kill-Switch)

If a critical vulnerability is actively being exploited:

1. Multisig signers (≥3-of-5) execute `Utility::batch` containing:
   - `Scheduler::cancel(all_scheduled)` — cancel all scheduled calls.
   - `X3SupplyLedger::halt()` (if implemented) — halt all mint/burn operations.
2. All cross-VM transfers will be rejected.
3. Normal block production continues; only mint/burn and cross-VM operations are halted.

### 5.2 Emergency Runtime Upgrade

If a runtime bug must be patched urgently:

1. Multisig signers propose a `System::set_code` call with the patched WASM blob.
2. Validator operators are notified to upgrade their nodes.
3. Once enacted, verify the new runtime is functioning correctly.

---

## 6. Communication Templates

### 6.1 Initial Incident Notification (Signal / Matrix)

```
🚨 X3 INCIDENT — SEV<0/1/2/3>
Time: <ISO timestamp>
Summary: <1-line description>
Affected: <mainnet / testnet / specific validators>
Current status: <investigating / containing / resolving>
Incident commander: <name>
Next update: <time>
```

### 6.2 Status Update (Every 30 Minutes for SEV0/SEV1)

```
📊 X3 INCIDENT UPDATE — SEV<0/1>
Time: <ISO timestamp>
Root cause: <identified / still investigating>
Actions taken: <bullet list>
Current chain status: <block production status / finality status>
Earliest resolution estimate: <time>
```

### 6.3 Resolution Notification

```
✅ X3 INCIDENT RESOLVED — SEV<0/1/2/3>
Time: <ISO timestamp>
Duration: <duration>
Root cause: <summary>
Actions taken: <bullet list>
Post-mortem: Expected within 72 hours at <link>
```

---

## 7. Diagnostic Commands

### 7.1 Chain Status
```bash
# Check current block number
curl -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' \
  http://localhost:9933

# Check if node is syncing
curl -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' \
  http://localhost:9933
```

### 7.2 Validator Status
```bash
# Check validator keys
curl -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"author_hasKey","params":["<publicKey>","babe"]}' \
  http://localhost:9933
```

### 7.3 Supply Invariant Check
```bash
# Via sidecar API (if running)
curl http://localhost:8080/supply/invariant
# Expected: true with current total and sum
```

---

## 8. Training and Drills

- **Tabletop exercises:** Quarterly — walk through a SEV0 scenario with all Tier 1 contacts.
- **Testnet fire drills:** Monthly — simulate a chain halt or validator outage on testnet and practice the response procedure.
- **Multisig signing drill:** Quarterly — verify all 5 multisig signers can successfully propose and execute a test governance call.

---

## 9. Document Maintenance

- This runbook must be reviewed and updated within 7 days after any real incident.
- Contact information in `EMERGENCY_CONTACTS.md` must be verified monthly.
- All changes must be reviewed by at least one Tier 1 contact.