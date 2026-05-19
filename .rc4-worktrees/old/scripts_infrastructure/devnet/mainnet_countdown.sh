#!/usr/bin/env bash
# X3 Mainnet Countdown Checklist
# Pre-launch verification of all subsystems.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CHECK_DIR="${CHECK_DIR:-/tmp/x3_countdown}"

echo "========================================"
echo "   X3 MAINNET COUNTDOWN CHECKLIST"
echo "========================================"
echo ""

cd "$PROJECT_ROOT"
PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" > /dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL + 1))
    fi
}

echo "--- Python Package ---"
check "x3_operator imports" "python3 -c 'import x3_operator'"
check "All 31 exports" "python3 -c 'import x3_operator; assert len(x3_operator.__all__) >= 30'"
check "CLI --help" "python3 -m x3_operator --help"
check "Config validation" "python3 -c 'from x3_operator.config import X3Config; c=X3Config(); assert not c.validate()'"
echo ""

echo "--- Hardware ---"
check "Doctor passes" "python3 -m x3_operator doctor"
echo ""

echo "--- Operator Lifecycle ---"
rm -rf "$CHECK_DIR"
check "Init operator" "python3 -m x3_operator --data-dir $CHECK_DIR init --role validator --network mainnet"
check "Bond operator" "python3 -m x3_operator --data-dir $CHECK_DIR bond 10000000000000"
check "Status check" "python3 -m x3_operator --data-dir $CHECK_DIR status"
check "Genesis ceremony" "python3 -m x3_operator --data-dir $CHECK_DIR genesis"
check "Exit (unbonding)" "python3 -m x3_operator --data-dir $CHECK_DIR exit-op"
echo ""

echo "--- Governance ---"
check "Simulate all attacks" "python3 -m x3_operator simulate"
check "Whale simulation" "python3 -m x3_operator simulate --attack whale"
check "Sybil simulation" "python3 -m x3_operator simulate --attack sybil"
check "Bribery simulation" "python3 -m x3_operator simulate --attack bribery"
check "Speed simulation" "python3 -m x3_operator simulate --attack speed"
echo ""

echo "--- Command Center ---"
check "Command center imports" "python3 -c 'from x3_operator.command_center import CommandCenterState'"
check "Command center state loads" "python3 -c '
from x3_operator.command_center import CommandCenterState
from pathlib import Path
s = CommandCenterState(Path(\"$CHECK_DIR\"))
assert s.identity is not None
'"
echo ""

echo "--- Modules ---"
check "Health module" "python3 -c 'from x3_operator.health import run_health_check; run_health_check()'"
check "Bonding module" "python3 -c 'from x3_operator.bonding import BondLedger; BondLedger()'"
check "Slashing module" "python3 -c '
from x3_operator.slashing import SlashingEngine
from x3_operator.config import X3Config
SlashingEngine(X3Config())
'"
check "Supervisor module" "python3 -c '
from x3_operator.supervisor import AgentSupervisor
from x3_operator.config import X3Config
AgentSupervisor(X3Config())
'"
check "Storage module" "python3 -c '
from x3_operator.storage import StorageRegistry
from x3_operator.config import X3Config
StorageRegistry(X3Config())
'"
check "Telemetry module" "python3 -c '
from x3_operator.telemetry import create_operator_metrics
m = create_operator_metrics()
assert len(m._metrics) >= 10
'"
echo ""

echo "========================================"
echo "   RESULTS: $PASS passed, $FAIL failed"
echo "========================================"

if [ "$FAIL" -gt 0 ]; then
    echo "   STATUS: NOT READY FOR LAUNCH"
    exit 1
else
    echo "   STATUS: READY FOR LAUNCH"
    exit 0
fi
