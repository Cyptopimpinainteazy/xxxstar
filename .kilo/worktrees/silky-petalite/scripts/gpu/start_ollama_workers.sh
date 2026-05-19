#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Starting Ollama GPU workers for X3 swarm..."
mkdir -p "$ROOT_DIR/logs/swarm"

if ! command -v ollama >/dev/null 2>&1; then
  echo "ERROR: ollama executable not found. Install Ollama or update PATH."
  exit 1
fi

echo "Launching Ollama worker processes..."

echo "- Single local Ollama service: swarm helper model qwen3:8b" > "$ROOT_DIR/logs/swarm/ollama_worker_roles.log"

ollama pull qwen3:8b >/dev/null
nohup ollama serve > "$ROOT_DIR/logs/swarm/ollama_worker.log" 2>&1 &
echo "$!" > "$ROOT_DIR/logs/swarm/ollama_worker.pid"
echo "Ollama GPU worker started with PID $(cat "$ROOT_DIR/logs/swarm/ollama_worker.pid")"
echo "Role mapping written to $ROOT_DIR/logs/swarm/ollama_worker_roles.log"
