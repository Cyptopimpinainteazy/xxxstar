#!/bin/bash
# Auto-run Ralph + Ollama end-to-end.
# Usage: ./scripts/ralph/ralph-auto.sh [model] [iterations]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
STRICT_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --strict)
      STRICT_MODE=true
      shift
      ;;
    *)
      if [ -z "$MODEL_INPUT" ]; then
        MODEL_INPUT="$1"
      elif [ -z "$ITERATIONS" ]; then
        ITERATIONS="$1"
      fi
      shift
      ;;
  esac
done

MODEL_INPUT="${MODEL_INPUT:-}"
ITERATIONS="${ITERATIONS:-1}"
MODEL_DEFAULT="qwen3:8b"

function tombstone() {
  echo "" >&2
  echo "ERROR: $1" >&2
  exit 1
}

# Ensure Ollama Server
if ! curl -s --max-time 3 http://localhost:11434/api/tags >/dev/null 2>&1; then
  echo "[ralph-auto] Ollama isn't running; starting ollama serve in background."
  nohup ollama serve > "$REPO_ROOT/ollama-serve.log" 2>&1 &
  OLLAMA_PID=$!
  echo "[ralph-auto] started Ollama serve PID $OLLAMA_PID"
  sleep 5
fi

if ! curl -s --max-time 3 http://localhost:11434/api/tags >/dev/null 2>&1; then
  tombstone "Ollama server did not respond after start. Check logs: $REPO_ROOT/ollama-serve.log"
fi

# Ensure we have a working model loaded.
if [ -n "$MODEL_INPUT" ]; then
  MODEL="$MODEL_INPUT"
else
  if ollama list | awk '{print $1}' | grep -qx 'qwen2.5-coder:0.5b'; then
    MODEL="qwen2.5-coder:0.5b"
  elif ollama list | awk '{print $1}' | grep -qx 'qwen2.5-coder:1.5b-base'; then
    MODEL="qwen2.5-coder:1.5b-base"
  elif ollama list | awk '{print $1}' | grep -qx 'qwen3:8b'; then
    MODEL="qwen3:8b"
  elif ollama list | awk '{print $1}' | grep -qx 'codellama:latest'; then
    MODEL="codellama:latest"
  else
    echo "[ralph-auto] no preferred model available, pulling qwen2.5-coder:0.5b..."
    ollama pull qwen2.5-coder:0.5b
    MODEL="qwen2.5-coder:0.5b"
  fi
fi

echo "[ralph-auto] using model: $MODEL"

# Make sure Ralph prompt exists in primary and fallback locations
if [ ! -f "$SCRIPT_DIR/prompt.md" ] && [ ! -f "$REPO_ROOT/ralph/prompt.md" ]; then
  tombstone "prompt.md missing. Add Ralph instructions to $SCRIPT_DIR/prompt.md or $REPO_ROOT/ralph/prompt.md."
fi

# Run Ralph loop
cd "$SCRIPT_DIR"
RALPH_CMD="MODEL=\"$MODEL\" STRICT_MODE=\"$STRICT_MODE\" ./ralph.sh --tool ollama"
if [ "$STRICT_MODE" = "true" ]; then
  RALPH_CMD="$RALPH_CMD --strict"
fi
RALPH_CMD="$RALPH_CMD $ITERATIONS"
eval "$RALPH_CMD" || true

# Validate coverage and lint (strict production checks)
if ! "$SCRIPT_DIR/ralph-check-coverage.sh"; then
  echo "[ralph-auto] coverage/lint check failed. Fix failing tests first." >&2
  exit 1
fi

# Print a short usage for next step
cat <<EOF
---
Complete: Ralph ran one iteration using Ollama and coverage checks passed.
Check progress at: $SCRIPT_DIR/progress.txt
If you want explicit patch apply behavior, I can add ralph-apply.sh.
---
EOF
