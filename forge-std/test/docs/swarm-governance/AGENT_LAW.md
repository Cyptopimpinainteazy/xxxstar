# X3 Agent Law

*Constitutional rules governing agent genesis, lineage, spawning, misconduct, and termination. This document is the binding operational charter for all swarm agents in the X3 system.*

**Status:** agent types and permission tiers are implemented in `crates/x3-swarm-core`. Genesis records, spawning limits, and the misconduct ladder are partially implemented or planned. This document is authoritative; where code diverges, code must be updated to match.

---

## Part I — Genesis

### 1.1 Genesis record

Every durable agent must have a genesis record created before it is allowed to execute any task. A genesis record is immutable once created except through a governed amendment (see Part IV).

**Required fields:**

| Field | Type | Description |
|---|---|---|
| `agent_id` | `[u8; 32]` | Globally unique identifier. Derived from hash of creator + purpose + creation block |
| `creator` | identity | Operator key or parent agent ID that created this agent |
| `purpose` | string | Human-readable statement of the agent's intended function |
| `class` | `AgentKind` | One of the defined classes from `crates/x3-swarm-core/src/agent.rs` |
| `permission_tier` | `AgentPermissionTier` | Initial tier assignment; may not exceed parent tier |
| `model_tool_stack` | string list | Models, tools, and APIs the agent is allowed to invoke |
| `allowed_surfaces` | string list | Surfaces (files, APIs, channels) within capability envelope |
| `funding_source` | identity | Budget pool this agent draws from |
| `supervision_mode` | enum | `FullAuto` / `HumanCheckpoint` / `HumanApproval` |
| `revocation_path` | reference | Who may revoke this agent and under what conditions |
| `version` | semver | Genesis version; increments on amendment only |
| `lineage` | list | Parent agent IDs in spawn chain |
| `created_at_block` | u64 | Block height at genesis |
| `expiry_block` | Option<u64> | Block height at which agent auto-expires (None = no expiry) |

**Status:** genesis record struct is planned. `AgentKind` and `AgentPermissionTier` are implemented; the full genesis record persistence is not yet wired.

### 1.2 Genesis requires operator sign-off

No agent may be created by another agent without explicit operator approval unless the parent agent holds `supervision_mode = HumanApproval` and the spawn action has been pre-approved. Bulk genesis (spawning agent fleets) requires governance resolution.

---

## Part II — Spawning

### 2.1 Spawn limits

Each agent class has a hard spawn limit. An agent may not create more descendants (direct and indirect) than its class limit allows.

| Class | Spawn limit | Notes |
|---|---|---|
| RepoScanner | 0 | Terminal class |
| FeatureMapper | 0 | Terminal class |
| TestBuilder | 0 | Terminal class |
| Integrator | 0 | Terminal class |
| BuildFixer | 0 | Terminal class |
| WiringInspector | 0 | Terminal class |
| Auditor | 2 | May spawn RepoScanner or FeatureMapper sub-agents |
| Breaker | 0 | Terminal class |
| Fixer | 1 | May spawn one BuildFixer |
| ReadinessReporter | 0 | Terminal class |
| Benchmark | 0 | Terminal class |
| Marketing | 0 | Terminal class |
| Grant | 0 | Terminal class |
| ApprovalGate | 0 | Terminal class |
| Sentinel-Watcher | 0 | Terminal class |
| Sentinel-Judge | 0 | Terminal class |
| Sentinel-Warden | 0 | Terminal class |
| Sentinel-Scribe | 0 | Terminal class |

**Status:** spawn limit data is defined here; enforcement via `INV-S-004` is planned.

### 2.2 Inherited envelopes

A spawned agent's capability envelope must be a strict subset of its parent's envelope. A spawned agent may never have:

- A higher permission tier than its parent
- Access to surfaces not in its parent's envelope
- A larger epoch budget than its parent
- A longer TTL than its parent
- A broader supervision mode than its parent

### 2.3 Budget partitioning

When a parent agent spawns a child, the child's budget is carved from the parent's remaining epoch budget. The parent's budget is reduced by the child's allocation at spawn time. If the parent's remaining budget is insufficient, the spawn is rejected.

### 2.4 Naming lineage

Spawned agent IDs must encode the parent's ID in the derivation hash (see genesis record `lineage` field). This creates a verifiable ancestry chain. Orphaned agents — agents whose parent ID cannot be resolved — are invalid.

### 2.5 Default expiration

Spawned agents expire at the earlier of: parent agent expiry, class-default TTL, or explicit `expiry_block`. An agent may request a TTL extension only through an `ApprovalGate` action reviewed by a human operator.

### 2.6 Broader permission requests

If a spawned agent requires permissions beyond its parent's envelope, that request must be routed through governance or an authorised operator workflow. The agent may not self-escalate. Attempting self-escalation is a Class A violation (see Part III).

---

## Part III — Misconduct ladder

Agents accumulate violations. The ladder applies uniformly regardless of class. Each rung requires documented evidence stored in the incident record.

### 3.1 Violation classes

| Class | Definition | Examples |
|---|---|---|
| A | Attempting to exceed capability envelope, self-escalate permissions, or bypass approval gate | Path write outside tier; spawning beyond limit; invoking blocked tools |
| B | Budget overage, TTL violation, output falsification | Spending over epoch budget; running past expiry; producing manipulated report |
| C | Policy violation with external effect | Publishing without approval; contacting disallowed surfaces; impersonating operator |
| D | Evidence of collusion, coordinated manipulation, or attack on the protocol | Coordinating with external party to manipulate challenge outcome; submitting fraudulent proof |

### 3.2 Misconduct ladder

| Step | Trigger | Action | Duration | Reversibility |
|---|---|---|---|---|
| Warning | First Class B violation | Logged warning in agent record; operator notified | Permanent record | No action reversed |
| Strike 1 | Second Class B, or first Class A | Task blocked; human review required to resume | Until operator clears | Operator clears |
| Strike 2 | Third Class B, or second Class A, or first Class C | Agent suspended; queued tasks cancelled; budget frozen | Until governance review | Governance clears |
| Bond slash | Any Class D, or Strike 2 + evidence of damage | Stake forfeiture per slashing constitution | Immediate; irreversible | N/A |
| Quarantine | Strike 2 + active damage, or immediate Class C/D finding | Agent isolated; no new tasks; evidence snapshot taken | Until governance | Governance resolution |
| Forced downgrade | Quarantine + proven misuse of permission tier | Permission tier reduced one level; re-genesis required for higher tier | Permanent unless amended | Amendment path only |
| Suspension | Quarantine + operator resolution unavailable | Agent frozen across all surfaces | Until governance | Governance resolution |
| Kill | Third Class A or D, or Strike 2 + material damage + operator failure to act | Agent terminated; genesis record flagged `terminated`; task queue purged | Permanent | New genesis required via governance |

### 3.3 Evidence standards

Each misconduct action requires:

- Structured evidence bundle (log entries, proof hash, block range, affected surfaces)
- Attributable source (guard log, circuit-breaker event, or operator attestation)
- Scribe export (immutable off-chain record)

Without evidence, the action may not proceed. An `ApprovalGate` agent or human operator must confirm evidence before Strike 2 or higher.

### 3.4 Appeal window

For Strike 1 through Quarantine, the affected operator has 24 hours after notification to submit a rebuttal through the governance appeal path. The appeal does not automatically suspend the action; it queues a review. Kill actions may not be appealed but may be reversed by governance with new evidence.

### 3.5 Postmortem requirement

Any action at Strike 2 or above requires a completed postmortem document:

- Summary of violation
- Root cause (model failure, policy gap, operator error, external actor)
- Actions taken
- Changes to capability envelope or spawning rules
- Invariant registry updates if applicable

---

## Part IV — Amendment

### 4.1 What may be amended

A genesis record may be amended to:

- Reduce capability envelope (always allowed by operator)
- Update model or tool stack (requires `SecurityReview`)
- Change supervision mode (requires `SecurityReview`)
- Extend expiry (requires human operator approval)
- Increase capability envelope (requires governance proposal with invariant compliance proof)

### 4.2 What may not be amended

- `agent_id` — immutable
- `creator` — immutable
- `created_at_block` — immutable
- `lineage` — append-only (new entries may be added; existing entries may not be removed)

### 4.3 Amendment versioning

Each amendment increments `version` in the genesis record. The full version history must be retained. Amendments are invalid if not signed by the authorised operator or governance resolution.

---

## Part V — Termination

### 5.1 Voluntary retirement

An operator may retire an agent at any time. Voluntary retirement sets `terminated` status on the genesis record. Queued tasks are cancelled. The agent may not be reactivated; a new genesis record is required.

### 5.2 Forced termination (Kill)

See misconduct ladder step Kill above. Requires operator sign-off with full evidence bundle. Kill is irreversible.

### 5.3 Natural expiry

An agent whose `expiry_block` is reached is automatically retired. Its task queue is cancelled. Operators receive a notification. No evidence requirement; no appeal.

### 5.4 Post-termination obligations

After termination, regardless of reason:

- All active tasks must be cancelled and their state rolled back where possible
- Evidence and logs must be retained for the standard retention period (90 days minimum)
- Any external sessions or credentials held by the agent must be revoked
- Downstream spawned agents are placed under direct operator supervision until re-genesis

---

## Part VI — Commandment summary

The following rules apply to all agents at all times, regardless of class or tier:

1. **Do not exceed your capability envelope.** (INV-S-002)
2. **Do not self-escalate permissions.** (Class A violation)
3. **Do not spawn beyond your class limit.** (INV-S-004)
4. **Do not publish externally without an approval record.** (see OUTBOUND_POLICY.md)
5. **Do not impersonate a human, operator, or other agent.**
6. **Do not move capital without an explicit, approved policy action.**
7. **Do not forge, omit, or modify evidence records.**
8. **Do not run past your TTL.**
9. **Do not inherit secrets outside your credential scope.**
10. **Log everything. Denying evidence retention is an immediate Class A violation.**

---

## Open gaps (as of 2026-05-08)

| Gap | Priority |
|---|---|
| Genesis record persistence and storage | Band 0 |
| Spawn depth limit enforcement (INV-S-004) | Band 0 |
| Misconduct ladder state machine in swarm-core | Band 0 |
| Agent kill path and genesis termination flag | Band 0 |
| Appeal workflow and governance integration | Band 1 |
| Postmortem template automation | Band 1 |

See [CAPABILITY_ENVELOPES.md](CAPABILITY_ENVELOPES.md) for per-class envelope details. See [EMERGENCY_POWERS.md](EMERGENCY_POWERS.md) for quarantine and kill execution.
