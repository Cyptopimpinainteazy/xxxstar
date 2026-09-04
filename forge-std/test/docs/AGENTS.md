# OpenSpec — Agent Workflow for X3 Chain

## Purpose

OpenSpec is the **authoritative specification workflow** for X3 Chain.
Any change that introduces new capabilities, breaking changes, architecture shifts,
or significant performance/security work **must** go through a change proposal before
implementation begins.

## When to Create a Proposal

| Trigger | Action |
|---------|--------|
| New pallet, crate, or subsystem | Proposal required |
| Breaking change to public APIs/types | Proposal required |
| Architecture shift (new VM backend, consensus change, etc.) | Proposal required |
| Performance/security initiative (>1 week scope) | Proposal required |
| Bug fix, test addition, doc update | No proposal needed — go straight to code |

## Directory Layout

```
openspec/
├── AGENTS.md                   ← you are here
├── changes/
│   └── <CHANGE-ID>/
│       ├── proposal.md         ← main specification
│       ├── diagrams/           ← optional Mermaid / SVG
│       └── notes/              ← supporting research, benchmarks
└── templates/
    └── proposal-template.md
```

## Proposal Lifecycle

```
DRAFT → REVIEW → APPROVED → IMPLEMENTING → DONE
                          ↘ REJECTED
```

1. **DRAFT** — Author writes `proposal.md` under `openspec/changes/<ID>/`.
2. **REVIEW** — Stakeholders read; feedback captured in `notes/`.
3. **APPROVED** — Spec is locked; implementation can begin.
4. **IMPLEMENTING** — Code PRs reference the proposal ID.
5. **DONE** — All code merged, tests pass, invariants registered.
6. **REJECTED** — Proposal closed with rationale.

## Proposal ID Convention

```
<DOMAIN>-<COMPONENT>-<SEQ>
```

Examples: `DEPIN-GPU-001`, `EXEC-PREDICT-002`, `PRIV-ENCLAVE-003`

## Validation

```bash
# (future) openspec validate <ID> --strict
# For now, manually verify:
#   1. proposal.md exists and has all required sections
#   2. invariants referenced in tests/invariants/registry.toml
#   3. status field is current
```

## Required Sections in proposal.md

1. **Title & ID**
2. **Status** (DRAFT | REVIEW | APPROVED | IMPLEMENTING | DONE | REJECTED)
3. **Authors**
4. **Summary** — one paragraph
5. **Motivation** — problem statement
6. **Design** — architecture, data flow, key types
7. **Integration Points** — what existing code is affected
8. **Invariants** — new entries for `tests/invariants/registry.toml`
9. **Testing Strategy**
10. **Rollout Plan** — phased delivery milestones
11. **Risks & Mitigations**
12. **Open Questions**

## For AI Agents

- **Always read this file** when the user's request mentions proposals, specs,
  architecture changes, or breaking changes.
- Create proposals under `openspec/changes/<ID>/`.
- Register invariants in `tests/invariants/registry.toml`.
- Reference proposal IDs in PR descriptions and commit messages.
- When in doubt about scope, ask one clarifying question.
