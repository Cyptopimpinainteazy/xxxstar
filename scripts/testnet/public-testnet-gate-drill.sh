#!/usr/bin/env bash
# Run the public testnet gate against a real seven-validator network.
#
# `scripts/mainnet/public_testnet_gate.sh` is the launch gate: fifteen criteria, and the
# verdict is what an operator publishes before opening public participation. It was
# repaired on 2026-09-26 and demonstrated once against a seven-validator network on this
# box — and nothing re-ran it afterwards, so the matrix row's `tested` score rested on a
# run that only exists in a gitignored report and a commit message. A readiness claim
# nothing can re-run is a claim, not a record; this is the re-run.
#
# What it owns is the half of the gate that is *about the chain*: criterion 1 (the
# authority set the chain configures *and* how many of those authorities are reachable
# right now — the two numbers that used to be one wrong number, `peers + 1`) and
# criterion 12 (the live RPC smoke). The other criteria are checked too, and printed, but
# they read artifacts on this box rather than the running network — gates 7/8/9 read drill
# reports, 13 runs the wallet app's own suite, 14 needs a dashboard actually serving on
# 3000/3001/8080 — so the drill does not pretend to own them. It asserts only that the
# verdict does not claim a clean sweep while something was skipped (the overclaim gate 6
# exists to prevent).
#
# `--self-test` stops one validator and requires criterion 1 to go from PASS to FAIL on
# the same network, so the criterion is load-bearing rather than a printed constant.
#
#   scripts/testnet/public-testnet-gate-drill.sh
#   scripts/testnet/public-testnet-gate-drill.sh --self-test
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/monitoring/lib-local3.sh
source "$SCRIPT_DIR/../monitoring/lib-local3.sh"   # local3_node_bin / local3_rpc only; the bring-up is the testnet launcher

ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

SELF_TEST=0
case "${1:-}" in
  --self-test) SELF_TEST=1 ;;
  "" ) : ;;
  * ) echo "usage: $(basename "$0") [--self-test]" >&2; exit 2 ;;
esac

COUNT="${X3_PTG_VALIDATORS:-7}"
# Ports nobody else holds: the monitoring gates use rpc 19800 / p2p 30500 / prom 19600 and
# the lifecycle gates 19945/9615, so this block stays clear of both. A second node that
# cannot bind its RPC talks past the first one without failing loudly.
RPC_BASE="${X3_PTG_RPC_BASE:-19820}"
P2P_BASE="${X3_PTG_P2P_BASE:-30520}"
PROM_BASE="${X3_PTG_PROM_BASE:-19620}"
MIN_FINALIZED="${X3_PTG_MIN_FINALIZED:-6}"
# A seven-node mesh does not hold six peers on every node every second; the two-hour soak on
# this box measured a minimum of 5 while all seven stayed on one chain.
CONNECTED_PEERS=$(( COUNT > 3 ? COUNT - 2 : COUNT - 1 ))
REPORT="$ROOT_DIR/reports/public_testnet_gate.md"

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-ptg.XXXXXX")"
SPEC_DIR="$WORK_DIR/spec"
BASE_DIR="$WORK_DIR/net"
LOG_DIR="$BASE_DIR/logs"
mkdir -p "$SPEC_DIR"

cleanup() {
  # By the launcher's own pid files. Never `pkill -f x3-chain-node`: other gates' nodes are
  # on this box, and this drill owns only the set it started.
  local f
  for f in "$BASE_DIR"/pids/node-*.pid; do
    [[ -f "$f" ]] || continue
    kill "$(cat "$f")" 2>/dev/null || true
  done
  sleep 2
  for f in "$BASE_DIR"/pids/node-*.pid; do
    [[ -f "$f" ]] || continue
    kill -9 "$(cat "$f")" 2>/dev/null || true
  done
}
trap cleanup EXIT

info() { printf '[ptg] %s\n' "$*"; }
fail() { printf '[ptg] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[ptg] PASS: %s\n' "$*"; }

rpc() { # <port> <method> [params]
  curl -s -m 8 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$2\",\"params\":${3:-[]}}" \
    "http://127.0.0.1:$1"
}

NODE_BIN="$(local3_node_bin)" || fail "node binary not found; build it with cargo build --release -p x3-chain-node"
# Freeze the binary for the whole run and print its digest: a shared path can be relinked
# under us by a sibling agent's `cargo build`, and two builds of the same tree have two
# genesis hashes, so a set spawned across a relink wedges rather than fails.
FREEZE_BIN="$WORK_DIR/x3-chain-node"
cp "$NODE_BIN" "$FREEZE_BIN" || fail "could not freeze $NODE_BIN"
chmod +x "$FREEZE_BIN"
info "node: $NODE_BIN"
info "frozen as $FREEZE_BIN (sha256 $(sha256sum "$FREEZE_BIN" | awk '{print $1}'))"

info "building a ${COUNT}-authority spec"
# `P2P_BASE` goes into the spec: its `bootNodes` name `P2P_BASE + i - 1`, so a spec built for
# one base and launched on another has every node dialing a port nobody listens on, and the
# mesh collapses to a hub with one peer each and no finality.
X3_NODE_BIN="$FREEZE_BIN" OUT_DIR="$SPEC_DIR" P2P_BASE="$P2P_BASE" \
  python3 "$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py" "$COUNT" \
  >"$WORK_DIR/spec.log" 2>&1 || { tail -20 "$WORK_DIR/spec.log" >&2; fail "spec build failed"; }

SPEC="$SPEC_DIR/x3-testnet-plain.json"
KEYS_DIR="$SPEC_DIR/validator-keys"
[[ -f "$SPEC" && -d "$KEYS_DIR" ]] || fail "spec builder did not produce $SPEC and $KEYS_DIR"

info "starting ${COUNT} validators (rpc $RPC_BASE, p2p $P2P_BASE, prometheus $PROM_BASE)"
RUST_LOG=info COUNT="$COUNT" RPC_BASE="$RPC_BASE" P2P_BASE="$P2P_BASE" PROM_BASE="$PROM_BASE" \
PROMETHEUS=1 BASE_DIR="$BASE_DIR" CHAIN_SPEC="$SPEC" KEYS_DIR="$KEYS_DIR" LOG_DIR="$LOG_DIR" \
SKIP_BUILD=1 bash "$ROOT_DIR/scripts/testnet/x3_testnet_up.sh" --skip-build \
  >"$WORK_DIR/launch.log" 2>&1 || { tail -20 "$WORK_DIR/launch.log" >&2; fail "the launcher could not start ${COUNT} validators"; }
info "launched; waiting for RPC, peers and finality"

deadline=$(( $(date +%s) + 900 ))
ready=0
declare -A LAST_PEERS LAST_FINALIZED
while [[ "$(date +%s)" -lt "$deadline" ]]; do
  ready=1
  for i in $(seq 1 "$COUNT"); do
    port=$(( RPC_BASE + i - 1 ))
    peers="$(rpc "$port" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
    [[ "${peers:-0}" -ge "$CONNECTED_PEERS" ]] || { ready=0; LAST_PEERS[$i]="${peers:-none}"; }
    # A *height* past genesis, not "a finalized head exists": a node that just started answers
    # with the genesis hash at height 0, so the weaker check passes while the node is at block 1.
    head="$(rpc "$port" chain_getFinalizedHead | sed -n 's/.*"result":"\([^"]*\)".*/\1/p' | head -1)"
    if [[ -z "$head" ]]; then
      ready=0
    else
      height="$(rpc "$port" chain_getHeader "[\"$head\"]" \
        | sed -n 's/.*"number":"\(0x[0-9a-f]*\)".*/\1/p' | head -1)"
      height=$(( ${height:-0x0} ))
      [[ "$height" -ge "$MIN_FINALIZED" ]] || { ready=0; LAST_FINALIZED[$i]="$height"; }
    fi
  done
  [[ "$ready" = 1 ]] && break
  sleep 3
done
if [[ "$ready" != 1 ]]; then
  detail=""
  for i in $(seq 1 "$COUNT"); do
    detail+="node-$i(peers=${LAST_PEERS[$i]:-ok},finalized=${LAST_FINALIZED[$i]:-ok}) "
  done
  fail "not all ${COUNT} validators reached ${CONNECTED_PEERS} peers and finalized height ${MIN_FINALIZED} in 900s (last: $detail)"
fi
pass "${COUNT} validators are on one chain (>= ${CONNECTED_PEERS} peers each, finalized past ${MIN_FINALIZED})"

# run_gate <rpc-port> <log-file>; returns the gate's exit status.
run_gate() {
  X3_RPC_URL="http://127.0.0.1:$1" X3_CHAIN_SPEC="$SPEC" X3_VALIDATOR_COUNT="$COUNT" \
  X3_TESTNET_HOURS=0 \
    bash "$ROOT_DIR/scripts/mainnet/public_testnet_gate.sh" >"$2" 2>&1
}

info "running the public testnet gate against validator 1"
run_gate "$RPC_BASE" "$WORK_DIR/gate.out"
GATE_RC=$?
cat "$WORK_DIR/gate.out"
[[ -f "$REPORT" ]] || fail "the gate wrote no report at $REPORT"
# Keep the operator's artifact as it was: the control below overwrites the same path.
cp "$REPORT" "$WORK_DIR/golden-report.md"

# ── the chain-derived criteria: these two are the drill's to prove ────────────
grep -q '^\[PASS\] min_7_validators$' "$WORK_DIR/gate.out" \
  || fail "criterion 1 did not pass on a ${COUNT}-authority network: $(grep -m1 'min_7_validators' "$WORK_DIR/gate.out")"
# The number has to be the chain's own authority set, and it has to be read from the chain.
AUTHORITY_LINE="$(grep -m1 'authority set:' "$WORK_DIR/gate.out")"
grep -qF "authority set: ${COUNT}, reachable now: ${COUNT}" <<<"$AUTHORITY_LINE" \
  || fail "criterion 1 reported $AUTHORITY_LINE — the authority set and the reachable count must both be ${COUNT}"
pass "criterion 1: $AUTHORITY_LINE"

grep -q '^\[PASS\] indexer_rpc_api_smoke$' "$WORK_DIR/gate.out" \
  || fail "criterion 12 (live RPC smoke) did not pass against the running node"
pass "criterion 12: live RPC smoke passes against validator 1"

# ── the verdict must not overclaim ───────────────────────────────────────────
# A SKIP is not a PASS. Gate 6 is skipped by `X3_TESTNET_HOURS=0`, so the only honest
# verdict here is the one that names it.
# …and the report has to be *this* run's report, or the check below reads someone else's.
grep -qF "\`http://127.0.0.1:$RPC_BASE\`" "$REPORT" \
  || fail "$REPORT does not name this run's RPC endpoint; refusing to read a stale verdict"
grep -q 'all 15 criteria met' "$REPORT" \
  && fail "the verdict claims all 15 criteria were met although criterion 6 was skipped (X3_TESTNET_HOURS=0)"
if grep -q 'PASS with skipped criteria' "$REPORT"; then
  grep -q 'block_production_stable_72h' "$REPORT" \
    || fail "the verdict says criteria were skipped without naming them"
  pass "the verdict names the skipped criterion instead of claiming all 15 were met"
fi
info "gate exit status: $GATE_RC (other criteria read this box, not the network; see the table above)"

# ── the negative control: criterion 1 must be able to fail ───────────────────
if [[ "$SELF_TEST" == 1 ]]; then
  VICTIM=$(( RPC_BASE + COUNT - 1 ))
  VICTIM_PID_FILE="$BASE_DIR/pids/node-$COUNT.pid"
  [[ -f "$VICTIM_PID_FILE" ]] || fail "no pid file for node-$COUNT at $VICTIM_PID_FILE"
  info "self-test: stopping node-$COUNT with one validator up..."
  kill "$(cat "$VICTIM_PID_FILE")" 2>/dev/null || fail "could not stop node-$COUNT"
  deadline=$(( $(date +%s) + 120 ))
  while [[ "$(date +%s)" -lt "$deadline" ]]; do
    peers="$(rpc "$RPC_BASE" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
    [[ "${peers:-$COUNT}" -le $(( COUNT - 2 )) ]] && break
    sleep 3
  done
  info "validator 1 now reports ${peers:-?} peers (${COUNT} authorities, one stopped)"
  run_gate "$RPC_BASE" "$WORK_DIR/control.out"
  grep -q '^\[FAIL\] min_7_validators' "$WORK_DIR/control.out" \
    || fail "criterion 1 still passed with one of ${COUNT} authorities stopped — it is not load-bearing"
  grep -q "only $(( COUNT - 1 )) reachable" "$WORK_DIR/control.out" \
    || fail "criterion 1 failed for a reason other than the reachable count: $(grep -m1 'min_7_validators' "$WORK_DIR/control.out")"
  pass "self-test: criterion 1 fails the same network once an authority is stopped ($(grep -m1 'min_7_validators' "$WORK_DIR/control.out"))"
  # Leave the operator's report as the golden run wrote it, not the control's FAIL.
  cp "$WORK_DIR/golden-report.md" "$REPORT"
fi

info "gate report: $REPORT"
info "proof logs kept in $WORK_DIR"
pass "the public testnet gate reads the chain's own numbers on a ${COUNT}-authority network"
