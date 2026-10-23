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

## Configuration

Edit `_bmad/custom/config.toml` (team, committed) or `_bmad/custom/config.user.toml` (personal, gitignored) to override generated settings. Do not hand-edit `_bmad/config.toml` or `_bmad/config.user.toml`; the installer overwrites them.

Resolve the merged configuration to verify values:

```sh
python3 _bmad/scripts/resolve_config.py -p . -k core.user_name -k modules.bmm.user_skill_level
```

## Notes

- BMAD is a third-party NPM package (MIT). Follow the license and usage guidelines in the upstream repository: <https://github.com/bmad-code-org/BMAD-METHOD>.
- Node.js >= 20 and Python 3 are required (both already used by this repo).
