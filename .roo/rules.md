# Roo Legion Rules

You are working on X3 Atomic Star.

Hard rules:
- No stubs.
- No fake completion.
- No skipped files.
- Track all work in CODE_COVERAGE_TRACKER.md.
- Every patch must update PATCH_LOG.md.
- Every mainnet claim must be backed by tests or explicit evidence.
- Do not weaken tests to pass.
- Do not delete features without recording why.
- Risky runtime, bridge, VM, DEX, asset-kernel, and mainnet config changes require tests and audit notes.

Cost rules:
- Default to the `free-scan` provider profile for scanning.
- Use the `X3 Cheap Scan` mode for first-pass repo work.
- Use cheap/local models only for broad scans, stale-doc checks, and blocker discovery.
- Escalate to stronger models only for final audit, hard bugs, and architecture/security decisions.
- Do not use write, command, or MCP tools automatically in this repo.
- Ask before mutating files, running shell commands, or touching MCP.

Required tracking:
- CODE_COVERAGE_TRACKER.md
- .x3/X3_FEATURE_REGISTRY.md
- .x3/X3_RISK_REGISTER.md
- PATCH_LOG.md
- MAINNET_READINESS_DELTA.md

Context engineering:
- `.x3/context/X3_ENGINEERING_CONSTITUTION.md` is the project constitution.
- `.x3/context/X3_SWARM_CONFIG.yaml` is the declarative swarm config.
- `.x3/evals/X3_EVALS.md` defines patch pass/fail expectations.
- `.x3/drift/X3_DRIFT_RULES.md` defines drift detection rules.

Traycer + Repomix workflow:
- Repomix builds stable context packs in `.repomix/`.
- Traycer turns `.traycer/X3_DEEP_DIVE_SPEC.md` and `.traycer/X3_TASK_CHAIN.md` into scoped implementation tasks.
- Roo scans one scoped task at a time and reports exact proof gaps.
- Roo patches code only when the user explicitly requests implementation.
- Do not use one monster prompt for scan + patch + audit when a Traycer task chain exists.

X3 proof rules:
- Code and command output outrank markdown claims.
- Keep old-project inventory separate from current-project inventory.
- Classify generated artifacts, archives, tests, and production code separately.
- Treat runtime, pallets, node, bridge/router, X3VM/EVM/SVM, Universal Asset Kernel, DEX/launchpad, proof, GPU validator, and mainnet launch paths as high-risk surfaces.

COST GUARDRAIL:
Do not re-read the entire repo unless required.
Use existing reports and trackers first.
Use cheap models for repetitive work.
Escalate to Claude only when blocked or auditing critical architecture/security.

Secret safety:
- Load `.x3/DO_NOT_TOUCH.md` before touching config, deployment, genesis, bridge admin, treasury, validator, wallet, or environment files.
- Never print or copy secrets into reports.

GraphOps rules:
- Load `.x3/graph/index.md` and `.x3/invariants/X3_INVARIANTS.md` before editing runtime, bridge/router, VM, DEX, asset-kernel, chain spec, genesis, validator, or launch paths.
- Before every patch, answer: what feature is this, what invariant can it affect, what files depend on it, and what test proves it still works.
- If those answers are missing, stop and map dependencies first.

Mutation rules:
- Classify every edited path as SAFE_ZONE or DANGER_ZONE using `.x3/mutations/SAFE_ZONES.md` and `.x3/mutations/DANGER_ZONES.md`.
- SAFE_ZONE files may be patched when scoped, with PATCH_LOG.md and relevant checks.
- DANGER_ZONE files require an explicit scoped task, tests, risk-register update, rollback notes, and no broad rewrites.
- Never patch secrets, treasury, genesis balances, validator keys, wallet seeds, bridge admin keys, or production deployment addresses without explicit approval.

Level 10 swarm rules:
- Many agents may analyze, but only one agent patches at a time.
- Run `.scripts/x3_level10_cycle.sh` to refresh context, drift, eval, mutation, graph, and attack reports.
- Completion of the cycle script is not proof; each report must be read for PASS/FAIL/BLOCKED state.
- Use `.swarm/prompts/COMMANDER_LEVEL10.md` for orchestration and role prompts in `.swarm/prompts/` for specialist tasks.
