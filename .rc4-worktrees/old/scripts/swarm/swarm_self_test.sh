#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
API_HEALTH_URL="http://127.0.0.1:8787/health"
TASKS_URL="http://127.0.0.1:8787/tasks"

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: curl is required for self-test"
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required for self-test"
  exit 1
fi

echo "Running X3 swarm self-test..."

if curl -fsS "$API_HEALTH_URL" >/dev/null 2>&1; then
  echo "- API health check passed"
else
  echo "ERROR: API health check failed"
  exit 1
fi

tasks_response=$(curl -fsS "$TASKS_URL") || {
  echo "ERROR: Tasks endpoint request failed"
  exit 1
}
TASK_COUNT=$(printf '%s' "$tasks_response" | python3 -c 'import sys, json; data=json.load(sys.stdin); print(len(data))') || {
  echo "ERROR: Tasks endpoint returned invalid JSON"
  exit 1
}
if [ "$TASK_COUNT" -ge 0 ]; then
  echo "- Tasks endpoint returned $TASK_COUNT task(s)"
  if [ "$TASK_COUNT" -eq 0 ]; then
    echo "WARNING: no tasks are currently queued. Run scripts/swarm/swarm_task_queue.sh to seed the swarm."
  fi
else
  echo "ERROR: Tasks endpoint returned invalid data"
  exit 1
fi

echo "- Generating swarm task report"
if "$ROOT_DIR/scripts/swarm/approve_task.sh" report "$ROOT_DIR/reports/swarm_task_summary.md" >/dev/null 2>&1; then
  echo "- Swarm task report created: $ROOT_DIR/reports/swarm_task_summary.md"
else
  echo "ERROR: failed to generate swarm task report"
  exit 1
fi

echo "Self-test completed. If you want to validate worker execution, inspect logs/swarm/x3-swarm-worker.log."
