#!/usr/bin/env bash
# Can a chain start with its Bitcoin trust root already pinned?
#
# The settlement engine refuses BTC evidence from a header that is not on a
# checkpoint-anchored chain, and `anchor_btc_checkpoint` is a root call — so until a
# checkpoint exists a chain settles no BTC at all. That is the right default and the wrong
# bring-up story for a testnet, which is why a spec can carry the checkpoint in genesis.
# This gate is the live half of that sentence: build a spec with a real Bitcoin regtest
# header pinned, boot a node from it, and read the anchor back out of its storage.
#
# It needs the **dev** runtime, which is the only variant whose `powLimit` is Bitcoin
# regtest's (`0x207fffff`). Without it the runtime correctly refuses a regtest-difficulty
# header — the drill would be measuring the default runtime's proof-of-work policy rather
# than the trust root. `--features dev` is therefore part of the gate, not a convenience.
#
# Set X3_BTC_DRILL_NODE_BIN to skip the build and use an already-built dev binary.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

DEV_BIN="${X3_BTC_DRILL_NODE_BIN:-}"
if [[ -z "$DEV_BIN" ]]; then
  echo "[btc-anchor] building x3-chain-node with the dev runtime"
  cargo build -p x3-chain-node --features dev
  DEV_BIN="$ROOT/target/debug/x3-chain-node"
fi
[[ -x "$DEV_BIN" ]] || { echo "[btc-anchor] no node binary at $DEV_BIN" >&2; exit 2; }

exec python3 "$ROOT/scripts/testnet/btc-checkpoint-genesis-drill.py" --node-bin "$DEV_BIN" "$@"
