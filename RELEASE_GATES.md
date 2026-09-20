# Release Gates

**Canonical source: `FEATURE_REGISTRY.toml`** — all readiness scores and blockers derive from it. Run `scripts/check-readiness-consistency.sh` to validate.

**Overall readiness: ~54%** (53.74% arithmetic mean across the 19 currently scored entries in `FEATURE_REGISTRY.toml`, recalculated 2026-09-20). A mainnet-ready claim is forbidden unless every feature scores ≥95%.

## Gate commands

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — mainnet release gate
- `make fresh-machine-check` — bootstrap validation on fresh machine

## Mainnet release gate (`make mainnet-check` → `scripts/mainnet_release_gate.py`)

Exit 0 = PASS. Exit 1 = FAIL — do NOT cut a release.

Validates: documentation existence, build, chain-spec, critical test suites, reproducible builds, secret hygiene.

## CI enforcement

Enforced in `.github/workflows/mainnet-readiness.yml` on every push/PR to main.

## Mainnet-ready claims

Forbidden unless all gates pass AND `FEATURE_REGISTRY.toml` scores ≥95% for every feature. Currently: ~54% registry-scored readiness; this is **not** a mainnet-ready score.