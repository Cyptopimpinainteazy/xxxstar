## Context
Swarm (Orchestra) operations need to be spec-driven so that agent work is constrained by approved requirements. OpenSpec provides the proposal/spec/tasks artifacts and strict validation. The integration should be lightweight and compatible with existing task and API flows.

## Goals / Non-Goals
- Goals:
  - Create or attach OpenSpec change IDs for swarm work.
  - Block major task execution unless OpenSpec validates cleanly.
  - Log the change ID alongside task activity for auditability.
- Non-Goals:
  - Replacing existing governance or jury flows.
  - Mandating OpenSpec for trivial, internal health tasks.

## Decisions
- Decision: Use `OPENSPEC_BIN` env var with PATH fallback to locate the CLI.
- Decision: Treat OpenSpec validation as a gate for major tasks only.
- Decision: Store `openspec_change_id` in task payload metadata and telemetry logs.

## Risks / Trade-offs
- Risk: CLI not available at runtime.
  - Mitigation: Explicit config + health endpoint to report availability.
- Risk: Validation latency for frequent task intake.
  - Mitigation: Cache validated change IDs for a short TTL.

## Migration Plan
1. Land spec delta and validate.
2. Add config + helpers.
3. Wire validation gate and metadata logging.
4. Expose API endpoints for create/validate/attach.

## Open Questions
- Which tasks are classified as major by default?
- What TTL should be used for validation caching?
