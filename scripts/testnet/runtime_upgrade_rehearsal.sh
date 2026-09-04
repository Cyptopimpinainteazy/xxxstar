#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "== X3 Testnet Runtime Upgrade Rehearsal =="

if [ -x "$ROOT_DIR/scripts/mainnet/runtime_upgrade_rehearsal.sh" ]; then
  bash "$ROOT_DIR/scripts/mainnet/runtime_upgrade_rehearsal.sh"
else
  echo "Runtime upgrade rehearsal script is not available; placeholder only." 
fi
