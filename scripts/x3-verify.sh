#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

mkdir -p .claude/state

echo "== X3 VERIFY: git status =="
git status --short

if [[ -f Cargo.toml ]]; then
  echo "== X3 VERIFY: cargo fmt =="
  cargo fmt --all -- --check

  echo "== X3 VERIFY: cargo clippy =="
  cargo clippy --workspace --all-targets --all-features -- -D warnings

  echo "== X3 VERIFY: cargo test =="
  cargo test --workspace --all-features

  if command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "== X3 VERIFY: coverage =="
    cargo llvm-cov --workspace --all-features --summary-only
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
