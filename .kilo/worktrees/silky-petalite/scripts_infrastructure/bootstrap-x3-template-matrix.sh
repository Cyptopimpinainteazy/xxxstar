#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT_DIR/templates/x3-chain"

echo "X3 Chain template matrix path:"
echo "  $TARGET_DIR"
echo ""
echo "Included starters:"
echo "  - polkadot-sdk-l1"
echo "  - polkadot-sdk-l2"
echo "  - papi-app"
echo "  - py-substrate-interface"
echo "  - substrate-sdk-ios"
echo ""
echo "Template docs:"
echo "  - docs/docs/docs/templates/X3_DEVELOPER_TEMPLATES.md"
