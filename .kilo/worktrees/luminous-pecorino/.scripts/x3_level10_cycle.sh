#!/usr/bin/env bash
set -euo pipefail

mkdir -p .cache .reports .x3/reports .x3/dashboards .swarm/state .swarm/logs

echo "=== X3 LEVEL 10 CYCLE START ==="

run_step() {
  local label="$1"
  shift
  echo "[$label] $*"
  "$@" || true
}

run_step "Repomix context packs" .scripts/x3_repomix_pack.sh
run_step "Full file scan" .scripts/x3_full_scan.sh
run_step "Smell/security scan" .scripts/x3_smell_scan.sh
run_step "Graph build" python3 .scripts/x3_graph_builder.py
run_step "Invariant dashboard" python3 .scripts/x3_invariant_dashboard.py
run_step "Drift scan" python3 .scripts/x3_drift_detector.py
run_step "Break-the-chain scan" .scripts/x3_break_the_chain.sh
run_step "Eval runner" .scripts/x3_eval_runner.sh

echo "[Git truth snapshot]"
git status --short > .reports/git_status.txt || true
git diff --stat > .reports/git_diff_stat.txt || true
git diff > .reports/git_diff.patch || true

date -Iseconds > .swarm/state/last_cycle.txt

echo "=== X3 LEVEL 10 CYCLE COMPLETE ==="
