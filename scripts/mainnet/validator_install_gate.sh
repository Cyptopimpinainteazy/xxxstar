#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# validator_install_gate.sh — prove the validator install path refuses the wrong
# things and accepts the right one
#
# `scripts/install-validator.sh` used to download
# `releases/download/latest/x3-chain-node` (a draft release with no assets), print
# "WARNING: No checksum file found. Skipping verification." and continue, and then
# install a systemd unit pointing at a chain spec it had failed to download. None
# of that could be seen from the repository, because nothing ran the script.
#
# This gate runs it in `--check` mode against a real, freshly generated Live
# genesis and a real node binary, and then against each way the inputs can be
# wrong. `--check` needs no root and writes nothing, so the whole path is
# testable on any box:
#
#   1. built binary + Live genesis            -> accepted, plan printed, nothing written
#   2. the same, but the genesis is a dev one -> refused (a mainnet validator
#                                                must not point at Development)
#   3. binary missing                          -> refused
#   4. declared --sha256 that does not match   -> refused
#   5. --from-release with no published release-> refused, with the build route
#   6. no source flag at all                   -> refused
#   7. no --chain                              -> refused
#
# Usage: bash scripts/mainnet/validator_install_gate.sh
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
WORK="$(mktemp -d)"

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

info() { printf '[install-gate] %s\n' "$*"; }
fail() {
  printf '[install-gate] FAIL: %s\n' "$*" >&2
  exit 1
}

NODE_BIN="${X3_NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  for candidate in \
    "$TARGET_DIR/release/x3-chain-node" \
    "$TARGET_DIR/debug/x3-chain-node" \
    "$ROOT/target/release/x3-chain-node" \
    "$ROOT/target/debug/x3-chain-node"; do
    if [ -x "$candidate" ]; then NODE_BIN="$candidate"; break; fi
  done
fi
[ -n "$NODE_BIN" ] && [ -x "$NODE_BIN" ] \
  || fail "node binary not found; build it with cargo build -p x3-chain-node"
info "node: $NODE_BIN"

INSTALLER="$ROOT/scripts/install-validator.sh"
[ -f "$INSTALLER" ] || fail "scripts/install-validator.sh is missing"

# ── inputs: a real Live genesis and a real dev genesis ───────────────────────
info "building a fixture Live genesis"
X3_NODE_BIN="$NODE_BIN" bash "$ROOT/scripts/mainnet/make-fixture-mainnet-spec.sh" "$WORK/spec" \
  >"$WORK/fixture.log" 2>&1 \
  || { tail -20 "$WORK/fixture.log" >&2; fail "could not build the fixture Live genesis"; }
LIVE_SPEC="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['spec'])" "$WORK/spec/fixture.json")"
info "Live genesis: $LIVE_SPEC"

"$NODE_BIN" build-spec --chain dev --disable-log-color >"$WORK/dev.json" 2>/dev/null \
  || fail "could not build a dev genesis for the negative case"

# Everything the installer would touch points inside $WORK, so "wrote nothing"
# is an assertion the gate can actually make.
export X3_INSTALL_DIR="$WORK/prefix/bin"
export X3_DATA_DIR="$WORK/prefix/data"
export X3_CONFIG_DIR="$WORK/prefix/etc"
export X3_LOG_DIR="$WORK/prefix/log"

run_check() {  # run_check <label> <expect: ok|fail> <args...>
  local label="$1" expect="$2"; shift 2
  local log="$WORK/$(printf '%s' "$label" | tr ' /' '--').log"
  bash "$INSTALLER" "$@" >"$log" 2>&1
  local rc=$?
  if [ "$expect" = ok ] && [ "$rc" -ne 0 ]; then
    tail -15 "$log" >&2
    fail "$label: expected the installer to accept, exit was $rc"
  fi
  if [ "$expect" = fail ] && [ "$rc" -eq 0 ]; then
    tail -15 "$log" >&2
    fail "$label: expected the installer to refuse, but it exited 0"
  fi
  info "$label -> exit $rc ($expect)"
  LAST_LOG="$log"
}

# ── 1. the documented build-from-source route ────────────────────────────────
run_check "built binary + Live genesis" ok \
  --check --binary "$NODE_BIN" --chain "$LIVE_SPEC"
grep -q "chain spec: X3 Chain Production" "$LAST_LOG" \
  || { tail -10 "$LAST_LOG" >&2; fail "the accepted plan does not report the genesis it validated"; }
grep -qi "check only" "$LAST_LOG" \
  || fail "the accepted run does not say it was a check"

# ── 2. a dev genesis must not be installable by accident ─────────────────────
run_check "dev genesis refused" fail \
  --check --binary "$NODE_BIN" --chain "$WORK/dev.json"
grep -q "not 'Live'" "$LAST_LOG" \
  || { tail -10 "$LAST_LOG" >&2; fail "the dev-genesis refusal does not explain the chain type"; }

# ...unless the operator says so out loud.
run_check "dev genesis accepted with --allow-non-live-chain" ok \
  --check --binary "$NODE_BIN" --chain "$WORK/dev.json" --allow-non-live-chain

# ── 3-4. the binary has to exist and match the digest you declared ───────────
run_check "missing binary refused" fail \
  --check --binary "$WORK/does-not-exist" --chain "$LIVE_SPEC"

run_check "wrong --sha256 refused" fail \
  --check --binary "$NODE_BIN" --chain "$LIVE_SPEC" \
  --sha256 0000000000000000000000000000000000000000000000000000000000000000
grep -q "sha256 mismatch" "$LAST_LOG" \
  || { tail -10 "$LAST_LOG" >&2; fail "the checksum failure does not say 'sha256 mismatch'"; }

CORRECT_SHA="$(sha256sum "$NODE_BIN" | awk '{print $1}')"
run_check "correct --sha256 accepted" ok \
  --check --binary "$NODE_BIN" --chain "$LIVE_SPEC" --sha256 "$CORRECT_SHA"
grep -q "sha256 verified" "$LAST_LOG" \
  || { tail -10 "$LAST_LOG" >&2; fail "a matching digest is not reported as verified"; }

# ── 5. the release route, which today must refuse ────────────────────────────
run_check "--from-release with no published release refused" fail \
  --check --from-release --chain "$LIVE_SPEC"
grep -q "no published release" "$LAST_LOG" \
  || { tail -10 "$LAST_LOG" >&2; fail "the release refusal does not explain that there is no published release"; }
grep -qi "Skipping verification" "$LAST_LOG" \
  && fail "the installer still skips checksum verification"
grep -q "cargo build --release" "$LAST_LOG" \
  || fail "the release refusal does not tell the operator how to proceed"

# ── 6-7. missing source and missing genesis ──────────────────────────────────
run_check "no source flag refused" fail --check --chain "$LIVE_SPEC"
run_check "no --chain refused" fail --check --binary "$NODE_BIN"

# ── the check mode must not have written anything ────────────────────────────
if [ -e "$WORK/prefix" ]; then
  printf '[install-gate] FAIL: check mode created %s:\n' "$WORK/prefix" >&2
  find "$WORK/prefix" -maxdepth 3 >&2
  exit 1
fi
info "check mode wrote nothing under $WORK/prefix"

echo
echo "[install-gate] PASS — the install path accepts a built binary with a Live genesis,"
echo "               refuses a dev genesis, a missing binary, a wrong digest, an"
echo "               unpublished release and a missing genesis, and writes nothing in check mode"
