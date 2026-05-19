## Context
We need a modular, swarm-native social agent system that generates drafts only in v1, grounded by Open Notebook and local Ollama. It must be safe-by-default and upgradeable to live API actions later.

## Goals / Non-Goals
- Goals:
  - Draft-only social content across the initial network set.
  - Ground content in Open Notebook + approved sources.
  - Full audit logs and rate limits.
  - Swarm task-queue integration end-to-end.
- Non-Goals:
  - Live posting, DMs, follows, or account creation in v1.
  - Bypassing network ToS or automation protections.

## Decisions
- Decision: Use a draft-only execution mode with a feature flag for future live actions.
- Decision: Implement Open Notebook adapter as the primary knowledge source; optional external APIs via allowlist.
- Decision: Represent network actions as typed draft artifacts in storage with full audit metadata.

## Risks / Trade-offs
- Risk: Over-broad network scope; mitigate by v1 limiting to first 10 networks and drafts-only.
- Risk: Content drift; mitigate via Open Notebook grounding and approved sources.

## Migration Plan
- v1: drafts-only, no external API actions.
- v2: enable live actions per-network behind explicit config and credentials.

## Open Questions
- Final list of v1 networks (assumed to be first 10 from user list, plus X).
- Location and schema of Open Notebook “approved groups” dataset.
