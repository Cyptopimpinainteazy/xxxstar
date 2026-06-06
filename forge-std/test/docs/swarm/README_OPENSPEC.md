# Swarm OpenSpec Integration

This guide describes how swarm (Orchestra) work stays spec-driven using OpenSpec.

## Overview
- Major tasks require an OpenSpec change ID and strict validation.
- Task metadata records `openspec_change_id` for auditability.
- OpenSpec change skeletons can be created via API.

## Environment
- `OPENSPEC_BIN`: optional absolute path to the OpenSpec CLI.
- If unset, the system resolves `openspec` from PATH.

## API Endpoints
- `GET /api/openspec/status`: OpenSpec CLI availability.
- `POST /api/openspec/change/create`: create a change skeleton.
- `POST /api/openspec/change/validate`: validate a change.
- `GET /api/openspec/change/status/{change_id}`: cached status.
- `POST /api/openspec/change/attach`: attach change ID to a task.

## Task Submission
Include OpenSpec metadata when a task is spec-driven:

```json
{
  "workload_type": "general_compute",
  "priority": "normal",
  "severity": "major",
  "openspec_change_id": "add-openspec-swarm-integration",
  "payload": {
    "work": "example"
  }
}
```

Major tasks will be rejected if validation fails or `openspec_change_id` is missing.

## Notes
- Validation uses `openspec validate <change-id> --strict`.
- Validation results are cached briefly to reduce latency.
