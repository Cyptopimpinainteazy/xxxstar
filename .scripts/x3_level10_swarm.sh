#!/usr/bin/env bash
set -euo pipefail

mkdir -p .cache .reports .repomix .swarm/logs .swarm/state .x3/graph .x3/dashboards

timestamp="$(date -Iseconds)"
echo "=== X3 LEVEL 10 SWARM CYCLE START: ${timestamp} ==="

run_step() {
  local label="$1"
  local output="$2"
  shift 2
  echo "[$label] $*"
  {
    echo "## $label"
    echo "timestamp=${timestamp}"
    echo "command=$*"
    "$@"
  } >"${output}" 2>&1 || true
}

run_step "repomix-pack" ".reports/repomix_pack.txt" .scripts/x3_repomix_pack.sh
run_step "full-scan" ".reports/x3_full_scan_cycle.txt" .scripts/x3_full_scan.sh
run_step "smell-scan" ".reports/x3_smell_scan_cycle.txt" .scripts/x3_smell_scan.sh
run_step "graph-build" ".reports/x3_graph_build.txt" python3 .scripts/x3_graph_builder.py
run_step "invariant-dashboard" ".reports/x3_invariant_dashboard.txt" python3 .scripts/x3_invariant_dashboard.py

run_step "cargo-fmt" ".reports/cargo_fmt.txt" cargo fmt --all -- --check
run_step "cargo-check" ".reports/cargo_check.txt" cargo check --workspace
run_step "cargo-test" ".reports/cargo_test.txt" cargo test --workspace
run_step "cargo-clippy" ".reports/cargo_clippy.txt" cargo clippy --workspace -- -D warnings

run_step "npm-lint" ".reports/npm_lint.txt" npm run lint
run_step "npm-test" ".reports/npm_test.txt" npm run test
run_step "npm-build" ".reports/npm_build.txt" npm run build

run_step "forge-test" ".reports/forge_test.txt" forge test
run_step "hardhat-test" ".reports/hardhat_test.txt" npx hardhat test
run_step "pytest" ".reports/pytest.txt" pytest

git status --short > .reports/git_status.txt || true
git diff --stat > .reports/git_diff_stat.txt || true
printf '%s\n' "${timestamp}" > .swarm/state/last_cycle.txt

echo "=== X3 LEVEL 10 SWARM CYCLE COMPLETE ==="
echo "Reports written to .reports/"
