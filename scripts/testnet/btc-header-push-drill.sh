#!/usr/bin/env bash
# Can the chain follow Bitcoin, not just validate a header?
#
# Companion to `btc-checkpoint-drill.sh`, which proves a chain can be *born* anchored. This one
# proves real headers from a real Bitcoin node can be pushed onto that anchored chain, in order,
# and that a header whose parent was never admitted is refused. It is the gate for the receiving
# half of TICKET-095.
#
# Two things it needs that are not in this repository: a Bitcoin Core install (`--bitcoind-dir`,
# or X3_BITCOIND_DIR) and the **dev** runtime node (the only runtime whose `powLimit` is regtest's,
# so it is the only one that accepts regtest headers). Set X3_BTC_DRILL_NODE_BIN to reuse a build.
#
# Without Bitcoin Core the drill **skips loudly** and verifies nothing — it does not pass.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ -z "${X3_BITCOIND_DIR:-}" ]]; then
  echo "[push-drill] SKIPPED — no Bitcoin Core available (set X3_BITCOIND_DIR to a bitcoin-28.1"
  echo "[push-drill] directory containing bin/). Nothing was verified."
  exit 0
fi

DEV_BIN="${X3_BTC_DRILL_NODE_BIN:-}"
if [[ -z "$DEV_BIN" ]]; then
  echo "[push-drill] building x3-chain-node with the dev runtime"
  cargo build -p x3-chain-node --features dev
  DEV_BIN="$ROOT/target/debug/x3-chain-node"
fi
[[ -x "$DEV_BIN" ]] || { echo "[push-drill] no node binary at $DEV_BIN" >&2; exit 2; }

exec python3 "$ROOT/scripts/testnet/btc-header-push-drill.py" \
  --node-bin "$DEV_BIN" --bitcoind-dir "$X3_BITCOIND_DIR" "$@"
