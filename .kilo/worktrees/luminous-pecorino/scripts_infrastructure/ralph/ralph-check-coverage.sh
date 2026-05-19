#!/bin/bash
# Validate tests/lint for the codebase after Ralph run.
set -euo pipefail

echo "[ralph-check-coverage] Running cargo fmt check..."
cargo fmt --all -- --check

echo "[ralph-check-coverage] Running workspace tests..."
cargo test --workspace --all-features

echo "[ralph-check-coverage] Running clippy..."
cargo clippy --workspace --all-targets --all-features -- -D warnings

if command -v cargo-tarpaulin >/dev/null 2>&1; then
  echo "[ralph-check-coverage] Running code coverage check..."
  cargo tarpaulin --workspace --all-features --fail-under 70
else
  echo "[ralph-check-coverage] cargo-tarpaulin not installed; skipping coverage check."
fi

if command -v terraform >/dev/null 2>&1 && [ -f "main.tf" ]; then
  echo "[ralph-check-coverage] Running terraform validate..."
  terraform validate
else
  echo "[ralph-check-coverage] terraform not installed or no main.tf found; skipping terraform check."
fi

if command -v cargo-audit >/dev/null 2>&1; then
  echo "[ralph-check-coverage] Running cargo audit..."
  cargo audit
else
  echo "[ralph-check-coverage] cargo audit not installed; skipping audit check."
fi

echo "[ralph-check-coverage] Success: test/lint audit steps passed."
