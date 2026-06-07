#!/usr/bin/env bash
# run_evals.sh — Run all X3 AI model eval cases against each model
# Usage:
#   ./run_evals.sh                     # eval all models
#   ./run_evals.sh lojak/cryptomaster   # eval specific model
#   ./run_evals.sh --quick              # eval 5 random cases per model
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
EVAL_CASES="$SCRIPT_DIR/eval_cases.jsonl"
REPORTS_DIR="$SCRIPT_DIR/reports"
QUICK="${QUICK:-false}"

mkdir -p "$REPORTS_DIR"

MODELS=(
  lojak/cryptomaster
  lojak/x3-auditor
  lojak/x3-rust-runtime
  lojak/x3-solidity-guard
  lojak/x3-svm-guard
  lojak/x3-cosmwasm-guard
  lojak/x3-btc-guard
  lojak/x3-arb-king
  lojak/x3-flashloan-executor
  lojak/x3-route-oracle
  lojak/x3-quant-risk
  lojak/x3-trade-ops
  lojak/x3-mev-defense
  lojak/x3-data-engineer
  lojak/x3-devops-commander
  lojak/x3-testsmith
  lojak/x3-docsmith
  lojak/x3-compliance-ops
  lojak/x3-eval-judge
  lojak/x3-cline-finisher
)

# Override models if specific model provided
if [ $# -gt 0 ] && [ "$1" != "--quick" ]; then
  MODELS=("$1")
fi

if [ "$1" == "--quick" ] 2>/dev/null; then
  QUICK=true
fi

echo "=== X3 AI Command System — Model Evals ==="
echo "Cases: $EVAL_CASES"
echo "Reports: $REPORTS_DIR"
echo "Quick mode: $QUICK"
echo

TIMESTAMP=$(date +%Y%m%d_%H%M%S)

for model in "${MODELS[@]}"; do
  MODEL_SAFE=$(echo "$model" | tr '/' '_')
  REPORT_FILE="$REPORTS_DIR/${MODEL_SAFE}_${TIMESTAMP}.jsonl"

  echo "--- Evaluating $model ---"

  # Count eval cases
  TOTAL=$(wc -l < "$EVAL_CASES")
  if [ "$QUICK" = true ]; then
    TOTAL=5
  fi

  echo "Running $TOTAL eval cases..."

  COUNT=0
  while IFS= read -r line; do
    if [ "$QUICK" = true ] && [ "$COUNT" -ge 5 ]; then
      break
    fi

    PROMPT=$(echo "$line" | python3 -c "import sys,json; print(json.load(sys.stdin)['prompt'])" 2>/dev/null || echo "")
    if [ -z "$PROMPT" ]; then
      continue
    fi

    # Run model
    RESPONSE=$(ollama run "$model" "$PROMPT" 2>/dev/null || echo "ERROR: model failed")

    # Save result
    echo "$line" | python3 -c "
import sys, json
case = json.load(sys.stdin)
result = {
    'model': '$model',
    'id': case.get('id', 'unknown'),
    'category': case.get('category', 'unknown'),
    'prompt': case.get('prompt', ''),
    'response': '''$RESPONSE'''[:8000],
    'must_include': case.get('must_include', []),
    'must_reject': case.get('must_reject', []),
}
print(json.dumps(result))
" >> "$REPORT_FILE" 2>/dev/null || true

    COUNT=$((COUNT + 1))
  done < "$EVAL_CASES"

  echo "  $COUNT cases evaluated. Report: $REPORT_FILE"
done

echo
echo "=== Eval run complete ==="
echo "Score reports with: python3 $SCRIPT_DIR/score_output.py --report $REPORT_FILE"