#!/usr/bin/env bash
# build_x3_models.sh — Build all five X3 Ollama models
# Usage:
#   ./build_x3_models.sh                    # defaults to qwen2.5-coder:14b
#   BASE_MODEL=qwen2.5-coder:32b ./build_x3_models.sh
set -euo pipefail

BASE_MODEL="${BASE_MODEL:-qwen2.5-coder:14b}"
ROOT="${ROOT:-$(cd "$(dirname "$0")" && pwd)}"

echo "=== X3 Ollama Model Pack Builder ==="
echo "Base model: $BASE_MODEL"
echo "Model dir:  $ROOT"
echo

# Pull base model if not present
echo "--- Pulling base model ---"
ollama pull "$BASE_MODEL"

# --- lojak/cryptomaster ---
echo "--- Building lojak/cryptomaster ---"
ollama create lojak/cryptomaster -f "$ROOT/cryptomaster/Modelfile"

# --- lojak/x3-auditor ---
echo "--- Building lojak/x3-auditor ---"
ollama create lojak/x3-auditor -f "$ROOT/x3-auditor/Modelfile"

# --- lojak/x3-rust-runtime ---
echo "--- Building lojak/x3-rust-runtime ---"
ollama create lojak/x3-rust-runtime -f "$ROOT/x3-rust-runtime/Modelfile"

# --- lojak/x3-solidity-guard ---
echo "--- Building lojak/x3-solidity-guard ---"
ollama create lojak/x3-solidity-guard -f "$ROOT/x3-solidity-guard/Modelfile"

# --- lojak/x3-cline-finisher ---
echo "--- Building lojak/x3-cline-finisher ---"
ollama create lojak/x3-cline-finisher -f "$ROOT/x3-cline-finisher/Modelfile"

echo
echo "=== Built models ==="
ollama list | grep -E 'lojak/(cryptomaster|x3-auditor|x3-rust-runtime|x3-solidity-guard|x3-cline-finisher)' || true

echo
echo "=== Smoke tests ==="
for model in \
  lojak/cryptomaster \
  lojak/x3-auditor \
  lojak/x3-rust-runtime \
  lojak/x3-solidity-guard \
  lojak/x3-cline-finisher
do
  echo "--- Testing $model ---"
  ollama run "$model" "In one short paragraph, state your X3 role and your main safety rule."
  echo
done

echo "=== Done ==="