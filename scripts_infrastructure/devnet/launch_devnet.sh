#!/usr/bin/env bash
# X3 Devnet Launch Script
# Initializes operators, bonds them, and starts command center.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEVNET_DIR="${DEVNET_DIR:-/tmp/x3_devnet}"

echo "=== X3 Devnet Launch ==="
echo "Project: $PROJECT_ROOT"
echo "Devnet dir: $DEVNET_DIR"
echo ""

# Operator roles to spin up
ROLES=("validator" "gpu" "storage")
BOND_AMOUNTS=(10000000000000 5000000000000 2000000000000)

for i in "${!ROLES[@]}"; do
    role="${ROLES[$i]}"
    bond="${BOND_AMOUNTS[$i]}"
    data_dir="${DEVNET_DIR}/operator-${role}"

    echo "--- Operator: ${role} ---"

    # Doctor check
    python3 -m x3_operator --data-dir "$data_dir" doctor --role "$role" || true
    echo ""

    # Init
    python3 -m x3_operator --data-dir "$data_dir" init --role "$role" --network devnet
    echo ""

    # Bond
    python3 -m x3_operator --data-dir "$data_dir" bond "$bond"
    echo ""

    # Status
    python3 -m x3_operator --data-dir "$data_dir" status
    echo ""
done

echo "=== Genesis Ceremony ==="
python3 -m x3_operator --data-dir "${DEVNET_DIR}/operator-validator" genesis --anchor
echo ""

echo "=== Governance Simulation ==="
python3 -m x3_operator --data-dir "${DEVNET_DIR}/operator-validator" simulate
echo ""

echo "=== Devnet Ready ==="
echo "To start command center:"
echo "  python3 -m x3_operator.command_center --data-dir ${DEVNET_DIR}/operator-validator --port 8900"
