# Change: add-offchain-jury

## Why
Introduce an auditable off-chain jury system to govern major tasks and proposals for the Orchestra. This enables stronger alignment guarantees (majority review for major tasks), safer experimentation (ephemeral stress-testers), and structured remediation (scrap yard) while keeping the core on-chain protocol stable.

## What Changes
- **ADDED** Off-chain Jury capability under `specs/orchestra-governance/spec.md` (voting, rotation, anonymity, logging).
- **ADDED** Task interface (.md specs) enforcement rules and workflow for minor/major classification and jury flow.
- **ADDED** APIs and orchestration modules for jury lifecycle (spawn, vote, aggregate, log) in `swarm/`.
- **ADDED** Audit & encryption rules for logging to ensure privacy with verifiable on-chain anchors (hashes).
- **IMPACT**: New backend work (swarm API + agents), tests, and docs. Minimal on-chain changes (immutable Score references + log hashes).

## Impact
- Affected specs: `specs/orchestra-governance/spec.md`
- Affected code: `swarm/api_server.py`, `swarm/core/`, `swarm/infra/` (jury, audit, snapshot), plus tests and migration scripts.
- Security: Requires cryptographic design for anonymous voting and encrypted logs; these are detailed in `design.md`.


## Next step
Create `tasks.md`, `design.md`, and spec delta for review and `openspec validate`.