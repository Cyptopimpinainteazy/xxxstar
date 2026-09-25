#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# harden_validator_gate.sh — prove the validator hardening path plans what it
# says and refuses what it cannot do safely
#
# `scripts/harden-validator.sh` is cited as evidence in
# `feature-matrix/consensus-l1.toml` and nothing ran it. It also used to apply
# `ufw --force reset` unconditionally, discarding whatever firewall the host
# already had, and to pass the literal placeholder `source address="YOUR-MGMT-CIDR"`
# to `firewall-cmd` — a rule matching nothing, on a host an operator would then
# believe was configured.
#
# This gate runs the script in `--check` mode, which needs no root and writes
# nothing, across every branch that a real host can take:
#
#   1. ufw          -> accepted; the plan names the rules it would add
#   2. firewalld, no --mgmt-cidr -> refused, with the reason
#   3. firewalld + CIDR          -> accepted; the plan carries the CIDR given
#   4. no firewall tool          -> accepted with a warning, nothing configured
#   5. a malformed CIDR          -> refused
#   6. --reset-firewall          -> the plan says it would reset; without the
#                                   flag the plan says it keeps the rules
#   7. an unknown argument       -> exit 2
#
# and checks that nothing was written under X3_HARDEN_ROOT, which the script
# prefixes onto every path it touches.
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$ROOT/scripts/harden-validator.sh"
WORK="$(mktemp -d)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

fail() { printf '[harden-gate] FAIL: %s\n' "$*" >&2; exit 1; }
info() { printf '[harden-gate] %s\n' "$*"; }

[ -f "$SCRIPT" ] || fail "scripts/harden-validator.sh is missing"

PREFIX="$WORK/prefix"
run_check() {  # run_check <label> <expect: ok|fail|usage> <args...>
  local label="$1" expect="$2"; shift 2
  local log="$WORK/$(printf '%s' "$label" | tr ' /' '--').log"
  X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" "$@" >"$log" 2>&1
  local rc=$?
  case "$expect" in
    ok)    [ "$rc" -eq 0 ] || { tail -15 "$log" >&2; fail "$label: expected success, exit $rc"; } ;;
    fail)  [ "$rc" -ne 0 ] || { tail -15 "$log" >&2; fail "$label: expected refusal, but it exited 0"; } ;;
    usage) [ "$rc" -eq 2 ] || { tail -15 "$log" >&2; fail "$label: expected usage exit 2, got $rc"; } ;;
  esac
  info "$label -> exit $rc ($expect)"
  LAST_LOG="$log"
}

CIDR="203.0.113.7/32"

# ── 1. the ufw branch ───────────────────────────────────────────────────────
run_check "ufw plan accepted" ok --check --mgmt-cidr "$CIDR" # tool forced below
X3_FIREWALL_TOOL=ufw X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" --check --mgmt-cidr "$CIDR" >"$WORK/ufw.log" 2>&1 \
  || { tail -15 "$WORK/ufw.log" >&2; fail "ufw branch refused a complete plan"; }
LAST_LOG="$WORK/ufw.log"
grep -q "check only" "$LAST_LOG" || fail "the plan does not say it is a check"
grep -q "30333/tcp" "$LAST_LOG" || fail "the plan does not name the P2P port"
grep -q "keeping the existing ufw rules" "$LAST_LOG" \
  || fail "the plan does not say it keeps the existing firewall rules unless asked to reset"
for step in "1/6" "2/6" "3/6" "4/6" "5/6" "6/6"; do
  grep -q "\[$step\]" "$LAST_LOG" || fail "the plan skips step $step"
done

# ── 6. the reset flag is what decides the destructive step ───────────────────
X3_FIREWALL_TOOL=ufw X3_RESET_FIREWALL=1 X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" --check --mgmt-cidr "$CIDR" \
  >"$WORK/reset.log" 2>&1 || fail "the reset plan was refused"
grep -q "would reset the existing ufw rules" "$WORK/reset.log" \
  || fail "--reset-firewall does not appear in the plan it changes"

# ── 2-3. firewalld, and the CIDR it needs ───────────────────────────────────
X3_FIREWALL_TOOL=firewalld X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" --check >"$WORK/fwd-nocidr.log" 2>&1
rc=$?
[ "$rc" -ne 0 ] || { tail -10 "$WORK/fwd-nocidr.log" >&2; fail "firewalld accepted a plan with no management CIDR"; }
grep -q "YOUR-MGMT-CIDR" "$WORK/fwd-nocidr.log" \
  || fail "the refusal does not explain the placeholder it replaced"
info "firewalld without --mgmt-cidr -> exit $rc (fail)"

X3_FIREWALL_TOOL=firewalld X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" --check --mgmt-cidr "$CIDR" >"$WORK/fwd.log" 2>&1 \
  || { tail -15 "$WORK/fwd.log" >&2; fail "firewalld refused a complete plan"; }
grep -q "source address=\\\"$CIDR\\\"" "$WORK/fwd.log" || fail "the firewalld plan does not carry the CIDR into the rule"

# ── 4. no firewall tool at all ──────────────────────────────────────────────
X3_FIREWALL_TOOL=none X3_HARDEN_ROOT="$PREFIX" bash "$SCRIPT" --check --mgmt-cidr "$CIDR" >"$WORK/none.log" 2>&1 \
  || { tail -10 "$WORK/none.log" >&2; fail "the no-tool plan was refused"; }
grep -qi "no firewall tool found" "$WORK/none.log" || fail "the no-tool branch does not warn"
grep -qi "Nothing was configured" "$WORK/none.log" || fail "the no-tool branch does not say nothing was done"

# ── 5, 7. bad input ─────────────────────────────────────────────────────────
run_check "malformed cidr refused" fail --check --mgmt-cidr "not-a-cidr"
run_check "unknown argument" usage --check --nonsense

# ── nothing was written, anywhere ───────────────────────────────────────────
if [ -e "$PREFIX" ]; then
  printf '[harden-gate] FAIL: check mode created %s:\n' "$PREFIX" >&2
  find "$PREFIX" -maxdepth 4 >&2
  exit 1
fi
info "check mode wrote nothing under $PREFIX"

echo
echo "[harden-gate] PASS — the hardening plan covers all six steps, names the P2P port,"
echo "              keeps the existing firewall unless --reset-firewall is given, requires"
echo "              the management CIDR for the firewalld rule, refuses bad input, and"
echo "              writes nothing in check mode"
