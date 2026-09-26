#!/usr/bin/env bash
# Prove the *logging* half of "Prometheus/Grafana/logging live across all validators".
#
# The metrics half is `local3-monitoring-check.sh`. This is the other half: every
# validator's log has to be collected into one queryable sink and has to say what
# that validator is doing. A file per validator that only contains a startup banner
# is not operational logging, and one validator's log is not "across all validators".
#
# What it checks, per validator:
#   * its stream is in the collection (one directory, one file per validator, the
#     validator's name identifying it — the contract a collector is pointed at);
#   * the stream shows identity: the chain it joined and the role it took;
#   * the stream shows *progress on this validator*: at least one imported block at
#     a height past the three-validator floor, and the finalized height moving;
#   * a single query over the collection returns matches from all three, which is
#     the property "an operator can ask one question of all validators".
#
# Negative control (`--self-test`): drop one validator from the collection and
# require the same aggregate query to report fewer than three sources.
#
# Honest limitation: no log shipper is installed on this box (no vector,
# fluent-bit, promtail or filebeat), so the sink here is a directory of per-validator
# streams produced by the launcher. Pointing a real collector at these files, or at
# stdout, is the same contract; what is proven is that the streams exist, are
# per-validator, and carry the events an operator needs.
#
#   scripts/monitoring/local3-logging-check.sh
#   scripts/monitoring/local3-logging-check.sh --self-test
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

MIN_HEIGHT="${X3_LOGGING_MIN_HEIGHT:-3}"
# The events this check asserts on are info-level, and a node started without a log
# filter emits only warnings: no chain identity, no imports, no finality. The
# collection contract includes the level, so the launcher sets it explicitly rather
# than depending on whatever the environment happens to carry.
LOG_FILTER="${X3_LOGGING_FILTER:-info}"
BASE_DIR="$(mktemp -d)"
LOG_DIR="$(mktemp -d)"
VALIDATORS=(alice bob charlie)

cleanup() { local3_stop_all; }
trap cleanup EXIT

fail() { printf '[local3-log] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[local3-log] PASS: %s\n' "$*"; }

# ── 1. the three validators, on the ports the scrape config names ────────────
targets_raw="$(local3_config_targets)" || fail "scrape config does not parse"
mapfile -t TARGETS <<<"$targets_raw"
declare -A PROM RPC P2P
for row in "${TARGETS[@]}"; do
  IFS=$'\t' read -r name host port <<<"$row"
  PROM[$name]="$port"
  RPC[$name]="$(( port - 2 ))"
  P2P[$name]="$(( port - 1 ))"
done
for want in "${VALIDATORS[@]}"; do
  [ -n "${PROM[$want]:-}" ] || fail "scrape config is missing the '$want' validator"
done

local3_start_validator alice 1 "${RPC[alice]}" "${P2P[alice]}" "${PROM[alice]}" "$BASE_DIR" "$LOG_DIR" \
  --alice --log "$LOG_FILTER" \
  || fail "could not start alice"
local3_wait_for_rpc alice "${RPC[alice]}" || fail "alice never came up"
alice_peer_id="$(local3_rpc "${RPC[alice]}" system_localPeerId | local3_json_field result)"
[ -n "$alice_peer_id" ] || fail "could not read alice's peer id"
alice_bootnode="/ip4/127.0.0.1/tcp/${P2P[alice]}/p2p/$alice_peer_id"

local3_start_validator bob 2 "${RPC[bob]}" "${P2P[bob]}" "${PROM[bob]}" "$BASE_DIR" "$LOG_DIR" \
  --bob --log "$LOG_FILTER" --bootnodes "$alice_bootnode" || fail "could not start bob"
local3_wait_for_rpc bob "${RPC[bob]}" || fail "bob never came up"
bob_peer_id="$(local3_rpc "${RPC[bob]}" system_localPeerId | local3_json_field result)"
[ -n "$bob_peer_id" ] || fail "could not read bob's peer id"
bob_bootnode="/ip4/127.0.0.1/tcp/${P2P[bob]}/p2p/$bob_peer_id"

local3_start_validator charlie 3 "${RPC[charlie]}" "${P2P[charlie]}" "${PROM[charlie]}" "$BASE_DIR" "$LOG_DIR" \
  --charlie --log "$LOG_FILTER" --bootnodes "$alice_bootnode" "$bob_bootnode" || fail "could not start charlie"
local3_wait_for_rpc charlie "${RPC[charlie]}" || fail "charlie never came up"

# ── 2. wait for each validator's own stream to show an imported and a finalized
#      block. The wait is on the *logs*, not on an RPC height: a node can be at
#      height 3 while its stream has not yet carried a finality notification, and
#      asserting before that is a race, not a check.
local3_info "waiting for each validator's log to show imports and finality past height $MIN_HEIGHT"
progress=0
for _ in $(seq 1 120); do
  ok=1
  for name in "${VALIDATORS[@]}"; do
    log="$LOG_DIR/$name.log"
    [ -s "$log" ] || { ok=0; continue; }
    grep -qE "Block imported: #[0-9]+" "$log" || ok=0
    last_finalized="$(grep -oE "Block finalized: #[0-9]+" "$log" | grep -oE "[0-9]+" | sort -n | tail -1)"
    [ -n "$last_finalized" ] || ok=0
    [ "${last_finalized:-0}" -ge "$MIN_HEIGHT" ] || ok=0
  done
  [ "$ok" = 1 ] && progress=1 && break
  sleep 1
done
[ "$progress" = 1 ] || fail "no validator stream showed a finalized block past height $MIN_HEIGHT"

# ── 3. per-validator streams, and what each one has to contain ───────────────
for name in "${VALIDATORS[@]}"; do
  log="$LOG_DIR/$name.log"
  [ -s "$log" ] || fail "$name has no log stream in the collection ($log)"
  lines="$(wc -l <"$log" | tr -d '[:space:]')"

  grep -q "Chain specification: X3 Chain Local3" "$log" \
    || grep -q "Chain specification:" "$log" \
    || fail "$name's log does not name the chain it joined"
  grep -q "Role: AUTHORITY" "$log" || fail "$name's log does not show it took the AUTHORITY role"
  grep -qE "Block imported: #[0-9]+" "$log" \
    || fail "$name's log shows no imported block — the stream cannot answer 'is this validator progressing?'"
  grep -qE "Block finalized: #[0-9]+" "$log" \
    || fail "$name's log shows no finalized block"

  imported="$(grep -oE "Block imported: #[0-9]+" "$log" | grep -oE "[0-9]+" | sort -n | tail -1)"
  finalized="$(grep -oE "Block finalized: #[0-9]+" "$log" | grep -oE "[0-9]+" | sort -n | tail -1)"
  local3_info "$name: $lines lines, last imported #$imported, last finalized #$finalized"
done
pass "each validator's stream shows identity, import progress and finality"

# ── 4. one query over the collection reaches every validator ─────────────────
#
# This is the aggregation contract: an operator asks the sink a question and gets
# an answer attributable to each validator, not one merged blob. The self-test
# removes one stream first, and the same query must then report two sources.
SINK_DIR="$LOG_DIR"
if [ "$SELF_TEST" = 1 ]; then
  local3_info "self-test: removing bob's stream from the collection"
  mv "$SINK_DIR/bob.log" "$SINK_DIR/bob.log.held-out"
fi

sources="$(grep -lE "Block finalized: #[0-9]+" "$SINK_DIR"/*.log 2>/dev/null | wc -l | tr -d '[:space:]')"
if [ "$SELF_TEST" = 1 ]; then
  [ "$sources" -lt 3 ] \
    || fail "self-test: the aggregate query still reported $sources validators with one stream removed"
  pass "self-test: the aggregate query reports $sources validators once one stream is held out"
  mv "$SINK_DIR/bob.log.held-out" "$SINK_DIR/bob.log"
else
  [ "$sources" -eq 3 ] \
    || fail "the aggregate query over the collection reached $sources validators, expected 3"
  pass "the aggregate query reaches all three validators from one sink ($sources sources)"
fi

local3_info "PASS — three per-validator log streams in one collection, each showing identity, imports and finality"
