# X3 Traycer Task Chain

## Task 1 - Pack Context

Run:

```bash
.scripts/x3_repomix_pack.sh
```

Confirm:
- `.repomix/MANIFEST.md` exists and was generated for this task
- `.repomix/current_x3_atomic_star.md` exists
- `.repomix/old_x3_project.md` exists
- `.repomix/x3_runtime_bridge_dex.md` exists

Do not start feature mapping from stale Repomix packs.
If `OLD_PROJECT_ROOT` is missing, log that as a blocker before comparing old/current features.

## Task 2 - Build Feature Map

Use Repomix outputs to identify:
- old features
- current features
- missing features
- duplicate/conflicting features
- dangerous/stubbed features

Write:
- `FILE_INDEX.md`
- `OLD_PROJECT_FEATURE_INVENTORY.md`
- `CURRENT_PROJECT_FEATURE_INVENTORY.md`
- `.x3/X3_SYSTEM_MAP.md`
- `.x3/X3_FEATURE_REGISTRY.md`

## Task 3 - Write Integration Plan

Create `INTEGRATION_PLAN.md` with:
- source path
- destination path
- risk
- dependencies
- tests
- acceptance criteria
- rollback plan

## Task 4 - Execute P0 Patches

Send one scoped task at a time to Roo.

Rules:
- edit only scoped files
- update `PATCH_LOG.md`
- update `.x3/X3_FEATURE_REGISTRY.md`
- update `.x3/X3_RISK_REGISTER.md`
- run relevant tests
- stop after this task
- report pass/fail

## Task 5 - Verify

Run test commands from:
- `.x3/TEST_COMMANDS.md`

Write:
- `TEST_RESULTS.md`
- `FAILED_CHECKS.md`
- `MAINNET_READINESS_DELTA.md`

## Task 6 - Audit

Generate:
- `FINAL_BOSS_AUDIT.md`
- `P0_BLOCKERS.md`
- `NEXT_10_PATCHES.md`

## Task 7 - Architect

Generate:
- `NEW_IDEAS_REPORT.md`
- `X3_COMPETITIVE_ADVANTAGE.md`
- `FASTEST_MAINNET_PLAN.md`
