# X3 Emergency Powers

*Formal map of every emergency action available in the X3 system. Each entry defines who may trigger it, what evidence is required, how long it stays active, what on-chain event is emitted, and how the system returns to normal.*

**Status:** `CircuitBreakerEngine` and its scopes are implemented in `crates/x3-circuit-breaker`. Validator quarantine primitives exist in `crates/x3-gpu-validator-swarm/src/quarantine.rs` and governance override in `crates/gpu-swarm/src/warden/governance.rs`. The expiry, audit-trail, and return-to-normal flows listed here are partially implemented or planned.

---

## Action taxonomy

The system distinguishes four escalating action tiers:

| Tier | Name | Effect | Reversibility |
|---|---|---|---|
| 1 | **Pause** | Halt a narrow scope; no state mutation | Reversible by authorised operator |
| 2 | **Degrade** | Reduce service to safe-mode for a scope | Reversible by authorised operator |
| 3 | **Quarantine** | Isolate a node or agent; block outbound effects | Reversible by governance or quorum |
| 4 | **Kill** | Terminate a node or agent; purge task queue | Requires governance ratification to undo |

No system component may skip a lower tier without documented justification in the incident record.

---

## Scope definitions

Scopes map to `CircuitBreakerScope` in `crates/x3-circuit-breaker/src/lib.rs`:

| Scope | Identifier | Description |
|---|---|---|
| Asset | `CircuitBreakerScope::Asset(id)` | A specific token or asset |
| Route | `CircuitBreakerScope::Route(id)` | A cross-chain or cross-VM execution route |
| Gateway | `CircuitBreakerScope::Gateway(id)` | An external chain gateway or bridge endpoint |
| DexPool | `CircuitBreakerScope::DexPool(id)` | A DEX pool or liquidity lane |
| Verifier | `CircuitBreakerScope::Verifier(id)` | A proof verifier node or service |
| Validator | separate — quarantine module | A consensus validator node |
| Agent | separate — swarm guard | A swarm agent identity |
| Governance | separate — constitution engine | The governance proposal queue |

---

## Emergency actions table

### PA-01 — Trip asset circuit breaker (Pause)

| Field | Value |
|---|---|
| Scope | Asset |
| Tier | Pause |
| Triggers | Proof of price manipulation, evidence of minting invariant violation, oracle anomaly |
| Who may trigger | `Sentinel-Warden` (quorum), on-chain governance, authorised operator key |
| Evidence required | Signed anomaly report from `Sentinel-Judge`, or on-chain state proof |
| Duration | Until reset; maximum 72 blocks without governance extension |
| On-chain event | `AssetCircuitTripped { asset_id, reason_hash, tripped_at_block }` |
| Normal return | `CircuitBreakerEngine::reset_circuit_breaker` with `privileged_origin = true` |
| Enforcement | `CircuitBreakerEngine::trip_circuit_breaker` — implemented |
| Gaps | Expiry enforcement and auto-reset at block not yet wired |

---

### PA-02 — Trip route circuit breaker (Pause)

| Field | Value |
|---|---|
| Scope | Route |
| Tier | Pause |
| Triggers | Failed cross-VM commit, proof mismatch, bridge desync signal |
| Who may trigger | `Sentinel-Warden` (quorum), validator majority, authorised operator |
| Evidence required | Cross-VM failure proof, or 3-of-5 validator attestation |
| Duration | Until reset; maximum 72 blocks |
| On-chain event | `RouteCircuitTripped { route_id, reason_hash, tripped_at_block }` |
| Normal return | Privileged reset after root cause confirmed |
| Enforcement | implemented |
| Gaps | Expiry gate planned |

---

### PA-03 — Trip gateway circuit breaker (Pause)

| Field | Value |
|---|---|
| Scope | Gateway |
| Tier | Pause |
| Triggers | Bridge partner desync, fraudulent proof from external chain |
| Who may trigger | Authorised operator, governance |
| Evidence required | Bridge anomaly log + external chain state proof |
| Duration | Until reset; no automatic expiry |
| On-chain event | `GatewayCircuitTripped { gateway_id, reason_hash, tripped_at_block }` |
| Normal return | Governance resolution or authorised operator reset |
| Enforcement | implemented |
| Gaps | No auto-expiry; governance-forced expiry planned |

---

### PA-04 — Degrade to safe-mode (Degrade)

| Field | Value |
|---|---|
| Scope | Any service or VM |
| Tier | Degrade |
| Triggers | Queue backlog exceeding threshold, partial node set, proof generation failure rate |
| Who may trigger | Authorised operator, automatic threshold rule (planned) |
| Evidence required | Telemetry snapshot exceeding defined threshold |
| Duration | Bounded; must re-evaluate within N blocks |
| On-chain event | `ServiceDegraded { service_id, reason, degraded_at_block, review_by_block }` |
| Normal return | Operator sign-off after telemetry returns to normal band |
| Enforcement | planned — no runtime degrade mode implemented yet |
| Gaps | Degrade state machine not yet built |

---

### PA-05 — Quarantine validator node (Quarantine)

| Field | Value |
|---|---|
| Scope | Validator |
| Tier | Quarantine |
| Triggers | Double-sign evidence, equivocation proof, repeated proof failure, consensus rule violation |
| Who may trigger | On-chain slashing logic, validator majority, `Sentinel-Warden` quorum |
| Evidence required | Equivocation proof or signed evidence bundle from `x3-proof-dispute` |
| Duration | Until governance review; maximum epoch boundary |
| On-chain event | `ValidatorQuarantined { validator_id, evidence_hash, quarantined_at_block }` |
| Normal return | Governance resolution with evidence review |
| Enforcement | `crates/x3-gpu-validator-swarm/src/quarantine.rs` — implemented |
| Gaps | Evidence-bundle format not fully standardised |

---

### PA-06 — Quarantine swarm agent (Quarantine)

| Field | Value |
|---|---|
| Scope | Agent |
| Tier | Quarantine |
| Triggers | Envelope violation (INV-S-002), budget overage (INV-S-001), misconduct strike ladder (see AGENT_LAW.md) |
| Who may trigger | `ApprovalGate`, `Sentinel-Warden`, authorised operator |
| Evidence required | Guard violation log from `crates/x3-swarm-core/src/guard.rs` |
| Duration | Task-scoped or operator-set; maximum one epoch |
| On-chain event | `AgentQuarantined { agent_id, violation_class, quarantined_at_block }` |
| Normal return | Operator review; agent re-activation requires human approval |
| Enforcement | guard module partially implemented; quarantine state planned |
| Gaps | Agent quarantine state in swarm-core is planned |

---

### PA-07 — Pause governance queue (Pause)

| Field | Value |
|---|---|
| Scope | Governance |
| Tier | Pause |
| Triggers | Governance capture attempt, missing compliance proof, constitutional violation detected |
| Who may trigger | Constitution engine (automatic on INV-C-008 violation), authorised operator |
| Evidence required | Missing proof receipt, or `InvariantViolation` from constitution engine |
| Duration | Until suspended proposal is resolved; maximum 7 days |
| On-chain event | `GovernancePaused { proposal_id, reason_hash, paused_at_block }` |
| Normal return | Proposal amended with valid proof; constitution engine re-validates |
| Enforcement | `crates/x3-constitution/src/engine.rs` — implemented |
| Gaps | 7-day automatic expiry not wired |

---

### PA-08 — Kill swarm agent (Kill)

| Field | Value |
|---|---|
| Scope | Agent |
| Tier | Kill |
| Triggers | Third strike, irreversible policy violation, confirmed malicious action |
| Who may trigger | Governance, authorised operator with documented evidence |
| Evidence required | Full incident bundle from `Sentinel-Scribe`, plus operator sign-off |
| Duration | Permanent; genesis record flagged `terminated` |
| On-chain event | `AgentTerminated { agent_id, evidence_hash, terminated_at_block }` |
| Normal return | Not reversible; new genesis record required via governance |
| Enforcement | planned — kill path in swarm-core is a stub |
| Gaps | Termination path and genesis-record mutation not yet implemented |

---

## Authority matrix

| Action | Sentinel-Warden | Authorised Operator | Governance | Auto (threshold) |
|---|---|---|---|---|
| Trip asset/route breaker | Quorum | Yes | Yes | Planned |
| Trip gateway breaker | No | Yes | Yes | No |
| Degrade service | No | Yes | Yes | Planned |
| Quarantine validator | Quorum | Yes | Yes | Via slashing |
| Quarantine agent | Quorum | Yes | Yes | Via guard |
| Pause governance | No | Yes | No | Auto (constitution) |
| Kill agent | No | Yes (+ evidence) | Yes | No |

---

## Return-to-normal checklist

Before any active emergency action is cleared:

1. Root cause identified and documented in incident record
2. Evidence bundle exported by `Sentinel-Scribe` (or operator if Scribe not yet operational)
3. Invariant that triggered the action confirmed restored
4. Affected scope tested at reduced load before full re-enable
5. Postmortem scheduled within 48 hours of resolution
6. Reset action logged with authorising identity and block number

---

## Audit trail requirements

Every emergency action — trip, degrade, quarantine, kill, and reset — must produce:

- On-chain event (listed above)
- Operator log entry with: actor, scope, evidence hash, timestamp, block
- Off-chain snapshot of affected state at time of action

Gaps: unified audit-trail service is planned but not yet built. Until it exists, operators must manually retain logs per incident.

---

## Open gaps (as of 2026-05-08)

| Gap | Required by | Priority |
|---|---|---|
| Expiry enforcement on circuit-breaker trips | INV-R-006 | Band 0 |
| Degrade state machine | PA-04 | Band 0 |
| Agent quarantine state in swarm-core | PA-06 | Band 0 |
| Agent kill path and genesis-record mutation | PA-08 | Band 0 |
| Unified audit-trail service | All | Band 0 |
| Auto-trip threshold rules | PA-01, PA-02 | Band 1 |

See [INVARIANT_REGISTRY.md](INVARIANT_REGISTRY.md) `INV-R-006` and `INV-E-001` for the formal invariant coverage.
