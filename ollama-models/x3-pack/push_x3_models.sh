#!/usr/bin/env bash
# push_x3_models.sh — Push all five X3 Ollama models to the registry
# Prerequisites:
#   1. ollama create already ran (./build_x3_models.sh)
#   2. ollama login completed (ollama login)
set -euo pipefail

echo "=== Push X3 Ollama Models ==="
echo

# Verify login
if ! ollama list >/dev/null 2>&1; then
  echo "ERROR: ollama not running or not logged in. Run: ollama login"
  exit 1
fi

MODELS=(
  lojak/cryptomaster
  lojak/x3-auditor
  lojak/x3-rust-runtime
  lojak/x3-solidity-guard
  lojak/x3-cline-finisher
)

for model in "${MODELS[@]}"; do
  echo "--- Pushing $model ---"
  ollama push "$model"
  echo
done

echo "=== All models pushed ==="
echo "Verify at: https://ollama.com/lojak"