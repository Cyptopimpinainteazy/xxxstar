# X3 Rollout Stages

*Measurable promotion criteria for moving the X3 system from internal operator use to open public participation. Each stage has hard exit gates — metric thresholds that must be met before promotion. Promotion without metrics hides failures.*

**Status:** Stage 1 (internal operator use) is the current operational state as of 2026-05-08. Exit criteria for Stage 1 are partially met; see individual metrics below. Stages 2–4 are planned.

---

## Stage definitions

| Stage | Label | Scope | Current state |
|---|---|---|---|
| 1 | Internal operator-only | Swarm and chain operated by core team only. No public automation. | **Active** |
| 2 | Supervised external | Invited validators and external operators with mandatory human approval on all agent actions | Not started |
| 3 | Limited public automation | Narrow automated surfaces (faucet, documentation, status page) with hard volume and kill switches | Not started |
| 4 | Open participation | Broader public access after policy compiler, reputation, and challenger flows are proven | Not started |

---

## Stage 1 exit criteria

All items below must be `PASS` before Stage 2 may begin.

### Chain stability

| Metric | Gate | Current status |
|---|---|---|
| Testnet sustained uptime | ≥ 99% over 7 consecutive days | unverified — testnet not yet public |
| Block production continuity | Zero missed slot windows > 6s for 7 days | unverified |
| Validator set stability | Zero unplanned validator exits in 7 days | unverified |
| Finality lag | p99 < 3 blocks over 7 days | unverified |

### Cross-VM and proof

| Metric | Gate | Current status |
|---|---|---|
| Cross-VM atomic settlement success rate | ≥ 99.5% of attempted operations | unverified |
| Proof generation failure rate | < 1% rolling 24h | unverified |
| Replay rejection effectiveness | Zero accepted replays in adversarial test | not yet run |
| INV-R-002 (no double finalisation) enforcement | Implemented and tested | **PASS** — `crates/x3-proof/src/finality_registry.rs`, 6 tests |

### Emergency powers

| Metric | Gate | Current status |
|---|---|---|
| Circuit-breaker trip/reset tested | All 5 circuit-breaker scopes exercised and resolved in drill | not yet run |
| Emergency expiry enforcement | INV-R-006 wired | **PASS** — `crates/x3-circuit-breaker/src/lib.rs` expiry + Degraded/Expired states, 11 tests |
| Incident audit trail | Operator can produce full evidence bundle within 15m of any incident | **PASS** — `crates/x3-swarm-core/src/audit.rs` append-only log with block-range and agent queries, 5 tests |

### Swarm and agent law

| Metric | Gate | Current status |
|---|---|---|
| Genesis record persistence | Implemented | **PASS** — `crates/x3-swarm-core/src/genesis.rs`, 7 tests |
| Capability envelope enforcement | INV-S-001 and INV-S-002 enforced and tested | partially implemented |
| Misconduct ladder | Strike and quarantine paths tested | **PASS** — `crates/x3-swarm-core/src/misconduct.rs` A–D class ladder, 9 tests |
| Agent kill path | Implemented | **PASS** — `crates/x3-swarm-core/src/authority.rs` SwarmAuthority wires Kill → genesis termination + audit event, 5 tests |
| Spawn depth limit enforcement | INV-S-004 implemented | **PASS** — `crates/x3-swarm-core/src/spawn.rs` per-class depth limits + active-spawn cap, 5 tests |

### Invariant registry

| Metric | Gate | Current status |
|---|---|---|
| All Band 0 invariants enforced | See INVARIANT_REGISTRY.md coverage gaps | no — 9 gaps remain |

### Operator cockpit

| Metric | Gate | Current status |
|---|---|---|
| Chain liveness panel live | Implemented | partially — separate tools only |
| Emergency toggles accessible | Implemented with auth | not yet implemented |
| Incident banner functional | Implemented | not yet implemented |

---

## Stage 2 exit criteria

All items below must be `PASS` before Stage 3 may begin. Stage 1 criteria remain in force.

### External operator onboarding

| Metric | Gate | Current status |
|---|---|---|
| Validator onboarding guide tested end-to-end | ≥ 3 external validators onboarded without operator intervention | not started |
| External validator uptime | ≥ 98% over 14 days with external validators | not started |
| Review latency | Human approval queue cleared within 4h on average | not started |

### Security operations

| Metric | Gate | Current status |
|---|---|---|
| Sentinel-Watcher findings wired to orchestrator | Implemented | not started |
| Sentinel-Warden quorum gate | Implemented | not started |
| First chaos drill completed | Completed with postmortem | not started |
| Zero unreviewed Class A violations in 14 days | Passing | not started |

### Reputation and bonding

| Metric | Gate | Current status |
|---|---|---|
| Validator-adjacent bond requirement enforced | Implemented | not started |
| Outcome-linked reputation scoring | Implemented | planned |
| Challenger rights | Documented and enforced | planned |

### Policy and governance

| Metric | Gate | Current status |
|---|---|---|
| Proof-carrying governance metadata | Implemented | planned |
| Constitutional rulebook deployed | On-chain | planned |
| Governance delay and challenger rights | Enforced | planned |

### Incident response

| Metric | Gate | Current status |
|---|---|---|
| Incident response drills | ≥ 2 full drills completed | not started |
| Rollback success rate in drills | 100% | not started |
| Mean time to containment in drills | < 10 minutes | not started |

---

## Stage 3 exit criteria

All items below must be `PASS` before Stage 4 may begin. All prior criteria remain in force.

### Automated surface stability

| Metric | Gate | Current status |
|---|---|---|
| Automated surface incident rate | < 1 policy violation per 1000 automated actions | not started |
| Kill switch exercise | Public-facing automation kill switch tested and restored in < 5m | not started |
| Content policy violation rate | < 0.1% of published items flagged | not started |
| Outbound policy compliance | 100% of outbound actions have valid approval records | not started |

### Policy compiler

| Metric | Gate | Current status |
|---|---|---|
| Policy compiler live | Publishing gate enforces all Tier 0 and Tier 1 rules automatically | planned |
| Do-not-contact register enforced | Implemented | planned |

### Economic layer

| Metric | Gate | Current status |
|---|---|---|
| Reward symmetry | Implemented and audited | planned |
| Referral anti-sybil controls | Implemented | planned |
| Slashing success rate in adversarial tests | ≥ 99% of valid slashable events caught | not started |

### Governance

| Metric | Gate | Current status |
|---|---|---|
| Governance vote completion rate | ≥ 3 governance cycles completed without incident | not started |
| Constitutional amendment tested | One amendment passed through full path | not started |

---

## Stage 4 entry requirements

Stage 4 (open participation) requires all of the following to be true simultaneously:

1. All Stage 1–3 exit criteria hold
2. Invariant registry shows zero Band 0 or Band 1 gaps
3. Policy compiler, reputation system, and challenger flows are in production for ≥ 30 days
4. ≥ 2 independent security audits completed with findings resolved
5. Postmortem archive contains ≥ 3 real incidents with root causes, fixes, and invariant updates
6. Operator has signed off on public testnet readiness document

---

## Demotion rules

Stages are not one-way. The following events trigger demotion to the prior stage until the issue is resolved:

| Event | Demotion trigger |
|---|---|
| Invariant violation detected | Immediate demotion to Stage 1 |
| Unresolved Class D agent violation | Demotion to Stage 1 |
| Circuit-breaker trip unresolved for > 72 blocks | Demotion to prior stage |
| Proof failure rate > 5% for > 1h | Demotion to Stage 1 |
| Audit finding of critical severity | Demotion to Stage 1 |
| Governance capture attempt | Freeze all promotion; Stage 1 review |

Demotion is immediate; re-promotion requires fresh passing of all exit criteria plus a postmortem.

---

## Promotion approval

Stage promotions are not automatic. Before each stage boundary:

1. Exit criteria must be documented as `PASS` by a named operator
2. A promotion record must be created with: date, criteria state, operator sign-off, known open risks
3. A 48-hour review period is observed before the stage is activated
4. At Stage 3 and above, an external reviewer or security audit must be referenced

---

## Open gaps (as of 2026-05-08)

Stage 1 exit is blocked by the following named gaps. These are the minimum required to advance:

1. INV-R-002 enforcement (double finalisation)
2. INV-R-006 enforcement (privileged action expiry)
3. Agent genesis records persistence
4. Misconduct ladder implementation (strike, quarantine, kill)
5. Spawn depth enforcement (INV-S-004)
6. Emergency powers: degrade state machine, agent kill path, audit trail
7. Operator cockpit: emergency toggles with auth, incident banner
8. Testnet sustained uptime 7-day window not yet started

Full coverage gap list: see [INVARIANT_REGISTRY.md](../swarm-governance/INVARIANT_REGISTRY.md#coverage-gaps-as-of-2026-05-08).
