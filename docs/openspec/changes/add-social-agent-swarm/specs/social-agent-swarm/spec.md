## ADDED Requirements

### Requirement: Draft-Only Social Agent Swarm
The system SHALL generate social action drafts (posts, comments, DMs, profiles, group interactions) without executing live network actions in v1.

#### Scenario: Draft generation only
- **WHEN** a social task is submitted
- **THEN** the system generates draft artifacts without executing live API actions

### Requirement: Open Notebook Grounding
The system SHALL ground all generated drafts in Open Notebook content and approved sources.

#### Scenario: Grounded draft
- **WHEN** a draft is generated
- **THEN** the draft includes provenance metadata referencing Open Notebook sources used

### Requirement: Local Ollama Inference
The system SHALL use local Ollama models for generation by default.

#### Scenario: Local model usage
- **WHEN** a generation task runs
- **THEN** the task uses the configured Ollama model endpoint

### Requirement: Network and Keyword Configuration
The system SHALL support a configurable list of v1 networks (first 10 plus X) and keyword expansion for discovery tasks.

#### Scenario: Config-driven network set
- **WHEN** the network configuration is updated
- **THEN** the agent pipeline targets only the configured networks

### Requirement: ToS-Safe Guardrails and Audit Logs
The system SHALL enforce rate limits, action quotas, and audit logging for all draft actions.

#### Scenario: Guardrail enforcement
- **WHEN** draft actions exceed configured thresholds
- **THEN** the system flags or throttles the action and records an audit event

### Requirement: Future Live Actions Feature Flag
The system SHALL include a feature flag to enable live network actions per network when credentials are configured.

#### Scenario: Live actions disabled in v1
- **WHEN** live actions are not enabled
- **THEN** all actions remain drafts regardless of intent
