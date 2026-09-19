#!/usr/bin/env bash
# Run the loom concurrency model checks.
#
# `tests/loom-concurrency` is `#![cfg(loom)]` end to end, and loom needs a
# nightly toolchain (it depends on `generator`). It is therefore excluded from
# the workspace and run from its own directory, the way the crate header
# documents:
#
#   RUSTFLAGS="--cfg loom" cargo +nightly-2026-05-01 test --package loom-concurrency
#
# The toolchain is pinned here rather than in the crate so the whole repository
# agrees on one nightly. Override with X3_LOOM_TOOLCHAIN when a different one is
# installed; a box without the pinned nightly fails with rustup's own
# "toolchain ... is not installed", which the local gate reports as BLOCKED
# (nothing was verified) rather than PASS.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="$ROOT/tests/loom-concurrency"
TOOLCHAIN="${X3_LOOM_TOOLCHAIN:-nightly-2026-05-01}"

if [ ! -d "$CRATE_DIR" ]; then
  echo "run-loom-tests: $CRATE_DIR does not exist" >&2
  exit 2
fi

echo "run-loom-tests: crate=$CRATE_DIR toolchain=$TOOLCHAIN"

# Fail *before* cargo does, so the reason is unambiguous in the log.
if command -v rustup >/dev/null 2>&1; then
  if ! rustup toolchain list 2>/dev/null | grep -q "^${TOOLCHAIN}"; then
    echo "error: toolchain '${TOOLCHAIN}' is not installed" >&2
    echo "install it with: rustup toolchain install ${TOOLCHAIN}" >&2
    echo "loom needs a nightly: it depends on the \`generator\` crate." >&2
    exit 1
  fi
fi

# `cargo +toolchain` only works when `cargo` on PATH is the rustup shim. The
# local CI puts the *toolchain* directory first (scripts/local-ci.sh does this
# on purpose, so a dangling shim cannot break every gate), which makes
# `cargo +nightly-...` fail with "no such command: `+nightly-...`". `rustup run`
# works either way.
if command -v rustup >/dev/null 2>&1; then
  CARGO=(rustup run "$TOOLCHAIN" cargo)
else
  CARGO=(cargo "+$TOOLCHAIN")
fi

cd "$CRATE_DIR"
RUSTFLAGS="${RUSTFLAGS:-} --cfg loom" "${CARGO[@]}" test --release "$@"
