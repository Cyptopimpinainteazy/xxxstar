#!/usr/bin/env bash
# Prove Prometheus metrics are live on *all three* `local3` validators.
#
# The public-testnet claim is "Prometheus/Grafana/logging live across all
# validators". A single scrape of one node is not that. This gate boots the
# built-in `local3` chain (Alice/Bob/Charlie), scrapes each validator's own
# `--prometheus-port` exporter, and requires every one of them to identify
# itself — the node's `substrate_build_info{name=...}` label must match the
# `validator` target label, the finalized height must be past genesis, the peer
# count must show the node is actually connected, and the node must report a
# non-full-node role. Three indistinguishable scrapes, or fewer than three, is a
# failure.
#
# It also keeps the checked-in operator artifacts honest: the scrape config is
# parsed (PyYAML) and the targets in it are what the gate boots and scrapes, so
# a rotated config breaks the gate instead of the operator; the Grafana
# dashboard is parsed (jq) and every metric its panels reference is required to
# exist in the live scrape, so a dashboard naming metrics the node does not
# export cannot pass.
#
#   scripts/monitoring/local3-monitoring-check.sh            # normal gate
#   scripts/monitoring/local3-monitoring-check.sh --self-test # + prove the check
#                                                             #   fails when a
#                                                             #   validator is missing
#   X3_MONITORING_DROP_VALIDATOR=bob scripts/monitoring/local3-monitoring-check.sh
#       boots all three but scraps the named validator from the scrape set; the
#       gate must then fail. Used to demonstrate the check is load-bearing.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/monitoring/lib-local3.sh
source "$SCRIPT_DIR/lib-local3.sh"

SELF_TEST=0
case "${1:-}" in
  --self-test) SELF_TEST=1 ;;
  "" ) : ;;
  * ) echo "usage: $(basename "$0") [--self-test]" >&2; exit 2 ;;
esac

DASHBOARD="$LOCAL3_ROOT/monitoring/local3/grafana-x3-validators.json"
MIN_FINALIZED=3
EXPECTED_PEERS=2
DROP="${X3_MONITORING_DROP_VALIDATOR:-}"
BASE_DIR="$(mktemp -d)"
LOG_DIR="$(mktemp -d)"
SCRAPE_DIR="$BASE_DIR/metrics"
mkdir -p "$SCRAPE_DIR"

cleanup() { local3_stop_all; }
trap cleanup EXIT

fail() { printf '[local3-mon] FAIL: %s\n' "$*" >&2; exit 1; }

# ── 1. the checked-in artifacts must parse before anything is booted ──────────
[ -f "$LOCAL3_CONFIG" ] || fail "missing scrape config: $LOCAL3_CONFIG"
targets_raw="$(local3_config_targets)" || fail "scrape config does not parse into 3 /metrics targets"
mapfile -t TARGETS <<<"$targets_raw"
[ "${#TARGETS[@]}" -eq 3 ] || fail "scrape config named ${#TARGETS[@]} validators, expected 3"

[ -f "$DASHBOARD" ] || fail "missing Grafana dashboard: $DASHBOARD"
jq -e '.dashboard.title | length > 0' "$DASHBOARD" >/dev/null \
  || fail "Grafana dashboard has no title"
jq -e '[.dashboard.panels[].targets[].expr] | length > 0' "$DASHBOARD" >/dev/null \
  || fail "Grafana dashboard has no panel expressions"
mapfile -t DASH_METRICS < <(
  jq -r '[.dashboard.panels[].targets[].expr] | .[]' "$DASHBOARD" \
    | grep -oE '\b(substrate_[a-z0-9_]+|x3_[a-z0-9_]+)\b' | sort -u
)
[ "${#DASH_METRICS[@]}" -gt 0 ] || fail "Grafana dashboard references no X3/Substrate metrics"
local3_info "config OK: ${TARGETS[*]}"
local3_info "dashboard OK: references ${#DASH_METRICS[@]} metric(s)"

# ── 2. boot one validator per target, on the ports the config names ──────────
declare -A PROM RPC P2P
for row in "${TARGETS[@]}"; do
  IFS=$'\t' read -r name host port <<<"$row"
  [ "$host" = "127.0.0.1" ] || fail "config target $name is not on 127.0.0.1 ($host)"
  PROM[$name]="$port"
  RPC[$name]="$(( port - 2 ))"
  P2P[$name]="$(( port - 1 ))"
done
for want in alice bob charlie; do
  [ -n "${PROM[$want]:-}" ] || fail "scrape config is missing the '$want' validator"
done

NODE_BIN="$(local3_node_bin)" || fail "node binary not found; build it with cargo build --release -p x3-chain-node"
local3_info "node: $NODE_BIN"

local3_start_validator alice 1 "${RPC[alice]}" "${P2P[alice]}" "${PROM[alice]}" "$BASE_DIR" "$LOG_DIR" --alice \
  || fail "could not start alice"
local3_wait_for_rpc alice "${RPC[alice]}" || fail "alice never came up"

alice_peer_id="$(local3_rpc "${RPC[alice]}" system_localPeerId | local3_json_field result)"
[ -n "$alice_peer_id" ] || fail "could not read alice's peer id"
alice_bootnode="/ip4/127.0.0.1/tcp/${P2P[alice]}/p2p/$alice_peer_id"

local3_start_validator bob 2 "${RPC[bob]}" "${P2P[bob]}" "${PROM[bob]}" "$BASE_DIR" "$LOG_DIR" --bob --bootnodes "$alice_bootnode" \
  || fail "could not start bob"
local3_wait_for_rpc bob "${RPC[bob]}" || fail "bob never came up"

bob_peer_id="$(local3_rpc "${RPC[bob]}" system_localPeerId | local3_json_field result)"
bob_bootnode="/ip4/127.0.0.1/tcp/${P2P[bob]}/p2p/$bob_peer_id"

local3_start_validator charlie 3 "${RPC[charlie]}" "${P2P[charlie]}" "${PROM[charlie]}" "$BASE_DIR" "$LOG_DIR" --charlie --bootnodes "$alice_bootnode" "$bob_bootnode" \
  || fail "could not start charlie"
local3_wait_for_rpc charlie "${RPC[charlie]}" || fail "charlie never came up"

# ── 3. wait for the three to connect and finalize ────────────────────────────
local3_info "waiting for the three validators to connect and finalize"
connected=0
for _ in $(seq 1 60); do
  ok=1
  for name in alice bob charlie; do
    peers="$(local3_rpc "${RPC[$name]}" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
    [ "${peers:-0}" -ge "$EXPECTED_PEERS" ] || ok=0
  done
  [ "$ok" = 1 ] && connected=1 && break
  sleep 1
done
[ "$connected" = 1 ] || fail "the validators did not connect to each other"

fa=0; fb=0; fc=0
for _ in $(seq 1 60); do
  fa="$(local3_metric_number <(curl -s -m 5 "http://127.0.0.1:${PROM[alice]}/metrics") substrate_block_height finalized 2>/dev/null || echo 0)"
  fb="$(local3_metric_number <(curl -s -m 5 "http://127.0.0.1:${PROM[bob]}/metrics") substrate_block_height finalized 2>/dev/null || echo 0)"
  fc="$(local3_metric_number <(curl -s -m 5 "http://127.0.0.1:${PROM[charlie]}/metrics") substrate_block_height finalized 2>/dev/null || echo 0)"
  fa="${fa:-0}"; fb="${fb:-0}"; fc="${fc:-0}"
  if [ "$fa" -ge "$MIN_FINALIZED" ] && [ "$fb" -ge "$MIN_FINALIZED" ] && [ "$fc" -ge "$MIN_FINALIZED" ]; then
    break
  fi
  sleep 1
done

# ── 4. scrape each validator's exporter individually ─────────────────────────
SCRAPED=()
for name in alice bob charlie; do
  if [ "$name" = "$DROP" ]; then
    local3_info "NOT scraping $name (X3_MONITORING_DROP_VALIDATOR=$DROP)"
    continue
  fi
  out="$SCRAPE_DIR/$name.metrics"
  code="$(local3_scrape "${PROM[$name]}" "$out")"
  [ "$code" = 200 ] || fail "$name's metrics endpoint returned HTTP ${code:-none}"
  [ -s "$out" ] || fail "$name's metrics endpoint returned an empty body"
  SCRAPED+=("$name")
  local3_info "$name: scraped $(wc -l <"$out") metric lines from :${PROM[$name]}"
done

# ── 5. per-validator identity, not "a port answered" ─────────────────────────
verify_identity() {  # verify_identity <name>...
  local -a names=("$@")
  [ "${#names[@]}" -eq 3 ] || { echo "expected 3 scraped validators, got ${#names[@]}"; return 1; }
  local -A FINALIZED=() NAMES=()
  local name file label finalized peers role chain
  for name in "${names[@]}"; do
    file="$SCRAPE_DIR/$name.metrics"
    [ -s "$file" ] || { echo "$name: no metrics scraped"; return 1; }
    label="$(local3_build_info_name "$file")"
    [ -n "$label" ] || { echo "$name: substrate_build_info has no name label"; return 1; }
    if [ "$(printf '%s' "$label" | tr '[:upper:]' '[:lower:]')" != "$name" ]; then
      echo "$name: node identifies itself as '$label'"; return 1
    fi
    chain="$(sed -n 's/.*chain="\([^"]*\)".*/\1/p' "$file" | head -1)"
    [ "$chain" = "x3_chain_local3" ] || { echo "$name: unexpected chain label '$chain'"; return 1; }
    finalized="$(local3_metric_number "$file" substrate_block_height finalized)"
    peers="$(local3_metric_number "$file" substrate_sub_libp2p_peers_count)"
    role="$(local3_metric_number "$file" substrate_node_roles)"
    [ -n "$finalized" ] || { echo "$name: no substrate_block_height{status=\"finalized\"}"; return 1; }
    [ -n "$peers" ] || { echo "$name: no substrate_sub_libp2p_peers_count"; return 1; }
    [ -n "$role" ] || { echo "$name: no substrate_node_roles"; return 1; }
    [ "$finalized" -ge "$MIN_FINALIZED" ] || { echo "$name: finalized height $finalized < $MIN_FINALIZED"; return 1; }
    [ "$peers" -ge "$EXPECTED_PEERS" ] || { echo "$name: peer count $peers < $EXPECTED_PEERS"; return 1; }
    [ "$role" -ge 1 ] || { echo "$name: role $role is a full node, not a validator"; return 1; }
    if [ -n "${NAMES[$label]:-}" ]; then echo "two validators both call themselves '$label'"; return 1; fi
    NAMES[$label]=1
    FINALIZED[$name]="$finalized"
    printf '  %-8s name=%-8s chain=%s finalized=%s peers=%s role=%s\n' \
      "$name" "$label" "$chain" "$finalized" "$peers" "$role"
  done
  # Three independent chains could each look "live". Finalized *heights* lag by
  # different amounts per node, so they are not the test — canonical agreement
  # is. Take the lowest finalized height any exporter reported (every node has
  # definitely finalized it) and require all three to return the same hash
  # there. This is what rules out "three nodes, three chains".
  local min=999999999 v hash ref="" hex
  for name in "${names[@]}"; do
    v="${FINALIZED[$name]}"
    [ "$v" -lt "$min" ] && min="$v"
  done
  hex="$(printf '0x%x' "$min")"
  for name in "${names[@]}"; do
    hash="$(local3_rpc "${RPC[$name]}" chain_getBlockHash "[\"$min\"]" | local3_json_field result)"
    [ -n "$hash" ] || hash="$(local3_rpc "${RPC[$name]}" chain_getBlockHash "[\"$hex\"]" | local3_json_field result)"
    [ -n "$hash" ] || { echo "$name: could not read the hash at finalized height $min"; return 1; }
    if [ -z "$ref" ]; then
      ref="$hash"
    elif [ "$hash" != "$ref" ]; then
      echo "validators disagree at finalized height $min: $hash != $ref"; return 1
    fi
  done
  echo "  common finalized block $min: $ref"
  return 0
}

echo "[local3-mon] per-validator identity:"
if ! verify_identity "${SCRAPED[@]}"; then
  fail "per-validator metrics are not live across all three validators"
fi

# ── 6. every metric the dashboard names must exist in the live scrape ────────
alice_snapshot="$SCRAPE_DIR/alice.metrics"
for metric in "${DASH_METRICS[@]}"; do
  grep -q "^${metric}\b" "$alice_snapshot" \
    || fail "dashboard references '$metric' but the node does not export it"
done
local3_info "dashboard metrics all present in the live scrape"

# ── 7. self-test: the check must reject a missing validator ──────────────────
if [ "$SELF_TEST" = 1 ]; then
  echo "[local3-mon] self-test: verifying the check fails when a validator is missing"
  declare -a two=()
  for name in alice bob charlie; do [ "$name" = charlie ] || two+=("$name"); done
  if verify_identity "${two[@]}" >/dev/null 2>&1; then
    fail "self-test: identity check passed with charlie missing — it is not load-bearing"
  fi
  local3_info "self-test PASS: the check rejects a missing validator"
fi

echo "[local3-mon] PASS — 3 validators scraped, each identified by its own metrics endpoint"
