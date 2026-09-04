# Change: Social Agent Swarm (Draft-Only v1)

## Why
We need a Velo-style multi-agent system specialized for X3 Chain that generates social engagement drafts (posts, comments, DMs, profiles) grounded in approved content (Open Notebook) and routed through the swarm workflow for auditability and later automation.

## What Changes
- Add a social agent swarm capability that produces drafts for outreach, influencer discovery, and community growth.
- Integrate local Ollama inference and Open Notebook as the primary content grounding source.
- Introduce configuration for supported networks (v1 includes first 10 + X), keyword expansion, and approved groups.
- Add audit logging and ToS-safe guardrails (rate limits, action quotas, no impersonation).
- Define a feature-flagged path for future live actions (v2) once APIs are configured.

## Impact
- Affected specs: new `social-agent-swarm` capability.
- Affected code: `swarm/` agents, task queue, telemetry, and API surface; new config and adapters.
