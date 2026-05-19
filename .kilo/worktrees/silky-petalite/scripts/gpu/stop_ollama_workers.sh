#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

PID_FILE="$ROOT_DIR/logs/swarm/ollama_worker.pid"
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if [[ "$PID" =~ ^[0-9]+$ ]] && ps -p "$PID" >/dev/null 2>&1; then
    echo "Stopping Ollama worker PID $PID"
    kill "$PID" 2>/dev/null || true
  else
    echo "Removing stale Ollama worker PID file."
  fi
  rm -f "$PID_FILE"
  echo "Stopped Ollama GPU worker."
else
  echo "No Ollama worker PID file found."
fi
