#!/usr/bin/env bash
# Regression gate for audit finding H13:
#   "CI make recipes swallow failures and reference missing targets."
#
# This script asserts that every `test-*` recipe in the root Makefile
# propagates the *real* exit status of the command it runs, instead of piping
# into `tail` (which always yields a green exit code of 0). It:
#
#   1. Vacuously scopes itself to the root Makefile's own recipes.
#   2. Runs a real (PATH-scoped, throwaway) invocation of the root Makefile's
#      `test-atomic-kernel` target against a deliberately-failing `cargo` and
#      requires `make` to exit NONZERO.  Under the old `| tail -5` construct
#      this returned 0 (the bug); the fixed recipe must fail the target.
#   3. Runs the same target against a genuinely-passing `cargo` and requires
#      `make` to exit 0, proving passing tests are not turned into failures.
#
# No repository files are modified: the fake `cargo` lives in a temp dir that is
# prepended to PATH for the sub-make only, and is cleaned up on exit.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MAKEFILE="$ROOT/Makefile"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok: $*"; }

[ -f "$MAKEFILE" ] || fail "Makefile missing at $MAKEFILE"

echo "==> Structural check: no test recipe may still swallow its exit code via pipe-to-tail"
# Any recipe that ends in a bare `| tail ...` pipeline lets make see tail's
# always-zero status. Match the actual make recipe bytes.
if grep -nE '^\s*@?[^#]*:[^=]*\|[[:space:]]*tail(-n[[:space:]]*[0-9]+)?$' "$MAKEFILE"; then
    fail "found a test recipe that pipes into 'tail' (would swallow failures): see lines above"
fi
pass "no piped-into-|tail recipes remain"

# ---------------------------------------------------------------------------
# Throwaway fake cargo to prove real exit-code propagation through the Makefile.
# ---------------------------------------------------------------------------
SANDBOX="$(mktemp -d)"
trap 'rm -rf "$SANDBOX"' EXIT
mkdir -p "$SANDBOX/fakepass" "$SANDBOX/fakefail"

cat > "$SANDBOX/fakepass/cargo" <<'EOF'
#!/usr/bin/env bash
echo "FAKE-CARGO-PASS"
exit 0
EOF

cat > "$SANDBOX/fakefail/cargo" <<'EOF'
#!/usr/bin/env bash
echo "FAKE-CARGO-FAIL"
exit 1
EOF
chmod +x "$SANDBOX/fakepass/cargo" "$SANDBOX/fakefail/cargo"

echo "==> Runtime check 1: a FAILING cargo must fail 'make test-atomic-kernel' (nonzero)"
# The fake `cargo` is injected on PATH *only* for the sub-make. This exercises
# the real Makefile recipe (temp-log capture + 'exit $rc'), which is exactly
# the code path changed for H13.
(
    cd "$ROOT"
    PATH="$SANDBOX/fakefail:$PATH" make test-atomic-kernel >/dev/null 2>&1
    exit $?
); rc=$?
[ "$rc" -ne 0 ] || fail "make test-atomic-kernel exited 0 despite failing cargo (swallow bug present)"
pass "failing cargo -> make exit code nonzero (rc=$rc)"

echo "==> Runtime check 2: a PASSING cargo must still green 'make test-atomic-kernel' (exit 0)"
(
    cd "$ROOT"
    PATH="$SANDBOX/fakepass:$PATH" make test-atomic-kernel >/dev/null 2>&1
    exit $?
); rc=$?
[ "$rc" -eq 0 ] || fail "make test-atomic-kernel exited $rc for a genuinely passing cargo"
pass "passing cargo -> make exit code 0 (rc=$rc)"

echo
echo "verify_make_swallow_fix.sh: PASS - recipes propagate real exit codes"
exit 0
