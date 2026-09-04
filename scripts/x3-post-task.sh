#!/usr/bin/env bash
set -euo pipefail
# x3-post-task.sh — run proof check, status report, and update proof ledger after work

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo ""
echo "=========================================="
echo "X3 POST-TASK — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "=========================================="

echo ""
echo "--- Running Proof Check ---"
if bash "$SCRIPT_DIR/x3-proof-check.sh"; then
    PROOF_STATUS="PASS"
else
    PROOF_STATUS="FAIL"
fi

echo ""
echo "--- Running Status Report ---"
bash "$SCRIPT_DIR/x3-status-report.sh"

echo ""
echo "--- Updating Proof Ledger ---"
bash "$SCRIPT_DIR/x3-update-proof-ledger.sh"

echo ""
echo "=========================================="
echo "POST-TASK COMPLETE"
echo "Final status: $PROOF_STATUS"
echo "=========================================="

if [ "$PROOF_STATUS" = "FAIL" ]; then
    exit 1
fi
exit 0