# MAINNET READINESS DELTA

## Purpose
This document explains the current difference between older RC-1 readiness signals and the new canary-first launch posture.

## Current delta
- Older RC-1 gate outputs are still present, but they are now provisional.
- Current authoritative readiness posture is based on the canary plan and refreshed gate artifacts.
- The repo should not treat stale RC-1 artifacts as a public mainnet readiness statement.

## Evidence of the delta
- Existing launch gate reports may still reference RC-1 readiness.
- `proof/reports/gap_gate_mainnet_20260426_194429.txt` and `proof/reports/todo_gate_mainnet_20260426_194331.txt` show unresolved material issues.
- `reports/testnet_readiness_report.md` may still be stale and must be regenerated.
- Public-facing readiness should be governed by the new canary launch plan.

## What this means
- The safe public message is now: internal RC-1 canary readiness, not broad mainnet launch.
- The current authoritative readiness story should be based on:
  - `docs/MAINNET_CANARY_PLAN.md`
  - `docs/MAINNET_READINESS_CHECKLIST.md`
  - `.x3/X3_MAINNET_GATES.md`
  - regenerated readiness reports

## Action items
1. Re-run the readiness engines and regenerate `reports/testnet_readiness_report.md` and `reports/mainnet_rc_report.md`.
2. Refresh `docs/CURRENT_MAINNET_STATUS.md` and `docs/MAINNET_READINESS_CHECKLIST.md` with reconciled gate outputs.
3. Publish a single canonical readiness scoreboard with green/yellow/red statuses.
4. Keep public messaging limited to the RC-1 canary scope and intentionally deferred features.

## Notes
This document is intentionally conservative: until the current gate artifacts are refreshed and reconciled, the repo should not market broad mainnet readiness.
