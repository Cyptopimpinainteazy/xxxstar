#!/usr/bin/env bash
# Bring up the three-validator `local3` chain with Prometheus exporters on, for
# an operator who wants to point Prometheus/Grafana at it.
#
# This is the bring-up half of the monitoring path; the gate that proves the
# exporters are actually there per-validator is
# scripts/monitoring/local3-monitoring-check.sh (and it wraps this same
# orchestration). Ports come from monitoring/local3/prometheus-local3.yml, so
# that file is the scrape config this command matches — edit the file, not this
# script, if the ports need to move.
#
#   scripts/monitoring/local3-monitoring-up.sh
#   # then, in another shell:
#   prometheus --config.file=monitoring/local3/prometheus-local3.yml
#   # import monitoring/local3/grafana-x3-validators.json into Grafana
#
# Ctrl-C tears the three nodes down.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/monitoring/lib-local3.sh
source "$SCRIPT_DIR/lib-local3.sh"

targets_raw="$(local3_config_targets)" || local3_fail "scrape config does not parse into 3 /metrics targets"
mapfile -t TARGETS <<<"$targets_raw"
[ "${#TARGETS[@]}" -eq 3 ] || local3_fail "scrape config named ${#TARGETS[@]} validators, expected 3"

RUN_DIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-local3-mon.XXXXXX")"
declare -A PROM RPC P2P
for row in "${TARGETS[@]}"; do
  IFS=$'\t' read -r name host port <<<"$row"
  PROM[$name]="$port"
  RPC[$name]="$(( port - 2 ))"
  P2P[$name]="$(( port - 1 ))"
done

LOCAL3_TORN_DOWN=0
cleanup() {
  [ "$LOCAL3_TORN_DOWN" = 1 ] && return 0
  LOCAL3_TORN_DOWN=1
  local3_info "shutting the three validators down"
  local3_stop_all
  local3_info "base path kept for inspection: $RUN_DIR"
}
trap cleanup EXIT INT TERM

local3_start_validator alice 1 "${RPC[alice]}" "${P2P[alice]}" "${PROM[alice]}" "$RUN_DIR" "$RUN_DIR" --alice \
  || local3_fail "could not start alice"
local3_wait_for_rpc alice "${RPC[alice]}" || exit 1
alice_peer_id="$(local3_rpc "${RPC[alice]}" system_localPeerId | local3_json_field result)"
alice_bootnode="/ip4/127.0.0.1/tcp/${P2P[alice]}/p2p/$alice_peer_id"

local3_start_validator bob 2 "${RPC[bob]}" "${P2P[bob]}" "${PROM[bob]}" "$RUN_DIR" "$RUN_DIR" --bob --bootnodes "$alice_bootnode" \
  || local3_fail "could not start bob"
local3_wait_for_rpc bob "${RPC[bob]}" || exit 1
bob_peer_id="$(local3_rpc "${RPC[bob]}" system_localPeerId | local3_json_field result)"
bob_bootnode="/ip4/127.0.0.1/tcp/${P2P[bob]}/p2p/$bob_peer_id"

local3_start_validator charlie 3 "${RPC[charlie]}" "${P2P[charlie]}" "${PROM[charlie]}" "$RUN_DIR" "$RUN_DIR" --charlie --bootnodes "$alice_bootnode" "$bob_bootnode" \
  || local3_fail "could not start charlie"
local3_wait_for_rpc charlie "${RPC[charlie]}" || exit 1

cat <<EOF
[local3-mon] the three validators are up. Prometheus scrape targets:
[local3-mon]   alice   http://127.0.0.1:${PROM[alice]}/metrics   (rpc ${RPC[alice]})
[local3-mon]   bob     http://127.0.0.1:${PROM[bob]}/metrics   (rpc ${RPC[bob]})
[local3-mon]   charlie http://127.0.0.1:${PROM[charlie]}/metrics   (rpc ${RPC[charlie]})
[local3-mon] scrape config: monitoring/local3/prometheus-local3.yml
[local3-mon] dashboard:     monitoring/local3/grafana-x3-validators.json
[local3-mon] node logs:     $RUN_DIR/{alice,bob,charlie}.log
[local3-mon] press Ctrl-C to stop.
EOF

wait
