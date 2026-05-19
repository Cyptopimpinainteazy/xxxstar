#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 '<prompt>'"
  exit 1
fi

PROMPT="$1"

echo "Asking GPU worker with provided prompt."
if command -v ollama >/dev/null 2>&1; then
  ollama run qwen3:8b "$PROMPT"
else
  echo "ERROR: ollama not installed or not available in PATH."
  exit 1
fi
