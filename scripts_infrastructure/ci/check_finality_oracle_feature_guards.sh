#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

mkdir -p target

invalid_log="target/finality-oracle-invalid-feature-check.log"

# This combo must always fail: it intentionally mixes test-only and production verifier modes.
if cargo check -p x3-finality-oracle --features "test-verifier,production" >"$invalid_log" 2>&1; then
  echo "::error::x3-finality-oracle accepted invalid feature combo: test-verifier + production"
  echo "::error::Expected this combo to fail at compile time."
  exit 1
fi

if ! grep -q 'feature `test-verifier` must never be enabled with `production`' "$invalid_log"; then
  echo "::error::x3-finality-oracle failed for an unexpected reason while validating invalid feature combo"
  echo "--- begin finality-oracle invalid combo log ---"
  sed -n '1,120p' "$invalid_log" || true
  echo "--- end finality-oracle invalid combo log ---"
  exit 1
fi

echo "✓ x3-finality-oracle guard active: test-verifier + production is rejected"

# Sanity-check valid modes still compile.
cargo check -p x3-finality-oracle --features test-verifier
cargo check -p x3-finality-oracle --features production

echo "✓ x3-finality-oracle valid feature modes compile"
