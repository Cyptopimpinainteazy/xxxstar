# X3 Funding OS Phase 1 Plan — Self-Review Corrections

This file is a mandatory companion to `2026-09-03-x3-funding-os-phase-1-funding-brain.md` and records corrections found during the required plan self-review.

## 1. Funding Score arithmetic correction

In Task 5, Step 1, the expected weighted total is **73.5**, not 73.0.

Use:

```rust
assert!((score.total_score - 73.5).abs() < f64::EPSILON);
```

The correct arithmetic is:

```text
100*.20 + 80*.15 + 60*.15 + 90*.15 + 70*.10 + 50*.10 + 40*.05 + 20*.05 + 80*.05 = 73.5
```

## 2. Source-file configuration consistency

Task 1 configuration must also include the source registry path used in Task 8.

Add to `Config`:

```rust
pub sources_file: std::path::PathBuf,
```

Load it with no production fallback:

```rust
let sources_file = std::env::var("FUNDING_SOURCES_FILE")
    .map(std::path::PathBuf::from)
    .map_err(|_| ConfigError::Missing("FUNDING_SOURCES_FILE"))?;
```

and include `sources_file` in the returned `Config`.

## 3. Phase boundary confirmation

The plan intentionally leaves `evidence_readiness` and `contact_quality` at explicit zero values in Phase 1 because Repo Intelligence and Contact Intel do not exist yet. Those are not placeholders: they are versioned scoring semantics for `funding-score-v1` and must be replaced by evidence-backed values in later formula versions when the corresponding subsystems are implemented.

No other spec-coverage gaps were found in the Phase 1 scope. Later spec requirements are explicitly assigned to Phases 2-6 rather than being silently omitted.
