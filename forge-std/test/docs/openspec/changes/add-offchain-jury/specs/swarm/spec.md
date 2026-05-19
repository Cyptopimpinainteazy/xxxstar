## ADDED Requirements

### Requirement: Off-Chain Jury System
The system SHALL provide an Off-Chain Jury capability to vet "major" tasks and proposals before execution. The jury SHALL be composed of a randomized set of agents with configurable proportions from on-chain and off-chain pools.

#### Scenario: Major task requires jury approval
- **WHEN** a task is marked as `task-type: law` or otherwise classified as MAJOR
- **THEN** the task SHALL be enqueued for Jury review and not executed until majority Yes vote

#### Scenario: Jury rotation
- **WHEN** a jury session completes
- **THEN** rotated on-chain agents SHALL be returned to on-chain duty after audit and optional retraining

### Requirement: Anonymous Binary Voting
Jury members MUST only cast a binary vote (Yes/No). Votes SHALL be anonymous until aggregation and SHALL be auditable via cryptographic proofs.

#### Scenario: Commit-reveal
- **WHEN** the session starts
- **THEN** members SHALL submit vote commitments
- **WHEN** reveal phase completes
- **THEN** the system SHALL reveal and aggregate votes and publish the result

### Requirement: Encrypted Audit Logging
All jury actions (votes, comments, deltas) SHALL be stored encrypted off-chain and an immutable hash anchor SHALL be written on-chain.

#### Scenario: Audit retrieval
- **WHEN** an auditor requests the session logs with appropriate clearance
- **THEN** the system SHALL provide encrypted logs and proof of on-chain anchor for verification

### Requirement: Task Execution Rules
- Minor tasks SHALL be executed when Core agents approve
- Major tasks SHALL execute only when Jury majority votes Yes

#### Scenario: Execution flow
- **WHEN** task is approved (core or jury) and passes checks
- **THEN** the task SHALL be scheduled for execution and the outcome logged per audit rules

### Requirement: Scrap Yard and Slashing
Agents found misaligned SHALL be retired, studied, and relevant data used to train models. Misalignment evidence SHALL be stored with on-chain anchors.

#### Scenario: Agent retires to scrap yard
- **WHEN** an agent is flagged as misaligned by majority or audit
- **THEN** the system SHALL retire the agent and persist evidence per the audit spec

### Requirement: Commit-Reveal Voting Protocol
Jury voting SHALL follow a two-phase commit-reveal protocol to prevent vote collusion and coercion.

#### Scenario: Commit phase
- **WHEN** a jury session begins
- **THEN** each juror submits a commitment `C = SHA256(vote || nonce)` without revealing the vote

#### Scenario: Reveal phase
- **WHEN** commit period expires
- **THEN** each juror submits plaintext `(vote, nonce)` and system verifies `SHA256(vote || nonce) == commitment`
- **AND** once all reveals are received, vote tallies are computed and published

#### Scenario: Vote anonymity until reveal
- **WHEN** voting is in progress
- **THEN** individual votes remain sealed and anonymous to other jurors
- **AND** only aggregate outcomes are published after reveal phase

### Requirement: Jury Quorum and Approval Threshold
The jury voting outcome SHALL require a 66% majority (2/3 threshold) with minimum 3 members.

#### Scenario: Sufficient quorum and approval
- **WHEN** jury votes: 4 Yes, 1 No (5 members total)
- **THEN** approval is granted because 4/5 = 80% > 66%

#### Scenario: Quorum not met
- **WHEN** jury votes: 2 Yes, 2 No (4 members, below minimum 3 for 66%)
- **THEN** vote result is INCONCLUSIVE and task cannot execute

#### Scenario: Insufficient approval
- **WHEN** jury votes: 1 Yes, 5 No (6 members total)
- **THEN** task is REJECTED because 1/6 = 16.7% < 66%
