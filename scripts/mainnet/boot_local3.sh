#!/usr/bin/env bash
# Boot a 3-validator local X3 testnet (Alice / Bob / Charlie)
# Usage: ./scripts/mainnet/boot_local3.sh
# Requires: target/release/x3-chain-node binary
set -euo pipefail

BINARY="./target/release/x3-chain-node"
PLAIN_SPEC="./chain-specs/x3-rc5-local3-plain.json"
RAW_SPEC="./chain-specs/x3-rc5-local3-raw.json"
LOG_DIR="./logs/local3"
REGENERATE_CHAIN_SPEC="${REGENERATE_CHAIN_SPEC:-1}"
# A network key per validator, generated once and kept. `--alice` supplies
# *session* keys for the dev genesis, not a libp2p identity, so a base path that
# has no network key yet makes the node exit with `NetworkKeyNotFound`. Keeping
# the key also keeps each validator's peer id stable across restarts.
NODE_KEY_DIR="${X3_LOCAL3_NODE_KEY_DIR:-./logs/local3/node-keys}"

capture_build_spec_json() {
  local out_file="$1"
  shift
  "$BINARY" build-spec "$@" \
    | awk 'BEGIN{emit=0} /^[[:space:]]*\{/ {emit=1} emit {print}' > "$out_file"

  if [ ! -s "$out_file" ]; then
    return 1
  fi
  jq -e 'type == "object"' "$out_file" >/dev/null 2>&1
}

# `build-spec` adds a default bootnode when a spec declares none, and that peer
# id belongs to a key no process holds — so the generated spec advertises a
# bootnode that cannot answer, and passing the real one on the same address makes
# libp2p refuse to start. `--disable-default-bootnode` is passed at both call
# sites below for that reason.
node_key_file() {  # node_key_file <name> -> prints a path
  local name="$1" file="$NODE_KEY_DIR/$1.nodekey"
  mkdir -p "$NODE_KEY_DIR"
  if [ ! -s "$file" ]; then
    head -c 32 /dev/urandom | od -An -v -tx1 | tr -d ' \n' > "$file"
  fi
  printf '%s' "$file"
}

if [ ! -f "$BINARY" ]; then
  echo "ERROR: $BINARY not found. Run 'cargo build --release -p x3-chain-node' first."
  exit 1
fi
if [ "$REGENERATE_CHAIN_SPEC" = "1" ]; then
  mkdir -p ./chain-specs
  if ! capture_build_spec_json "$PLAIN_SPEC" --chain local3 --disable-default-bootnode; then
    echo "ERROR: failed to generate valid plain local3 chain spec"
    exit 1
  fi
  if ! capture_build_spec_json "$RAW_SPEC" --chain "$PLAIN_SPEC" --raw --disable-default-bootnode; then
    echo "ERROR: failed to generate valid raw local3 chain spec"
    exit 1
  fi
elif [ ! -f "$RAW_SPEC" ]; then
  if [ -f "./chain-specs/x3-local3-raw.json" ]; then
    RAW_SPEC="./chain-specs/x3-local3-raw.json"
  else
    echo "ERROR: $RAW_SPEC not found. Generate chain specs first."
    exit 1
  fi
fi

mkdir -p "$LOG_DIR"
mkdir -p /tmp/x3-alice /tmp/x3-bob /tmp/x3-charlie

echo "=== Booting X3 local3 testnet ==="

# Alice — RPC on 9944, P2P on 30333
"$BINARY" \
  --chain "$RAW_SPEC" \
  --alice \
  --base-path /tmp/x3-alice \
  --node-key-file "$(node_key_file alice)" \
  --prometheus-port 9615 \
  --port 30333 \
  --rpc-port 9944 \
  --rpc-cors all \
  --rpc-methods unsafe \
  --validator \
  --log info,runtime::x3=debug \
  2>&1 | tee "$LOG_DIR/alice.log" &
ALICE_PID=$!
echo "Alice PID: $ALICE_PID"

# Wait for Alice to announce her peer identity
echo "Waiting for Alice's node identity..."
for i in $(seq 1 30); do
  BOOTNODE=$(grep -m1 "Local node identity is:" "$LOG_DIR/alice.log" 2>/dev/null | awk '{print $NF}' || true)
  if [ -n "$BOOTNODE" ]; then break; fi
  sleep 2
done
if [ -z "${BOOTNODE:-}" ]; then
  echo "ERROR: Could not get Alice's node identity after 60s"
  kill $ALICE_PID 2>/dev/null || true
  exit 1
fi
ALICE_ADDR="/ip4/127.0.0.1/tcp/30333/p2p/$BOOTNODE"
echo "Alice bootnode: $ALICE_ADDR"

# Bob — RPC on 9945, P2P on 30334
"$BINARY" \
  --chain "$RAW_SPEC" \
  --bob \
  --base-path /tmp/x3-bob \
  --node-key-file "$(node_key_file bob)" \
  --prometheus-port 9616 \
  --port 30334 \
  --rpc-port 9945 \
  --rpc-cors all \
  --rpc-methods unsafe \
  --validator \
  --bootnodes "$ALICE_ADDR" \
  --log info,runtime::x3=debug \
  2>&1 | tee "$LOG_DIR/bob.log" &
BOB_PID=$!
echo "Bob PID: $BOB_PID"

# Charlie — RPC on 9946, P2P on 30335
"$BINARY" \
  --chain "$RAW_SPEC" \
  --charlie \
  --base-path /tmp/x3-charlie \
  --node-key-file "$(node_key_file charlie)" \
  --prometheus-port 9617 \
  --port 30335 \
  --rpc-port 9946 \
  --rpc-cors all \
  --rpc-methods unsafe \
  --validator \
  --bootnodes "$ALICE_ADDR" \
  --log info,runtime::x3=debug \
  2>&1 | tee "$LOG_DIR/charlie.log" &
CHARLIE_PID=$!
echo "Charlie PID: $CHARLIE_PID"

echo ""
echo "All 3 validators started."
echo "  Alice   RPC: http://localhost:9944  log: $LOG_DIR/alice.log"
echo "  Bob     RPC: http://localhost:9945  log: $LOG_DIR/bob.log"
echo "  Charlie RPC: http://localhost:9946  log: $LOG_DIR/charlie.log"
echo ""
echo "Watching for block production (Ctrl+C to stop)..."
tail -F "$LOG_DIR/alice.log" | grep --line-buffered -E "Imported|Finalized|peers"
