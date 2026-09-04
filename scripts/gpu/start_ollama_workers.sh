#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Starting Ollama GPU workers for X3 swarm..."
mkdir -p "$ROOT_DIR/logs/swarm"

cat > "$ROOT_DIR/logs/swarm/ollama_worker_roles.log" <<'MAP'
GPU role split:
- GPU0 / RTX: PlannerAgent + CodeAgent
- GPU1 / GTX1070: TestBuilderAgent
- GPU2 / GTX1070: AuditorAgent + BreakerAgent
- GPU3 / GTX1070: MarketingAgent + GrantAgent
All task outputs require tests before trust.
MAP

if ! command -v ollama >/dev/null 2>&1; then
  echo "WARNING: ollama executable not found. Role map generated only."
  exit 0
fi

echo "Launching local Ollama service..."
ollama pull qwen3:8b >/dev/null || true
nohup ollama serve > "$ROOT_DIR/logs/swarm/ollama_worker.log" 2>&1 &
echo "$!" > "$ROOT_DIR/logs/swarm/ollama_worker.pid"
echo "Ollama GPU worker started with PID $(cat "$ROOT_DIR/logs/swarm/ollama_worker.pid")"
echo "Role mapping written to $ROOT_DIR/logs/swarm/ollama_worker_roles.log"
