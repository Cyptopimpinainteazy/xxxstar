#!/usr/bin/env bash
# fuzz-x3-language.sh — repeatable cargo-fuzz runner for the X3-language targets.
#
# Why this script exists (evidence: external-tool-gap-analysis.md §1.3):
#   * cargo-fuzz defaults to the musl target, where ASAN is incompatible with a
#     statically linked libc ("sanitizer is incompatible with statically linked
#     libc"). Every X3 fuzz build must pass --target x86_64-unknown-linux-gnu.
#   * cargo-fuzz needs nightly for -Zsanitizer; the repo default toolchain is
#     stable 1.90.0, so the toolchain is selected here rather than assumed.
#   * pkg-config is absent on the lab host, so openssl-sys is pointed at the
#     system OpenSSL explicitly.
#
# Usage:
#   scripts/external-toolchain/fuzz-x3-language.sh [target] [runs] [seed]
# Defaults: target=compile_and_run runs=3000 seed=20261001
#
# Evidence lands in audit-artifacts/external-toolchain/cargo-fuzz/.

set -euo pipefail

TARGET="${1:-compile_and_run}"
RUNS="${2:-3000}"
SEED="${3:-20261001}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FUZZ_DIR="$REPO_ROOT/crates/x3-integration/fuzz"
EVIDENCE_DIR="$REPO_ROOT/audit-artifacts/external-toolchain/cargo-fuzz"

case "$TARGET" in
    compile_and_run|x3bc_engines) ;;
    *) echo "unknown target '$TARGET' (expected compile_and_run or x3bc_engines)" >&2; exit 2 ;;
esac

if ! command -v cargo-fuzz >/dev/null 2>&1; then
    echo "error: cargo-fuzz is not installed (cargo install cargo-fuzz)" >&2
    exit 1
fi

if ! rustup toolchain list | grep -q '^nightly'; then
    echo "nightly toolchain required for -Zsanitizer: rustup toolchain install nightly" >&2
    exit 1
fi

mkdir -p "$EVIDENCE_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG="$EVIDENCE_DIR/run-${TARGET}-seed${SEED}-${STAMP}.log"

echo "target=$TARGET runs=$RUNS seed=$SEED"
echo "log=$LOG"

cd "$FUZZ_DIR"
RUSTUP_TOOLCHAIN=nightly \
OPENSSL_NO_VENDOR=1 \
OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}" \
OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}" \
cargo fuzz run --target x86_64-unknown-linux-gnu "$TARGET" -- \
    -runs="$RUNS" -seed="$SEED" -print_final_stats=1 2>&1 | tee "$LOG"

echo
echo "corpus:    $FUZZ_DIR/corpus/$TARGET"
echo "artifacts: $FUZZ_DIR/artifacts/$TARGET (empty = no crash)"
