## Context
The Orchestra introduces an off-chain jury layer to review major tasks and law proposals that affect on-chain state, governance rules, or agent privileges. The system must remain auditable, anonymous in voting, and resistant to collusion. It must preserve a minimal human intervention surface (delete-only veto) while keeping the Score immutable.

## Goals / Non-Goals
- Goals:
	- Provide a deterministic, auditable flow for major task approval.
	- Keep voting anonymous while preserving verifiable outcomes.
	- Rotate on-chain agents into off-chain jury duty without state write access.
	- Ensure task intent is explicit, structured, and human-readable.
- Non-Goals:
	- Defining AGI behavior or consciousness criteria.
	- Replacing the core on-chain decision engine.
	- Allowing free-text or narrative-only justifications to reach the decision layer.

## Decisions
- Decision: Use task intent files (.md) with YAML front matter as the single human-readable interface.
	- Why: Enables auditability, deletion-only veto, and structured parsing.
- Decision: Jury uses anonymous binary voting with commit-reveal or sealed log mechanics.
	- Why: Prevents collusion, signaling, and vote coercion.
- Decision: Rotate a bounded subset of on-chain agents into off-chain duty using read-only snapshots.
	- Why: Injects system expertise without allowing direct influence on execution.
- Decision: Split requirements by severity (major vs minor).
	- Why: Keeps latency and throughput stable for low-risk tasks.

## Risks / Trade-offs
- Risk: Jury bottleneck for major tasks.
	- Mitigation: Severity gating, batching, and epoch-based voting windows.
- Risk: Homogeneous juries reduce disagreement.
	- Mitigation: Cap per section and enforce rotation diversity.
- Risk: Overexposure of internal state to off-chain jurors.
	- Mitigation: Read-only snapshots with least-privilege views.

## Migration Plan
1. Land Orchestra governance spec delta and validate.
2. Implement minimal task intake and severity classification.
3. Implement jury lifecycle (selection, voting, aggregation, logs).
4. Add rotation and isolation controls.
5. Incrementally enable major-task gating.

## Open Questions (Resolved)
- **What cryptographic scheme will be used for anonymous vote commitment?**
  - Decision: Commit-reveal protocol with SHA256 hash commitments
  - Implementation: During commit phase, juror submits `H(vote || nonce)`. During reveal phase, juror submits `(vote, nonce)` and system verifies `SHA256(vote || nonce) == commitment`. All votes remain anonymous until reveal phase completes.
  - Rationale: Prevents vote manipulation while maintaining anonymity; widely-used cryptographic pattern.

- **What is the exact severity taxonomy for major tasks in each subsystem?**
  - Decision: Adopt severity gating rules per category below
  - **MAJOR** (requires jury): Governance changes, treasury/payments >T, agent role changes, security patches, schema migrations, security boundary changes, scoring rule changes
  - **MINOR** (core approval only): Config updates, monitoring/telemetry changes, documentation, minor bug fixes, routine operations
  - **CONTEXT**: Additional subsystems may define custom rules in their spec files (extensible model)
  - Rationale: Balances safety (core rules are clear) with velocity (routine ops bypass jury)

- **What minimum quorum should be enforced for jury votes?**
  - Decision: Majority rule with 66% threshold and 3-member minimum quorum
  - Formula: `approval_count / jury_size >= 0.66` AND `jury_size >= 3`
  - Rationale: 2/3 supermajority protects against collusion; 3+ members prevent single-agent veto ("at least 2 must agree")
  - Bootstrap: Start with 5-member jury (4 must approve for major task)

## Next Ten Agent Roles (Initial Expansion)
1. Score Guardian - monitors invariant violations in task proposals.
2. Law Linter - statically validates law proposals against the Score.
3. Adversarial Prosecutor - argues against risky major tasks.
4. Simulation Conductor - runs counterfactuals and summarizes outcomes.
5. Rotation Auditor - verifies rotation fairness and diversity caps.
6. Vote Anomaly Detector - flags statistical vote deviations.
7. Scrap Yard Forensicist - analyzes retired agents for failure modes.
8. Task Severity Classifier - gatekeeps major vs minor classification.
9. Section Balancer - prevents monoculture in jury composition.
10. Archivist - maintains immutable, human-readable history summaries.
