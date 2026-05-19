#!/usr/bin/env bash
set -euo pipefail

RPC_URL="${1:-${RPC_URL:-http://127.0.0.1:9944}}"

RESPONSE="$(curl -fsS -H "Content-Type: application/json" \
  -d '{"id":1,"jsonrpc":"2.0","method":"rpc_methods","params":[]}' \
  "$RPC_URL")"

required_methods=(
  "x3_submitCrossVmTransaction"
  "x3_submitSvmTransaction"
  "x3_submitX3vmTransaction"
)

missing=0
for method in "${required_methods[@]}"; do
  if ! grep -q "\"$method\"" <<<"$RESPONSE"; then
    echo "MISSING: $method"
    missing=1
  fi
done

if [[ "$missing" -ne 0 ]]; then
  echo "FAIL: required X3 RPC methods are missing at $RPC_URL"
  exit 1
fi

echo "OK: all required X3 RPC methods are present at $RPC_URL"
