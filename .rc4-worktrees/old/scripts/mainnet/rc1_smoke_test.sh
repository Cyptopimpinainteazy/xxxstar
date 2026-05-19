#!/usr/bin/env bash
# RC1 internal settlement testnet smoke test
# Connects to Alice's RPC (localhost:9944) and verifies:
#   1. Chain is producing blocks
#   2. Finality is advancing
#   3. Runtime metadata exposes X3SupplyLedger + X3CrossVmRouter pallets
#   4. ExternalBridgesEnabled storage = false
set -euo pipefail

RPC="${RPC_URL:-http://localhost:9944}"
PASS=0
FAIL=0

ok()   { echo "[PASS] $1"; ((PASS+=1)); }
fail() { echo "[FAIL] $1"; ((FAIL+=1)); }
rpc()  { curl -s -X POST -H "Content-Type: application/json" --data "$1" "$RPC"; }

echo "=== X3 RC1 Smoke Test ==="
echo "RPC: $RPC"
echo ""

# --- 1. Chain head is advancing ---
echo "-- Block production --"
HEAD1=$(rpc '{"id":1,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['result']['number'],16))")
sleep 6
HEAD2=$(rpc '{"id":2,"jsonrpc":"2.0","method":"chain_getHeader","params":[]}' | python3 -c "import sys,json; d=json.load(sys.stdin); print(int(d['result']['number'],16))")
if [ "$HEAD2" -gt "$HEAD1" ]; then
  ok "Block production: advanced from #$HEAD1 to #$HEAD2"
else
  fail "Block production: stuck at #$HEAD1"
fi

# --- 2. Finality is advancing ---
echo "-- Finality --"
FIN1=$(rpc '{"id":3,"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[]}' | python3 -c "import sys,json; print(json.load(sys.stdin)['result'])")
sleep 12
FIN2=$(rpc '{"id":4,"jsonrpc":"2.0","method":"chain_getFinalizedHead","params":[]}' | python3 -c "import sys,json; print(json.load(sys.stdin)['result'])")
if [ "$FIN1" != "$FIN2" ]; then
  ok "Finality: advancing (was $FIN1, now $FIN2)"
else
  fail "Finality: not advancing — GRANDPA may be stalled"
fi

# --- 3. Runtime metadata pallets ---
echo "-- Runtime metadata --"
META_HEX=$(rpc '{"id":5,"jsonrpc":"2.0","method":"state_getMetadata","params":[]}' | python3 -c "import sys,json; print(json.load(sys.stdin)['result'])")
META_BYTES=$(python3 -c "import sys; h=sys.argv[1].lstrip('0x'); print(bytes.fromhex(h).decode('latin-1'))" "$META_HEX" 2>/dev/null || echo "$META_HEX")

if echo "$META_BYTES" | grep -q "X3SupplyLedger"; then
  ok "Pallet X3SupplyLedger present in metadata"
else
  fail "Pallet X3SupplyLedger NOT found in metadata"
fi

if echo "$META_BYTES" | grep -q "X3CrossVmRouter"; then
  ok "Pallet X3CrossVmRouter present in metadata"
else
  fail "Pallet X3CrossVmRouter NOT found in metadata"
fi

# --- 4. External bridges disabled ---
echo "-- External bridges disabled --"
# ExternalBridgesEnabled storage key (twox128("X3BridgeAdapters") + twox128("ExternalBridgesEnabled"))
STORAGE_KEY="0x$(python3 -c "
MASK64 = (1 << 64) - 1
PRIME1 = 11400714785074694791
PRIME2 = 14029467366897019727
PRIME3 = 1609587929392839161
PRIME4 = 9650029242287828579
PRIME5 = 2870177450012600261

def rotl(value, bits):
  return ((value << bits) | (value >> (64 - bits))) & MASK64

def round_acc(acc, lane):
  acc = (acc + lane * PRIME2) & MASK64
  acc = rotl(acc, 31)
  return (acc * PRIME1) & MASK64

def merge_round(acc, val):
  val = round_acc(0, val)
  acc ^= val
  return (acc * PRIME1 + PRIME4) & MASK64

def avalanche(value):
  value ^= value >> 33
  value = (value * PRIME2) & MASK64
  value ^= value >> 29
  value = (value * PRIME3) & MASK64
  value ^= value >> 32
  return value & MASK64

def xxh64(data, seed):
  index = 0
  length = len(data)
  if length >= 32:
    v1 = (seed + PRIME1 + PRIME2) & MASK64
    v2 = (seed + PRIME2) & MASK64
    v3 = seed & MASK64
    v4 = (seed - PRIME1) & MASK64
    limit = length - 32
    while index <= limit:
      lanes = [int.from_bytes(data[index + offset:index + offset + 8], 'little') for offset in (0, 8, 16, 24)]
      v1 = round_acc(v1, lanes[0])
      v2 = round_acc(v2, lanes[1])
      v3 = round_acc(v3, lanes[2])
      v4 = round_acc(v4, lanes[3])
      index += 32
    h64 = (rotl(v1, 1) + rotl(v2, 7) + rotl(v3, 12) + rotl(v4, 18)) & MASK64
    h64 = merge_round(h64, v1)
    h64 = merge_round(h64, v2)
    h64 = merge_round(h64, v3)
    h64 = merge_round(h64, v4)
  else:
    h64 = (seed + PRIME5) & MASK64
  h64 = (h64 + length) & MASK64
  while index + 8 <= length:
    lane = int.from_bytes(data[index:index + 8], 'little')
    lane = round_acc(0, lane)
    h64 ^= lane
    h64 = (rotl(h64, 27) * PRIME1 + PRIME4) & MASK64
    index += 8
  if index + 4 <= length:
    h64 ^= int.from_bytes(data[index:index + 4], 'little') * PRIME1 & MASK64
    h64 = (rotl(h64, 23) * PRIME2 + PRIME3) & MASK64
    index += 4
  while index < length:
    h64 ^= data[index] * PRIME5 & MASK64
    h64 = (rotl(h64, 11) * PRIME1) & MASK64
    index += 1
  return avalanche(h64)

def twox128(value):
  data = value.encode()
  return xxh64(data, 0).to_bytes(8, 'little').hex() + xxh64(data, 1).to_bytes(8, 'little').hex()

print(twox128('X3BridgeAdapters') + twox128('ExternalBridgesEnabled'))
")"
VAL=$(rpc "{\"id\":6,\"jsonrpc\":\"2.0\",\"method\":\"state_getStorage\",\"params\":[\"$STORAGE_KEY\"]}" | python3 -c "import sys,json; value=json.load(sys.stdin).get('result'); print('null' if value is None else value)")
if [ "$VAL" = "null" ] || [ "$VAL" = "0x00" ]; then
  ok "ExternalBridgesEnabled = false (value: $VAL)"
else
  fail "ExternalBridgesEnabled = $VAL — external bridges appear ENABLED"
fi

# --- Summary ---
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && echo "RC1 smoke test: PASSED" && exit 0 || echo "RC1 smoke test: FAILED" && exit 1
