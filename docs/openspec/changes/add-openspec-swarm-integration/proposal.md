# Change: add-openspec-swarm-integration

## Why
Ensure swarm (Orchestra) operations remain spec-driven by integrating OpenSpec into task intake, validation, and audit logging. This keeps agent work aligned to approved requirements and provides a verifiable trace from tasks to specs.

## What Changes
- **ADDED** OpenSpec integration capability for swarm task lifecycle (skeleton creation, validation gate, spec linkage).
- **ADDED** Runtime configuration for locating the OpenSpec CLI.
- **ADDED** API endpoints to create/validate changes and attach change IDs to tasks.
- **IMPACT**: Swarm API, task queue, and telemetry logging gain OpenSpec hooks.

## Impact
- Affected specs: `specs/orchestra-ops/spec.md`
- Affected code: `swarm/api_server.py`, `swarm/agents/task_queue.py`, `swarm/core/orchestrator.py`, `swarm/telemetry/agent_registry.py`
- Security: Validation gate prevents unapproved changes from executing.
