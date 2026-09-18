# Generated feature-matrix reports

The `feature-matrix` GitHub Actions workflow generates the following files for each exact commit SHA and uploads them as a workflow artifact:

- `X3_FEATURE_MATRIX.json`
- `X3_FEATURE_MATRIX.csv`
- `X3_FEATURE_MATRIX.md`
- `X3_FEATURE_SUMMARY.md`

They are generated outputs. `FEATURE_MATRIX.toml` plus `feature-matrix/*.toml` remain the versioned source. The gate itself does not depend on spreadsheet libraries or binary XLSX files.
