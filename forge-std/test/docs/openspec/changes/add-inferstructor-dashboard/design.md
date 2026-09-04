## Context
Operator features are scattered across dashboards with inconsistent naming. The control plane needs a single, professional surface with clear data-driven copy and predictable navigation.

## Goals / Non-Goals
- Goals:
  - Consolidate operational views into Inferstructor dashboard.
  - Enforce non‑hype copy and production-safe UX patterns.
  - Provide direct access to validator, swap, proof, faucet, and funding controls.
- Non-Goals:
  - Consumer marketing site content.
  - Experimental or speculative UI language.

## Decisions
- Rename all UI and docs references to “Inferstructor”.
- Use `apps/inferstructor-dashboard/` as the canonical operator surface.
- Keep routes shallow (1–2 levels) and avoid dead-end pages.

## Risks / Trade-offs
- Renaming and consolidation will touch many imports; mitigate with a centralized route map.
- Admin features depend on backend APIs; stub with guarded UI until endpoints exist.

## Migration Plan
1. Rename app directory and update build tooling.
2. Consolidate components from legacy dashboards.
3. Replace copy and update docs.

## Open Questions
- Which dashboards are authoritative sources for validator lifecycle controls.
