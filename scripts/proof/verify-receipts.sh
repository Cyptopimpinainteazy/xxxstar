#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERIFY_SCRIPT="$ROOT_DIR/scripts/proof/verify_receipts.py"

if [[ ! -f "$VERIFY_SCRIPT" ]]; then
  echo "ERROR: Missing verifier script: $VERIFY_SCRIPT" >&2
  exit 2
fi

if command -v /usr/bin/python >/dev/null 2>&1; then
  PYTHON_BIN="/usr/bin/python"
elif command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
else
  echo "ERROR: python is required to validate receipts." >&2
  exit 2
fi

MAX_AGE_HOURS="${RECEIPT_MAX_AGE_HOURS:-24}"
ARGS=("$PYTHON_BIN" "$VERIFY_SCRIPT" --root "$ROOT_DIR" --max-age-hours "$MAX_AGE_HOURS")

if [[ "${RECEIPT_SKIP_PROVENANCE_CHECK:-0}" == "1" ]]; then
  ARGS+=(--skip-provenance-check)
fi

if [[ "${RECEIPT_SKIP_FRESHNESS_CHECK:-0}" == "1" ]]; then
  ARGS+=(--skip-freshness-check)
fi

exec "${ARGS[@]}"
