#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/logs/swarm"
API_URL="http://127.0.0.1:8787"
HEALTH_URL="$API_URL/health"
STATUS_URL="$API_URL/status"
TASKS_URL="$API_URL/tasks"
CURL_OPTS=(-fsS --connect-timeout 2 --max-time 5)

if ! command -v curl >/dev/null 2>&1; then
  echo "ERROR: curl is required for swarm health checks"
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "ERROR: python3 is required for swarm health checks"
  exit 1
fi

echo "== X3 Swarm Health Check =="

api_ok=false
all_ok=true

if curl "${CURL_OPTS[@]}" "$HEALTH_URL" >/dev/null 2>&1; then
  echo "- API /health: ok"
  api_ok=true
else
  echo "ERROR: API /health failed at $HEALTH_URL"
  all_ok=false
fi

if curl "${CURL_OPTS[@]}" "$STATUS_URL" >/dev/null 2>&1; then
  echo "- API /status: ok"
else
  echo "WARNING: API /status endpoint is unavailable"
fi

if tasks_response=$(curl "${CURL_OPTS[@]}" "$TASKS_URL" 2>/dev/null); then
  task_count=$(printf '%s' "$tasks_response" | python3 -c 'import sys, json; data=json.load(sys.stdin); print(len(data))')
  echo "- API /tasks: $task_count task(s) returned"
else
  echo "WARNING: API /tasks endpoint is unavailable"
fi

echo "== PID health check =="

if [ -f "$LOG_DIR/x3-swarm-api.pid" ]; then
  api_pid=$(<"$LOG_DIR/x3-swarm-api.pid")
  if ps -p "$api_pid" >/dev/null 2>&1; then
    echo "- x3-swarm-api running (pid $api_pid)"
  else
    echo "WARNING: x3-swarm-api pid file exists but process $api_pid is not running"
    if [ "$api_ok" = false ]; then
      all_ok=false
    fi
  fi
else
  echo "WARNING: x3-swarm-api pid file not found"
  if [ "$api_ok" = false ]; then
    all_ok=false
  fi
fi

if [ -f "$LOG_DIR/x3-swarm-worker.pid" ]; then
  worker_pid=$(<"$LOG_DIR/x3-swarm-worker.pid")
  if ps -p "$worker_pid" >/dev/null 2>&1; then
    echo "- x3-swarm-worker running (pid $worker_pid)"
  else
    echo "ERROR: x3-swarm-worker pid file exists but process $worker_pid is not running"
    all_ok=false
  fi
else
  echo "WARNING: x3-swarm-worker pid file not found"
  all_ok=false
fi

if [ -f "$LOG_DIR/ollama_worker_roles.log" ]; then
  echo "- Ollama worker role file present"
else
  echo "WARNING: Ollama worker role log not found"
fi

if [ "$all_ok" = true ]; then
  echo "X3 Swarm Health Check: PASS"
  exit 0
else
  echo "X3 Swarm Health Check: FAIL"
  exit 1
fi

