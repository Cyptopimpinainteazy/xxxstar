#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
LOG_DIR="$ROOT_DIR/logs/swarm"

mkdir -p "$LOG_DIR" "$ROOT_DIR/reports" "$ROOT_DIR/data/agent-memory"

echo "== X3 Swarm Up =="

if [ -x "$ROOT_DIR/scripts/gpu/start_ollama_workers.sh" ]; then
  "$ROOT_DIR/scripts/gpu/start_ollama_workers.sh" || true
fi

echo "Starting swarm API if available..."

if [ -d "$ROOT_DIR/services/x3-swarm-api" ]; then
  (
    cd "$ROOT_DIR/services/x3-swarm-api"
    if [ -f package.json ]; then
      nohup pnpm dev > "$LOG_DIR/x3-swarm-api.log" 2>&1 &
      echo $! > "$LOG_DIR/x3-swarm-api.pid"
    elif [ -f Cargo.toml ]; then
      nohup cargo run > "$LOG_DIR/x3-swarm-api.log" 2>&1 &
      echo $! > "$LOG_DIR/x3-swarm-api.pid"
    else
      echo "No runnable swarm API detected."
    fi
  )
fi

if [ -d "$ROOT_DIR/services/x3-swarm-worker" ]; then
  (
    cd "$ROOT_DIR/services/x3-swarm-worker"
    if [ -f Cargo.toml ]; then
      nohup cargo run > "$LOG_DIR/x3-swarm-worker.log" 2>&1 &
      echo $! > "$LOG_DIR/x3-swarm-worker.pid"
    else
      echo "No runnable swarm worker detected."
    fi
  )
fi

echo "X3 Swarm startup complete."
