#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/logs/swarm"
API_HEALTH_URL="http://127.0.0.1:8787/health"

mkdir -p "$LOG_DIR"

API_BIN=""
API_BIN_CANDIDATES=(
  "$ROOT_DIR/target/debug/x3-swarm-api"
  "$ROOT_DIR/crates/x3-swarm-core/services/x3-swarm-api/target/debug/x3-swarm-api"
  "$ROOT_DIR/services/x3-swarm-api/target/debug/x3-swarm-api"
)
WORKER_BIN=""
WORKER_BIN_CANDIDATES=(
  "$ROOT_DIR/target/debug/x3-swarm-worker"
  "$ROOT_DIR/services/x3-swarm-worker/target/debug/x3-swarm-worker"
  "$ROOT_DIR/crates/x3-swarm-core/services/x3-swarm-worker/target/debug/x3-swarm-worker"
)

pid_alive() {
  local pid_file="$1"
  if [ -f "$pid_file" ]; then
    local pid
    pid=$(<"$pid_file")
    if [[ "$pid" =~ ^[0-9]+$ ]] && ps -p "$pid" >/dev/null 2>&1; then
      return 0
    fi
    rm -f "$pid_file"
  fi
  return 1
}

for candidate in "${API_BIN_CANDIDATES[@]}"; do
  if [ -x "$candidate" ]; then
    API_BIN="$candidate"
    break
  fi
done
for candidate in "${WORKER_BIN_CANDIDATES[@]}"; do
  if [ -x "$candidate" ]; then
    WORKER_BIN="$candidate"
    break
  fi
done

echo "== X3 Swarm Up =="

if [ -x "$ROOT_DIR/scripts/gpu/start_ollama_workers.sh" ]; then
  "$ROOT_DIR/scripts/gpu/start_ollama_workers.sh" || true
fi

echo "Starting swarm API if available..."

if [ -n "$API_BIN" ]; then
  if pid_alive "$LOG_DIR/x3-swarm-api.pid"; then
    echo "x3-swarm-api already running (pid $(<"$LOG_DIR/x3-swarm-api.pid"))"
  elif curl -fsS "$API_HEALTH_URL" >/dev/null 2>&1; then
    echo "x3-swarm-api already responding at $API_HEALTH_URL"
  else
    echo "Launching x3-swarm-api..."
    nohup "$API_BIN" > "$LOG_DIR/x3-swarm-api.log" 2>&1 &
    api_pid=$!
    sleep 0.5
    if ps -p "$api_pid" >/dev/null 2>&1; then
      echo "$api_pid" > "$LOG_DIR/x3-swarm-api.pid"
      echo "x3-swarm-api started (pid $api_pid, logs: $LOG_DIR/x3-swarm-api.log)"
    else
      echo "ERROR: x3-swarm-api failed to start. Check $LOG_DIR/x3-swarm-api.log"
    fi
  fi
else
  echo "ERROR: x3-swarm-api binary not found. Build it with cargo build --manifest-path $ROOT_DIR/crates/x3-swarm-core/services/x3-swarm-api/Cargo.toml"
fi

if [ -n "$WORKER_BIN" ]; then
  if pid_alive "$LOG_DIR/x3-swarm-worker.pid"; then
    echo "x3-swarm-worker already running (pid $(<"$LOG_DIR/x3-swarm-worker.pid"))"
  else
    echo "Launching x3-swarm-worker..."
    nohup "$WORKER_BIN" > "$LOG_DIR/x3-swarm-worker.log" 2>&1 &
    worker_pid=$!
    sleep 0.5
    if ps -p "$worker_pid" >/dev/null 2>&1; then
      echo "$worker_pid" > "$LOG_DIR/x3-swarm-worker.pid"
      echo "x3-swarm-worker started (pid $worker_pid, logs: $LOG_DIR/x3-swarm-worker.log)"
    else
      echo "ERROR: x3-swarm-worker failed to start. Check $LOG_DIR/x3-swarm-worker.log"
    fi
  fi
else
  echo "WARNING: x3-swarm-worker binary not found. Build it with cargo build --manifest-path $ROOT_DIR/crates/x3-swarm-core/services/x3-swarm-worker/Cargo.toml"
fi

echo "Waiting for API to become healthy..."
sleep 2
if command -v curl >/dev/null 2>&1 && curl -fsS --connect-timeout 2 --max-time 5 "$API_HEALTH_URL" >/dev/null 2>&1; then
  echo "Seeding swarm task queue..."
  "$ROOT_DIR/scripts/swarm/swarm_task_queue.sh" > "$ROOT_DIR/reports/swarm_task_queue.json" || true
else
  echo "API did not become healthy in time; skipping task seeding."
fi

echo "X3 swarm startup complete. Use scripts/swarm/swarm_health.sh to verify components."
