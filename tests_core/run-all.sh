#!/usr/bin/env bash
set -euo pipefail

# Avoid known rustc incremental cache instability on large multi-crate test runs.
export CARGO_INCREMENTAL=0

echo "→ Running Rust workspace tests (fast)"
cargo test --workspace --lib --tests -- --test-threads=1

echo "→ Running Python swarm unit tests (if venv active)"
if command -v pytest >/dev/null 2>&1; then
  pytest -q
else
  echo "pytest not found; skipping Python tests"
fi

echo "→ Completed. See previous output for failures."
