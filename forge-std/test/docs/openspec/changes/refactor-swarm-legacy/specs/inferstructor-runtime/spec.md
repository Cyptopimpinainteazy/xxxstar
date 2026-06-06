## ADDED Requirements

### Requirement: Swarm independence for launch
The production launch path SHALL NOT depend on legacy swarm modules unless explicitly enabled by feature flags.

#### Scenario: Default deployment
- **WHEN** the system is deployed with default settings
- **THEN** no legacy swarm service is required for core runtime operation

#### Scenario: Legacy reference retained
- **WHEN** an operator audits the codebase
- **THEN** legacy swarm modules remain available for reference without being part of the default runtime
