#!/usr/bin/env bash
# build_all_models.sh — Build all 20 X3 AI Command System Ollama models
# Usage:
#   ./build_all_models.sh                              # defaults to qwen2.5-coder:14b
#   BASE_MODEL=qwen2.5-coder:32b ./build_all_models.sh
set -euo pipefail

BASE_MODEL="${BASE_MODEL:-qwen2.5-coder:14b}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=============================================="
echo "  X3 AI Command System — Model Pack Builder"
echo "=============================================="
echo "Base model: $BASE_MODEL"
echo "Script dir: $SCRIPT_DIR"
echo

# Pull base model
echo "--- Pulling base model ---"
ollama pull "$BASE_MODEL"

# Model list
MODELS=(
  cryptomaster
  x3-auditor
  x3-rust-runtime
  x3-solidity-guard
  x3-svm-guard
  x3-cosmwasm-guard
  x3-btc-guard
  x3-arb-king
  x3-flashloan-executor
  x3-route-oracle
  x3-quant-risk
  x3-trade-ops
  x3-mev-defense
  x3-data-engineer
  x3-devops-commander
  x3-testsmith
  x3-docsmith
  x3-compliance-ops
  x3-eval-judge
  x3-cline-finisher
)

# Update FROM lines to use the selected base model
for model in "${MODELS[@]}"; do
  MODLEFILE="$SCRIPT_DIR/models/$model/Modelfile"
  if [ -f "$MODLEFILE" ]; then
    # Update FROM line
    sed -i "s|^FROM qwen2.5-coder:.*$|FROM $BASE_MODEL|" "$MODLEFILE"
  fi
done

# Create all models
for model in "${MODELS[@]}"; do
  MODLEFILE="$SCRIPT_DIR/models/$model/Modelfile"
  if [ -f "$MODLEFILE" ]; then
    echo "--- Building lojak/$model ---"
    ollama create "lojak/$model" -f "$MODLEFILE"
  else
    echo "WARNING: $MODLEFILE not found, skipping"
  fi
done

echo
echo "=== Built models ==="
ollama list | grep 'lojak/' || true

echo
echo "=== Smoke tests ==="
for model in "${MODELS[@]}"; do
  echo "--- Testing lojak/$model ---"
  ollama run "lojak/$model" "State your X3 role and main safety rule in one sentence." 2>/dev/null || echo "FAILED: lojak/$model"
  echo
done

echo "=== Done. 20 models built. ==="