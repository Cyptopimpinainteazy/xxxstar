#!/usr/bin/env bash
# run-fresh-validators.sh — boot a LOCAL X3 testnet from freshly generated keys.
#
# Node 1 = bootnode; nodes 2..N peer to it. Each node is started with X3_DEV_SEED=<its
# master seed>, which is the ONLY mechanism on this binary that surfaces the Aura+GRANDPA
# keys to the block-authoring worker (verified 2026-09-04: file-only keystore injection
# does not drive Aura; service maybe_insert_dev_keys inserts programmatically).
#
# Usage: ./scripts/testnet/run-fresh-validators.sh [count]    (count <= generated su-r files)
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
NODE_BIN="$ROOT/target/release/x3-chain-node"
FRESH="$ROOT/deployment/chain-specs/fresh"
SPEC="$FRESH/x3-testnet-plain.json"
KEYS="$FRESH/validator-keys"
COUNT="${1:-$(ls "$KEYS"/validator-*.suri 2>/dev/null | wc -l)}"
: "${COUNT:=7}"
[[ "$COUNT" -lt 1 ]] && { echo "[err] no seeds in $KEYS"; exit 1; }
BASE="${TESTNET_BASE:-$HOME/.local/share/x3/testnet-fresh}"
LOG="$BASE/logs"; PID="$BASE/pids"; mkdir -p "$LOG" "$PID"
# stop existing
for f in "$PID"/node-*.pid; do [[ -f "$f" ]] && kill "$(cat "$f")" 2>/dev/null || true; done
sleep 2
rm -rf "$BASE"/node-* 2>/dev/null || true

start_one() {
  local i="$1" boot="${2:-}"
  local p2p=$((30533+i-1)) rpc=$((9950+i-1)) prom=$((9630+i-1))
  local bdir="$BASE/node-$i"
  mkdir -p "$bdir"
  local seed; seed="$(grep '^seed=' "$KEYS/validator-$i.suri" | cut -d= -f2)"
  local boot_args=(); [[ -n "$boot" ]] && boot_args=(--bootnodes "$boot")
  X3_DEV_SEED="$seed" nohup "$NODE_BIN" --chain "$SPEC" --base-path "$bdir" --name "x3-v$i" \
    --rpc-port "$rpc" --rpc-methods=Unsafe --rpc-cors=all --rpc-external \
    --unsafe-force-node-key-generation --validator --force-authoring --allow-private-ip \
    --listen-addr "/ip4/0.0.0.0/tcp/${p2p}" --no-mdns --no-telemetry \
    --prometheus-port "$prom" --disable-log-color "${boot_args[@]}" > "$LOG/node-$i.log" 2>&1 &
  echo $! > "$PID/node-$i.pid"
  echo "[node] x3-v$i started (X3_DEV_SEED set) p2p=${p2p} rpc=${rpc} pid $!"
}

echo "== fresh-key testnet (${COUNT} nodes) chain=$SPEC =="
start_one 1
# wait until RPC is live AND node-1 has begun authoring (block>0) so libp2p is accepting
PEER=""
for _ in $(seq 1 90); do
  PEER="$(curl -s -m 2 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' "http://127.0.0.1:9950/" | python3 -c 'import json,sys;print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  [[ -n "$PEER" ]] && break; sleep 1
done
# confirm authoring started (genesis #0 -> authored #>0) before peers dial
for _ in $(seq 1 60); do
  n="$(curl -s -m 2 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"chain_getHeader","params":[]}' "http://127.0.0.1:9950/" | python3 -c 'import json,sys;h=json.load(sys.stdin).get("result") or {};print(int((h.get("number") or "0x0"),16))' 2>/dev/null || echo 0)"
  [[ "$n" -gt 1 ]] && break; sleep 1
done
echo "[net] node-1 ready (peer=$PEER, block=$n)"
[[ -z "$PEER" ]] && { echo "[err] node1 peer id not detected"; tail -20 "$LOG/node-1.log"; exit 1; }
BOOT_IP="${BOOT_IP:-127.0.0.1}"
BOOT="/ip4/${BOOT_IP}/tcp/30533/p2p/$PEER"
echo "[net] bootnode $BOOT"
for i in $(seq 2 "$COUNT"); do start_one "$i" "$BOOT"; done
echo "== started: $COUNT validators =="
for i in $(seq 1 "$COUNT"); do echo "  rpc ws://127.0.0.1:$((9950+i-1))  log $LOG/node-$i.log"; done
echo "[next] ./scripts/testnet/status-fresh-testnet.sh  |  stop: kill pids in $PID"
