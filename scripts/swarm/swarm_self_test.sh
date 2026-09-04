#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPORT_FILE="$ROOT_DIR/reports/swarm_self_test_report.md"
mkdir -p "$ROOT_DIR/reports"

echo "# X3 Swarm Self Test Report" > "$REPORT_FILE"
echo >> "$REPORT_FILE"

echo "Running X3 swarm self-test..."

if cargo test -p x3-swarm-core swarm_ -- --nocapture; then
  echo "- [OK] x3-swarm-core swarm_* tests passed" | tee -a "$REPORT_FILE"
else
  echo "- [WARN] x3-swarm-core swarm_* tests failed" | tee -a "$REPORT_FILE"
  echo "- Blocker: inspect failing tests in crates/x3-swarm-core" >> "$REPORT_FILE"
fi

if cargo test -p x3-readiness swarm_ -- --nocapture; then
  echo "- [OK] x3-readiness swarm_* tests passed" | tee -a "$REPORT_FILE"
else
  echo "- [WARN] x3-readiness swarm_* tests failed" | tee -a "$REPORT_FILE"
  echo "- Blocker: inspect failing tests in crates/x3-readiness" >> "$REPORT_FILE"
fi

if curl -fsS http://127.0.0.1:8787/health >/dev/null 2>&1; then
  echo "- [OK] API health endpoint reachable" | tee -a "$REPORT_FILE"
else
  echo "- [WARN] API health endpoint not reachable" | tee -a "$REPORT_FILE"
  echo "- Blocker: run scripts/swarm/swarm_up.sh before API-integrated tests" >> "$REPORT_FILE"
fi

echo >> "$REPORT_FILE"
echo "Generated: $(date -u +'%Y-%m-%dT%H:%M:%SZ')" >> "$REPORT_FILE"

cat "$REPORT_FILE"
