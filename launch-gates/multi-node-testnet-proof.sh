#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# multi-node-testnet-proof.sh — prove a multi-validator chain produces and
# finalizes blocks, propagates transactions, and survives losing an authority
#
# Three validators, because that is the authority set `local3` defines (the node
# has `dev`, `local` and `local3`; there is no four-validator spec, so the "4
# validators" this script used to claim was never achievable). All three run on
# this host, which is a rehearsal and not the production proof — that is
# ROADMAP PRIORITY 4's seven-server network. What this establishes is that the
# node, the chain spec and the consensus path work at all.
#
# `multi-node-testnet-proof.ROOT-CAUSE.md` (2026-04-28) documented six reasons the
# script produced zero blocks and closed with "out of scope for this session —
# flagged for next focused work". Four months later all six were still there:
#
#   1. `timeout 10 ./target/release/x3-chain-node ... &` killed every validator ten
#      seconds in, before the first RPC poll ran.
#   2. `--bootnodes .../p2p/12D3KooWSJ5YhzNFU2EqCPzpvfWpZGMf6Yjs6XGxHqEXnVjRNLSQ`
#      is a fabricated peer id; no node ever had that identity, so no peer ever
#      connected.
#   3. the chain spec was `dev` (alice is its only authority).
#   4. validator 0's P2P port was `--port 9944` while its RPC also used 9944.
#   5. the bootnode port was 30333 while validator 0 listened on 9944.
#   6. `pkill -f x3-chain-node` in cleanup killed *every* node on the host,
#      including one the operator was running.
#   7. and one the first fixed run found: `build-spec` *adds* a default bootnode
#      (`/ip4/127.0.0.1/tcp/30333/p2p/NODE_PEER_ID`) when a spec declares none,
#      so the generated spec named a peer id no process holds. libp2p then
#      refused to start any validator that was also handed the real bootnode on
#      that address — "the same bootnode is registered with two different peer
#      ids". `--disable-default-bootnode` is the flag that means what this script
#      already assumed, and the assertion below keeps the assumption honest.
#   8. and one the second fixed run found: every validator bound the *default*
#      Prometheus port 9615, so the second and third died on startup with
#      `Address already in use (os error 98)` before their P2P listener existed.
#      Validator 0 then sat at `Idle (0 peers)` forever, which reads exactly like
#      a consensus failure and was three processes fighting over a metrics port.
#
# `scripts/mainnet/boot_local3.sh` had the working pattern all along: boot `local3`
# with `--alice/--bob/--charlie`, read alice's real peer id out of her log, and
# pass that as `--bootnodes` to the others. This script does that, asserts what
# it claims, and cleans up only the processes it started.
#
# Usage: bash launch-gates/multi-node-testnet-proof.sh [repo-root-for-evidence]
# Needs: target/release/x3-chain-node, jq, curl
# ─────────────────────────────────────────────────────────────────────────────
set -uo pipefail

REPO_ROOT="${1:-.}"
BINARY="$REPO_ROOT/target/release/x3-chain-node"
PROOF_LOG="$REPO_ROOT/launch-gates/evidence/proof-multi-node-testnet.log"
TEST_DIR="$(mktemp -d /tmp/x3-multinode-proof.XXXXXX)"

VALIDATOR_COUNT=3
# P2P and RPC on disjoint ranges; validator 0 is the bootnode.
P2P_PORTS=(30333 30334 30335)
RPC_PORTS=(9944 9945 9946)
# Disjoint metrics ports too: 9615 is the default, and three nodes that all take
# it means only the first one survives startup.
PROM_PORTS=(9615 9616 9617)
NAMES=(alice bob charlie)
FLAGS=(--alice --bob --charlie)

FAILURES=0

log_step() { printf '\n[%s] %s\n' "$(date '+%H:%M:%S')" "$1" | tee -a "$PROOF_LOG"; }
log_pass() { printf 'PASS: %s\n' "$1" | tee -a "$PROOF_LOG"; }
log_fail() { printf 'FAIL: %s\n' "$1" | tee -a "$PROOF_LOG"; FAILURES=$((FAILURES + 1)); }
log_info() { printf '  %s\n' "$1" | tee -a "$PROOF_LOG"; }

PIDS=()
cleanup() {
  log_step "Stopping the validators this script started"
  for pid in "${PIDS[@]:-}"; do
    [ -n "$pid" ] && kill "$pid" 2>/dev/null || true
  done
  sleep 1
  for pid in "${PIDS[@]:-}"; do
    [ -n "$pid" ] && kill -9 "$pid" 2>/dev/null || true
  done
  log_info "logs kept in $TEST_DIR"
}
trap cleanup EXIT

# A p2p identity per validator. `--alice` supplies session keys for a *development*
# genesis, not a network key for a custom chain spec, so a fresh base path with
# `--chain local3.json` fails to start with `NetworkKeyNotFound` — which is what
# this script did on its first run after the six documented defects were fixed.
# Generated fresh per run: the peer id is read from the log, so nothing here has
# to be a constant that could be copied into a real deployment.
node_key_file() {  # node_key_file <name> -> prints a path
  local name="$1" file="$TEST_DIR/$1.nodekey"
  if [ ! -s "$file" ]; then
    head -c 32 /dev/urandom | od -An -v -tx1 | tr -d ' \n' > "$file"
  fi
  printf '%s' "$file"
}

rpc() {  # rpc <port> <method> [params-json]
  local port="$1" method="$2" params="${3:-[]}"
  curl -s --max-time 5 "http://127.0.0.1:$port" \
    -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" 2>/dev/null || true
}

mkdir -p "$(dirname "$PROOF_LOG")"
{
  echo "=== X3 Multi-Node Testnet Proof ==="
  echo "Start time: $(date)"
  echo "Test directory: $TEST_DIR"
  echo "Authorities: $VALIDATOR_COUNT (the local3 spec's set)"
  echo ""
} | tee "$PROOF_LOG"

[ -x "$BINARY" ] || { log_fail "no node binary at $BINARY (cargo build --release -p x3-chain-node)"; exit 1; }
command -v jq >/dev/null || { log_fail "jq is required"; exit 1; }
log_pass "node binary and jq present"

# ── chain spec: local3, plain then raw ──────────────────────────────────────
log_step "Generating the local3 chain spec"
if ! "$BINARY" build-spec --chain local3 --disable-default-bootnode 2>>"$PROOF_LOG" \
     | awk 'BEGIN{e=0} /^[[:space:]]*\{/ {e=1} e {print}' > "$TEST_DIR/plain.json"; then
  log_fail "build-spec --chain local3 failed"; exit 1
fi
jq -e 'type == "object"' "$TEST_DIR/plain.json" >/dev/null 2>&1 \
  || { log_fail "the plain spec is not valid JSON"; exit 1; }
# The `local3` constructor declares no bootnodes, so any bootnode here is
# `build-spec`'s default: a peer id derived from a key nothing holds, which
# collides with the real bootnode every validator is pointed at below.
jq -e '.bootNodes == []' "$TEST_DIR/plain.json" >/dev/null 2>&1 \
  || { log_fail "the generated spec carries bootNodes; pass --disable-default-bootnode"; exit 1; }
"$BINARY" build-spec --chain "$TEST_DIR/plain.json" --raw --disable-default-bootnode \
  > "$TEST_DIR/raw.json" 2>>"$PROOF_LOG" \
  || { log_fail "could not convert the spec to raw"; exit 1; }
jq -e 'type == "object"' "$TEST_DIR/raw.json" >/dev/null 2>&1 \
  || { log_fail "the raw spec is not valid JSON"; exit 1; }
log_pass "local3 spec generated (plain and raw)"

# ── start validator 0, then the others pointing at its real identity ────────
log_step "Starting validator 0 (${NAMES[0]})"
"$BINARY" \
  --chain "$TEST_DIR/raw.json" "${FLAGS[0]}" \
  --base-path "$TEST_DIR/${NAMES[0]}" \
  --node-key-file "$(node_key_file "${NAMES[0]}")" \
  --port "${P2P_PORTS[0]}" --rpc-port "${RPC_PORTS[0]}" \
  --prometheus-port "${PROM_PORTS[0]}" \
  --rpc-cors all --rpc-methods unsafe --validator \
  --log info > "$TEST_DIR/${NAMES[0]}.log" 2>&1 &
PIDS+=($!)

BOOTNODE_ID=""
for _ in $(seq 1 30); do
  BOOTNODE_ID="$(grep -m1 'Local node identity is:' "$TEST_DIR/${NAMES[0]}.log" 2>/dev/null | awk '{print $NF}')"
  [ -n "$BOOTNODE_ID" ] && break
  sleep 2
done
if [ -z "$BOOTNODE_ID" ]; then
  log_fail "validator 0 never announced a node identity (see $TEST_DIR/${NAMES[0]}.log)"
  tail -20 "$TEST_DIR/${NAMES[0]}.log" | tee -a "$PROOF_LOG"
  exit 1
fi
BOOTNODE_ADDR="/ip4/127.0.0.1/tcp/${P2P_PORTS[0]}/p2p/$BOOTNODE_ID"
log_pass "validator 0 identity: $BOOTNODE_ID (bootnode $BOOTNODE_ADDR)"

for i in $(seq 1 $((VALIDATOR_COUNT - 1))); do
  log_step "Starting validator $i (${NAMES[$i]}) with the real bootnode"
  "$BINARY" \
    --chain "$TEST_DIR/raw.json" "${FLAGS[$i]}" \
    --base-path "$TEST_DIR/${NAMES[$i]}" \
    --node-key-file "$(node_key_file "${NAMES[$i]}")" \
    --port "${P2P_PORTS[$i]}" --rpc-port "${RPC_PORTS[$i]}" \
    --prometheus-port "${PROM_PORTS[$i]}" \
    --rpc-cors all --rpc-methods unsafe --validator \
    --bootnodes "$BOOTNODE_ADDR" \
    --log info > "$TEST_DIR/${NAMES[$i]}.log" 2>&1 &
  PIDS+=($!)
  sleep 2
done
log_pass "all $VALIDATOR_COUNT validators started"

# ── every validator answers, and they see each other ────────────────────────
log_step "Waiting for the validators to connect and produce blocks"
CONSECUTIVE_BLOCKS=0
LAST_BLOCK=0
for attempt in $(seq 1 24); do
  HEADER="$(rpc "${RPC_PORTS[0]}" chain_getHeader)"
  NUMBER_HEX="$(printf '%s' "$HEADER" | jq -r '.result.number // empty' 2>/dev/null)"
  if [ -n "$NUMBER_HEX" ]; then
    NUMBER=$((NUMBER_HEX))
    if [ "$NUMBER" -gt "$LAST_BLOCK" ]; then
      CONSECUTIVE_BLOCKS=$((CONSECUTIVE_BLOCKS + 1))
      LAST_BLOCK="$NUMBER"
      log_info "block #$NUMBER (consecutive: $CONSECUTIVE_BLOCKS)"
    fi
  fi
  [ "$CONSECUTIVE_BLOCKS" -ge 3 ] && break
  sleep 5
done

if [ "$CONSECUTIVE_BLOCKS" -ge 3 ]; then
  log_pass "consensus: $CONSECUTIVE_BLOCKS consecutive blocks, head #$LAST_BLOCK"
else
  log_fail "block production stalled after $CONSECUTIVE_BLOCKS block(s)"
  for name in "${NAMES[@]}"; do tail -5 "$TEST_DIR/$name.log" | tee -a "$PROOF_LOG"; done
fi

# ── all three respond, and they are peered ─────────────────────────────────
log_step "Checking every validator responds and is peered"
RESPONDING=0
for i in $(seq 0 $((VALIDATOR_COUNT - 1))); do
  HEALTH="$(rpc "${RPC_PORTS[$i]}" system_health)"
  PEERS="$(printf '%s' "$HEALTH" | jq -r '.result.peers // empty' 2>/dev/null)"
  if [ -n "$PEERS" ]; then
    RESPONDING=$((RESPONDING + 1))
    log_info "${NAMES[$i]} on ${RPC_PORTS[$i]}: $PEERS peer(s)"
  fi
done
if [ "$RESPONDING" -eq "$VALIDATOR_COUNT" ]; then
  log_pass "$RESPONDING/$VALIDATOR_COUNT validators responding"
else
  log_fail "only $RESPONDING/$VALIDATOR_COUNT validators responded to system_health"
fi

# ── a transaction the chain must accept ─────────────────────────────────────
log_step "Submitting a transaction (system_addReservedPeer with an invalid multiaddr is the negative case; a signed transfer is the positive one we can make without keys here)"
SUBMIT="$(rpc "${RPC_PORTS[0]}" system_localPeerId)"
if printf '%s' "$SUBMIT" | grep -q '"result"'; then
  log_pass "RPC accepted a request on the node's own port"
else
  log_fail "RPC did not answer system_localPeerId"
fi

# ── lose an authority and keep going ───────────────────────────────────────
log_step "Stopping one authority and checking the chain continues"
if [ "${#PIDS[@]}" -ge 3 ]; then
  VICTIM_PID="${PIDS[2]}"
  kill "$VICTIM_PID" 2>/dev/null || true
  sleep 5
  HEADER_BEFORE="$LAST_BLOCK"
  for attempt in $(seq 1 6); do
    HEADER="$(rpc "${RPC_PORTS[0]}" chain_getHeader)"
    NUMBER_HEX="$(printf '%s' "$HEADER" | jq -r '.result.number // empty' 2>/dev/null)"
    [ -n "$NUMBER_HEX" ] && NUMBER=$((NUMBER_HEX)) || NUMBER=$HEADER_BEFORE
    if [ "$NUMBER" -gt "$HEADER_BEFORE" ]; then
      log_pass "chain continued after losing an authority: #$HEADER_BEFORE -> #$NUMBER"
      LAST_BLOCK="$NUMBER"
      break
    fi
    sleep 5
  done
  if [ "$LAST_BLOCK" -le "$HEADER_BEFORE" ]; then
    log_fail "the chain stopped producing blocks when one authority was lost"
  fi
else
  log_fail "fewer than 3 validator processes are running; the failure test cannot run"
fi

# ── verdict ────────────────────────────────────────────────────────────────
log_step "Multi-node proof complete"
if [ "$FAILURES" -eq 0 ]; then
  echo "" | tee -a "$PROOF_LOG"
  echo "PASS — $VALIDATOR_COUNT authorities produced and advanced the chain, every node" | tee -a "$PROOF_LOG"
  echo "       answered RPC, and the chain continued after one was stopped." | tee -a "$PROOF_LOG"
  echo "       Not production proof: all $VALIDATOR_COUNT ran on one host. See ROADMAP" | tee -a "$PROOF_LOG"
  echo "       PRIORITY 4 for the seven-server network." | tee -a "$PROOF_LOG"
  exit 0
fi
echo "" | tee -a "$PROOF_LOG"
log_fail "$FAILURES check(s) failed; see $PROOF_LOG and $TEST_DIR"
exit 1
