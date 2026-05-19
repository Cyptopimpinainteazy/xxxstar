#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="$ROOT_DIR/chain-specs"
mkdir -p "$OUT_DIR"

cd "$ROOT_DIR"

cargo build --release -p x3-chain-node

NODE_BIN="$ROOT_DIR/target/release/x3-chain-node"

"$NODE_BIN" build-spec --chain production --disable-default-bootnode > "$OUT_DIR/x3-mainnet-plain.json"
"$NODE_BIN" build-spec --chain "$OUT_DIR/x3-mainnet-plain.json" --raw --disable-default-bootnode > "$OUT_DIR/x3-mainnet-raw.json"

echo "generated: $OUT_DIR/x3-mainnet-plain.json"
echo "generated: $OUT_DIR/x3-mainnet-raw.json"
