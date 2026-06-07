# X3 Swarm Capability Envelopes

*Formal capability specification for every agent class and node role in the X3 swarm. Envelopes are enforced by `crates/x3-swarm-core/src/permissions.rs` and `src/guard.rs`. Anything not listed is denied by default.*

**Status:** agent kinds and permission tiers are implemented in `crates/x3-swarm-core`; hardware requirements, stake rules, and cross-class isolation are partially implemented or planned.

---

## Envelope format

Each class entry specifies:

- **Reads** — data sources the class may query
- **Writes** — paths or surfaces the class may modify
- **Invokes** — services, tools, or capabilities the class may call
- **Spends** — maximum token budget per epoch (maps to `AgentBudgetBound` INV-C-005)
- **Publishes** — external surfaces the class may write to
- **Pauses** — what the class may halt or circuit-break
- **Spawn limit** — maximum descendants this class may create
- **Approval required** — which review tier must approve sensitive actions
- **Stake requirement** — minimum bond required (0 = no bond)
- **Enforcement** — current implementation status

---

## Agent classes (from `AgentKind`)

### RepoScanner

| Field | Value |
|---|---|
| Reads | repository source tree, CI logs |
| Writes | `reports/` only |
| Invokes | grep, file search, static analysis tools |
| Spends | 0 (read-only class) |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` for all writes |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports` — implemented |

### FeatureMapper

| Field | Value |
|---|---|
| Reads | source tree, PRD, planning artifacts |
| Writes | `docs/`, `reports/` |
| Invokes | semantic search, file read |
| Spends | 0 |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports` — implemented |

### TestBuilder

| Field | Value |
|---|---|
| Reads | source tree, test fixtures |
| Writes | `tests/`, `reports/` |
| Invokes | cargo test, test framework tools |
| Spends | 0 |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports` — implemented |

### Integrator

| Field | Value |
|---|---|
| Reads | source tree, integration configs |
| Writes | `apps/` wiring paths — `AgentPermissionTier::TauriServiceWiring` |
| Invokes | build tools, service connectors |
| Spends | 0 |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::TauriServiceWiring` — implemented |

### BuildFixer

| Field | Value |
|---|---|
| Reads | build logs, source tree |
| Writes | source files within assigned scope; not runtime pallets |
| Invokes | cargo check, cargo fix |
| Spends | 0 |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` for any runtime path touch |
| Stake | 0 |
| Enforcement | partially-implemented (tier-based path filtering) |

### WiringInspector

| Field | Value |
|---|---|
| Reads | source tree, runtime modules |
| Writes | `reports/` only |
| Invokes | static analysis, call-graph tools |
| Spends | 0 |
| Publishes | none |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports` — implemented |

### Auditor

| Field | Value |
|---|---|
| Reads | all source paths, chain state snapshots, proof logs |
| Writes | `reports/`, `docs/security/` |
| Invokes | read-only forensic tools |
| Spends | 0 |
| Publishes | audit reports to `reports/` only |
| Pauses | none (recommend only; Warden executes) |
| Spawn limit | 0 |
| Approval | `SecurityReview` for security report publication |
| Stake | 0 |
| Enforcement | partially-implemented |

### Breaker

| Field | Value |
|---|---|
| Reads | chain liveness signals, threat feed |
| Writes | none directly |
| Invokes | circuit-breaker trip path (`x3-circuit-breaker`) — requires quorum |
| Spends | 0 |
| Publishes | incident alert to operator dashboard |
| Pauses | Asset, Route, Gateway, DexPool, Verifier scopes via `CircuitBreakerEngine` |
| Spawn limit | 0 |
| Approval | `SecurityReview` + quorum (Sentinel-Warden quorum — planned) |
| Stake | planned — must post bond before pause rights activate |
| Enforcement | circuit-breaker crate implemented; quorum gate planned |

### Fixer

| Field | Value |
|---|---|
| Reads | source tree, incident evidence |
| Writes | scoped to incident-specific patch path |
| Invokes | build tools, test harness |
| Spends | 0 |
| Publishes | patch proposal to `proposals/` |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `SecurityReview` for any runtime patch |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::RuntimeProposalOnly` — implemented |

### ReadinessReporter

| Field | Value |
|---|---|
| Reads | readiness state (`crates/x3-readiness`, `crates/x3-readiness-report`) |
| Writes | `reports/` only |
| Invokes | readiness check runners |
| Spends | 0 |
| Publishes | readiness report to `reports/` |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | implemented |

### Benchmark

| Field | Value |
|---|---|
| Reads | source tree, bench configs |
| Writes | `bench-results/`, `reports/` |
| Invokes | cargo bench, TPS harness |
| Spends | 0 |
| Publishes | benchmark reports only |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` |
| Stake | 0 |
| Enforcement | implemented |

### Marketing

| Field | Value |
|---|---|
| Reads | approved content assets, analytics data |
| Writes | draft assets in `reports/content/` only |
| Invokes | content tools (allowlisted) |
| Spends | up to `InvariantBounds::max_agent_epoch_budget` |
| Publishes | **only with explicit human approval** — see [OUTBOUND_POLICY.md](../swarm-ops/OUTBOUND_POLICY.md) |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` for every outbound publish |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports`; outbound gate planned |

### Grant

| Field | Value |
|---|---|
| Reads | grant databases, project documentation |
| Writes | `docs/`, `reports/` |
| Invokes | research tools |
| Spends | 0 |
| Publishes | draft proposals; human approves before external submission |
| Pauses | none |
| Spawn limit | 0 |
| Approval | `HumanReview` for all external submissions |
| Stake | 0 |
| Enforcement | `AgentPermissionTier::DocsTestsReports` — implemented |

### ApprovalGate

| Field | Value |
|---|---|
| Reads | pending approval queue (`crates/x3-swarm-core/src/approval.rs`) |
| Writes | approval decisions only |
| Invokes | notification tools |
| Spends | 0 |
| Publishes | approval events to swarm event stream |
| Pauses | can block pending tasks |
| Spawn limit | 0 |
| Approval | self-sufficient for approve/reject; not for scope changes |
| Stake | 0 |
| Enforcement | `src/approval.rs` — implemented |

---

## Security swarm node classes (Sentinel-*)

Defined in `x3-security-swarm/agents/templates/`. These four classes have tighter isolation than general swarm agents.

### Sentinel-Watcher

| Field | Value |
|---|---|
| Reads | chain events, validator telemetry, proof logs, threat feeds |
| Writes | evidence findings (internal only) |
| Invokes | anomaly detection tools, log correlation |
| Spends | 0 |
| Publishes | findings to Sentinel-Judge only |
| Pauses | none |
| Spawn limit | 0 |
| Approval | automatic (evidence-only, no action) |
| Stake | 0 |
| Credential scope | read-only chain access; no validator credentials |
| Enforcement | partially-implemented (wiring to orchestrator planned) |

### Sentinel-Judge

| Field | Value |
|---|---|
| Reads | Watcher findings |
| Writes | attribution score bundles |
| Invokes | correlation engine |
| Spends | 0 |
| Publishes | threat assessment to Warden + Scribe |
| Pauses | none |
| Spawn limit | 0 |
| Approval | automatic; output requires quorum of Warden before acting |
| Stake | 0 |
| Credential scope | no chain credentials |
| Enforcement | planned |

### Sentinel-Warden

| Field | Value |
|---|---|
| Reads | Judge assessments |
| Writes | reversible containment records |
| Invokes | `CircuitBreakerEngine::trip_circuit_breaker`, validator quarantine path |
| Spends | 0 |
| Publishes | containment events to operator dashboard |
| Pauses | Asset, Route, Gateway, DexPool, Verifier scopes — reversible only |
| Spawn limit | 0 |
| Approval | quorum of Sentinel-Wardens required before any pause action |
| Stake | planned — bond required |
| Credential scope | circuit-breaker write; no validator signing keys |
| Enforcement | circuit-breaker path implemented; quorum gate planned |

### Sentinel-Scribe

| Field | Value |
|---|---|
| Reads | all Sentinel outputs |
| Writes | immutable incident bundles (off-chain + on-chain anchor) |
| Invokes | evidence export, IPFS/storage anchor |
| Spends | 0 |
| Publishes | incident record with evidence lineage |
| Pauses | none |
| Spawn limit | 0 |
| Approval | automatic |
| Stake | 0 |
| Credential scope | read-only + storage write |
| Enforcement | planned |

---

## GPU validator-adjacent workers

Defined in `crates/x3-gpu-validator-swarm`. These run on validator-adjacent hardware and have the highest trust requirements.

| Class | Allowed actions | Stake required | Status |
|---|---|---|---|
| GPU proof worker | Generate and sign proofs for assigned jobs | Yes (implementation pending) | partially-implemented |
| Challenger / watcher | Submit challenges to proof-dispute module | Yes | planned |
| Determinism verifier | Replay-verify deterministic job outputs | No stake, validator-assigned | partially-implemented |
| Metrics indexer | Collect performance data, no signing | No | implemented |

---

## Denied-by-default rules

Regardless of class, no agent may:

1. Access validator signing keys or node credentials
2. Access treasury module write paths
3. Publish to any external channel without an explicit `HumanReview` or `SecurityReview` approval record
4. Spawn a descendant agent with a broader permission tier than itself
5. Invoke any action after its task TTL has expired
6. Access secrets scoped to a different agent class

---

## Open gaps (as of 2026-05-08)

- Stake requirements for Breaker and Sentinel-Warden are defined here but not yet enforced on-chain
- Sentinel-Judge and Sentinel-Scribe classes have no runtime enforcement
- Quorum gate for Warden actions is specified but not yet implemented
- Marketing outbound gate exists at the `HumanReview` approval tier but no dedicated publishing-gate service is wired
- Spawn depth enforcement for all classes is planned (INV-S-004)

See [INVARIANT_REGISTRY.md](INVARIANT_REGISTRY.md) for the invariant IDs that cover envelope enforcement (INV-S-001, INV-S-002, INV-S-004).
