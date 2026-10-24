#!/usr/bin/env bash
# Build the narrowed RC1 runtime, then prove a disposable three-validator boot.
set -euo pipefail
if (( $# != 0 )); then
  echo "Usage: rc1_boot_gate.sh (configure X3_RC1_PROFILE and X3_RC1_EVIDENCE_DIR through environment)" >&2
  exit 2
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"
PROFILE="${X3_RC1_PROFILE:-release}"
case "$PROFILE" in
  release) BUILD_ARGS=(--release) ;;
  debug) BUILD_ARGS=() ;;
  *) echo "X3_RC1_PROFILE must be release or debug" >&2; exit 2 ;;
esac
EVIDENCE="${X3_RC1_EVIDENCE_DIR:-$ROOT_DIR/audit-artifacts/rc1-boot/$(git rev-parse HEAD)}"
env -u SKIP_WASM_BUILD CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
  cargo build -p x3-chain-node --features mainnet-rc1 "${BUILD_ARGS[@]}"
python3 scripts/mainnet/rc1_boot_test.py \
  --binary "${CARGO_TARGET_DIR:-$ROOT_DIR/target}/$PROFILE/x3-chain-node" \
  --evidence "$EVIDENCE"
