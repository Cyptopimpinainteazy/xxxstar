#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "== X3 Testnet Genesis Lint =="

echo "Checking for chain spec and genesis config..."
if [ ! -d "$ROOT_DIR/chain-specs" ]; then
  echo "ERROR: chain-specs directory not found" >&2
  exit 1
fi

echo "Testnet genesis lint is a placeholder. Ensure chain-spec and genesis validation logic is integrated." 
