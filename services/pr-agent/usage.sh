#!/usr/bin/env bash
# Reports what PR-Agent spent, read from the x3-ai-router usage table.
#
#   usage.sh                      # today, every provider
#   usage.sh 2026-10-24           # one day
#   usage.sh today deepseek_flash # one provider
#   PR_AGENT_USAGE_FILE=x.json usage.sh 2026-10-24   # offline, from a saved snapshot
#
# Only runs recorded through `review.sh --via-router` appear here: a direct
# DeepSeek call never passes the router, so there is nothing to account for it.
set -euo pipefail

ROUTER_BASE="${X3_ROUTER_BASE:-http://127.0.0.1:11435/v1}"
DAY="${1:-today}"
PROVIDER="${2:-}"

if [ "$DAY" = "today" ]; then
  DAY="$(date -u +%Y-%m-%d)"
fi

if [ -n "${PR_AGENT_USAGE_FILE:-}" ]; then
  [ -r "$PR_AGENT_USAGE_FILE" ] || { echo "error: cannot read $PR_AGENT_USAGE_FILE" >&2; exit 1; }
  source_command=(cat "$PR_AGENT_USAGE_FILE")
else
  # The router authenticates every GET, /health and /v1/usage included.
  auth=()
  [ -n "${X3_ROUTER_TOKEN:-}" ] && auth=(-H "Authorization: Bearer $X3_ROUTER_TOKEN")
  curl -sf -m 5 "${auth[@]}" "${ROUTER_BASE%/v1}/health" >/dev/null \
    || { echo "error: x3-ai-router is not answering at $ROUTER_BASE (or X3_ROUTER_TOKEN is wrong)" >&2; exit 1; }
  source_command=(curl -sf -m 10 "${auth[@]}" "${ROUTER_BASE%/v1}/v1/usage")
fi

"${source_command[@]}" | python3 -c '
import json, sys

day, provider_filter = sys.argv[1], sys.argv[2]
try:
    rows = json.load(sys.stdin).get("usage", [])
except json.JSONDecodeError as error:
    sys.exit("error: router returned non-JSON usage: " + str(error))

selected = [
    row for row in rows
    if row.get("day") == day
    and (not provider_filter or row.get("provider") == provider_filter)
]
if not selected:
    suffix = " on " + provider_filter if provider_filter else ""
    print("no usage recorded for " + day + suffix)
    raise SystemExit(0)

width = max(len(row.get("provider") or "") for row in selected)
total = 0.0
print(day + ":")
for row in sorted(selected, key=lambda r: r.get("provider") or ""):
    cost = row.get("cost_usd") or 0.0
    total += cost
    note = " (free)" if cost == 0 else ""
    print("  {:<{w}}  {:>4} req  ${:.4f}{}".format(
        row.get("provider") or "?", row.get("requests") or 0, cost, note, w=width))
print("  total: ${:.4f}".format(total))
' "$DAY" "$PROVIDER"
