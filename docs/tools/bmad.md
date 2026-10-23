# BMAD Method Integration

This repository has the [BMAD Method](https://bmadcode.com/) (`bmad-method` v6) installed at the project root with the **BMAD Method (BMM)** module and the **GitHub Copilot** tool target.

## What is installed

| Path | Contents |
| --- | --- |
| `_bmad/` | Core + BMM module definitions, shared Python scripts, and generated config. Commit this directory. |
| `_bmad/config.toml` | Installer-managed configuration (regenerated on reinstall). |
| `_bmad/config.user.toml` | Personal install answers (user name, language, skill level). |
| `_bmad/custom/config.toml` | Team overrides. Committed. |
| `_bmad/custom/config.user.toml` | Personal overrides. Gitignored. |
| `.agents/skills/bmad-*` | 29 BMAD skills (agents, workflows, help) discoverable by Copilot. |
| `.github/agents/bmad-agent-*.agent.md` | Five Copilot custom agent entry points for the BMAD personas. |
| `_bmad-output/` | Generated planning and implementation artifacts. Gitignored. |

## Version status

Installed: **v6.12.4**, the latest stable release (tagged 2026-10-14, `_bmad/_config/manifest.yaml` confirms). Verified byte-identical to the published `bmad-method@6.12.4` tarball for every installed skill.

BMad ships on two lines:

| Line | Where | Install | Status |
| --- | --- | --- | --- |
| v6.12 | npm `bmad-method` | `npx bmad-method install` | Stable, bug fixes only. **What this repo uses.** |
| v7 | GitHub `main` | `npx skills add bmad-code-org/BMAD-METHOD --skill '*'` | Preview alpha — breaking changes between updates |

v7 is not on npm; `dist-tags.next` is `6.12.3-next.1`, which predates the stable line. Any v7 upgrade here is a `skills` CLI install, not an npm bump.

## Usage

Open the repository in VS Code with GitHub Copilot and invoke the `bmad-help` skill to be guided through the workflow. Typical entry points:

- `bmad-help` — orientation and next-step recommendation
- `bmad-project-context` — establish project-wide context
- `bmad-prd` / `bmad-product-brief` — planning artifacts
- `bmad-architecture` — architecture decisions
- `bmad-create-epics-and-stories` → `bmad-sprint-planning` → `bmad-build` — delivery loop
- `bmad-agent-analyst`, `bmad-agent-pm`, `bmad-agent-architect`, `bmad-agent-ux-designer`, `bmad-agent-dev` — persona agents

Planning artifacts land in `_bmad-output/planning-artifacts/`; implementation artifacts land in `_bmad-output/implementation-artifacts/`.

## Install, update, and reinstall

```sh
# Install (already done; safe to re-run, answers are remembered)
npm run bmad:install

# Update an existing installation in place
npm run bmad:update
```

Both scripts wrap:

```sh
npx bmad-method@6.12.4 install --directory . --modules bmm --tools github-copilot --yes
```

Useful installer flags:

- `--list-tools` — supported IDE/agent targets and their directories
- `--list-options` — configurable module keys
- `--set core.user_name=<name>` — change a config answer non-interactively
- `--action update` — refresh an existing install

## Installing v7 preview instead

Not done here — recorded so the decision is reversible and informed. v7 changes things this repo currently relies on:

- `bmad-help` is replaced by the `bmad` hub skill (`bmad status`, `bmad setup`).
- `bmad-spec` and `bmad-correct-course` are gone; Sofia the Product Owner (`bmad-agent-po`) replaces them.
- `planning_artifacts` and `implementation_artifacts` are no longer read or seeded — a v6 project needs `bmad migrate method`.
- Documents move under `<type>-<slug>/` inside an optional `initiative-<slug>/`.
- Web bundles, the HTML review report, and translated docs are removed.

```sh
# Install the preview (asks which coding tools to target)
npx skills add bmad-code-org/BMAD-METHOD --skill '*'

# Then, in your agent: ask the `bmad` skill to run `bmad setup`
# and, for the v6 config layout: `bmad migrate method`
```

Rolling back means reinstalling v6 (`npm run bmad:install`) — `bmad setup` offers to delete skills a v6 installer left behind, so a clean revert needs the old skills restored from git.

## Configuration

Edit `_bmad/custom/config.toml` (team, committed) or `_bmad/custom/config.user.toml` (personal, gitignored) to override generated settings. Do not hand-edit `_bmad/config.toml` or `_bmad/config.user.toml`; the installer overwrites them.

Resolve the merged configuration to verify values (this is the invocation the skills themselves use):

```sh
uv run _bmad/scripts/resolve_config.py --project-root . -k core.user_name -k modules.bmm.user_skill_level
```

## Notes

- BMAD is a third-party package, MIT (Copyright (c) 2025 BMad Code, LLC). Upstream: <https://github.com/bmad-code-org/BMAD-METHOD>.
- Requires Node.js >= 20.12, Python >= 3.10, and `uv` for setup and the Python scripts. All three are present here (`uv` at `~/.local/bin/uv`).
