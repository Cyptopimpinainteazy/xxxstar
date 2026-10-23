# BMAD Method Integration

This repository runs **BMAD Method v7 preview** (`6.13.0-next`) as skills, installed with the `skills` CLI. Skills are plain files the agent reads — there is no `bmad` command on `PATH`.

## What is installed

| Path | Contents | Committed |
| --- | --- | --- |
| `.agents/skills/bmad-*`, `.agents/skills/bmod-*` | 30 skills — the set GitHub Copilot loads | yes |
| `.claude/skills/bmad-*`, `.claude/skills/bmod-*` | 30 skills — the set Claude Code loads | no (`.claude/` is gitignored) |
| `_bmad/` | Module records, shared Python scripts, generated config | yes |
| `_bmad/custom/config.toml` | Team overrides | yes |
| `_bmad/custom/config.user.toml` | Personal overrides | gitignored |
| `skills-lock.json` | `skills` CLI lockfile (all installed skills, every source) | yes |
| `_bmad-output/` | Generated planning and delivery artifacts | no |

Three modules: `core-tools` (hub + review/brainstorm/recon), `method` (agents, PRD, architecture, build), `toolsmith` (build agents and skills, `bmad-eval`).

## Usage

Open the repo in VS Code with Copilot and invoke a skill by name in chat:

- `bmad` — answers BMAD questions, recommends the next skill, and runs setup / status / migrate
- `bmad-project-context` — repo agent instructions (produced the `AGENTS.md` block here)
- `bmad-prd` → `bmad-architecture` → `bmad-build` — plan then deliver
- `bmad-agent-po` — Sofia, the Product Owner: tickets, epics, stories, board
- `bmad-agent-analyst` / `-pm` / `-architect` / `-ux-designer` / `-dev` — personas

v7 writes documents under `_bmad-output/<type>-<slug>/`, inside an optional active initiative.

## Install, update, status

```sh
npm run bmad:install   # npx skills add bmad-code-org/BMAD-METHOD --skill '*' --copy -y
npm run bmad:update    # npx skills update -p -y
npm run bmad:status    # setup.py --status, machine-readable install state
```

`--copy` is required here. The CLI defaults to symlinks, which break in a committed repo.

`bmad:install` writes into **every** agent directory the CLI detects. It created a root `skills/` (4 MB) and `.hermes/skills/` in this repo; both were deleted. After a reinstall, check for and remove directories for harnesses that are not used.

## Known defect: `bmad-retrospective` cannot install

v7's `skills/bmad-retrospective/SKILL.md` has malformed YAML frontmatter — the description contains `each ticket's story file`, and the apostrophe in `ticket's` closes the single-quoted scalar. The CLI rejects the file:

```
⚠ Skipped .../bmad-retrospective/SKILL.md — YAML parse error: Unexpected scalar at node end at line 2, column 129
```

Reproduced in a clean temp directory, so it is upstream, not local. Consequences: `setup.py --status` lists `bmad-retrospective` as an absent skill in `method`, and `unmet_recommendations` reports that `bmad-agent-dev` requires it. Retry after the next `npm run bmad:update`; do not hand-patch the file, since an update overwrites it.

## Migration record

Migrated from the v6.12.4 install (commit `f832e95c`, tag `pre-bmad-v7`) on 2026-10-23.

- v6 traces removed from `_bmad/` (11 legacy files, 7 stale config keys) by `migrate_v6.py --clean`.
- Backup: `.v6-v7-migration-backup-20261023-1510/` (708 KB). Delete once the v7 install is trusted.
- Superseded skills removed by `setup.py --remove-retired`: `bmad-help` (→ `bmad`), `bmad-spec`, `bmad-correct-course`, `bmad-create-epics-and-stories`, `bmad-sprint-planning`.
- `core.user_name` and the language answers did not survive; v7's config schema does not carry them.

### Reverting to v6

```sh
git reset --hard pre-bmad-v7 && npm run bmad:install   # v6 installer is gone; see note
```

The v6 npm installer command was replaced, so a revert needs the v6 invocation restored:
`npx bmad-method@6.12.4 install --directory . --modules bmm --tools github-copilot --yes`.

## Notes

- BMAD is MIT (Copyright (c) 2025 BMad Code, LLC), <https://github.com/bmad-code-org/BMAD-METHOD>. v7 is preview alpha on `main` and changes without notice.
- Requires Node.js >= 20.12, Python >= 3.10, and `uv`. All three are present.
