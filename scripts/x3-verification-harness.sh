#!/usr/bin/env bash
# X3 Verification Harness — Kani | Miri | Shuttle | sanitizers | cargo-mutants
#
# Exit contract (the reason this script is shaped the way it is):
#   0  every section this box could run passed, and none were skipped
#   1  at least one section ran and failed
#   2  BLOCKED: at least one section could not run (tool or target missing)
#
# The previous version ended each section with `|| echo "FAILED"`, so the
# script exited 0 even when every tool failed. A harness that cannot fail is
# worse than no harness: the report it leaves behind reads "complete" while
# nothing was verified. Missing tools are now loud (BLOCKED), failures are
# fatal (1), and a failing section no longer hides the sections after it.
#
# Knobs:
#   X3_VERIFY_ONLY=kani,miri   run only those sections (unknown names refused)
#   --list                     print the section names
#   --self-test                exercise the classification/exit logic and exit
#
# Reports land in proof/verification-reports/ (one file per tool+target).
set -uo pipefail   # deliberately not -e: one failure must not hide the rest

WORKSPACE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="$WORKSPACE/proof/verification-reports"

CARGO="$HOME/.cargo/bin/cargo"
NIGHTLY="$CARGO +nightly-2026-05-01"

SECTIONS=(kani miri shuttle sanitizers mutants)

PASSED=()
FAILED=()
MISSING=()

run_one() { # label available(0|1) report_file cmd...
    local label="$1" available="$2" report="$3"
    shift 3
    if [ "$available" != 1 ]; then
        echo "SKIP  $label — tool or target not available"
        MISSING+=("$label")
        return 0
    fi
    if "$@" >"$report" 2>&1; then
        echo "PASS  $label"
        PASSED+=("$label")
    else
        local status=$?
        echo "FAIL  $label (exit $status) — $report"
        tail -n 5 "$report" | sed 's/^/      /'
        FAILED+=("$label")
    fi
}

status_exit() {
    if [ "${#FAILED[@]}" -gt 0 ]; then echo 1; return; fi
    if [ "${#MISSING[@]}" -gt 0 ]; then echo 2; return; fi
    echo 0
}

section_enabled() {
    [ -z "${X3_VERIFY_ONLY:-}" ] && return 0
    local want
    IFS=',' read -r -a wants <<<"$X3_VERIFY_ONLY"
    for want in "${wants[@]}"; do [ "$want" = "$1" ] && return 0; done
    return 1
}

validate_only() {
    [ -z "${X3_VERIFY_ONLY:-}" ] && return 0
    local want known
    IFS=',' read -r -a wants <<<"$X3_VERIFY_ONLY"
    for want in "${wants[@]}"; do
        known=0
        for name in "${SECTIONS[@]}"; do [ "$want" = "$name" ] && known=1; done
        if [ "$known" != 1 ]; then
            echo "x3-verification-harness: X3_VERIFY_ONLY names unknown section '$want'" >&2
            echo "known sections: ${SECTIONS[*]}" >&2
            exit 2
        fi
    done
}

# ── sections ────────────────────────────────────────────────────────────────

run_kani() {
    local crates=(crates/x3-atomic-swap crates/x3-fees crates/x3-flash-finality)
    local available=0
    command -v kani >/dev/null 2>&1 && available=1
    local crate name
    for crate in "${crates[@]}"; do
        name="$(basename "$crate")"
        if [ ! -f "$WORKSPACE/$crate/Cargo.toml" ]; then
            run_one "kani $name (manifest absent)" 0 "$REPORT_DIR/kani-$name.txt"
        else
            run_one "kani $name" "$available" "$REPORT_DIR/kani-$name.txt" \
                kani --output-format terse "$crate"
        fi
    done
}

run_miri() {
    local crates=(crates/x3-accel crates/x3-vm crates/x3-backend crates/x3-proof crates/x3-gpu-validator-swarm)
    local available=0
    $NIGHTLY miri --version >/dev/null 2>&1 && available=1
    local crate name
    for crate in "${crates[@]}"; do
        name="$(basename "$crate")"
        if [ ! -f "$WORKSPACE/$crate/Cargo.toml" ]; then
            run_one "miri $name (manifest absent)" 0 "$REPORT_DIR/miri-$name.txt"
        else
            run_one "miri $name" "$available" "$REPORT_DIR/miri-$name.txt" \
                $NIGHTLY miri test --package "$name"
        fi
    done
}

run_shuttle() {
    local crates=(crates/x3-rpc crates/x3-relayer crates/northern-swarm crates/x3-gateway)
    local available=0
    $NIGHTLY shuttle --version >/dev/null 2>&1 && available=1
    local crate name
    for crate in "${crates[@]}"; do
        name="$(basename "$crate")"
        if [ ! -f "$WORKSPACE/$crate/Cargo.toml" ]; then
            run_one "shuttle $name (manifest absent)" 0 "$REPORT_DIR/shuttle-$name.txt"
        else
            run_one "shuttle $name" "$available" "$REPORT_DIR/shuttle-$name.txt" \
                $NIGHTLY shuttle test --package "$name"
        fi
    done
}

run_sanitizers() {
    local crates=(crates/x3-atomic-swap crates/x3-gateway crates/northern-swarm crates/x3-bitcoin-vault)
    local available=0
    $NIGHTLY --version >/dev/null 2>&1 && available=1
    local crate name
    for crate in "${crates[@]}"; do
        name="$(basename "$crate")"
        if [ ! -f "$WORKSPACE/$crate/Cargo.toml" ]; then
            run_one "asan $name (manifest absent)" 0 "$REPORT_DIR/asan-$name.txt"
        else
            run_one "asan $name" "$available" "$REPORT_DIR/asan-$name.txt" \
                env RUSTFLAGS=-Zsanitizer=address $NIGHTLY test --package "$name" --target x86_64-unknown-linux-gnu
        fi
    done
}

run_mutants() {
    local crates=(crates/x3-common crates/x3-fees crates/x3-packet-schema)
    local available=0
    $CARGO mutants --version >/dev/null 2>&1 && available=1
    local crate name
    for crate in "${crates[@]}"; do
        name="$(basename "$crate")"
        if [ ! -f "$WORKSPACE/$crate/Cargo.toml" ]; then
            run_one "mutants $name (manifest absent)" 0 "$REPORT_DIR/mutants-$name.txt"
        else
            run_one "mutants $name" "$available" "$REPORT_DIR/mutants-$name.txt" \
                $CARGO mutants --package "$name" --timeout 30
        fi
    done
}

# ── self-test ───────────────────────────────────────────────────────────────

self_test() {
    local tmp failures=0
    tmp="$(mktemp -d)"

    check() { # description expected actual
        if [ "$2" != "$3" ]; then
            echo "SELF-TEST FAIL: $1 (expected '$2', got '$3')"
            failures=1
        else
            echo "SELF-TEST PASS: $1"
        fi
    }

    PASSED=(); FAILED=(); MISSING=()
    run_one "synthetic pass A" 1 "$tmp/a" true
    run_one "synthetic pass B" 1 "$tmp/b" true
    check "all pass -> 0" 0 "$(status_exit)"

    PASSED=(); FAILED=(); MISSING=()
    run_one "synthetic pass" 1 "$tmp/c" true
    run_one "synthetic fail" 1 "$tmp/d" false
    run_one "synthetic pass after" 1 "$tmp/e" true
    check "one fail -> 1" 1 "$(status_exit)"
    check "a failure does not stop later sections" 2 "${#PASSED[@]}"

    PASSED=(); FAILED=(); MISSING=()
    run_one "synthetic missing" 0 "$tmp/f" true
    check "missing -> 2 (BLOCKED, loud)" 2 "$(status_exit)"

    PASSED=(); FAILED=(); MISSING=()
    run_one "synthetic missing" 0 "$tmp/g" true
    run_one "synthetic fail" 1 "$tmp/h" false
    check "fail wins over missing" 1 "$(status_exit)"

    python3 -c "import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)" "$tmp"
    exit "$failures"
}

# ── main ────────────────────────────────────────────────────────────────────

case "${1:-}" in
    --self-test) self_test ;;
    --list) printf '%s\n' "${SECTIONS[@]}"; exit 0 ;;
    "") ;;
    *) echo "x3-verification-harness: unknown argument '$1'" >&2; exit 2 ;;
esac

validate_only

echo "=== X3 Verification Harness — $(date -Iseconds) ==="
echo "workspace: $WORKSPACE"
echo "sections:  ${X3_VERIFY_ONLY:-${SECTIONS[*]}}"
echo

mkdir -p "$REPORT_DIR"
cd "$WORKSPACE"

section_enabled kani && run_kani
section_enabled miri && run_miri
section_enabled shuttle && run_shuttle
section_enabled sanitizers && run_sanitizers
section_enabled mutants && run_mutants

echo
echo "=== summary: ${#PASSED[@]} passed, ${#FAILED[@]} failed, ${#MISSING[@]} not runnable ==="
for label in "${FAILED[@]}"; do echo "FAILED: $label"; done
for label in "${MISSING[@]}"; do echo "NOT RUN: $label"; done

exit "$(status_exit)"
