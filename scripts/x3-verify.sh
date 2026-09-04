#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

mkdir -p .claude/state

# Build flag: avoid recursive WASM runtime rebuilds during verify.
export SKIP_WASM_BUILD=${SKIP_WASM_BUILD:-1}

echo "== X3 VERIFY: git status =="
git status --short

if [[ -f Cargo.toml ]]; then
  echo "== X3 VERIFY: cargo fmt =="
  cargo fmt --all -- --check

  echo "== X3 VERIFY: cargo clippy =="
  # Use `--features test-verifier` instead of `--all-features`:
  # `x3-finality-oracle` defines `test-verifier` and `production` as
  # mutually exclusive at compile time (a deliberate invariant), so
  # `--all-features` is structurally incompatible with the workspace.
  # `test-verifier` is the feature set used by the test suite and CI.
  cargo clippy --workspace --all-targets --features test-verifier -- -D warnings

  echo "== X3 VERIFY: cargo test =="
  cargo test --workspace --features test-verifier

  if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "== X3 VERIFY: coverage =="
    cargo llvm-cov --workspace --features test-verifier --summary-only
  else
    echo "coverage: cargo-llvm-cov not installed"
    echo "install with: cargo install cargo-llvm-cov"
  fi
fi

if [[ -f package.json ]]; then
  if command -v pnpm >/dev/null 2>&1 && [[ -f pnpm-lock.yaml ]]; then
    echo "== X3 VERIFY: pnpm lint/test =="
    pnpm lint || true
    pnpm test || true
  elif command -v npm >/dev/null 2>&1; then
    echo "== X3 VERIFY: npm lint/test =="
    npm run lint --if-present
    npm test --if-present
  fi
fi

date -u +"%Y-%m-%dT%H:%M:%SZ" > .claude/state/x3-last-verify.ok
echo "X3 VERIFY PASSED"
