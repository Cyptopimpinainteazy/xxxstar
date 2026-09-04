#!/usr/bin/env bash
# run-fresh-validators.sh — drive a LOCAL X3 testnet from freshly generated keys/spec.
#
# Creates N validator base dirs, injects session keys (aura=sr25519, grandpa=ed25519)
# from deployment/chain-specs/fresh/validator-keys/*.suri into each node keystore, then
# starts node 1 as bootnode and the rest peering to it. Uses the CLI surface the current
# binary actually accepts (verified 2026-09-04).
#
# Usage: ./scripts/testnet/run-fresh-validators.sh [count]        (count<= # of generated suri files)
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
NODE_BIN="$ROOT/target/release/x3-chain-node"
FRESH="$ROOT/deployment/chain-specs/fresh"
SPEC="$FRESH/x3-testnet-raw.json"
KEYS="$FRESH/validator-keys"
COUNT="${1:-$(ls "$KEYS"/validator-*.suri 2>/dev/null | wc -l)}"
if [[ "$COUNT" -lt 1 ]]; then echo "[err] no generated validator suri files in $KEYS"; exit 1; fi
CHAIN_ID="$(python3 -c "import json;print(json.load(open('$SPEC'))['id'])")"
BASE="${TESTNET_BASE:-/tmp/x3-fresh-testnet}"
LOG="$BASE/logs"; PID="$BASE/pids"; mkdir -p "$LOG" "$PID"

# stop old
[[ -n "$(ls "$PID" 2>/dev/null)" ]] && for f in "$PID"/*.pid; do kill "$(cat "$f")" 2>/dev/null || true; done; sleep 2
rm -rf "$BASE"/node-* 2>/dev/null || true

start_one () {
  local i="$1" bootnode="${2:-}"
  local p2p=$((30443 + i - 1)) rpc=$((9950 + i - 1)) prom=$((9620 + i - 1))
  local bdir="$BASE/node-$i"
  local kdir="$bdir/chains/$CHAIN_ID/keystore"; mkdir -p "$kdir"
  # inject aura + grandpa session keys
  local AURA GRAN; AURA="$(grep '^aura=' "$KEYS/validator-$i.suri" | cut -d= -f2)"
  GRAN="$(grep '^grandpa=' "$KEYS/validator-$i.suri" | cut -d= -f2)"
  local apub gpub afile gfile
  apub="$(subkey inspect --scheme sr25519 "$AURA" | awk '/Public key \(hex\):/{print $4}')"
  gpub="$(subkey inspect --scheme ed25519 "$GRAN" | awk '/Public key \(hex\):/{print $4}')"
  afile="61757261${apub#0x}"; gfile="6772616e${gpub#0x}"
  printf '%s' "$AURA" > "$kdir/$afile"; chmod 600 "$kdir/$afile"
  printf '%s' "$GRAN" > "$kdir/$gfile"; chmod 600 "$kdir/$gfile"

  local boot_args=(); [[ -n "$bootnode" ]] && boot_args=(--bootnodes "$bootnode")
  local kp=(); kp=(--rpc-port "$rpc" --rpc-methods=Unsafe --rpc-cors=all --rpc-external --unsafe-force-node-key-generation --node-key "$(python3 -c "import secrets;print(secrets.token_hex(32))")")
  nohup "$NODE_BIN" --chain "$SPEC" --base-path "$bdir" --name "fresh-v$i" \
    "${kp[@]}" --validator --force-authoring --allow-private-ip \
    --listen-addr "/ip4/0.0.0.0/tcp/${p2p}" --no-mdns --no-telemetry \
    --prometheus-port "$prom" --prometheus-external --disable-log-color \
    --execution=native-else-wasm "${boot_args[@]}" > "$LOG/node-$i.log" 2>&1 &
  echo $! > "$PID/node-$i.pid"
  echo "[node] fresh-v$i started (p2p=${p2p} rpc=${rpc}); pid $!"
}

echo "== fresh-key testnet (${COUNT} validators) chain_id=$CHAIN_ID =="
start_one 1
# discover peer id
PEER=""
for _ in $(seq 1 60); do
  PEER="$(curl -s -m 2 -H 'Content-Type: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"system_localPeerId","params":[]}' "http://127.0.0.1:9950/" | python3 -c 'import json,sys;print(json.load(sys.stdin).get("result",""))' 2>/dev/null || true)"
  [[ -n "$PEER" ]] && break
  sleep 1
done
if [[ -z "$PEER" ]]; then echo "[err] node1 peer id not detected"; tail -15 "$LOG/node-1.log"; exit 1; fi
BOOT="/ip4/0.0.0.0/tcp/30443/p2p/$PEER"
echo "[net] bootnode: $BOOT"
for i in $(seq 2 "$COUNT"); do start_one "$i" "$BOOT"; done
sleep 3
echo "== started; logs under $LOG =="
for i in $(seq 1 "$COUNT"); do echo "  rpc http://127.0.0.1:$((9950+i-1)) -> $LOG/node-$i.log"; done
