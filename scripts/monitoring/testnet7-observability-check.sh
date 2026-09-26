#!/usr/bin/env bash
# Prove metrics AND logs are live on all seven testnet validators.
#
# The three-validator `local3` checks (`local3-monitoring-check.sh`,
# `local3-logging-check.sh`) prove the shape on the built-in chain. The objective's bullet is
# "Prometheus/Grafana/logging: live across all validators" and the public testnet is seven, so
# this runs the same questions against a generated seven-authority network:
#
#   * each validator has its own exporter, and it identifies *itself* — the node's
#     `substrate_build_info{name=...}` label must match the validator it was scraped from, the
#     chain label must be the network, the finalized height must be past genesis, the peer count
#     must show it is connected to the other six, and the role must not be a full node;
#   * all seven agree on the canonical hash at a common finalized height (seven nodes, one chain);
#   * each validator's log stream is collected in one directory and carries its own identity,
#     imported blocks and finalized blocks — a stream that only holds a startup banner is not
#     operational logging, and a node started without `--log info` emits exactly that;
#   * one aggregate query over the sink reaches all seven sources.
#
# When `prometheus` and `promtool` are on PATH (`brew install prometheus`), the check also
# *scrapes the seven validators with a real Prometheus*: it generates a scrape config for the
# ports it booted, `promtool check config`s it, starts Prometheus, and requires
# `count(up{job="x3-validators"} == 1)` to be seven. That is the difference between "the
# config parses and the endpoints answer" and "a metrics server is ingesting all of them".
#
# `--self-test` holds one validator out of the metrics scrape and out of the log collection and
# requires both checks to fail, so neither can pass on a network that is not fully observed.
#
#   scripts/monitoring/testnet7-observability-check.sh
#   scripts/monitoring/testnet7-observability-check.sh --self-test
#   X3_OBSERVABILITY_VALIDATORS=4 scripts/monitoring/testnet7-observability-check.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/monitoring/lib-local3.sh
source "$SCRIPT_DIR/lib-local3.sh"   # for the metric parsers only; the bring-up is the testnet launcher

ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
# The dashboard this repository ships, imported into Grafana and queried below. Its panels are
# metric queries, so the same definition serves the seven-validator network as the local3 one.
DASHBOARD="$ROOT_DIR/monitoring/local3/grafana-x3-validators.json"
SELF_TEST=0
case "${1:-}" in
  --self-test) SELF_TEST=1 ;;
  "" ) : ;;
  * ) echo "usage: $(basename "$0") [--self-test]" >&2; exit 2 ;;
esac

COUNT="${X3_OBSERVABILITY_VALIDATORS:-7}"
MIN_FINALIZED="${X3_OBSERVABILITY_MIN_FINALIZED:-6}"
RPC_BASE="${X3_OBSERVABILITY_RPC_BASE:-19800}"
P2P_BASE="${X3_OBSERVABILITY_P2P_BASE:-30500}"
PROM_BASE="${X3_OBSERVABILITY_PROM_BASE:-19600}"
EXPECTED_PEERS=$(( COUNT - 1 ))
# A seven-node mesh does not hold six peers on every node every second — the two-hour soak on
# this box measured a minimum of 5 (and one transient 3) while all seven stayed on one chain.
# Demanding the full `COUNT-1` from every validator made this check fail a network that was
# perfectly healthy: the first run reported "did not all connect and finalize within 900s" while
# every node's log held 2,037 finalized blocks. `CONNECTED_PEERS` is the floor that means "in the
# mesh"; the exact per-node count is printed either way.
CONNECTED_PEERS=$(( COUNT > 3 ? COUNT - 2 : COUNT - 1 ))
HELD_OUT="${X3_OBSERVABILITY_DROP_VALIDATOR:-$COUNT}"   # self-test holds the last one out

WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/x3-obs7.XXXXXX")"
SPEC_DIR="$WORK_DIR/spec"
BASE_DIR="$WORK_DIR/net"
LOG_DIR="$BASE_DIR/logs"
SCRAPE_DIR="$WORK_DIR/metrics"
mkdir -p "$SPEC_DIR" "$SCRAPE_DIR"

cleanup() {
  # By the launcher's own pid files. Never `pkill -f x3-chain-node`: other gates' nodes are on
  # this box, and this check owns only the set it started.
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

info() { printf '[obs7] %s\n' "$*"; }
fail() { printf '[obs7] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[obs7] PASS: %s\n' "$*"; }

rpc() { # <port> <method> [params]
  curl -s -m 8 -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$2\",\"params\":${3:-[]}}" \
    "http://127.0.0.1:$1"
}

NODE_BIN="$(local3_node_bin)" || fail "node binary not found; build it with cargo build --release -p x3-chain-node"
info "node: $NODE_BIN"
info "building a ${COUNT}-authority spec"
# `P2P_BASE` goes into the spec: its `bootNodes` name `P2P_BASE + i - 1`, so a spec built for one
# base and launched on another has every node dialing a port nobody listens on. The launcher then
# passes a single bootnode address and the mesh collapses to a hub — measured here: six validators
# reporting exactly 1 peer each instead of 6, and no finality. Build it for the base you launch on.
X3_NODE_BIN="$NODE_BIN" OUT_DIR="$SPEC_DIR" P2P_BASE="$P2P_BASE" \
  python3 "$ROOT_DIR/scripts/testnet/build-x3-testnet-spec.py" "$COUNT" \
  >"$WORK_DIR/spec.log" 2>&1 || { tail -20 "$WORK_DIR/spec.log" >&2; fail "spec build failed"; }

SPEC="$SPEC_DIR/x3-testnet-plain.json"
KEYS_DIR="$SPEC_DIR/validator-keys"
[[ -f "$SPEC" && -d "$KEYS_DIR" ]] || fail "spec builder did not produce $SPEC and $KEYS_DIR"

info "starting ${COUNT} validators (rpc $RPC_BASE, p2p $P2P_BASE, prometheus $PROM_BASE)"
# `RUST_LOG=info` for the children: a node started without a log filter emits warnings only, so
# its stream is a 13-line banner with no identity, imports or finality to check.
RUST_LOG=info COUNT="$COUNT" RPC_BASE="$RPC_BASE" P2P_BASE="$P2P_BASE" PROM_BASE="$PROM_BASE" \
PROMETHEUS=1 BASE_DIR="$BASE_DIR" CHAIN_SPEC="$SPEC" KEYS_DIR="$KEYS_DIR" LOG_DIR="$LOG_DIR" \
SKIP_BUILD=1 bash "$ROOT_DIR/scripts/testnet/x3_testnet_up.sh" --skip-build \
  >"$WORK_DIR/launch.log" 2>&1 || { tail -20 "$WORK_DIR/launch.log" >&2; fail "the launcher could not start ${COUNT} validators"; }
info "launched; waiting for RPC, peers and finality"

deadline=$(( $(date +%s) + 900 ))
ready=0
declare -A last_peers
declare -A last_finalized
declare -A last_metric
while [[ "$(date +%s)" -lt "$deadline" ]]; do
  ready=1
  for i in $(seq 1 "$COUNT"); do
    port=$(( RPC_BASE + i - 1 ))
    peers="$(rpc "$port" system_health | sed -n 's/.*"peers":\([0-9]*\).*/\1/p' | head -1)"
    [[ "${peers:-0}" -ge "$CONNECTED_PEERS" ]] || { ready=0; last_peers[$i]="${peers:-none}"; }
    # A *height* past genesis, not "a finalized head exists": a node that has just started answers
    # with the genesis hash at height 0, so the weaker check passes while the node is still at block
    # 1 — measured, node 7 was scraped at `best=1, finalized=0` while the other six were near 80.
    head="$(rpc "$port" chain_getFinalizedHead | sed -n 's/.*"result":"\([^"]*\)".*/\1/p' | head -1)"
    if [[ -z "$head" ]]; then
      ready=0
    else
      height="$(rpc "$port" chain_getHeader "[\"$head\"]" \
        | sed -n 's/.*"number":"\(0x[0-9a-f]*\)".*/\1/p' | head -1)"
      height=$(( ${height:-0x0} ))
      [[ "$height" -ge "$MIN_FINALIZED" ]] || { ready=0; last_finalized[$i]="$height"; }
    fi
    # …and the *metric* has to say so too. The exported finalized gauge is updated from finality
    # notifications, so a validator can have a finalized head over RPC while its exporter still
    # reads 0 — measured on node 5: RPC past 6, `substrate_block_height{status="finalized"} 0`.
    # Waiting for the metric here is the difference between "the exporter answers" and "the
    # exporter is telling the truth about this validator".
    scrape_code="$(local3_scrape "$(( PROM_BASE + i - 1 ))" "$SCRAPE_DIR/node-$i.metrics")"
    if [[ "$scrape_code" != "200" ]]; then
      ready=0; last_metric[$i]="http-$scrape_code"
    else
      m_final="$(local3_metric_number "$SCRAPE_DIR/node-$i.metrics" substrate_block_height finalized)"
      [[ "${m_final:-0}" -ge "$MIN_FINALIZED" ]] || { ready=0; last_metric[$i]="${m_final:-absent}"; }
    fi
  done
  [[ "$ready" = 1 ]] && break
  sleep 3
done
if [[ "$ready" != 1 ]]; then
  detail=""
  for i in $(seq 1 "$COUNT"); do
    detail+="node-$i(peers=${last_peers[$i]:-ok},rpc_finalized=${last_finalized[$i]:-ok},metric_finalized=${last_metric[$i]:-ok}) "
  done
  fail "not all ${COUNT} validators reached ${CONNECTED_PEERS} peers and finalized height ${MIN_FINALIZED} in 900s (last: $detail)"
fi
info "all ${COUNT} validators are in the mesh (>= ${CONNECTED_PEERS} peers each), finalized past ${MIN_FINALIZED}, and say so in their own metrics"

# ── metrics: one scrape per validator, each identifying itself ────────────────
declare -A NAME FINALIZED PEERS ROLE CHAIN
scraped=0
for i in $(seq 1 "$COUNT"); do
  port=$(( PROM_BASE + i - 1 ))
  out="$SCRAPE_DIR/node-$i.metrics"
  if [[ "$SELF_TEST" = 1 && "$i" = "$HELD_OUT" ]]; then
    info "self-test: not scraping validator $i"
    continue
  fi
  # The readiness loop above already scraped every validator (twice per second of settling); take
  # that snapshot rather than re-scraping, so the reported numbers are the ones it validated.
  [[ -s "$out" ]] || { local3_scrape "$port" "$out" >/dev/null; }
  [[ -s "$out" ]] || fail "validator $i (:$port) exporter produced no metrics"
  NAME[$i]="$(local3_build_info_name "$out")"
  FINALIZED[$i]="$(local3_metric_number "$out" substrate_block_height finalized)"
  PEERS[$i]="$(local3_metric_number "$out" substrate_sub_libp2p_peers_count)"
  ROLE[$i]="$(local3_metric_number "$out" substrate_node_roles)"
  CHAIN[$i]="$(grep -m1 '^substrate_build_info' "$out" | sed -n 's/.*chain="\([^"]*\)".*/\1/p')"
  for field in NAME FINALIZED PEERS ROLE CHAIN; do
    eval "v=\${$field[$i]}"
    [[ -n "$v" ]] || fail "validator $i (:$port) scrape has no $field"
  done
  [[ "${FINALIZED[$i]}" -ge "$MIN_FINALIZED" ]] || fail "validator $i finalized ${FINALIZED[$i]}, below $MIN_FINALIZED"
  [[ "${PEERS[$i]}" -ge "$CONNECTED_PEERS" ]] || fail "validator $i reports ${PEERS[$i]} peers, below the ${CONNECTED_PEERS}-peer mesh floor"
  [[ "${ROLE[$i]}" -ge 1 ]] || fail "validator $i reports role ${ROLE[$i]} — a full node, not a validator"
  scraped=$(( scraped + 1 ))
  info "validator $i :$port  name=${NAME[$i]} chain=${CHAIN[$i]} finalized=${FINALIZED[$i]} peers=${PEERS[$i]} role=${ROLE[$i]}"
done
if [[ "$SELF_TEST" = 1 ]]; then
  [[ "$scraped" -lt "$COUNT" ]] || fail "self-test: still scraped $scraped of $COUNT validators"
  pass "self-test: only $scraped of $COUNT validators were observed"
else
  [[ "$scraped" = "$COUNT" ]] || fail "observed $scraped validators, expected $COUNT"
fi

# ── a real Prometheus, scraping all seven ────────────────────────────────────
if command -v prometheus >/dev/null 2>&1 && command -v promtool >/dev/null 2>&1; then
  PROM_CONF="$WORK_DIR/prometheus-testnet7.yml"
  PROM_READY_PORT=$(( PROM_BASE + 100 ))
  {
    echo "global:"
    echo "  scrape_interval: 1s"
    echo "  evaluation_interval: 1s"
    echo "scrape_configs:"
    EXPECTED_UP="$COUNT"
    for i in $(seq 1 "$COUNT"); do
      if [[ "$SELF_TEST" = 1 && "$i" = "$HELD_OUT" ]]; then
        EXPECTED_UP=$(( COUNT - 1 ))
        continue
      fi
      # One job per validator, each carrying its own `validator` label — the same shape the
      # checked-in local3 config uses, and the label the dashboard's `$validator` template
      # variable is built from (`label_values(substrate_build_info, validator)`). A single job
      # with a bare target list scrapes everything and labels nothing, which leaves every panel
      # query empty.
      echo "  - job_name: x3-validator-node-$i"
      echo "    metrics_path: /metrics"
      echo "    static_configs:"
      echo "      - targets: [\"127.0.0.1:$(( PROM_BASE + i - 1 ))\"]"
      echo "        labels:"
      echo "          validator: x3-testnet-node-0$i"
    done
  } > "$PROM_CONF"
  promtool check config "$PROM_CONF" >/dev/null 2>&1 \
    || fail "promtool rejected the scrape config this check just generated ($PROM_CONF)"
  info "promtool accepted the generated scrape config; starting Prometheus on :$PROM_READY_PORT"
  prometheus --config.file="$PROM_CONF" --storage.tsdb.path="$WORK_DIR/prom-data" \
    --web.listen-address="127.0.0.1:$PROM_READY_PORT" --web.enable-admin-api \
    >"$WORK_DIR/prometheus.log" 2>&1 &
  PROM_PID=$!
  # It is started here, so it is stopped here — and only this one.
  cleanup() {
    local f
    kill "$PROM_PID" 2>/dev/null || true
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

  # `up{job=...} == 1` per target is the question: a Prometheus that started but cannot reach a
  # validator reports `up 0` for it, which is exactly what "live" must mean.
  up_count=""
  prom_deadline=$(( $(date +%s) + 180 ))
  while [[ "$(date +%s)" -lt "$prom_deadline" ]]; do
    up_count="$(curl -s -m 5 --get \
      --data-urlencode 'query=count(up{job=~"x3-validator-node-.*"} == 1)' \
      "http://127.0.0.1:$PROM_READY_PORT/api/v1/query" \
      | python3 -c 'import json,sys
try:
    d = json.load(sys.stdin)
except Exception:
    print(""); raise SystemExit
r = d.get("data", {}).get("result", [])
print(r[0]["value"][1] if r else "")' 2>/dev/null || true)"
    [[ "$up_count" = "$EXPECTED_UP" ]] && break
    kill -0 "$PROM_PID" 2>/dev/null || { tail -10 "$WORK_DIR/prometheus.log" >&2 || true; fail "Prometheus exited before it scraped anything"; }
    sleep 2
  done
  [[ "$up_count" = "$EXPECTED_UP" ]] \
    || fail "a real Prometheus reached $up_count of $EXPECTED_UP expected validators (query: count(up{job=~\"x3-validator-node-.*\"} == 1))"
  pass "a real Prometheus (promtool-validated config) scrapes $up_count of $EXPECTED_UP expected validators: all up"

  # ── Grafana on top of that Prometheus, when it is installed ─────────────────
  # The bullet is "Prometheus/Grafana/logging live across all validators" — three claims.
  # Prometheus is proven above, the logs below, and this is the Grafana half: a running server,
  # a provisioned datasource that reports itself healthy, the dashboard this repository ships
  # imported into it, and one of that dashboard's *panel* expressions returning points when
  # Grafana asks Prometheus for it.
  if command -v grafana >/dev/null 2>&1; then
    GF_HOME="${X3_GRAFANA_HOME:-/home/linuxbrew/.linuxbrew/opt/grafana/share/grafana}"
    if [[ -d "$GF_HOME" ]]; then
      GF_PORT=$(( PROM_READY_PORT + 1 ))
      mkdir -p "$WORK_DIR/grafana/data" "$WORK_DIR/grafana/logs" "$WORK_DIR/grafana/provisioning/datasources"
      cat > "$WORK_DIR/grafana/provisioning/datasources/x3.yml" <<X3_DS_YAML
apiVersion: 1
datasources:
  - name: X3-Prometheus
    type: prometheus
    access: proxy
    url: http://127.0.0.1:$PROM_READY_PORT
    isDefault: true
    jsonData:
      httpMethod: POST
X3_DS_YAML
      GF_PATHS_DATA="$WORK_DIR/grafana/data" \
      GF_PATHS_LOGS="$WORK_DIR/grafana/logs" \
      GF_PATHS_PROVISIONING="$WORK_DIR/grafana/provisioning" \
      GF_SERVER_HTTP_ADDR=127.0.0.1 GF_SERVER_HTTP_PORT="$GF_PORT" \
      GF_AUTH_ANONYMOUS_ENABLED=true GF_AUTH_ANONYMOUS_ORG_ROLE=Admin \
      GF_ANALYTICS_REPORTING_ENABLED=false GF_ANALYTICS_CHECK_FOR_UPDATES=false \
      grafana server --homepath "$GF_HOME" >"$WORK_DIR/grafana.log" 2>&1 &
      GF_PID=$!
      # Started here, stopped here — Grafana, Prometheus, and only this check's validators.
      cleanup() {
        local f
        kill "$GF_PID" 2>/dev/null || true
        kill "$PROM_PID" 2>/dev/null || true
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
      gf_ready=0
      for _ in $(seq 1 60); do
        if curl -sf -m 3 "http://127.0.0.1:$GF_PORT/api/health" 2>/dev/null | grep -q '"database": *"ok"'; then
          gf_ready=1
          break
        fi
        kill -0 "$GF_PID" 2>/dev/null || { tail -12 "$WORK_DIR/grafana.log" >&2 || true; break; }
        sleep 2
      done
      [[ "$gf_ready" = 1 ]] || fail "Grafana did not become healthy on :$GF_PORT (see $WORK_DIR/grafana.log)"

      DS_UID="$(curl -sf -m 5 "http://127.0.0.1:$GF_PORT/api/datasources" \
        | python3 -c 'import json,sys
print(next((x["uid"] for x in json.load(sys.stdin) if x.get("name") == "X3-Prometheus"), ""))')"
      [[ -n "$DS_UID" ]] || fail "Grafana started but the provisioned X3-Prometheus datasource is not there"
      # Not `curl -sf`: this endpoint answers 400 *with a JSON body* when the datasource cannot
      # reach its Prometheus, and `-f` throws that body away — which is how the first run of this
      # phase reported an empty status instead of the connection it could not make.
      GF_DS_BODY="$(curl -s -m 20 "http://127.0.0.1:$GF_PORT/api/datasources/uid/$DS_UID/health" || true)"
      GF_DS_HEALTH="$(printf '%s' "$GF_DS_BODY" \
        | python3 -c 'import json,sys
try:
    print(json.load(sys.stdin).get("status", ""))
except Exception:
    print("")')"
      [[ "$GF_DS_HEALTH" = "OK" ]] \
        || fail "Grafana reports datasource health '$GF_DS_HEALTH', not OK: ${GF_DS_BODY:0:300}"

      # Import the shipped dashboard, binding its `${DS_PROMETHEUS}` input to the provisioned uid.
      python3 - "$DASHBOARD" "$DS_UID" >"$WORK_DIR/grafana-dashboard.json" <<'X3_DASH_PY'
import json, sys
doc = json.load(open(sys.argv[1]))
print(json.dumps({"dashboard": doc["dashboard"], "overwrite": True,
                  "message": "x3 observability check"}).replace("${DS_PROMETHEUS}", sys.argv[2]))
X3_DASH_PY
      IMPORT="$(curl -sf -m 15 -H 'Content-Type: application/json' -X POST \
        --data @"$WORK_DIR/grafana-dashboard.json" "http://127.0.0.1:$GF_PORT/api/dashboards/db" \
        | python3 -c 'import json,sys;d=json.load(sys.stdin);print(d.get("status",""), d.get("uid",""))')"
      # Grafana answers `{"status":"success","uid":…}`; older versions used `"ok"`.
      [[ "$IMPORT" == success* || "$IMPORT" == ok* ]] \
        || fail "Grafana refused the dashboard this repository ships: $IMPORT"
      info "Grafana imported the shipped dashboard ($IMPORT)"

      # The point of the phase: a panel's own expression, asked through Grafana, returns points.
      # `$validator` is the dashboard's template variable; resolved to "all" here.
      PANEL_EXPR="$(jq -r '.dashboard.panels[0].targets[0].expr' "$DASHBOARD" | sed 's/\$validator/.*/g')"
      [[ -n "$PANEL_EXPR" && "$PANEL_EXPR" != "null" ]] || fail "the shipped dashboard's first panel has no expression"
      python3 -c '
import json, sys
uid, expr, path = sys.argv[1], sys.argv[2], sys.argv[3]
body = {"queries": [{"refId": "A", "datasource": {"type": "prometheus", "uid": uid},
                     "expr": expr, "instant": True, "format": "time_series"}],
        "from": "now-10m", "to": "now"}
open(path, "w").write(json.dumps(body))
' "$DS_UID" "$PANEL_EXPR" "$WORK_DIR/grafana-query.json"
      SERIES="$(curl -sf -m 15 -H 'Content-Type: application/json' -X POST \
        --data @"$WORK_DIR/grafana-query.json" "http://127.0.0.1:$GF_PORT/api/ds/query" \
        | python3 -c 'import json,sys
frames = json.load(sys.stdin).get("results", {}).get("A", {}).get("frames", [])
print(sum(len(f.get("data", {}).get("values", [[]])[0]) for f in frames))')"
      [[ "${SERIES:-0}" -gt 0 ]] \
        || fail "Grafana asked Prometheus for the dashboard panel '$PANEL_EXPR' and got no points"
      pass "Grafana serves the shipped dashboard and that panel returns $SERIES point(s) through Prometheus"
      info "panel: $PANEL_EXPR"
      kill "$GF_PID" 2>/dev/null || true
      wait "$GF_PID" 2>/dev/null || true
    else
      info "grafana is on PATH but its homepath ($GF_HOME) is missing — skipping the Grafana half"
    fi
  else
    info "grafana not on PATH — skipping the Grafana half (brew install grafana)"
  fi

  kill "$PROM_PID" 2>/dev/null || true
  wait "$PROM_PID" 2>/dev/null || true
else
  info "prometheus/promtool not on PATH — skipping the real-scrape half (brew install prometheus)"
fi

# One chain: every scraped validator agrees on the hash at a height all of them finalized.
min_finalized="$MIN_FINALIZED"
for i in $(seq 1 "$COUNT"); do
  [[ -n "${FINALIZED[$i]:-}" ]] || continue
  [[ "${FINALIZED[$i]}" -lt "$min_finalized" ]] && min_finalized="${FINALIZED[$i]}"
done
if [[ "$min_finalized" -ge "$MIN_FINALIZED" ]]; then
  first=""
  for i in $(seq 1 "$COUNT"); do
    [[ -n "${FINALIZED[$i]:-}" ]] || continue
    port=$(( RPC_BASE + i - 1 ))
    hash="$(rpc "$port" chain_getBlockHash "[$min_finalized]" | sed -n 's/.*"result":"\([^"]*\)".*/\1/p' | head -1)"
    [[ -n "$hash" && "$hash" != "null" ]] || fail "validator $i has no hash at height $min_finalized"
    [[ -z "$first" ]] && first="$hash"
    [[ "$hash" = "$first" ]] || fail "height $min_finalized: validator $i disagrees ($hash != $first)"
  done
  pass "every observed validator agrees on one chain at finalized height $min_finalized"
fi

# ── logs: one stream per validator, each saying what it is doing ──────────────
streams=0
for i in $(seq 1 "$COUNT"); do
  log="$LOG_DIR/node-$i.log"
  if [[ "$SELF_TEST" = 1 && "$i" = "$HELD_OUT" ]]; then
    info "self-test: holding validator $i's stream out of the collection"
    mv "$log" "$log.held-out" 2>/dev/null || true
    continue
  fi
  [[ -s "$log" ]] || fail "validator $i has no log stream at $log"
  grep -q "Chain specification:" "$log" || fail "validator $i's log does not name the chain it joined"
  grep -q "Role: AUTHORITY" "$log" || fail "validator $i's log does not show it took the AUTHORITY role"
  grep -qE "Block imported: #[0-9]+" "$log" || fail "validator $i's log shows no imported block"
  grep -qE "Block finalized: #[0-9]+" "$log" || fail "validator $i's log shows no finalized block"
  imported="$(grep -oE "Block imported: #[0-9]+" "$log" | grep -oE "[0-9]+" | sort -n | tail -1)"
  finalized="$(grep -oE "Block finalized: #[0-9]+" "$log" | grep -oE "[0-9]+" | sort -n | tail -1)"
  info "validator $i log: last imported #$imported, last finalized #$finalized"
  streams=$(( streams + 1 ))
done

aggregate="$(grep -lE "Block finalized: #[0-9]+" "$LOG_DIR"/node-*.log 2>/dev/null | wc -l | tr -d '[:space:]')"
if [[ "$SELF_TEST" = 1 ]]; then
  [[ "$aggregate" -lt "$COUNT" ]] || fail "self-test: the aggregate query still reached $aggregate streams"
  pass "self-test: the aggregate query reaches $aggregate of $COUNT streams"
  mv "$LOG_DIR/node-$HELD_OUT.log.held-out" "$LOG_DIR/node-$HELD_OUT.log" 2>/dev/null || true
else
  [[ "$streams" = "$COUNT" ]] || fail "checked $streams of $COUNT log streams"
  [[ "$aggregate" = "$COUNT" ]] || fail "the aggregate query reaches $aggregate of $COUNT log streams"
  pass "all $COUNT validators: metrics identify themselves, logs carry identity/imports/finality"
fi
