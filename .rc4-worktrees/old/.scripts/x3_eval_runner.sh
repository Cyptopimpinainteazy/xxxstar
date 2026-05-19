#!/usr/bin/env bash
set -euo pipefail

mkdir -p .x3/reports .reports

results=".x3/reports/EVAL_RESULTS.md"
{
  echo "# X3 Eval Results"
  echo
  echo "Generated: $(date -Iseconds)"
  echo
} > "${results}"

run_eval() {
  local name="$1"
  shift
  local report=".reports/${name}.txt"

  {
    echo "## ${name}"
    echo
    echo '```text'
    printf '%q ' "$@"
    echo
    echo '```'
    echo
  } >> "${results}"

  if "$@" > "${report}" 2>&1; then
    echo "PASS" >> "${results}"
  else
    echo "FAIL - see ${report}" >> "${results}"
  fi
  echo >> "${results}"
}

run_eval cargo_check cargo check --workspace
run_eval cargo_test cargo test --workspace
run_eval cargo_clippy cargo clippy --workspace -- -D warnings
run_eval mutation_gate python3 .scripts/x3_mutation_gate.py
run_eval drift_detector python3 .scripts/x3_drift_detector.py
run_eval graph_builder python3 .scripts/x3_graph_builder.py
run_eval invariant_dashboard python3 .scripts/x3_invariant_dashboard.py

cat "${results}"
