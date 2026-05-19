#!/usr/bin/env bash
# X3 Key Ceremony Rehearsal
# Dry-run of the genesis ceremony with validation checks.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CEREMONY_DIR="${CEREMONY_DIR:-/tmp/x3_ceremony_rehearsal}"

echo "=== X3 Key Ceremony Rehearsal ==="
echo "Output: $CEREMONY_DIR"
echo ""

cd "$PROJECT_ROOT"

# Initialize a validator for the ceremony
rm -rf "$CEREMONY_DIR"
python3 -m x3_operator --data-dir "$CEREMONY_DIR" init --role validator --network testnet
echo ""

# Run genesis ceremony in dry-run mode (no anchor)
echo "Running genesis ceremony (dry run)..."
python3 -m x3_operator --data-dir "$CEREMONY_DIR" genesis --chain-id x3-testnet --chain-name "X3 Testnet Rehearsal"
echo ""

# Verify the chain spec was produced
if [ -f "$CEREMONY_DIR/chain-spec.json" ]; then
    echo "Chain spec produced successfully:"
    python3 -c "
import json
spec = json.load(open('$CEREMONY_DIR/chain-spec.json'))
print(f'  Chain: {spec[\"name\"]} ({spec[\"id\"]})')
print(f'  Validators: {len(spec[\"genesis\"][\"runtime\"][\"aura\"][\"authorities\"])}')
print(f'  Genesis hash: {spec[\"x3_genesis_hash\"][:32]}...')
print(f'  Token: {spec[\"properties\"][\"tokenSymbol\"]}')
"
else
    echo "ERROR: Chain spec not generated!"
    exit 1
fi

echo ""
echo "=== Rehearsal Complete ==="
echo "Review chain spec at: $CEREMONY_DIR/chain-spec.json"
