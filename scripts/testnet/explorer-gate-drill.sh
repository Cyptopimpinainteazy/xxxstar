#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# scripts/testnet/explorer-gate-drill.sh
#
# Criterion 14 of the public testnet gate ("explorer/dashboard reachable") is the one
# criterion that depends on something outside the chain, and it has been wrong in both
# directions on this box:
#
#   * it passed while nothing served a block explorer — any HTTP service on 3000/3001/8080
#     satisfied it, including the wallet app's own `next dev`; the recorded "14 of 15 PASS"
#     run was one of those.
#   * it then failed on a healthy seven-validator network, because `apps/explorer` exists but
#     nothing starts it. A criterion that is red for a reason the gate cannot fix is a
#     criterion an operator learns to ignore.
#
# This drill makes the criterion mean something in both directions and leaves the evidence:
#
#   1. decoy    — a plain HTTP server on the pinned port answers 200 with a page that is not
#                 an explorer. Criterion 14 must FAIL, and say the URL answered without
#                 identifying itself.
#   2. matching — the real `apps/explorer` (Next.js) serves on the same port, pointed at a stub
#                 JSON-RPC that reports finalized head #4242, and the gate is pointed at the same
#                 stub. Criterion 14 must PASS, name the URL, and the page must show #4242.
#   3. mismatch — the same explorer, with the gate pointed at a *different* stub reporting
#                 #9999. Criterion 14 must FAIL and say which number it saw where.
#
# It does not boot a chain: the other fourteen criteria are not this drill's business, and the
# run is expected to end `public_testnet_gate: FAIL` for reasons that have nothing to do with
# the explorer. What is asserted is criterion 14's row in reports/public_testnet_gate.md, both
# times, plus the exit code of each phase (decoy must fail the gate, positive need not pass it).
#
# Usage:
#   bash scripts/testnet/explorer-gate-drill.sh [--port 3410] [--keep]
#
# Exit 0 -> criterion 14 refused the decoy and passed on the real explorer, naming it.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PORT=3410
KEEP=0
while [[ $# -gt 0 ]]; do
    case "$1" in
        --port) PORT="$2"; shift 2 ;;
        --keep) KEEP=1; shift ;;
        -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

EXPLORER_URL="http://127.0.0.1:${PORT}"
RPC_MATCH_PORT=$((PORT + 1))
RPC_OTHER_PORT=$((PORT + 2))
RPC_MATCH="http://127.0.0.1:${RPC_MATCH_PORT}"
RPC_OTHER="http://127.0.0.1:${RPC_OTHER_PORT}"
REPORT="$ROOT_DIR/reports/public_testnet_gate.md"
REPORT_BEFORE="$(mktemp)"
WORK_DIR="$(mktemp -d)"
DECOY_PID=""
EXPLORER_PID=""
RPC_PIDS=()

# Keep the operator's report: this drill overwrites reports/public_testnet_gate.md twice.
if [[ -f "$REPORT" ]]; then cp "$REPORT" "$REPORT_BEFORE"; fi

cleanup() {
    local status=$?
    [[ -n "$DECOY_PID" ]] && kill "$DECOY_PID" 2>/dev/null || true
    # `npx next start` is a shell that execs a child, and it was the child (`next-server`) that
    # held the port: killing the pid we recorded left a listener behind, and the next run then
    # failed its own port preflight. The server is started with `setsid`, so its pid is its
    # process-group id and killing the group takes the whole tree.
    [[ -n "$EXPLORER_PID" ]] && kill -TERM -"$EXPLORER_PID" 2>/dev/null || true
    [[ -n "$EXPLORER_PID" ]] && kill "$EXPLORER_PID" 2>/dev/null || true
    for pid in "${RPC_PIDS[@]:-}"; do
        [[ -n "$pid" ]] && kill -TERM -"$pid" 2>/dev/null || true
        [[ -n "$pid" ]] && kill "$pid" 2>/dev/null || true
    done
    pkill -f -- "http.server ${PORT}" 2>/dev/null || true
    pkill -f -- "fake-rpc.py ${RPC_MATCH_PORT}" 2>/dev/null || true
    pkill -f -- "fake-rpc.py ${RPC_OTHER_PORT}" 2>/dev/null || true
    if [[ -s "$REPORT_BEFORE" ]]; then cp "$REPORT_BEFORE" "$REPORT"; fi
    rm -f "$REPORT_BEFORE"
    if [[ "$KEEP" == "1" ]]; then
        echo "kept: $WORK_DIR"
    else
        rm -rf "$WORK_DIR"
    fi
    exit "$status"
}
trap cleanup EXIT

say()  { echo "  $*"; }
die()  { echo "FAIL: $*" >&2; exit 1; }

# The same port must be free before either phase starts: a leftover listener is exactly the
# confusion the observability check hit on 2026-09-26 (two runs, one network, a PASS that
# scraped the other run's nodes).
if command -v ss >/dev/null 2>&1 && ss -ltn 2>/dev/null | grep -q ":${PORT}\b"; then
    die "port ${PORT} is already in use (the drill cannot tell its own server from a leftover)"
fi

# A stub X3 JSON-RPC speaking exactly the two calls the explorer and criterion 14 use. `number`
# is what it claims its finalized head is, which is the whole point: the explorer and the gate
# must agree on it.
cat > "$WORK_DIR/fake-rpc.py" <<'PY'
import http.server, json, sys

port, number = int(sys.argv[1]), int(sys.argv[2])


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", 0))
        request = json.loads(self.rfile.read(length) or b"{}")
        method = request.get("method")
        if method == "chain_getFinalizedHead":
            result = "0x" + "ab" * 32
        elif method == "chain_getHeader":
            result = {"number": hex(number), "parentHash": "0x" + "00" * 32}
        else:
            result = None
        body = json.dumps({"jsonrpc": "2.0", "id": request.get("id", 1), "result": result}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


http.server.HTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY

start_fake_rpc() { # <port> <finalized number>
    setsid python3 "$WORK_DIR/fake-rpc.py" "$1" "$2" > "$WORK_DIR/fake-rpc-$1.log" 2>&1 &
    RPC_PIDS+=("$!")
    for _ in $(seq 1 20); do
        curl -sf -m 2 -H 'content-type: application/json' \
            -d '{"jsonrpc":"2.0","id":1,"method":"chain_getFinalizedHead","params":[]}' \
            "http://127.0.0.1:$1" >/dev/null 2>&1 && return 0
        sleep 0.5
    done
    die "the stub RPC on port $1 never came up"
}

# criterion_row <artifact> -> the row 14 line of the report
criterion_row() {
    grep -E '^\| 14 \|' "$1" || die "no criterion 14 row in the report"
}

# run_gate <basename> — runs the gate and keeps both its terminal output (which is where the
# per-URL reasoning goes: "[info] … does not identify itself as the X3 explorer") and its report
# (which is where criterion 14's verdict goes).
run_gate() { # <basename> <rpc-url>
    local base="$1"
    local rpc="$2"
    # The other fourteen criteria are expected to fail against a stub RPC or an unreachable one;
    # this drill only reads criterion 14.
    X3_EXPLORER_URL="$EXPLORER_URL" \
    X3_RPC_URL="$rpc" \
    X3_TESTNET_HOURS=0 \
        bash "$ROOT_DIR/scripts/mainnet/public_testnet_gate.sh" > "$base.log" 2>&1 || true
    cp "$REPORT" "$base"
}

echo "explorer-gate-drill: decoy phase (a plain HTTP server must not satisfy criterion 14)"
cat > "$WORK_DIR/index.html" <<'HTML'
<!doctype html><title>Not an explorer</title><h1>hello</h1>
HTML
(cd "$WORK_DIR" && exec python3 -m http.server "$PORT" --bind 127.0.0.1) > "$WORK_DIR/decoy.log" 2>&1 &
DECOY_PID=$!
for _ in $(seq 1 20); do
    curl -sf -m 2 "$EXPLORER_URL" >/dev/null 2>&1 && break
    sleep 0.5
done
curl -sf -m 5 "$EXPLORER_URL" >/dev/null || die "the decoy server never came up on ${PORT}"
say "decoy answering on $EXPLORER_URL with a page that is not an X3 explorer"

# Unreachable RPC for the decoy: with no chain to compare against, the criterion falls back to
# the identity check, which is what this phase is about.
run_gate "$WORK_DIR/decoy-report.md" "http://127.0.0.1:9"
DECOY_ROW="$(criterion_row "$WORK_DIR/decoy-report.md")"
say "report row: $DECOY_ROW"
grep -q '| FAIL' <<<"$DECOY_ROW" || die "criterion 14 PASSED on a decoy HTTP server: ${DECOY_ROW}"
grep -q 'does not identify itself as the X3 explorer' "$WORK_DIR/decoy-report.md.log" \
    || die "the gate failed the decoy without saying why (expected a line about the body not identifying itself)"
say "the gate said why: $(grep -m1 'does not identify itself as the X3 explorer' "$WORK_DIR/decoy-report.md.log" | sed 's/^ *//')"
kill "$DECOY_PID" 2>/dev/null || true
wait "$DECOY_PID" 2>/dev/null || true
DECOY_PID=""
sleep 1
say "criterion 14 refused the decoy ✔"

echo "explorer-gate-drill: matching phase (the real apps/explorer, reading the stub the gate reads)"
if [[ ! -f "$ROOT_DIR/apps/explorer/package.json" ]]; then
    die "apps/explorer is missing"
fi
start_fake_rpc "$RPC_MATCH_PORT" 4242
say "stub X3 RPC reporting finalized head #4242 at $RPC_MATCH"
if [[ ! -d "$ROOT_DIR/apps/explorer/.next" ]]; then
    if [[ ! -d "$ROOT_DIR/apps/explorer/node_modules" ]]; then
        say "apps/explorer has no node_modules; running 'npm ci' first (this takes a few minutes)"
        (cd "$ROOT_DIR/apps/explorer" && npm ci --no-audit --no-fund --prefer-offline) \
            > "$WORK_DIR/explorer-npm-ci.log" 2>&1 \
            || die "apps/explorer npm ci failed — see $WORK_DIR/explorer-npm-ci.log"
    fi
    say "no .next build; running 'npm run build' in apps/explorer (this takes a few minutes)"
    (cd "$ROOT_DIR/apps/explorer" && npm run build) > "$WORK_DIR/explorer-build.log" 2>&1 \
        || die "apps/explorer failed to build — see $WORK_DIR/explorer-build.log"
fi
# `npx next start` is a shell that execs `next-server`, and it is the child that holds the
# port — killing the captured pid alone left a listener behind and the next run failed its own
# port preflight. Start the server in its own session so cleanup can signal the whole group.
setsid bash -c "cd '$ROOT_DIR/apps/explorer' && X3_EXPLORER_RPC='$RPC_MATCH' exec npx --no-install next start -p '$PORT' -H 127.0.0.1" \
    > "$WORK_DIR/explorer.log" 2>&1 &
EXPLORER_PID=$!

BODY=""
for _ in $(seq 1 60); do
    BODY="$(curl -sf -m 3 "$EXPLORER_URL" 2>/dev/null || true)"
    if grep -qiE "X3 Chain Explorer|X3 Chain Block Explorer|Block explorer for X3" <<<"$BODY"; then
        break
    fi
    BODY=""
    sleep 1
done
if [[ -z "$BODY" ]]; then
    die "apps/explorer never served a body identifying as the X3 explorer on ${PORT} (log: $WORK_DIR/explorer.log)"
fi
say "apps/explorer answering on $EXPLORER_URL ($(wc -c <<<"$BODY" | tr -d '[:space:]') bytes)"
grep -q 'data-x3-explorer-height="4242"' <<<"$BODY" \
    || die "apps/explorer is not showing the finalized head it was given (expected data-x3-explorer-height=\"4242\")"
say "the page shows the head it read from the stub ✔"

run_gate "$WORK_DIR/explorer-report.md" "$RPC_MATCH"
PASS_ROW="$(criterion_row "$WORK_DIR/explorer-report.md")"
say "report row: $PASS_ROW"
grep -q '| PASS' <<<"$PASS_ROW" || die "criterion 14 did not PASS on the real explorer: ${PASS_ROW}"
grep -qF "$EXPLORER_URL" <<<"$PASS_ROW" \
    || die "criterion 14 passed without naming the explorer it reached: ${PASS_ROW}"
grep -q '#4242' <<<"$PASS_ROW" || die "criterion 14 passed without recording the head it checked: ${PASS_ROW}"
say "criterion 14 passed, named $EXPLORER_URL and recorded the head it read ✔"

echo "explorer-gate-drill: mismatch phase (an explorer showing a different chain must be refused)"
start_fake_rpc "$RPC_OTHER_PORT" 9999
say "second stub X3 RPC reporting finalized head #9999 at $RPC_OTHER, while the explorer reads $RPC_MATCH"
run_gate "$WORK_DIR/mismatch-report.md" "$RPC_OTHER"
MISMATCH_ROW="$(criterion_row "$WORK_DIR/mismatch-report.md")"
say "report row: $MISMATCH_ROW"
grep -q '| FAIL' <<<"$MISMATCH_ROW" || die "criterion 14 PASSED while the explorer showed #4242 and the gate read #9999: ${MISMATCH_ROW}"
grep -q '4242' <<<"$MISMATCH_ROW" && grep -q '9999' <<<"$MISMATCH_ROW" \
    || die "criterion 14 failed the mismatch without naming both numbers: ${MISMATCH_ROW}"
say "criterion 14 refused the mismatch and named both heads ✔"
kill -TERM -"${RPC_PIDS[1]}" 2>/dev/null || true
kill "${RPC_PIDS[1]}" 2>/dev/null || true

echo "explorer-gate-drill: no-chain phase (an explorer with no chain must say so, not invent a height)"
# Same explorer, restarted with an endpoint nothing answers on. This is the deployment shape on a
# machine with no node, and the one where a stale or hard-coded number would hide: the criterion
# has no chain to compare against, so it requires the page's own fail-closed marker instead.
kill -TERM -"$EXPLORER_PID" 2>/dev/null || true
kill "$EXPLORER_PID" 2>/dev/null || true
EXPLORER_PID=""
sleep 1
DEAD_RPC="http://127.0.0.1:9"
setsid bash -c "cd '$ROOT_DIR/apps/explorer' && X3_EXPLORER_RPC='$DEAD_RPC' exec npx --no-install next start -p '$PORT' -H 127.0.0.1" \
    > "$WORK_DIR/explorer-norpc.log" 2>&1 &
EXPLORER_PID=$!
DEAD_BODY=""
for _ in $(seq 1 60); do
    DEAD_BODY="$(curl -sf -m 3 "$EXPLORER_URL" 2>/dev/null || true)"
    if grep -q 'data-x3-explorer-error="rpc-unreachable"' <<<"$DEAD_BODY"; then
        break
    fi
    DEAD_BODY=""
    sleep 1
done
grep -q 'data-x3-explorer-error="rpc-unreachable"' <<<"$DEAD_BODY" \
    || die "the explorer did not take its fail-closed branch with no chain at $DEAD_RPC (log: $WORK_DIR/explorer-norpc.log)"
grep -q 'data-x3-explorer-height' <<<"$DEAD_BODY" \
    && die "the explorer shows a height it cannot have read — that is the fail-open case this phase exists for"
say "the page renders no height and states the chain is unreachable ✔"
run_gate "$WORK_DIR/nochain-report.md" "$DEAD_RPC"
NOCHAIN_ROW="$(criterion_row "$WORK_DIR/nochain-report.md")"
say "report row: $NOCHAIN_ROW"
grep -q '| PASS' <<<"$NOCHAIN_ROW" || die "criterion 14 refused an explorer that honestly reports an unreachable chain: ${NOCHAIN_ROW}"
say "criterion 14 accepted the explicit no-chain statement ✔"

# Proof artifacts, so the claim does not live only in this terminal (AGENTS.md: evidence that
# survives the run). Written outside the operator's working set.
ARTIFACT_DIR="$ROOT_DIR/.ai/runlogs/explorer-gate-drill-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$ARTIFACT_DIR"
cp "$WORK_DIR/decoy-report.md" "$WORK_DIR/decoy-report.md.log" \
   "$WORK_DIR/explorer-report.md" "$WORK_DIR/explorer-report.md.log" \
   "$WORK_DIR/mismatch-report.md" "$WORK_DIR/mismatch-report.md.log" \
   "$WORK_DIR/nochain-report.md" "$WORK_DIR/nochain-report.md.log" "$ARTIFACT_DIR/"
{
    echo "# explorer-gate-drill — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "Criterion 14 of the public testnet gate, four directions, on the bytes in this tree."
    echo
    echo "- decoy (plain \`python3 -m http.server\` pinned via \`X3_EXPLORER_URL\` on ${EXPLORER_URL}): FAIL"
    echo "  \`${DECOY_ROW}\`"
    echo "- matching (apps/explorer reading the same stub RPC as the gate, finalized head #4242):"
    echo "  PASS, named, and the head it checked is recorded"
    echo "  \`${PASS_ROW}\`"
    echo "- mismatch (explorer still reading #4242, gate pointed at a stub reporting #9999): FAIL"
    echo "  \`${MISMATCH_ROW}\`"
    echo "- no chain (explorer pointed at an endpoint nothing answers on): PASS, because the page"
    echo "  renders no height and says the chain is unreachable"
    echo "  \`${NOCHAIN_ROW}\`"
} > "$ARTIFACT_DIR/summary.md"

{
    echo
    echo "explorer-gate-drill: PASS"
    echo "  decoy     (not an explorer)        -> FAIL, as required"
    echo "  matching  (same chain as the gate) -> PASS, named, head recorded"
    echo "  mismatch  (different chain)        -> FAIL, both heads named"
    echo "  no chain  (endpoint unreachable)   -> PASS, page renders no height and says so"
    echo "  evidence: $ARTIFACT_DIR/"
}
