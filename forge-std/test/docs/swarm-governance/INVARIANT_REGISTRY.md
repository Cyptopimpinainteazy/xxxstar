# X3 Invariant Registry

*Machine-readable canonical register of all system invariants. Each entry has a stable ID, enforcement location, test coverage status, and incident guidance. IDs are immutable once assigned.*

**Status:** partially implemented — constitutional invariants wired in `crates/x3-constitution`; cross-VM and swarm-level invariants are defined here but enforcement wiring is in progress or planned.

---

## Registry format

| Field | Description |
|---|---|
| **ID** | Stable identifier. Never reassigned. Deprecated entries keep their ID. |
| **Name** | Short human label. |
| **Invariant** | Formal statement of what must hold. |
| **Tier** | `constitutional` / `runtime` / `swarm` / `cross-vm` / `economic` |
| **Enforcement** | Crate, module, or service that checks this. |
| **Status** | `implemented` / `partially-implemented` / `planned` / `stubbed` |
| **Tests** | Test file or coverage note. |
| **Incident** | First-response action if violated. |

---

## Constitutional invariants (INV-C-*)

These are enforced by `crates/x3-constitution/src/invariants.rs` via `CoreInvariant`. Violations are detected by `ConstitutionEngine` and produce `InvariantViolation` events.

| ID | Name | Invariant | Status | Enforcement | Tests | Incident |
|---|---|---|---|---|---|---|
| INV-C-001 | SupplyCap | Total token supply must never exceed `InvariantBounds::max_supply` | implemented | `x3-constitution/src/invariants.rs`, `x3-constitution/src/engine.rs` | `x3-constitution` unit tests | Halt mint; emit on-chain event; page operator |
| INV-C-002 | TreasuryBound | Treasury balance must never exceed `max_treasury_pct` of total supply | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Freeze treasury sends; audit treasury state |
| INV-C-003 | AgentCountLimit | Registered agent count must not exceed `max_agent_count` | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Block new agent genesis; alert operator |
| INV-C-004 | GovernanceDepthBound | Governance call stack depth must not exceed `max_proposal_depth` | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Reject proposal; emit depth violation event |
| INV-C-005 | AgentBudgetBound | A single agent must not spend more than `max_agent_epoch_budget` per epoch | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Block agent spend; slash if repeated |
| INV-C-006 | ExecutionDeterminism | All state transitions must be deterministic: no floating-point, no wall clock, no external entropy | implemented | `x3-constitution/src/invariants.rs`, `crates/dylint-determinism` | lint tests | Reject transaction; flag node for review |
| INV-C-007 | ExecutionTermination | All computations must terminate within gas bounds. Unbounded loops are forbidden | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Out-of-gas revert; no partial state |
| INV-C-008 | GovernanceProofRequirement | Governance proposals touching invariants must carry a verified proof of compliance before execution | implemented | `x3-constitution/src/invariants.rs` | `x3-constitution` unit tests | Reject proposal without proof; log proposer |

---

## Runtime invariants (INV-R-*)

Checked by runtime modules and cross-VM kernel paths. Some are enforced in pallets; others rely on service-layer checks.

| ID | Name | Invariant | Status | Enforcement | Tests | Incident |
|---|---|---|---|---|---|---|
| INV-R-001 | AtomicSettlementIntegrity | A cross-VM transaction must either finalize all legs or revert all legs. No partial completion | partially-implemented | `crates/cross-vm-coordinator`, `pallets/` | integration-tests partial | Circuit-break affected route; quarantine executor node |
| INV-R-002 | NoDoubleFinalisation | A given transaction hash must never be finalized more than once across all VMs | **implemented** | `crates/x3-proof/src/finality_registry.rs` | 6 unit tests (`x3-proof`) | Halt affected VM path; forensic audit |
| INV-R-003 | BondConservation | Total bonded value must equal sum of individual bond records at every block | partially-implemented | `crates/x3-slash/src/bond.rs` | bond unit tests | Pause slashing; audit bond ledger |
| INV-R-004 | ReplayRejection | A transaction with a previously-seen nonce or replay domain must be rejected | partially-implemented | `crates/x3-proof`, runtime nonce checks | partial | Drop transaction; emit replay attempt event |
| INV-R-005 | CrossVMStateAgreement | After a cross-VM commit, all participating VM states must reflect the same committed outcome | planned | `crates/cross-vm-bridge`, `crates/cross-vm-coordinator` | not yet | Freeze bridge; trigger challenge; notify validators |
| INV-R-006 | PrivilegedActionExpiry | Every privileged or emergency action must carry an expiry block. Expired actions must not execute | **implemented** | `crates/x3-circuit-breaker/src/lib.rs` (expiry + Degraded/Expired states) | 11 unit tests (`x3-circuit-breaker`) | Reject expired action; log attempt |
| INV-R-007 | ProofFreshness | Proofs older than the defined freshness window must be rejected | planned | `crates/x3-proof`, `crates/x3-proof-envelope` | not yet | Reject stale proof; request re-proof |

---

## Swarm invariants (INV-S-*)

Checked by the swarm control plane (`crates/x3-swarm-core`, `crates/x3-gpu-validator-swarm`).

| ID | Name | Invariant | Status | Enforcement | Tests | Incident |
|---|---|---|---|---|---|---|
| INV-S-001 | AgentBudgetCeiling | No agent may exceed its capability-envelope token budget in a single task | partially-implemented | `crates/x3-swarm-core/src/permissions.rs` | guard tests | Block task; trigger strike ladder |
| INV-S-002 | CapabilityEnvelopeRespect | No agent may read, write, invoke, or publish outside its declared capability envelope | partially-implemented | `crates/x3-swarm-core/src/permissions.rs`, `src/guard.rs` | guard tests | Kill task; quarantine agent |
| INV-S-003 | MutationPipelineRequired | No prompt, policy, or routing change may be promoted without passing the mutation pipeline | planned | mutation-proposal flow (not yet implemented) | not yet | Reject change; flag requesting agent |
| INV-S-004 | SpawnDepthLimit | Agent spawning depth must not exceed class-specific spawn limit | **implemented** | `crates/x3-swarm-core/src/spawn.rs` | 5 unit tests (`x3-swarm-core`) | Block spawn; alert operator |
| INV-S-005 | DeterminismClassEnforcement | Jobs in the `Deterministic` or `BoundedDeterministic` tier must produce challengeable receipts | planned | scheduler policy engine (planned) | not yet | Reject job output; challenge node |

---

## Economic invariants (INV-E-*)

Checked by economic and treasury modules.

| ID | Name | Invariant | Status | Enforcement | Tests | Incident |
|---|---|---|---|---|---|---|
| INV-E-001 | EmergencyAuthorityLimits | Emergency authority holders must not exceed their scope, duration, or action set | planned | emergency-powers module (planned) | not yet | Expire authority; audit actions taken |
| INV-E-002 | SlashSymmetry | A slash event must produce exactly the declared debit from slashed party and credit to declared destination | partially-implemented | `crates/x3-slash/src/engine.rs` | slash unit tests | Audit slash record; freeze further slashes |
| INV-E-003 | OutcomeLinkedReward | Reward events must reference a verified outcome receipt, not raw activity counters | planned | reputation module (planned) | not yet | Block reward; require evidence |

---

## Amendments to this registry

Additions require: a new stable ID, formal invariant statement, assigned enforcement location, and at least one linked test or simulation result. Removals are not permitted; deprecated invariants are marked `deprecated` and retained for audit history.

**Amendment path:** governance proposal via `x3-constitution/src/amendment.rs` with attached compliance proof (`INV-C-008`).

---

## Coverage gaps (as of 2026-05-08)

The following invariant classes are defined in this registry but have no runtime enforcement yet:

- INV-R-005 (cross-VM state agreement)
- INV-R-007 (proof freshness)
- INV-S-003 (mutation pipeline)

Implemented this session (2026-05-08): INV-R-002, INV-R-006, INV-S-004.
- INV-S-005 (determinism class)
- INV-E-001 (emergency authority limits)
- INV-E-003 (outcome-linked reward)

These gaps must be resolved before Band 0 testnet hardening is called complete. See [EMERGENCY_POWERS.md](EMERGENCY_POWERS.md) for the authority limit enforcement plan and [CAPABILITY_ENVELOPES.md](CAPABILITY_ENVELOPES.md) for the envelope enforcement plan.
