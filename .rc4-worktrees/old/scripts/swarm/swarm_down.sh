#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/logs/swarm"

echo "Stopping X3 swarm components..."
mkdir -p "$LOG_DIR"
for pidfile in "$LOG_DIR"/*.pid; do
  if [ -f "$pidfile" ]; then
    pid=$(cat "$pidfile")
    if [[ "$pid" =~ ^[0-9]+$ ]] && ps -p "$pid" >/dev/null 2>&1; then
      echo "Stopping process $pid from $pidfile"
      kill "$pid" 2>/dev/null || true
    else
      echo "Removing stale PID file $pidfile"
    fi
    rm -f "$pidfile"
  fi
done

if [ -x "$ROOT_DIR/scripts/gpu/stop_ollama_workers.sh" ]; then
  echo "Stopping Ollama GPU workers..."
  "$ROOT_DIR/scripts/gpu/stop_ollama_workers.sh" || true
fi

echo "X3 swarm teardown complete."
