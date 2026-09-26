#!/usr/bin/env bash
# Shared helpers for the three-validator `local3` monitoring path.
#
# Sourced by scripts/monitoring/local3-monitoring-up.sh (operator bring-up) and
# scripts/monitoring/local3-monitoring-check.sh (the gate). Not meant to be run
# on its own.
#
# The bring-up mirrors scripts/local-network-smoke.sh — same `--chain local3`,
# same deterministic per-validator node keys, same `--no-mdns`/`--bootnodes`
# wiring — but keeps the three nodes alive after they finalize so their
# Prometheus exporters can be scraped. That is the one thing the smoke script
# does not expose, which is why this is a sibling file rather than a wrapper.

LOCAL3_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOCAL3_CONFIG="$LOCAL3_ROOT/monitoring/local3/prometheus-local3.yml"
declare -a LOCAL3_PIDS=()

local3_info() { printf '[local3-mon] %s\n' "$*"; }
local3_fail() { printf '[local3-mon] FAIL: %s\n' "$*" >&2; return 1; }

# Resolve the node binary the same way the other chain gates do.
local3_node_bin() {
  local target_dir="${CARGO_TARGET_DIR:-$LOCAL3_ROOT/target}"
  local bin="${X3_NODE_BIN:-}"
  if [ -z "$bin" ]; then
    for candidate in "$target_dir/release/x3-chain-node" "$LOCAL3_ROOT/target/release/x3-chain-node"; do
      [ -x "$candidate" ] && bin="$candidate" && break
    done
  fi
  [ -n "$bin" ] && [ -x "$bin" ] || return 1
  printf '%s' "$bin"
}

local3_rpc() {  # local3_rpc <port> <method> [params-json]
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$2\",\"params\":${3:-[]},\"id\":1}" \
    "http://127.0.0.1:$1"
}

local3_json_field() { sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p" | head -1; }

# local3_start_validator <name> <key-seed> <rpc> <p2p> <prom> <base-dir> <log-dir> [args...]
local3_start_validator() {
  local name="$1" key_seed="$2" rpc="$3" p2p="$4" prom="$5" base_dir="$6" log_dir="$7"
  shift 7
  local bin
  bin="$(local3_node_bin)" || { local3_fail "node binary not found (build: cargo build --release -p x3-chain-node)"; return 1; }
  local3_info "starting $name (rpc $rpc, p2p $p2p, prometheus $prom)"
  # No `--tmp`: each validator gets its own base path. `--alice`/`--bob`/`--charlie`
  # add session keys only; an explicit `--node-key` is what the network needs.
  "$bin" --chain local3 --base-path "$base_dir/$name" \
    --rpc-port "$rpc" --port "$p2p" --prometheus-port "$prom" \
    --no-mdns --node-key "$(printf '%064x' "$key_seed")" "$@" \
    >"$log_dir/$name.log" 2>&1 &
  LOCAL3_PIDS+=("$!")
}

local3_wait_for_rpc() {  # local3_wait_for_rpc <name> <rpc-port> [tries]
  local name="$1" port="$2" tries="${3:-90}"
  local i
  for i in $(seq 1 "$tries"); do
    if local3_rpc "$port" chain_getHeader | grep -q '"number"'; then
      local3_info "$name answered RPC"
      return 0
    fi
    sleep 1
  done
  local3_fail "$name never answered chain_getHeader on 127.0.0.1:$port"
  return 1
}

local3_stop_all() {
  local pid
  for pid in "${LOCAL3_PIDS[@]:-}"; do
    kill "$pid" 2>/dev/null || true
  done
  sleep 1
  for pid in "${LOCAL3_PIDS[@]:-}"; do
    kill -9 "$pid" 2>/dev/null || true
  done
  LOCAL3_PIDS=()
}

# The three (validator, prometheus-port) pairs the checked-in scrape config names.
# Read through python3 so the gate fails loudly if the YAML stops parsing or stops
# naming exactly three validators; the ports here are what the live scrape uses.
local3_config_targets() {
  python3 - "$LOCAL3_CONFIG" <<'PY'
import sys
try:
    import yaml
except Exception as exc:  # pragma: no cover - environment guard
    sys.stderr.write("pyyaml missing: %s\n" % exc)
    sys.exit(3)

path = sys.argv[1]
with open(path) as fh:
    doc = yaml.safe_load(fh)

jobs = (doc or {}).get("scrape_configs") or []
out = []
for job in jobs:
    if job.get("metrics_path") != "/metrics":
        sys.stderr.write("job %r does not scrape /metrics\n" % job.get("job_name"))
        sys.exit(6)
    label = None
    target = None
    for sc in job.get("static_configs", []):
        for t in sc.get("targets", []):
            target = t
        labels = sc.get("labels") or {}
        if "validator" in labels:
            label = labels["validator"]
    if label is None or target is None:
        sys.stderr.write("job %r is missing a validator label or a target\n" % job.get("job_name"))
        sys.exit(4)
    host, _, port = target.rpartition(":")
    out.append((label, host, port))

if len(out) != 3:
    sys.stderr.write("expected exactly 3 validator targets, found %d\n" % len(out))
    sys.exit(5)
for line in out:
    print("\t".join(line))
PY
}

# local3_scrape <port> <out-file> — fetch the exporter's /metrics snapshot.
local3_scrape() {
  local port="$1" out="$2"
  curl -s -m 8 -o "$out" -w '%{http_code}' "http://127.0.0.1:$port/metrics"
}

# local3_metric_number <file> <metric> [substring] — first line's value ($NF).
local3_metric_number() {
  local file="$1" metric="$2" needle="${3:-}"
  awk -v m="$metric" -v n="$needle" '
    index($1, m) == 1 && (n == "" || index($0, n) > 0) { print $NF; exit }
  ' "$file"
}

# local3_build_info_name <file> — the node's own `name` label (Alice/Bob/Charlie).
local3_build_info_name() {
  grep -m1 '^substrate_build_info' "$1" | sed -n 's/.*name="\([^"]*\)".*/\1/p'
}
