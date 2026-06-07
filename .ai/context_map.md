# X3 Autonomous Context Map

This repo should be treated as a code-first launch-readiness surface.
Markdown reports are useful hints, but live code, executable proof runners,
tests, generated receipts, and build outputs are the source of truth.

Primary bootstrap artifacts:
- `.roo/rules.md` - autonomous integration guardrails
- `.ai/system_prompt.md` - senior engineer/auditor operating prompt
- `.ai/tasks.md` - phase driver
- `.cache/file_list.txt` - generated file enumeration
- `CODE_COVERAGE_TRACKER.md` - scan coverage ledger
- `FEATURE_GAP_REPORT.md` - extracted gaps and proof status
- `INTEGRATION_PLAN.md` - patch ordering
- `MAINNET_READINESS_DELTA.md` - launch-readiness changes and blockers

Roo model routing:
- `free-scan`: file listing, initial scans, boring passes
- `cheap-coder`: repo analysis, feature extraction, patching, most coding
- `heavy-claude`: architecture fixes, critical bugs, final audit

High-signal proof surfaces already present in this checkout:
- `Cargo.toml`
- `proof-forge/`
- `proof/claims/registry.yml`
- `proof/receipts/claims/`
- `launch-gates/`
- `runtime/`
- `node/`
- `pallets/`
- `crates/`

Do not promote readiness claims unless the matching code and command evidence
exist in the repo.
