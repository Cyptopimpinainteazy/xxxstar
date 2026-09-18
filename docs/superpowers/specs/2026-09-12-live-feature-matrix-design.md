# Live Feature Matrix Design

Date: 2026-09-12
Repository: `Cyptopimpinainteazy/xxxstar`
Branch: `feat/live-feature-matrix-20260912`

## Goal

Turn the current spreadsheet-style feature inventory into a repository-native, machine-checked readiness system that stays synchronized with the codebase and existing ProofGate/readiness controls.

The system must prevent readiness claims from outrunning evidence. It may lower or flag readiness automatically when required paths, tests, proofs, or CI evidence are missing. It must never silently raise readiness merely because code exists.

## Existing source of truth

`FEATURE_REGISTRY.toml` remains the canonical coarse product/readiness registry. It already carries product-level mode, code path, required tests, proof report, readiness score, blockers, and dangerous paths. `scripts/check-readiness-consistency.sh` already verifies registry paths and cited test names and rejects contradictory status claims.

The new system does not replace that registry. It adds a granular engineering feature matrix beneath it.

## Chosen architecture

### 1. `FEATURE_MATRIX.toml`

Add a granular feature inventory with roughly 100-150 engineering features. Each feature has a stable ID and belongs to one subsystem.

Minimum schema per feature:

```toml
[[feature]]
id = "X3-CROSS-001"
name = "Cross-VM atomic router"
subsystem = "cross_vm_atomic"
paths = ["pallets/x3-cross-vm-router/src/lib.rs"]
source = "master"
implemented = 92
tested = 85
mainnet_ready = 88
priority = "P0"
confidence = "high"
blockers = ["External bridge path remains governance-disabled at genesis"]
evidence = [
  "FEATURE_REGISTRY.toml#atomic_router",
  "reports/six_route_invariants.md"
]
required_tests = [
  "test_all_six_internal_routes_succeed",
  "test_duplicate_message_replay_rejected"
]
registry_feature = "atomic_router"
claim_risk = false
```

Optional fields:

- `open_prs`: PR numbers carrying not-yet-merged work.
- `required_workflows`: named workflows expected to succeed before a mainnet-ready score may be treated as current.
- `evidence_globs`: evidence files/artifacts that must exist.
- `notes`: factual implementation notes.
- `claim_risk`: marks marketing/documentation claims that must not be treated as production capability.
- `launch_scope`: `core`, `guarded`, `experimental`, `dev_tooling`, or `research`.

### 2. `scripts/feature_matrix.py`

One standard-library Python entry point with subcommands:

- `validate`: fail closed on invalid feature metadata or missing evidence.
- `generate`: emit Markdown and JSON reports from the TOML source.
- `check`: run validation plus anti-inflation/readiness rules for CI.
- `summary`: print subsystem and P0/P1 readiness summaries.

The script uses Python 3.11 `tomllib` and no third-party dependencies.

### 3. Generated outputs

Generated files:

- `reports/feature-matrix/X3_FEATURE_MATRIX.md`
- `reports/feature-matrix/X3_FEATURE_MATRIX.json`
- `reports/feature-matrix/X3_FEATURE_SUMMARY.md`

The XLSX artifact is generated in CI from the JSON/CSV-compatible data, not treated as source of truth. If repository policy avoids committing binary files, CI uploads it as an artifact instead of committing it.

### 4. CI workflow

Add `.github/workflows/feature-matrix.yml`.

It runs on pull requests, `master`, and merge-group if supported by the repository workflow conventions.

Required steps:

1. checkout exact head;
2. Python 3.11;
3. `python3 scripts/feature_matrix.py check`;
4. `python3 scripts/feature_matrix.py generate --check-clean`;
5. upload generated JSON/Markdown and XLSX/CSV artifact when available.

The gate is hard-fail. No `|| true`, no continue-on-error for validation.

## Validation rules

### Structural rules

Every feature must have:

- unique stable ID;
- unique name within the matrix;
- known subsystem;
- implementation/test/mainnet scores from 0 to 100;
- priority P0-P3;
- confidence high/medium/low;
- at least one blocker when `mainnet_ready < 80`;
- at least one evidence entry;
- at least one path unless the feature is explicitly `research` or `claim_risk`.

### Code/path evidence

For every declared code path:

- path must exist;
- path may be a file or directory;
- no feature receives implementation evidence from a path that does not exist.

### Test evidence

For every `required_tests` entry:

- the named test function must exist under one of the feature paths or declared test paths;
- missing test citation is a hard failure;
- a feature may not claim `tested >= 80` with zero required tests unless explicit non-Rust evidence is declared.

### Evidence files

Every local evidence path must exist. URLs and PR references are recognized separately and are not treated as local paths.

### Coarse-registry consistency

When `registry_feature` is set:

- the named section must exist in `FEATURE_REGISTRY.toml`;
- granular matrix `mainnet_ready` may be above or below the coarse registry score for an individual subfeature, but aggregate generation must flag any subsystem aggregate that would make the registry feature look healthier than its canonical score;
- generated summary displays the registry score next to matrix-derived scores instead of silently replacing it.

### Readiness anti-inflation

Hard-fail conditions:

- `mainnet_ready >= 80` with a P0 blocker;
- `mainnet_ready >= 80` while `source` references an open PR rather than merged master;
- `mainnet_ready >= 80` with missing required evidence;
- `claim_risk = true` with `mainnet_ready >= 40`;
- `launch_scope = "research"` with `mainnet_ready >= 40`;
- implemented/tested/mainnet scores outside 0-100;
- nonexistent paths/tests/evidence.

Warnings, surfaced in generated reports:

- implementation exceeds testing by more than 35 points;
- testing exceeds mainnet readiness by more than 30 points;
- feature has not been touched by current evidence in a configurable age window;
- feature refers to open PR work.

Warnings do not lower scores automatically in v1. They make stale/unsupported scoring visible.

## Scoring

The matrix stores the three human/evidence-reviewed scores separately:

- Implementation: 35% of composite.
- Testing: 25% of composite.
- Mainnet readiness: 40% of composite.

Composite is generated, not stored:

`round(implemented * 0.35 + tested * 0.25 + mainnet_ready * 0.40)`

Generated readiness classes:

- 80-100: `mainnet_candidate`
- 60-79: `guarded_near_ready`
- 40-59: `partial`
- 20-39: `experimental`
- 0-19: `claim_or_research`

These labels are descriptive only. They do not override release gates.

## GitHub / PR integration

Version 1 keeps GitHub state deterministic and offline: open PRs are declared as `open_prs = [166]` and treated as not merged until the matrix entry is updated in the same PR that lands the feature.

A later optional online mode may query GitHub to verify PR status, but main CI correctness must not depend on external API availability.

This design avoids creating hidden network-dependent readiness logic.

## XLSX generation

The repository-native source is TOML and generated JSON/Markdown. The spreadsheet remains an output artifact for planning.

The CI artifact should preserve these columns:

- Feature
- Subsystem
- File / Path
- Master / Open PR
- Implemented %
- Tested %
- Mainnet-Ready %
- Composite %
- Readiness Class
- Blocker
- Evidence
- Confidence
- Priority

If XLSX generation would require a large dependency, v1 emits CSV plus JSON/Markdown and a later small tooling job can convert it. The gate itself must not depend on spreadsheet libraries.

## Integration with existing readiness tooling

`scripts/check-readiness-consistency.sh` continues to validate coarse registry/status-document consistency.

The production gate should eventually run both:

```bash
bash scripts/check-readiness-consistency.sh
python3 scripts/feature_matrix.py check
```

The new matrix checker must not duplicate every rule in the shell checker. Its responsibility is granular engineering evidence and score integrity.

## No-slop policy

The feature matrix is explicitly allowed to contain negative/claim-risk entries such as:

- "MEV-proof marketing claim"
- "million-TPS GPU claim"
- "cross-chain complete claim"

These entries exist to stop unsupported claims from being confused with implemented features.

Generated reports must use factual labels and must not translate `partial` into marketing language such as "production ready" or "complete."

## Initial migration

Seed the matrix from the current 145-feature audit with conservative scores. During migration:

1. map each row to real repo paths;
2. map coarse features to `FEATURE_REGISTRY.toml` where applicable;
3. mark open-PR features explicitly;
4. reject rows that are only vague concepts unless they are marked research/claim-risk;
5. validate every test/evidence citation before enabling the CI gate as required.

The first PR should be reviewable and may start the new workflow as non-required at GitHub branch-protection level, but the workflow itself remains hard-fail. Promotion to a required branch check happens only after the seeded matrix is clean on exact head.

## Testing strategy

Add unit tests for the Python checker covering:

- duplicate IDs;
- missing paths;
- nonexistent required tests;
- invalid score ranges;
- P0 + >=80 mainnet conflict;
- open PR + >=80 mainnet conflict;
- claim-risk inflation;
- research inflation;
- valid feature passes;
- coarse registry key missing;
- generation is deterministic.

Add a repository integration test that runs against the real `FEATURE_MATRIX.toml` and `FEATURE_REGISTRY.toml`.

## Rollout

1. Add schema + checker tests in RED state.
2. Implement checker until tests pass.
3. Seed the initial feature matrix.
4. Generate Markdown/JSON/CSV outputs.
5. Add CI workflow.
6. Wire matrix check into the existing production/readiness gate only after the standalone workflow passes on exact head.
7. Open a draft PR and require normal exact-head review before merge.

## Non-goals for v1

- Automatically changing readiness scores based on code churn.
- Automatically merging PRs.
- Treating open PR content as master capability.
- Scraping GitHub during the mandatory CI gate.
- Replacing `FEATURE_REGISTRY.toml`.
- Claiming external certification or audit status.

## Success criteria

The system is successful when:

- every canonical granular feature has a stable machine-readable entry;
- nonexistent paths/tests/evidence fail CI;
- unsupported readiness inflation fails CI;
- generated reports are deterministic;
- coarse product readiness and granular engineering readiness are displayed together without contradiction;
- a developer can answer "where is this feature, how tested is it, what blocks mainnet, and what evidence proves it?" from one generated report;
- no binary spreadsheet is required for CI correctness.
