#!/usr/bin/env bash
# Keep the watcher and loopback-only report server together in one terminal.
set -euo pipefail
readiness_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
readiness_port="${1:-8765}"
readiness_python="${READINESS_PYTHON:-/usr/bin/python3}"
readiness_watch_pid=''
readiness_http_pid=''
cleanup() {
  if [[ -n "$readiness_watch_pid" ]]; then kill "$readiness_watch_pid" 2>/dev/null || true; fi
  if [[ -n "$readiness_http_pid" ]]; then kill "$readiness_http_pid" 2>/dev/null || true; fi
}
trap cleanup EXIT INT TERM
"$readiness_python" "$readiness_root/scripts/readiness/workflow.py" watch &
readiness_watch_pid=$!
"$readiness_python" -m http.server "$readiness_port" --bind 127.0.0.1 --directory "$readiness_root/audit-artifacts/mainnet-readiness" &
readiness_http_pid=$!
printf 'Live report: http://127.0.0.1:%s/live/\nKeep this terminal open. Ctrl-C stops both processes.\n' "$readiness_port"
wait -n "$readiness_watch_pid" "$readiness_http_pid"
