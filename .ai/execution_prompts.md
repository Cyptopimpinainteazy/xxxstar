# Roo Legion Execution Prompts

Paste this into every Roo task:

```text
COST GUARDRAIL:
Do not re-read the entire repo unless required.
Use existing reports and trackers first.
Use cheap models for repetitive work.
Escalate to Claude only when blocked or auditing critical architecture/security.
```

## Step 0 - Context Refresh First

Do this before Scanner, Integrator, Auditor, Architect, GraphOps, or Commander passes:

```text
Run .scripts/x3_repomix_pack.sh first unless .repomix/MANIFEST.md proves the packs are fresh enough for this exact task.

Then load context in this order:
1. .repomix/MANIFEST.md
2. .repomix/current_x3_atomic_star.md
3. .repomix/old_x3_project.md
4. .repomix/x3_runtime_bridge_dex.md
5. CODE_COVERAGE_TRACKER.md
6. FILE_INDEX.md
7. Relevant .x3, .swarm, .legion, and .traycer files

Never begin patch planning from stale packs.
If OLD_PROJECT_ROOT is missing, document that blocker before old/current comparison.
```

## Pass 1 - Free Scanner

Switch to `free-scan`, then run:

```text
Load .roo/rules.md and .legion/SCANNER.md.

Run .scripts/x3_repomix_pack.sh first.
Read .repomix/MANIFEST.md.
Run .scripts/x3_full_scan.sh
Run .scripts/x3_smell_scan.sh

Create:
- FILE_INDEX.md
- CODE_COVERAGE_TRACKER.md
- OLD_PROJECT_FEATURE_INVENTORY.md
- CURRENT_PROJECT_FEATURE_INVENTORY.md
- .x3/X3_SYSTEM_MAP.md

Process every file from .cache/x3_full_file_list.txt.
Do not integrate yet.
```

## Pass 2 - Cheap Integration

Switch to `cheap-coder`, then run:

```text
Load .roo/rules.md and .legion/INTEGRATOR.md.

First:
- Run .scripts/x3_repomix_pack.sh unless .repomix/MANIFEST.md proves current packs are fresh.
- Read .repomix/MANIFEST.md.

Using:
- CODE_COVERAGE_TRACKER.md
- OLD_PROJECT_FEATURE_INVENTORY.md
- CURRENT_PROJECT_FEATURE_INVENTORY.md
- .repomix/current_x3_atomic_star.md
- .repomix/old_x3_project.md
- .repomix/x3_runtime_bridge_dex.md
- .reports/x3_smells.txt
- .traycer/X3_TASK_CHAIN.md

Create:
- MIGRATION_INVENTORY.md
- FEATURE_GAP_REPORT.md
- INTEGRATION_PLAN.md

Then implement P0/P1 features only.
Run tests after each patch.
Update PATCH_LOG.md.
Update .x3/X3_FEATURE_REGISTRY.md.
Update .x3/X3_RISK_REGISTER.md.
```

## Pass 3 - Claude Audit

Switch to `heavy-claude`, then run:

```text
Run .scripts/x3_repomix_pack.sh unless .repomix/MANIFEST.md proves current packs are fresh.
Read .repomix/MANIFEST.md.

Audit entire repo after integration.

Focus:
- architecture flaws
- security risks
- mainnet blockers
- missing invariants

Produce:
- DEEP_AUDIT_REPORT.md
- SECURITY_BLOCKERS.md
- MAINNET_READINESS_DELTA.md
- FINAL_AUDIT_REPORT.md

Do not make broad rewrites unless required.
Find blockers.
Rank them P0/P1/P2.
```

## Pass 4 - Invention Pass

Switch to `cheap-coder` or `heavy-claude`, then run:

```text
Load .roo/rules.md and .legion/ARCHITECT.md.

Run .scripts/x3_repomix_pack.sh unless .repomix/MANIFEST.md proves current packs are fresh.
Read .repomix/MANIFEST.md.

Create NEW_IDEAS_REPORT.md.
Create X3_COMPETITIVE_ADVANTAGE.md.
Create FASTEST_MAINNET_PLAN.md.

Give me:
- better architecture ideas
- competitor features worth copying
- X3-only differentiators
- mainnet launch shortcuts
- what to cut
- what to build next
```

## Autopilot Loop

Optional terminal loop:

```bash
watch -n 10 "cargo check || npm run build || pytest"
```

Then tell Roo:

```text
Monitor output and fix errors continuously.
```

## Fixer Pass

Use `cheap-coder` or `local-ollama`, then run:

```text
Load:
- .roo/agents/X3_AGENT.md
- .roo/rules.md
- .legion/FIXER.md

Fix current failing build/test only.
Do not add features.
Do not refactor unrelated code.
Stop when the failing command passes or blocker is documented.

Failing command:
[paste exact command]
```

## Final Boss Pass

Use `heavy-claude`, then run:

```text
Final boss pass.

Load:
- .roo/rules.md
- .x3/X3_MAINNET_GATES.md
- .x3/ACCEPTANCE_CRITERIA.md
- .x3/TEST_COMMANDS.md
- .x3/DO_NOT_TOUCH.md

Audit X3 Atomic Star as if real money is going live.

Find:
- every remaining P0 blocker
- every fake-complete feature
- every test gap
- every dangerous config
- every doc/code mismatch

Output:
- FINAL_BOSS_AUDIT.md
- P0_BLOCKERS.md
- NEXT_10_PATCHES.md

Do not patch yet. Report first.
```

## GraphOps Pass

Use `cheap-coder` for the first pass, then `heavy-claude` only for danger-zone review:

```text
Load:
- .roo/rules.md
- .x3/graph/index.md
- .x3/invariants/X3_INVARIANTS.md
- .x3/mutations/SAFE_ZONES.md
- .x3/mutations/DANGER_ZONES.md
- .x3/attacks/BREAK_THE_CHAIN_SCENARIOS.md
- .x3/X3_MAINNET_GATES.md
- .x3/X3_RISK_REGISTER.md

Mission:
Run X3 GraphOps.

Steps:
1. Run .scripts/x3_repomix_pack.sh unless .repomix/MANIFEST.md proves current packs are fresh
2. Run python3 .scripts/x3_graph_builder.py
3. Run python3 .scripts/x3_invariant_dashboard.py
4. Read .repomix/MANIFEST.md
5. Read .x3/dashboards/INVARIANT_COVERAGE.md
6. Pick the highest-risk feature with missing tests
7. Create a scoped patch plan
8. Do not patch danger-zone files without an explicit scoped task
9. Update PATCH_LOG.md and .x3/X3_RISK_REGISTER.md for every real change

Output:
- GRAPHOPS_REPORT.md
- NEXT_SAFE_PATCHES.md
- NEXT_DANGER_ZONE_PATCHES.md
```

## Level 10 Commander Pass

Use `cheap-coder` as Commander, then escalate only scoped danger-zone review to `heavy-claude`:

```text
Load:
- .swarm/prompts/COMMANDER_LEVEL10.md
- .swarm/agents/AGENT_ROSTER.md
- .swarm/state/task_queue.md
- .roo/rules.md
- .x3/context/X3_ENGINEERING_CONSTITUTION.md
- .x3/context/X3_SWARM_CONFIG.yaml
- .x3/X3_MAINNET_GATES.md
- .x3/TEST_COMMANDS.md
- .x3/graph/index.md
- .x3/dashboards/INVARIANT_COVERAGE.md

Run:
.scripts/x3_level10_cycle.sh

Then:
1. Read .repomix/MANIFEST.md first.
2. Read .reports/git_status.txt and failing check reports.
3. Read .x3/reports/EVAL_RESULTS.md, .x3/reports/DRIFT_REPORT.md, .x3/reports/MUTATION_GATE.md, and .x3/reports/BREAK_THE_CHAIN_RESULTS.md.
4. Read GRAPHOPS_REPORT.md and NEXT_SAFE_PATCHES.md.
5. Pick exactly one highest-value safe task.
6. Execute only that task.
7. Update PATCH_LOG.md, .swarm/state/decisions.md, and relevant X3 trackers.

Rule:
Many agents analyze. One agent patches. Tests decide truth.
```
