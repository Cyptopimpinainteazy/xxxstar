#!/usr/bin/env bash
set -euo pipefail

NODE_URL="${NODE_URL:-http://127.0.0.1:9944}"

rpc_call() {
  local method="$1"
  local params="$2"
  curl -sS -m 8 \
    -H 'Content-Type: application/json' \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
    "${NODE_URL}"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local label="$3"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "[FAIL] ${label}"
    echo "Expected to find: ${needle}"
    echo "Response: ${haystack}"
    exit 1
  fi
  echo "[PASS] ${label}"
}

assert_contains_any() {
  local haystack="$1"
  local label="$2"
  shift 2

  for needle in "$@"; do
    if [[ "$haystack" == *"$needle"* ]]; then
      echo "[PASS] ${label}"
      return 0
    fi
  done

  echo "[FAIL] ${label}"
  echo "Expected one of: $*"
  echo "Response: ${haystack}"
  exit 1
}

echo "Running Frontier RPC smoke against ${NODE_URL}"

health="$(rpc_call "system_health" "[]")"
assert_contains "$health" '"result"' "system_health responds"
assert_contains "$health" '"isSyncing"' "system_health includes sync status"

estimate_default="$(rpc_call "eth_estimateGas" '[{"to":"0x0000000000000000000000000000000000000000","data":"0x"}]')"
assert_contains_any "$estimate_default" "eth_estimateGas returns deterministic outcome" '"result":"0x' 'Gas estimation failed'

estimate_decimal="$(rpc_call "eth_estimateGas" '[{"to":"0x0000000000000000000000000000000000000000","data":"0x","gas":"42000"}]')"
assert_contains_any "$estimate_decimal" "eth_estimateGas accepts decimal-string gas" '"result":"0x' 'Gas estimation failed'

estimate_invalid_gas="$(rpc_call "eth_estimateGas" '[{"to":"0x0000000000000000000000000000000000000000","data":"0x","gas":{"value":1}}]')"
assert_contains "$estimate_invalid_gas" '"error"' "eth_estimateGas rejects invalid gas type"
assert_contains "$estimate_invalid_gas" 'Invalid gas value' "eth_estimateGas error message is explicit"

call_ok="$(rpc_call "eth_call" '[{"to":"0x0000000000000000000000000000000000000000","data":"0x"},"latest"]')"
assert_contains_any "$call_ok" "eth_call returns deterministic outcome" '"result":"0x' 'EVM call failed'

call_missing_to="$(rpc_call "eth_call" '[{"data":"0x"},"latest"]')"
assert_contains "$call_missing_to" '"error"' "eth_call rejects missing to"
assert_contains "$call_missing_to" 'Missing to address' "eth_call missing-to message is explicit"

call_bad_to="$(rpc_call "eth_call" '[{"to":"0x1234","data":"0x"},"latest"]')"
assert_contains "$call_bad_to" '"error"' "eth_call rejects malformed to"
assert_contains "$call_bad_to" 'Address must be 20 bytes' "eth_call malformed-to message is explicit"

echo "Frontier RPC smoke completed successfully."