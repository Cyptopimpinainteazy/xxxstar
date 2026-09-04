#!/usr/bin/env bash
# X3 developer time-to-first-value (TTFV) benchmark.
#
# Definition of "first value" for X3: a fresh developer can build the
# proof-forge CLI from this checkout and successfully verify a real claim
# end-to-end (we use x3.flashloans.attack_resistance because it exercises
# the EVM↔SVM parity-core harness and produces a signed receipt).
#
# This script measures wall-clock seconds for each step and writes a JSON
# benchmark to proof/onboarding/ttfv_benchmark.json. The proof-forge
# onboarding runner reads that file and asserts:
#   - the benchmark exists and parses,
#   - it was measured within TTFV_FRESHNESS_DAYS (default 30) days,
#   - total_seconds <= budget_seconds (default 600 = 10 minutes).
#
# Usage: scripts/onboarding/measure_ttfv.sh [--budget SECONDS]
set -euo pipefail

BUDGET_SECONDS=600
while [[ $# -gt 0 ]]; do
  case "$1" in
    --budget) BUDGET_SECONDS="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

OUT_DIR="$ROOT/proof/onboarding"
mkdir -p "$OUT_DIR"
OUT="$OUT_DIR/ttfv_benchmark.json"

now_iso() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
now_ns()  { date +%s%N; }

run_step() {
  local label="$1"; shift
  local t0 t1 dur status
  echo "[ttfv] step: $label"
  t0=$(now_ns)
  if "$@" >/tmp/ttfv_step.log 2>&1; then
    status="ok"
  else
    status="failed"
  fi
  t1=$(now_ns)
  dur=$(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.3f", (b-a)/1e9}')
  STEP_LABELS+=("$label")
  STEP_DURATIONS+=("$dur")
  STEP_STATUSES+=("$status")
  if [[ "$status" != "ok" ]]; then
    echo "[ttfv] step '$label' FAILED — see /tmp/ttfv_step.log"
    return 1
  fi
}

STEP_LABELS=()
STEP_DURATIONS=()
STEP_STATUSES=()
START_ISO=$(now_iso)
START_NS=$(now_ns)

PASSED=true
if ! run_step "cargo build -p proof-forge --release" \
       cargo build -p proof-forge --release; then
  PASSED=false
fi

if $PASSED && ! run_step "x3-proof verify x3.flashloans.attack_resistance" \
       ./target/release/x3-proof verify x3.flashloans.attack_resistance; then
  PASSED=false
fi

if $PASSED && ! run_step "x3-proof verify x3.contracts.evm_svm_parity" \
       ./target/release/x3-proof verify x3.contracts.evm_svm_parity; then
  PASSED=false
fi

END_NS=$(now_ns)
TOTAL_SECONDS=$(awk -v a="$START_NS" -v b="$END_NS" 'BEGIN{printf "%.3f",(b-a)/1e9}')

# Build the steps JSON array.
steps_json="["
for i in "${!STEP_LABELS[@]}"; do
  if [[ $i -gt 0 ]]; then steps_json+=","; fi
  steps_json+=$(printf '{"label":"%s","seconds":%s,"status":"%s"}' \
    "${STEP_LABELS[$i]}" "${STEP_DURATIONS[$i]}" "${STEP_STATUSES[$i]}")
done
steps_json+="]"

WITHIN_BUDGET=$(awk -v t="$TOTAL_SECONDS" -v b="$BUDGET_SECONDS" 'BEGIN{print (t<=b)?"true":"false"}')
PASSED_STR=$([[ "$PASSED" == "true" ]] && echo "true" || echo "false")

cat >"$OUT" <<EOF
{
  "measured_at": "$START_ISO",
  "host": "$(uname -srm)",
  "rustc": "$(rustc --version 2>/dev/null || echo unknown)",
  "budget_seconds": $BUDGET_SECONDS,
  "total_seconds": $TOTAL_SECONDS,
  "within_budget": $WITHIN_BUDGET,
  "passed": $PASSED_STR,
  "first_value_definition": "build proof-forge from source and verify two real claim receipts (flashloans.attack_resistance + contracts.evm_svm_parity)",
  "steps": $steps_json
}
EOF

echo "[ttfv] wrote $OUT"
echo "[ttfv] total: ${TOTAL_SECONDS}s (budget ${BUDGET_SECONDS}s, within_budget=$WITHIN_BUDGET, passed=$PASSED_STR)"

if [[ "$PASSED" != "true" ]]; then exit 1; fi
if [[ "$WITHIN_BUDGET" != "true" ]]; then exit 3; fi
