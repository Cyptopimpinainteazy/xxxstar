#!/usr/bin/env bash
# fuzz-gate.sh — the cargo-fuzz gate for the local CI of record.
#
# Wired into scripts/local-ci.sh as the opt-in `--fuzz` set (also in `--all`).
# Opt-in because it needs a nightly toolchain and compiles with ASAN, which the
# default fast run does not do (same reasoning as the `--loom` set).
#
# It proves two things, both bounded:
#   1. the X3-language fuzz targets build and execute (compile_and_run, x3bc_engines)
#   2. the toolchain actually DETECTS a defect (known-good vs known-bad fixture)
#
# A missing nightly or a missing cargo-fuzz is reported the way local-ci treats a
# missing toolchain: an error naming `... is not installed`, which local-ci
# classifies as BLOCKED (nothing was verified) rather than PASS.
#
# Environment:
#   X3_FUZZ_RUNS     runs per target (default 4000)
#   X3_FUZZ_TOOLCHAIN  the nightly channel (default: nightly)

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOLCHAIN="${X3_FUZZ_TOOLCHAIN:-nightly}"
RUNS="${X3_FUZZ_RUNS:-4000}"
TARGET_TRIPLE=x86_64-unknown-linux-gnu

echo "fuzz-gate: toolchain=$TOOLCHAIN runs=$RUNS"

# Local CI puts a *pinned* toolchain directory first on PATH and exports RUSTC to
# that compiler. cargo-fuzz needs nightly for -Zsanitizer, so RUSTC/CARGO are
# re-pointed at the nightly toolchain by absolute path here; inheriting the
# pinned RUSTC would make every fuzz build die with
#   error: the option `Z` is only accepted on the nightly compiler
if ! command -v rustup >/dev/null 2>&1; then
    echo "error: rustup is not installed; cannot select the $TOOLCHAIN toolchain" >&2
    exit 1
fi
if ! rustup toolchain list 2>/dev/null | grep -q "^${TOOLCHAIN}"; then
    echo "error: toolchain '${TOOLCHAIN}' is not installed" >&2
    echo "install it with: rustup toolchain install ${TOOLCHAIN}" >&2
    echo "cargo-fuzz needs a nightly: -Zsanitizer is unstable." >&2
    exit 1
fi
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export RUSTC="$(rustup which --toolchain "$TOOLCHAIN" rustc)"
export CARGO="$(rustup which --toolchain "$TOOLCHAIN" cargo)"

# Nightly + ASAN artifacts must not land in the shared `target/` that the pinned
# stable toolchain writes: mixing them makes the next stable gate die with
# "found crate `x` compiled by an incompatible version of rustc", which reads
# like a defect in the change under test. Give the fuzz build its own target dir.
export CARGO_TARGET_DIR="${X3_FUZZ_TARGET_DIR:-$ROOT/target-fuzz}"

if ! command -v cargo-fuzz >/dev/null 2>&1; then
    echo "error: cargo-fuzz is not installed" >&2
    echo "install it with: cargo install cargo-fuzz" >&2
    exit 1
fi

# cargo-fuzz defaults to the musl target, where ASAN cannot link a statically
# linked libc ("sanitizer is incompatible with statically linked libc"). Every
# X3 fuzz build must pass --target x86_64-unknown-linux-gnu.
if ! rustup target list --toolchain "$TOOLCHAIN" --installed 2>/dev/null | grep -q "$TARGET_TRIPLE"; then
    echo "error: target '$TARGET_TRIPLE' is not installed for $TOOLCHAIN" >&2
    echo "install it with: rustup target add --toolchain ${TOOLCHAIN} ${TARGET_TRIPLE}" >&2
    exit 1
fi

# pkg-config is absent on the lab host; point openssl-sys at the system OpenSSL
# so fuzz crates that pull OpenSSL can still build.
export OPENSSL_NO_VENDOR=1
export OPENSSL_LIB_DIR="${OPENSSL_LIB_DIR:-/usr/lib/x86_64-linux-gnu}"
export OPENSSL_INCLUDE_DIR="${OPENSSL_INCLUDE_DIR:-/usr/include}"

rc=0

echo
echo "== X3-language fuzz targets =="

# Build every target once, with bounded parallelism. A cold ASAN build of the
# X3-language graph (it pulls x3-compiler -> frame-support) is large; on a box
# already under memory pressure an over-parallel build gets SIGTERM'd by
# systemd-oomd and the gate would read as a code failure. Four jobs keeps peak
# RSS sane and the run targets then only execute.
if ! (cd "$ROOT/crates/x3-integration/fuzz" && CARGO_BUILD_JOBS="${X3_FUZZ_JOBS:-4}" cargo fuzz build --target "$TARGET_TRIPLE") >/tmp/fuzz-gate-build.log 2>&1; then
    if grep -qE "Terminated|Killed|signal: 9" /tmp/fuzz-gate-build.log; then
        echo "error: fuzz build was terminated (likely memory pressure); nothing verified" >&2
        tail -5 /tmp/fuzz-gate-build.log >&2
    else
        echo "error: fuzz build failed" >&2
        tail -15 /tmp/fuzz-gate-build.log >&2
    fi
    exit 1
fi
echo "PASS  fuzz targets built"

for t in compile_and_run x3bc_engines; do
    if X3_FUZZ_RUNS="$RUNS" bash "$ROOT/scripts/external-toolchain/fuzz-x3-language.sh" "$t" "$RUNS" 20261001 >/tmp/fuzz-gate-$t.log 2>&1; then
        echo "PASS  fuzz target $t"
    else
        echo "FAIL  fuzz target $t (see /tmp/fuzz-gate-$t.log)"
        tail -5 /tmp/fuzz-gate-$t.log
        rc=1
    fi
done

echo
echo "== defect-detection proof =="
if bash "$ROOT/scripts/external-toolchain/validate-fuzz-detection.sh" >/tmp/fuzz-gate-detection.log 2>&1; then
    echo "PASS  libFuzzer distinguishes known-good from known-bad"
else
    echo "FAIL  defect-detection proof (see /tmp/fuzz-gate-detection.log)"
    tail -8 /tmp/fuzz-gate-detection.log
    rc=1
fi

echo
if [ "$rc" -eq 0 ]; then
    echo "fuzz-gate: PASS"
else
    echo "fuzz-gate: FAIL"
fi
exit "$rc"
