# X3 Operator Cockpit Specification

*Unified live dashboard specification for the X3 operator surface. This document defines what must be visible, actionable, and monitorable from a single view during normal operations and incidents.*

**Status:** data sources exist across multiple crates and services. The unified dashboard is planned. Individual telemetry, chain liveness, and proof metrics are partially available through separate tools. This spec is the target state.

---

## Purpose

Operators should not need to join six tools mentally during an incident. The cockpit aggregates chain health, cross-VM flow state, proof backlog, scheduler queues, node health, active agents, emergency toggles, incident banners, and growth/content pipelines into one queryable, actionable surface.

---

## Panels

### Panel 1 — Chain Liveness

| Widget | Data source | Refresh | Alert threshold |
|---|---|---|---|
| Block height (per VM: X3, EVM, SVM) | Node RPC — `crates/x3-rpc` | 1s | No new block in 6s |
| Finality lag (blocks behind tip) | Finality oracle — `crates/x3-finality-oracle` | 1s | Lag > 3 blocks |
| Peer count | Node networking | 5s | < 3 peers |
| Validator set active / total | Staking pallet | 1 epoch | Active < 2/3 of total |
| Last slot producer | Consensus module | 1s | Slot missed |
| TPS (1s rolling average) | `crates/tps-tracker` | 1s | Drop > 30% from baseline |
| Mempool depth | Private mempool service | 1s | Depth > 5000 |

**Status:** `tps-tracker` and RPC implemented; finality oracle partially wired; unified liveness feed not yet assembled.

---

### Panel 2 — Cross-VM Flow State

| Widget | Data source | Refresh | Alert threshold |
|---|---|---|---|
| In-flight cross-VM transactions | `crates/cross-vm-coordinator` | 1s | Count > 100 |
| Pending prepare / commit / abort counts | cross-vm-coordinator | 1s | Any stuck > 30s |
| Route circuit-breaker status (per route) | `crates/x3-circuit-breaker` | 2s | Any `Tripped` |
| Gateway status (per gateway) | circuit-breaker, gateway health | 5s | Any `Tripped` |
| DexPool status | circuit-breaker | 5s | Any `Tripped` |
| Last cross-VM settlement age | cross-vm-coordinator | 5s | > 60s |

**Status:** circuit-breaker status reads are available; cross-VM in-flight count feed is planned.

---

### Panel 3 — Proof Pipeline

| Widget | Data source | Refresh | Alert threshold |
|---|---|---|---|
| Proof generation queue depth | GPU validator swarm — `crates/x3-gpu-validator-swarm` | 2s | Depth > 50 |
| Proof verification backlog | `crates/x3-verifier` | 2s | Backlog > 20 |
| Proof failure rate (rolling 5m) | verifier telemetry | 5s | > 5% |
| Active proof disputes | `crates/x3-proof-dispute` | 5s | Any open > 5m |
| Pending slashing resolutions | `crates/x3-slash` | 10s | Any unresolved > 10m |
| Evidence bundles exported today | Sentinel-Scribe (planned) | 10s | N/A |

**Status:** proof crates exist; unified telemetry feed is not yet assembled.

---

### Panel 4 — Scheduler and Swarm

| Widget | Data source | Refresh | Alert threshold |
|---|---|---|---|
| Job queue depth (by class) | `crates/x3-swarm-core/src/scheduler.rs` | 2s | Any class queue > 20 |
| Active agent count (by kind) | swarm-core agent registry | 5s | > `max_agent_count` |
| Jobs blocked at approval | `crates/x3-swarm-core/src/approval.rs` | 5s | Blocked > 2h |
| Agent violations today | guard log | 10s | Any Class A/C/D |
| Quarantined agents | guard state | 10s | Any quarantine active |
| Spawn depth warnings | spawning module (planned) | 10s | Any depth > limit |

**Status:** scheduler and approval modules exist; unified approval age tracking is planned.

---

### Panel 5 — Node Health

| Widget | Data source | Refresh | Alert threshold |
|---|---|---|---|
| Validator node status (online / syncing / offline) | Node health API | 5s | Any offline |
| GPU worker status | GPU swarm telemetry | 5s | Any offline |
| CPU utilisation (per node) | node telemetry | 10s | > 90% for > 60s |
| GPU utilisation (per GPU worker) | GPU swarm telemetry | 10s | > 95% for > 60s |
| Disk usage (per node) | node telemetry | 30s | > 85% |
| Network latency (inter-validator, p50/p99) | validator networking | 10s | p99 > 200ms |

**Status:** GPU swarm telemetry is implemented; unified node health view is not yet assembled.

---

### Panel 6 — Emergency Toggles

Live action buttons. Each button requires operator authentication before executing.

| Button | Action | Requires | Confirmation |
|---|---|---|---|
| Trip asset breaker | `CircuitBreakerEngine::trip_circuit_breaker(Asset, ...)` | Operator key | Confirm + reason required |
| Trip route breaker | `trip_circuit_breaker(Route, ...)` | Operator key | Confirm + reason required |
| Trip gateway breaker | `trip_circuit_breaker(Gateway, ...)` | Operator key | Confirm + reason required |
| Reset breaker | `reset_circuit_breaker(scope, true)` | Operator key | Confirm |
| Quarantine validator | quarantine path in GPU swarm | Operator key + evidence hash | Confirm + evidence required |
| Pause governance queue | constitution engine pause | Operator key | Confirm |
| Suspend agent | swarm-core quarantine | Operator key | Confirm |
| Kill agent | swarm-core kill path (planned) | Operator key + evidence bundle | Double-confirm |
| Emergency dump: chain state snapshot | snapshot exporter | Operator key | N/A |

**Status:** circuit-breaker reset/trip are implemented; UI surface and authentication wrapper are planned.

---

### Panel 7 — Incident Banner and Log

| Element | Description |
|---|---|
| Active incident banner | Sticky header shown when any emergency action is active. Shows scope, action type, triggered-at block, triggered-by, and return-to-normal checklist |
| Recent actions log | Last 50 emergency actions with: actor, scope, action, block, evidence hash, status (active / resolved) |
| Alert history | All threshold alerts in the last 24h |
| Postmortem queue | Open postmortems with age and assigned operator |

**Status:** planned. No unified incident log surface exists yet.

---

### Panel 8 — Content and Outreach Pipeline (read-only)

| Widget | Data source | Refresh |
|---|---|---|
| Drafts awaiting human approval | content approval queue | 30s |
| Posts approved and scheduled | publishing pipeline (planned) | 30s |
| Outbound actions today (by surface) | outbound policy log (planned) | 5m |
| Policy violations today | outbound gate log | 5m |

**Status:** planned. See [OUTBOUND_POLICY.md](OUTBOUND_POLICY.md) for gate definitions.

---

## Query interface

The cockpit must expose a query API so operators can search by:

- Entity (agent ID, validator ID, route ID, asset ID)
- Campaign or task class
- Time range
- Event type (alert, action, violation, proof failure)
- Invariant ID

**Status:** planned. Individual crates expose their own query endpoints; unified cross-entity search is not yet built.

---

## Access control

| Role | Panels visible | Panels actionable |
|---|---|---|
| Read-only operator | All | None |
| Operator | All | Emergency toggles, agent management |
| Security operator | All + incident detail | Emergency toggles, quarantine |
| Governance key | All | Full |

**Status:** planned. No role-based dashboard ACL is implemented yet.

---

## Integration targets

When built, the cockpit pulls from:

- `crates/x3-rpc` — chain liveness
- `crates/tps-tracker` — TPS
- `crates/x3-circuit-breaker` — breaker status
- `crates/cross-vm-coordinator` — cross-VM flow
- `crates/x3-gpu-validator-swarm` — GPU metrics and quarantine
- `crates/x3-swarm-core` — agent and scheduler state
- `crates/x3-slash` — slashing and bond state
- `crates/x3-proof-dispute` — open disputes
- `crates/x3-finality-oracle` — finality lag
- `x3-security-swarm` — Sentinel findings (planned)

---

## Open gaps (as of 2026-05-08)

| Gap | Priority |
|---|---|
| Unified telemetry aggregation service | Band 0 |
| Dashboard frontend (web or Tauri-based) | Band 0 |
| Emergency toggle authentication wrapper | Band 0 |
| Incident banner and postmortem queue | Band 0 |
| Cross-entity search and query API | Band 1 |
| Role-based access control | Band 1 |
| Content/outreach pipeline panel | Band 2 |
