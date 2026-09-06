#!/bin/bash
# Startup smoke tests for X3 Chain
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Run bootstrap in a separate process so an `exit` inside doesn't kill this smoke test.
# Use --detach so we don't hang waiting on foreground services.
bash "$ROOT/run-everything.sh" --detach >/dev/null 2>&1 || true

echo "Running startup smoke checks..."

# Config
OLLAMA_URL="${OLLAMA_URL:-http://127.0.0.1:11434}"
BLOCKCHAIN_RPC="http://127.0.0.1:9944"
SWARM_HEALTH="http://127.0.0.1:8080/ready"

fail() {
  echo "[FAIL] $1"
  exit 2
}

# Ollama
if curl -sf "$OLLAMA_URL/api/tags" >/dev/null 2>&1; then
  echo "[OK] Ollama /api/tags reachable at $OLLAMA_URL"
else
  fail "Ollama /api/tags not reachable at $OLLAMA_URL"
fi

# Blockchain JSON-RPC
if curl -sf -X POST -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","method":"system_health","params":[],"id":1}' "$BLOCKCHAIN_RPC" >/dev/null 2>&1; then
  echo "[OK] Blockchain JSON-RPC responding at $BLOCKCHAIN_RPC"
else
  fail "Blockchain JSON-RPC not responding at $BLOCKCHAIN_RPC"
fi

# Swarm readiness
if curl -sf "$SWARM_HEALTH" >/dev/null 2>&1; then
  echo "[OK] Swarm readiness OK at $SWARM_HEALTH"
else
  fail "Swarm readiness endpoint failed at $SWARM_HEALTH"
fi

# Basic UI ports
for port in 3000 3001 3002 3003; do
  if ss -lntH "sport = :$port" >/dev/null 2>&1; then
    echo "[OK] App listening on port $port"
  else
    echo "[WARN] No service listening on port $port"
  fi
done

echo "Startup smoke checks completed successfully."
