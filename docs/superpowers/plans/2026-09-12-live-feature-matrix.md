# Live Feature Matrix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a repository-native 145-feature readiness matrix with fail-closed validation, deterministic reports, and a standalone GitHub Actions gate.

**Architecture:** `FEATURE_REGISTRY.toml` remains the coarse canonical product registry. `FEATURE_MATRIX.toml` adds granular engineering features; `scripts/feature_matrix.py` validates evidence and anti-inflation rules and generates JSON/CSV/Markdown reports. A standalone workflow runs the checker on exact PR heads without network/API dependencies.

**Tech Stack:** Python 3.11 standard library (`argparse`, `csv`, `json`, `pathlib`, `tomllib`), TOML source data, pytest, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-12-live-feature-matrix-design.md`

## Global Constraints

- `FEATURE_REGISTRY.toml` remains authoritative for coarse product readiness.
- Mandatory CI correctness must not depend on GitHub API/network state.
- Validation is hard-fail; no `|| true` or `continue-on-error` on matrix checks.
- The system may flag or lower confidence but must never silently raise readiness scores.
- Open-PR work is never treated as merged master capability.
- Research and claim-risk rows cannot claim production readiness.
- CI uses Python 3.11 standard library only; XLSX is optional artifact tooling and cannot be required for correctness.

---

### Task 1: Validator and anti-inflation rules

**Files:**
- Create: `scripts/feature_matrix.py`
- Create: `tests/test_feature_matrix.py`

**Interfaces:**
- Consumes: `FEATURE_MATRIX.toml`, `FEATURE_REGISTRY.toml`, repository filesystem.
- Produces: `load_matrix(path)`, `load_registry(path)`, `validate_feature(feature, repo_root, registry)`, `validate_matrix(matrix, repo_root, registry)`, `composite(feature)`, `readiness_class(score)`, CLI subcommands `validate`, `check`, `generate`, `summary`.

- [ ] Write tests for duplicate IDs, invalid scores, missing paths, missing required tests, missing registry keys, P0+80 conflict, open-PR+80 conflict, claim-risk inflation, research inflation, and a valid feature.
- [ ] Run `pytest -q tests/test_feature_matrix.py` and confirm RED because the implementation does not exist.
- [ ] Implement parsing and structural validation with `tomllib`.
- [ ] Implement path/test/evidence validation and anti-inflation rules.
- [ ] Re-run `pytest -q tests/test_feature_matrix.py` and require green.
- [ ] Commit validator + tests.

### Task 2: Canonical 145-feature seed

**Files:**
- Create: `FEATURE_MATRIX.toml`

**Interfaces:**
- Consumes: approved 145-feature audit, current `master` code paths, `FEATURE_REGISTRY.toml` sections.
- Produces: stable `[[feature]]` entries with IDs, scores, blockers, evidence, priority, confidence, launch scope, open PR metadata, and registry mappings where applicable.

- [ ] Seed 145 entries using conservative scores from the approved audit.
- [ ] Mark PR-only features with `source = "open_pr"` and `open_prs`.
- [ ] Mark unsupported marketing statements with `claim_risk = true`.
- [ ] Mark roadmap-only consensus/performance ideas `launch_scope = "research"`.
- [ ] Run `python3 scripts/feature_matrix.py check` against the real repository and fix every hard failure without deleting rules.
- [ ] Commit the clean seed.

### Task 3: Deterministic generated reports

**Files:**
- Modify: `scripts/feature_matrix.py`
- Create/generated: `reports/feature-matrix/X3_FEATURE_MATRIX.json`
- Create/generated: `reports/feature-matrix/X3_FEATURE_MATRIX.csv`
- Create/generated: `reports/feature-matrix/X3_FEATURE_MATRIX.md`
- Create/generated: `reports/feature-matrix/X3_FEATURE_SUMMARY.md`
- Modify: `tests/test_feature_matrix.py`

**Interfaces:**
- Produces: `generate_outputs(matrix, repo_root, output_dir)` and deterministic serialization ordered by feature ID.

- [ ] Add deterministic-generation test using a temporary repository fixture.
- [ ] Confirm RED for missing generation implementation.
- [ ] Implement JSON, CSV, Markdown matrix, and subsystem/P0 summary generation.
- [ ] Add `--check-clean` mode that regenerates in memory/temp and fails when tracked outputs differ.
- [ ] Run `python3 scripts/feature_matrix.py generate`.
- [ ] Run `python3 scripts/feature_matrix.py generate --check-clean` and pytest; require green.
- [ ] Commit generator + generated reports.

### Task 4: Standalone exact-head CI gate

**Files:**
- Create: `.github/workflows/feature-matrix.yml`

**Interfaces:**
- Runs `pytest -q tests/test_feature_matrix.py`, `python3 scripts/feature_matrix.py check`, and `python3 scripts/feature_matrix.py generate --check-clean` on PR/push/merge-group.

- [ ] Add checkout and Python 3.11 setup.
- [ ] Run tests and matrix check with no soft-fail behavior.
- [ ] Upload JSON/CSV/Markdown outputs as workflow artifacts.
- [ ] Do not wire into `production-gate.yml` yet; standalone gate must prove itself first.
- [ ] Commit workflow.

### Task 5: Exact-head verification and review PR

**Files:** none unless verification exposes a defect.

- [ ] Fetch the exact branch head and its workflow runs.
- [ ] Require validator tests and feature-matrix workflow to execute on that exact SHA.
- [ ] If a genuine failure appears, repair it without weakening rules and re-run on the new exact head.
- [ ] Open a draft PR to `master` with the generated summary and explicit note that production-gate integration is deferred until this standalone gate is green.
- [ ] Record final exact-head SHA and check state in the PR body/comment.
