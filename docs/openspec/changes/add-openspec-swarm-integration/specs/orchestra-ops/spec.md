## ADDED Requirements
### Requirement: OpenSpec CLI Discovery
The system SHALL locate the OpenSpec CLI using `OPENSPEC_BIN` when set, and otherwise fall back to the system PATH.

#### Scenario: Env override used
- **WHEN** `OPENSPEC_BIN` is set
- **THEN** the system uses that path for OpenSpec commands

#### Scenario: PATH fallback
- **WHEN** `OPENSPEC_BIN` is not set
- **THEN** the system resolves `openspec` from PATH

### Requirement: OpenSpec Change Skeletons
The system SHALL support creating OpenSpec change skeletons (proposal, specs, tasks) for new swarm work items.

#### Scenario: Skeleton created
- **WHEN** a new work item is classified as spec-driven
- **THEN** a change directory with proposal/specs/tasks is created

### Requirement: Validation Gate for Major Tasks
The system SHALL validate OpenSpec changes before executing major tasks and MUST block execution on validation failure.

#### Scenario: Validation blocks execution
- **WHEN** `openspec validate <change-id> --strict` fails
- **THEN** the major task is rejected or queued for remediation

### Requirement: Spec Linkage in Task Metadata
The system SHALL attach an `openspec_change_id` to task payloads and logs when the work is spec-driven.

#### Scenario: Change ID logged
- **WHEN** a spec-driven task executes
- **THEN** the task log includes the `openspec_change_id`

### Requirement: Validation Status Reporting
The system SHALL expose the last validation status for a change ID to operators and audit processes.

#### Scenario: Status query
- **WHEN** an operator queries validation status for a change
- **THEN** the system returns the latest validation result and timestamp

### Requirement: OpenSpec Integration APIs
The system SHALL provide API endpoints to create a change skeleton, validate a change, and attach a change ID to queued tasks.

#### Scenario: API create and validate
- **WHEN** a client requests a new change and validation
- **THEN** the system returns the change ID and validation result
