#!/usr/bin/env bash
# validate-fuzz-detection.sh — prove the fuzzing toolchain DETECTS a defect.
#
# Integration is not "the binary runs". Per external-tool-gap-analysis.md §69
# and §90, a testing tool must distinguish a known-good fixture from a
# deliberately-broken one. This script runs libFuzzer against both:
#
#   known-good : benign seeds           -> expect exit 0, no crash artifact
#   known-bad  : a seed with the defect -> expect non-zero exit + artifact
#
# Exits non-zero if either expectation is violated, so it can gate CI.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FUZZ_DIR="$REPO_ROOT/tools/tool-validation/fuzz"
# cargo-fuzz requires the crate to live at <project>/fuzz and be invoked from
# <project>, so the working directory is the validation project root.
PROJ_DIR="$(dirname "$FUZZ_DIR")"
SEEDS="$REPO_ROOT/tools/tool-validation/seeds"
EVIDENCE_DIR="$REPO_ROOT/audit-artifacts/external-toolchain/cargo-fuzz"
TARGET=x86_64-unknown-linux-gnu
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

export RUSTUP_TOOLCHAIN=nightly
export OPENSSL_NO_VENDOR=1
export OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}"
export OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}"

if ! command -v cargo-fuzz >/dev/null 2>&1; then
    echo "error: cargo-fuzz is not installed (cargo install cargo-fuzz)" >&2
    exit 1
fi

mkdir -p "$EVIDENCE_DIR"
cd "$PROJ_DIR"

echo "== build =="
cargo fuzz build --target "$TARGET" crash_if_magic || { echo "BUILD FAILED"; exit 1; }

run_case() { # <label> <seed-dir> <expect-crash:yes|no> <logfile>
    local label="$1" seed_dir="$2" expect="$3" log="$4"
    local max_len="$5"
    # cargo-fuzz keeps corpus and crash artifacts under <project>/fuzz/.
    rm -rf "$FUZZ_DIR/corpus/crash_if_magic" "$FUZZ_DIR/artifacts/crash_if_magic"
    mkdir -p "$FUZZ_DIR/corpus/crash_if_magic"
    cp "$seed_dir"/* "$FUZZ_DIR/corpus/crash_if_magic/"

    cargo fuzz run --target "$TARGET" crash_if_magic -- \
        -runs=2000 -seed=20261001 -max_len="$max_len" -print_final_stats=1 >"$log" 2>&1
    local rc=$?

    local n_artifacts=0
    [ -d "$FUZZ_DIR/artifacts/crash_if_magic" ] && \
        n_artifacts=$(find "$FUZZ_DIR/artifacts/crash_if_magic" -type f | wc -l)

    if [ "$expect" = "yes" ]; then
        if [ "$rc" -ne 0 ] && [ "$n_artifacts" -gt 0 ]; then
            echo "PASS  $label: exit=$rc artifacts=$n_artifacts  (defect detected)"
            echo "$label detected: rc=$rc artifacts=$n_artifacts"
            return 0
        fi
        echo "FAIL  $label: expected a crash + artifact, got rc=$rc artifacts=$n_artifacts"
        return 1
    else
        if [ "$rc" -eq 0 ] && [ "$n_artifacts" -eq 0 ]; then
            echo "PASS  $label: exit=0 artifacts=0  (no false positive)"
            return 0
        fi
        echo "FAIL  $label: expected clean run, got rc=$rc artifacts=$n_artifacts"
        return 1
    fi
}

echo "== known-good =="
# The defect needs a 14-byte magic; -max_len=13 makes it unreachable, so a
# clean run here is a real negative control, not a lucky seed.
run_case "known-good" "$SEEDS/good" no  "$EVIDENCE_DIR/validation-known-good-$STAMP.log" 13
good_rc=$?

echo "== known-bad =="
run_case "known-bad"  "$SEEDS/bad"  yes "$EVIDENCE_DIR/validation-known-bad-$STAMP.log" 64
bad_rc=$?

echo
if [ "$good_rc" -eq 0 ] && [ "$bad_rc" -eq 0 ]; then
    echo "RESULT: PASS — libFuzzer distinguishes known-good from known-bad."
    exit 0
fi
echo "RESULT: FAIL — tool did not discriminate."
exit 1
