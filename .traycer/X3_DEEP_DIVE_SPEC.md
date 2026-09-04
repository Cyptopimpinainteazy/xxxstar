# X3 Atomic Star Deep Dive Spec

Goal:
Deep scan old X3 and current X3 Atomic Star, extract useful features, integrate missing P0/P1 systems, audit mainnet blockers, and propose better architecture.

Inputs:
- `.repomix/old_x3_project.md`
- `.repomix/current_x3_atomic_star.md`
- `.repomix/x3_runtime_bridge_dex.md`
- `CODE_COVERAGE_TRACKER.md`
- `.x3/X3_MAINNET_GATES.md`
- `.x3/X3_FEATURE_REGISTRY.md`
- `.x3/X3_RISK_REGISTER.md`
- `.x3/DO_NOT_TOUCH.md`
- `.x3/COST_CONTROL.md`
- `.x3/ACCEPTANCE_CRITERIA.md`
- `.x3/TEST_COMMANDS.md`

Phases:
1. Inventory
2. Feature gap report
3. P0/P1 integration plan
4. Patch execution
5. Test loop
6. Security audit
7. New ideas report
8. Mainnet readiness delta

Rules:
- No skipped files.
- No stubs.
- No fake completion.
- No blind old-code copying.
- No mainnet claims without tests.
- Risky bridge/runtime/VM changes require feature flags and tests.
- Do not touch secrets or production keys listed in `.x3/DO_NOT_TOUCH.md`.
- Use cached reports and Repomix packs before rereading the whole repo.

Outputs:
- `FILE_INDEX.md`
- `OLD_PROJECT_FEATURE_INVENTORY.md`
- `CURRENT_PROJECT_FEATURE_INVENTORY.md`
- `FEATURE_GAP_REPORT.md`
- `INTEGRATION_PLAN.md`
- `DEEP_AUDIT_REPORT.md`
- `SECURITY_BLOCKERS.md`
- `NEW_IDEAS_REPORT.md`
- `MAINNET_READINESS_DELTA.md`
- `FINAL_DEEP_DIVE_SUMMARY.md`
