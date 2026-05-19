## ADDED Requirements
### Requirement: Orchestra Terminology
The system SHALL treat "Orchestra" as the canonical name for the multi-agent governance system and SHALL avoid using "swarm" in governance specifications.

#### Scenario: Governance docs use Orchestra naming
- **WHEN** governance specs are authored
- **THEN** they reference Orchestra terminology as the canonical naming

### Requirement: Task Intent Spec
The system SHALL require every queued task to be described by a task intent .md file with YAML front matter before execution.

#### Scenario: Valid task intent accepted
- **WHEN** a task intent file includes required fields
- **THEN** the task is eligible for severity classification

#### Scenario: Missing fields rejected
- **WHEN** a task intent file omits a required field
- **THEN** the task is rejected and not queued

### Requirement: Task Intent Schema
The task intent front matter MUST include: `id`, `type`, `severity`, `proposer`, `section`, `created_at`, and `hash`.

#### Scenario: Required schema validated
- **WHEN** a task intent file is submitted
- **THEN** the schema is validated against the required fields

### Requirement: Severity Gating
The system SHALL classify tasks as `major` or `minor` and SHALL route all major tasks to the jury.

#### Scenario: Major task sent to jury
- **WHEN** a task is classified as major
- **THEN** it is queued for jury voting and not executed directly

#### Scenario: Minor task bypasses jury
- **WHEN** a task is classified as minor
- **THEN** it may execute after core on-chain approval

### Requirement: Major Task Definition
Major tasks MUST include any task that modifies governance rules, agent privileges, on-chain state schema, security boundaries, or treasury balances.

#### Scenario: Governance rule change is major
- **WHEN** a task proposes a governance rule change
- **THEN** it is classified as major

### Requirement: Jury Composition
The jury SHALL include permanent off-chain auditors and a rotating subset of on-chain agents.

#### Scenario: Rotation includes on-chain observers
- **WHEN** a jury epoch begins
- **THEN** the selected jury includes both off-chain and rotated on-chain members

### Requirement: Jury Diversity Caps
The jury MUST enforce per-section caps to prevent monoculture in composition.

#### Scenario: Cap enforced
- **WHEN** selection exceeds a section cap
- **THEN** the excess selections are replaced by eligible alternatives

### Requirement: Jury Isolation
Rotated on-chain agents on jury duty SHALL operate from a read-only snapshot and MUST NOT write to on-chain state.

#### Scenario: Write attempts blocked
- **WHEN** a rotated agent attempts to write on-chain state
- **THEN** the action is rejected

### Requirement: Anonymous Binary Voting
Jury members MUST cast anonymous binary votes (`YES` or `NO`) for each major task.

#### Scenario: Vote anonymity enforced
- **WHEN** a vote is cast
- **THEN** the vote is sealed and not visible to other jurors

### Requirement: Vote Aggregation
The system SHALL publish only the final pass/fail outcome for a task and SHALL withhold individual votes until retirement or audit release.

#### Scenario: Outcome published without individual votes
- **WHEN** voting completes
- **THEN** only the aggregate outcome is published

### Requirement: Human Veto by Deletion
Humans MAY veto any queued task by deleting its task intent .md file, and the system MUST treat the deletion as a hard stop.

#### Scenario: Deleted task cannot execute
- **WHEN** a task intent file is deleted before execution
- **THEN** the task is removed from queues and cannot execute

### Requirement: Relayer Normalization
All off-chain audit submissions MUST be normalized into a structured claim schema before reaching on-chain decision makers.

#### Scenario: Free-text is rejected
- **WHEN** an audit submission is free-text only
- **THEN** it is rejected by the relayer

### Requirement: Law Proposal Lifecycle
The system SHALL support law proposals that follow the lifecycle: propose, simulate, jury vote, ratify, enforce, and audit.

#### Scenario: Law proposal requires simulation
- **WHEN** a law proposal is submitted
- **THEN** simulation output is required before voting

### Requirement: Immutable Score
The Score (immutable rules) MUST NOT be modified by law proposals or task execution.

#### Scenario: Attempted Score modification rejected
- **WHEN** a task attempts to modify the Score
- **THEN** the task is rejected as invalid

### Requirement: Audit Log Anchoring
The system SHALL anchor jury outcomes and task intent hashes on-chain and SHALL store detailed vote logs off-chain in encrypted form.

#### Scenario: Anchor and log created
- **WHEN** a jury completes a vote
- **THEN** the outcome hash is anchored on-chain and the encrypted log is stored off-chain

### Requirement: Scrap Yard Retirement
Agents with repeated misaligned votes MUST be retired to the Scrap Yard for forensic analysis and replacement training.

#### Scenario: Repeated misalignment triggers retirement
- **WHEN** an agent exceeds the misalignment threshold
- **THEN** it is retired and queued for forensic analysis

### Requirement: Master Prompt Anchoring
The system SHALL support a master prompt in agent bootstrap configuration that defines security, proof, logging, and escalation obligations.

#### Scenario: Master prompt applied
- **WHEN** a first-wave agent is instantiated
- **THEN** the master prompt is applied before any role-specific sub-prompts

### Requirement: Action Logging and Auditability
The system SHALL log every agent action and decision in a verifiable and auditable manner.

#### Scenario: Action log recorded
- **WHEN** an agent performs an action or decision
- **THEN** a verifiable log entry is emitted to the telemetry/log hook and anchored for audit

### Requirement: Failure Escalation Continuity
The system SHALL escalate invariant violations, failures, and malicious activity to successor agents and maintain learning continuity.

#### Scenario: Failure escalated to successor
- **WHEN** an agent detects a failure that meets escalation criteria
- **THEN** it escalates to a successor agent with relevant context and learning artifacts

### Requirement: Formal Invariant Discipline
The system SHALL NOT violate formal invariants except during sanctioned failure simulations.

#### Scenario: Unsanctioned invariant violation blocked
- **WHEN** a task would violate a formal invariant without a sanctioned simulation flag
- **THEN** the task is rejected

### Requirement: Proof-Verified Proposals
All proposals, trades, or amendments MUST be verifiable with proofs before execution.

#### Scenario: Proof required for proposal
- **WHEN** a proposal is submitted
- **THEN** the proposal includes proof artifacts that can be verified prior to execution

### Requirement: System Health Priority
The system SHALL prioritize system health, security, and recursive proof integrity over short-term gains.

#### Scenario: Risky short-term gain rejected
- **WHEN** a task yields short-term gains but degrades system health or proof integrity
- **THEN** the task is rejected or reclassified for simulation-only execution

### Requirement: Core Skills Baseline
Every first-wave agent MUST be provisioned with a core skill baseline including observability, verification, simulation, governance analysis, risk assessment, autonomous decision-making, recursive proof handling, failure escalation, and learning/adaptation.

#### Scenario: Core skills validated
- **WHEN** a first-wave agent is onboarded
- **THEN** its skill profile is validated against the baseline

### Requirement: Tool Access Guardrails
Agents MUST receive access only to role-appropriate tools and endpoints, with telemetry/logging configured for every action.

#### Scenario: Tool access scoped
- **WHEN** an agent is granted tool access
- **THEN** the access is scoped to role permissions and logged

### Requirement: Violation and Malicious Activity Workflow
The system SHALL enforce a workflow for violations and malicious activity: detection, quarantine, audit, scrap yard, and successor escalation.

#### Scenario: Malicious activity handled
- **WHEN** malicious activity is detected
- **THEN** the agent is quarantined, an audit is triggered, and escalation proceeds per workflow
