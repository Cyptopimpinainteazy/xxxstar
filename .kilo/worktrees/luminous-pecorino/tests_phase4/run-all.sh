#!/usr/bin/env bash
set -euo pipefail

echo "→ Running Rust workspace tests (fast)"
cargo test --workspace --lib --tests -- --test-threads=1

echo "→ Running Python swarm unit tests (if venv active)"
if command -v pytest >/dev/null 2>&1; then
  pytest -q || true
else
  echo "pytest not found; skipping Python tests"
fi

echo "→ Completed. See previous output for failures."
