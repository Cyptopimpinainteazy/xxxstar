#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_URL="http://127.0.0.1:8787"
OUT_DIR="$ROOT_DIR/reports"
OUT_JSON="$OUT_DIR/swarm_task_queue.json"

mkdir -p "$OUT_DIR"

cargo run -p x3-readiness -- swarm-tasks --out "$OUT_DIR" >/dev/null

if [ ! -f "$OUT_JSON" ]; then
  echo "ERROR: swarm task queue was not generated at $OUT_JSON" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required to synchronize swarm tasks" >&2
  exit 1
fi

if command -v curl >/dev/null 2>&1 && curl -fsS --connect-timeout 2 --max-time 5 "$API_URL/health" >/dev/null 2>&1; then
  echo "API available at $API_URL. Syncing task queue to swarm API..." >&2
  export TASK_QUEUE="$(cat "$OUT_JSON")"
  python3 - <<'PY'
import json, os, sys, urllib.error, urllib.request
api = os.environ.get('API_URL', 'http://127.0.0.1:8787')
tasks = json.loads(os.environ['TASK_QUEUE'])
results = []
for task in tasks:
    req = urllib.request.Request(api + '/tasks', data=json.dumps(task).encode('utf-8'), headers={'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            results.append(json.loads(resp.read().decode('utf-8')))
    except urllib.error.HTTPError as error:
        body = error.read().decode('utf-8')
        if error.code == 409:
            results.append({'title': task.get('title'), 'status': 'duplicate', 'api_response': body})
            continue
        print(f'ERROR: failed to create task {task.get("title")}: HTTP {error.code} {body}', file=sys.stderr)
        raise SystemExit(1)
    except urllib.error.URLError as error:
        print(f'ERROR: failed to create task {task.get("title")}: {error}', file=sys.stderr)
        raise SystemExit(1)
print(json.dumps(results, indent=2))
PY
else
  cat "$OUT_JSON"
  echo "
Swarm API unavailable; generated task queue is output only. Run scripts/swarm/swarm_up.sh first to synchronize tasks." >&2
fi
