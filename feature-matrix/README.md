# X3 Live Feature Matrix

`FEATURE_MATRIX.toml` is the canonical granular engineering-readiness manifest. It includes subsystem fragments in this directory so changes remain reviewable.

`FEATURE_REGISTRY.toml` remains the coarse product/readiness authority. The matrix does not override release gates or automatically raise scores.

## Commands

```bash
python3 scripts/feature_matrix.py check
python3 scripts/feature_matrix.py generate
python3 scripts/feature_matrix.py generate --check-clean
python3 scripts/feature_matrix.py summary
```

The checker fails on nonexistent code/evidence paths, fictional required tests, invalid scores, missing coarse-registry mappings, unsupported high-readiness claims, and open-PR/research/claim-risk inflation.

The generator emits JSON, CSV, Markdown, and subsystem/P0 summaries under `reports/feature-matrix/`. CI generates and uploads those reports for the exact commit SHA.

## Score changes

Scores are reviewed evidence, not code-coverage percentages. New code may justify a score review, but the tooling never raises a score automatically. Missing evidence is a reason to fail or lower a score; the presence of code alone is not a reason to raise one.
