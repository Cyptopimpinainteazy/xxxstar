#!/usr/bin/env bash
# push_all_models.sh — Push all 20 X3 AI Command System models to Ollama registry
# Prerequisites:
#   1. ollama create already ran (./build_all_models.sh)
#   2. ollama login completed (ollama login)
set -euo pipefail

echo "=== Push X3 AI Command System Models ==="
echo

# Verify login
if ! ollama list >/dev/null 2>&1; then
  echo "ERROR: ollama not running or not logged in. Run: ollama login"
  exit 1
fi

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

for model in "${MODELS[@]}"; do
  echo "--- Pushing lojak/$model ---"
  ollama push "lojak/$model"
  echo
done

echo "=== All 20 models pushed ==="
echo "Verify at: https://ollama.com/lojak"