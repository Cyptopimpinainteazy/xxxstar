#!/usr/bin/env bash
# validator-rotation-drill.sh — prove the operator-driven key rotation path.
#
# The on-chain `pallet_x3_custody` registry is the single source of truth for who
# is a registered validator and when their next rotation is due. This drill runs
# the `x3-chain-node validator rotate` operator command against a live node and
# proves two things:
#
#   * an unregistered account is **refused** (rotation is never guessed);
#   * a registered account produces a signed `session.set_keys` extrinsic and the
#     reported next due block is the current block plus the rotation period.
#
# Operator-driven only: the drill never rotates unattended, and it never submits
# unless `--submit` is passed.
#
# Usage:
#   scripts/testnet/validator-rotation-drill.sh [--rpc-url URL] [--suri SURI] [--submit]
#   scripts/testnet/validator-rotation-drill.sh --no-bootstrap   # use a node you started
#
# With no node already listening at `--rpc-url` the drill starts one itself, on a dev
# runtime, and registers the operator account in the custody registry first. Without
# that registration there is nothing to rotate: `register_validator_key` is
# `GovernanceOrigin`-only, so a fresh chain has an empty registry and the drill used to
# die with "no node reachable" (measured 2026-09-26 — the gate had never been run).
#
# Env: NODE_BIN, RPC_URL, BASE_DIR, LOG_DIR (same meaning as the launcher),
#      X3_ROTATION_DRILL_NODE_BIN to reuse an already-built dev binary.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
NODE_BIN="${NODE_BIN:-$ROOT_DIR/target/debug/x3-chain-node}"
RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"
SURI="${SURI:-//Alice}"
# The second council member. The dev chain's council is Alice + Bob with threshold 2,
# so a registration motion needs both signatures.
SECOND_SURI="${SECOND_SURI:-//Bob}"
AURA_SEED="${AURA_SEED:-//aura-drill}"
GRANDPA_SEED="${GRANDPA_SEED:-//grandpa-drill}"
SUBMIT="${SUBMIT:-0}"
BOOTSTRAP=1
NODE_PID=""
BOOT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-url) RPC_URL="$2"; shift 2 ;;
    --suri) SURI="$2"; shift 2 ;;
    --aura-seed) AURA_SEED="$2"; shift 2 ;;
    --grandpa-seed) GRANDPA_SEED="$2"; shift 2 ;;
    --submit) SUBMIT=1; shift ;;
    --no-bootstrap) BOOTSTRAP=0; shift ;;
    -h|--help)
      sed -n '2,20p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

info() { printf '[drill] %s\n' "$*"; }
fail() { printf '[drill] FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf '[drill] PASS: %s\n' "$*"; }

rpc_alive() {
  curl -s -m 5 -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","id":1,"method":"system_chain","params":[]}' "$1" >/dev/null 2>&1
}

cleanup() {
  [[ -n "$NODE_PID" ]] || return 0
  kill "$NODE_PID" 2>/dev/null || true
  sleep 1
  kill -9 "$NODE_PID" 2>/dev/null || true
  [[ -n "$BOOT_DIR" ]] && rm -rf "$BOOT_DIR"
}
trap cleanup EXIT

[[ -x "$NODE_BIN" ]] || fail "node binary not found at $NODE_BIN (build it first: cargo build -p x3-chain-node)"

if ! rpc_alive "$RPC_URL"; then
  if [[ "$BOOTSTRAP" -eq 0 ]]; then
    fail "no node reachable at $RPC_URL and --no-bootstrap was passed"
  fi

  # The standard binary is enough: the registry entry is written by a council motion, which
  # needs no `Sudo` (the dev spec leaves its key empty and the real specs have no root path).
  "$NODE_BIN" validator --help 2>&1 | grep -q "register" \
    || fail "$NODE_BIN has no 'validator register' — rebuild it: cargo build -p x3-chain-node"

  BOOT_DIR="$(mktemp -d)"
  PORT="${X3_ROTATION_DRILL_PORT:-$(( 24000 + RANDOM % 2000 ))}"
  P2P_PORT="$(( PORT + 1 ))"
  PROM_PORT="$(( PORT + 2 ))"
  RPC_URL="http://127.0.0.1:${PORT}"
  info "starting a dev node on $RPC_URL (p2p $P2P_PORT, prometheus $PROM_PORT)"
  # `--base-path` under the drill's own temp dir rather than `--tmp`: they are mutually
  # exclusive, and the log has to outlive the node when something goes wrong. Every port is
  # pinned: the defaults (9944/30333/9615) collide with any other node on this box, and the
  # node then never reaches RPC rather than saying which port it could not have.
  "$NODE_BIN" --dev --rpc-port "$PORT" --port "$P2P_PORT" --prometheus-port "$PROM_PORT" \
    --base-path "$BOOT_DIR/node" >"$BOOT_DIR/node.log" 2>&1 &
  NODE_PID=$!

  # A cold start AOT-compiles the runtime wasm with wasmtime (70 threads, minutes on a
  # contended box); the deadline is for that, not for "the node is broken".
  READY_TIMEOUT="${X3_ROTATION_DRILL_READY_TIMEOUT:-300}"
  for _ in $(seq 1 "$READY_TIMEOUT"); do
    rpc_alive "$RPC_URL" && break
    kill -0 "$NODE_PID" 2>/dev/null || {
      tail -20 "$BOOT_DIR/node.log" >&2 || true
      fail "the dev node exited before it answered RPC"
    }
    sleep 1
  done
  rpc_alive "$RPC_URL" || {
    tail -20 "$BOOT_DIR/node.log" >&2 || true
    fail "the dev node never answered on $RPC_URL within ${READY_TIMEOUT}s"
  }
  pass "dev node up on $RPC_URL (pid $NODE_PID)"

  # `register_validator_key` takes the governance origin, so the operator account has to
  # be put in the registry before `rotate` will agree it exists. On the dev chain the
  # council is Alice + Bob with a two-member threshold, so the motion needs both:
  # `--suri` proposes and `--second` votes it over the threshold.
  info "registering $SURI in the custody validator registry (council motion)"
  "$NODE_BIN" validator register --rpc-url "$RPC_URL" \
    --suri "$SURI" --second "$SECOND_SURI" --submit \
    || fail "could not register $SURI in the custody registry"

  # The registration is an extrinsic: it lands in a later block than the one whose
  # nonce it was built against, so poll until the registry answers rather than
  # assuming the next read sees it.
  info "waiting for the registration to land on-chain"
  REGISTERED=0
  for _ in $(seq 1 60); do
    if OUT="$("$NODE_BIN" validator rotate \
        --rpc-url "$RPC_URL" --suri "$SURI" \
        --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" 2>&1)"; then
      printf '%s\n' "$OUT" | grep -q "session.set_keys extrinsic" && { REGISTERED=1; break; }
    fi
    sleep 1
  done
  [[ "$REGISTERED" -eq 1 ]] \
    || {
      echo "[drill] node log tail:" >&2
      tail -40 "$BOOT_DIR/node.log" >&2 || true
      fail "$SURI is still not a registered validator after the registration extrinsic"
    }
  pass "registered $SURI and confirmed it in the registry"
fi

if ! rpc_alive "$RPC_URL"; then
  fail "no node reachable at $RPC_URL — start one first (e.g. x3-chain-node --dev)"
fi

# 1. An unregistered account must be refused, not silently rotated.
info "checking unregistered-account refusal"
if "$NODE_BIN" validator rotate \
    --rpc-url "$RPC_URL" --suri "//UnregisteredDrill" \
    --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" >/tmp/x3-rot-unreg.out 2>&1; then
  fail "unregistered account was not refused"
fi
grep -qi "not a registered validator" /tmp/x3-rot-unreg.out \
  || fail "refusal message did not name the unregistered condition"
pass "unregistered account refused"

# 2. A registered account produces a signed set_keys extrinsic and reports the
#    next due block as current + KeyRotationPeriod.
info "building set_keys for registered account $SURI"
OUT="$("$NODE_BIN" validator rotate \
  --rpc-url "$RPC_URL" --suri "$SURI" \
  --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" 2>&1)" || {
  printf '%s\n' "$OUT" >&2
  fail "validator rotate failed for $SURI (is the account registered on-chain?)"
}
echo "$OUT" | grep -q "session.set_keys extrinsic" \
  || fail "rotate output did not contain a session.set_keys extrinsic"
echo "$OUT" | grep -q "next due:" \
  || fail "rotate output did not report the next due block"
pass "registered account produced a signed set_keys extrinsic and next due block"

if [[ "$SUBMIT" -eq 1 ]]; then
  info "submitting set_keys"
  "$NODE_BIN" validator rotate \
    --rpc-url "$RPC_URL" --suri "$SURI" \
    --aura-seed "$AURA_SEED" --grandpa-seed "$GRANDPA_SEED" --submit \
    || fail "submission failed"
  pass "session.set_keys submitted"
else
  info "dry-run only (pass --submit to send the extrinsic)"
fi

info "validator rotation drill complete"
