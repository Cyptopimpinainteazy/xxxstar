# Roo Legion Mode Setup

Date: 2026-05-02

Configured profile import files:
- `/home/lojak/.config/roo/roo-cline-wrapper-settings.json`
- `/home/lojak/.config/roo/roo-code-nightly-wrapper-settings.json`

Profiles:
- `free-scan`: OpenRouter `openrouter/free`, max tokens 4096, thinking 1024,
  temperature 0.2, rate limit 2s, error limit 3
- `cheap-coder`: OpenRouter `qwen/qwen3-coder-next`, max tokens 8192,
  thinking 2048, temperature 0.2, rate limit 1s, error limit 3
- `heavy-claude`: OpenRouter `anthropic/claude-sonnet-4.5`, max tokens 16384,
  thinking 8192, temperature 0.1, rate limit 2s, error limit 3
- `local-ollama`: Ollama `qwen2.5-coder:7b`, free fallback and bulk boring work

Legion roles:
- `.roo/agents/X3_AGENT.md`
- `.legion/SCANNER.md`
- `.legion/INTEGRATOR.md`
- `.legion/AUDITOR.md`
- `.legion/ARCHITECT.md`
- `.legion/FIXER.md`

Global modes:
- `x3-autonomous-deep-dive`
- `x3-legion-scanner`
- `x3-legion-integrator`
- `x3-legion-auditor`
- `x3-legion-architect`
- `x3-legion-fixer`

MCP servers:
- `github`: existing GitHub MCP
- `filesystem`: scoped to `/home/lojak/Desktop/X3_ATOMIC_STAR`
- `repomix`: `npx -y repomix --mcp`
- `sequentialthinking`: existing planning MCP

VS Code Roo settings:
- Expanded `roo-cline.allowedCommands`
- Expanded `roo-code-nightly.allowedCommands`
- Enabled `roo-cline.newTaskRequireTodos`
- Enabled `roo-cline.preventCompletionWithOpenTodos`
- Kept command execution timeout unlimited for long scans

Validation:
- JSON parsed successfully for Roo wrapper files and VS Code settings.
- YAML parsed successfully for Roo custom modes.
- OpenRouter cache contains all three requested model IDs.
- `local-ollama` profile points at `qwen2.5-coder:7b`.
- `.scripts/full_scan.sh` completed: 114410 files in `.cache/full_file_list.txt`.
- `.scripts/smell_scan.sh` completed: 35477 lines in `.reports/smells.txt`.
- `CODE_COVERAGE_TRACKER.md` regenerated from `.cache/full_file_list.txt`.
- Traycer specs added in `.traycer/`.
- X3 guardrails added in `.x3/`.
- Repomix packer added: `.scripts/x3_repomix_pack.sh`.
- Repomix MCP added to Roo MCP settings.
- Filesystem MCP scoped to the X3 repo path.
- AI loop added: `.scripts/x3_ai_loop.sh`.
- X3 scanner completed: 114435 files in `.cache/x3_full_file_list.txt`.
- X3 smell scan completed: 55881 lines in `.reports/x3_smells.txt`.
- `CODE_COVERAGE_TRACKER.md` regenerated from `.cache/x3_full_file_list.txt`.
- GraphOps builder completed: 45638 nodes, 52453 edges, 0 unreadable files, 9 recorded large-file skips.
- Invariant dashboard generated: `.x3/dashboards/INVARIANT_COVERAGE.md`.
- GraphOps reports generated: `GRAPHOPS_REPORT.md`, `NEXT_SAFE_PATCHES.md`, `NEXT_DANGER_ZONE_PATCHES.md`.
- Level 10 swarm shell added: `.swarm/agents/AGENT_ROSTER.md`, `.swarm/prompts/COMMANDER.md`, `.swarm/state/task_queue.md`, `.scripts/x3_level10_swarm.sh`, `.scripts/x3_swarm_loop.sh`.
- Swarm shell validation passed: `bash -n .scripts/x3_level10_swarm.sh .scripts/x3_swarm_loop.sh`; `python3 -m json.tool .swarm/state/swarm_state.json`.
- Level 10 control plane added: `.x3/context/X3_ENGINEERING_CONSTITUTION.md`, `.x3/context/X3_SWARM_CONFIG.yaml`, `.x3/evals/X3_EVALS.md`, `.x3/drift/X3_DRIFT_RULES.md`, `.scripts/x3_drift_detector.py`, `.scripts/x3_mutation_gate.py`, `.scripts/x3_eval_runner.sh`, `.scripts/x3_break_the_chain.sh`, `.scripts/x3_level10_cycle.sh`, and specialist prompts in `.swarm/prompts/`.
- Control plane validation passed: Python compile for detector/gate/GraphOps scripts; shell syntax for eval, attack, and cycle scripts.
- Drift detector wrote `.x3/reports/DRIFT_REPORT.md`.
- Mutation gate wrote `.x3/reports/MUTATION_GATE.md` and intentionally blocked the current worktree due to 49 danger-zone paths.
- Break-the-chain scanner wrote `.x3/reports/BREAK_THE_CHAIN_RESULTS.md`.

Operational note:
- Restart or reload VS Code so Roo imports the wrapper settings.
- OpenRouter profiles still require a valid OpenRouter API key in Roo's normal
  key storage or environment.
- `../old-x3-project` was not present during setup; set `OLD_PROJECT_ROOT` if
  the old project lives somewhere else.
- Repomix was not run during validation to avoid a surprise first-run network
  install and large context-pack generation. Run `.scripts/x3_repomix_pack.sh`
  when ready.
- GraphOps-lite is heuristic; use it for patch planning and blast-radius prompts,
  not as a formal proof of feature completeness.
- `.scripts/x3_level10_cycle.sh` runs expensive checks through `.scripts/x3_eval_runner.sh`.
  Use it when you are ready for a full evidence cycle, not as a cheap heartbeat.
