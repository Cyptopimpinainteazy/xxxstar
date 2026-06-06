#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_INCREMENTAL=0
export RUST_MIN_STACK="${RUST_MIN_STACK:-67108864}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/bench-node}"

NODE_BIN="$ROOT_DIR/$CARGO_TARGET_DIR/release/x3-chain-node"

info() { echo "[refresh-runtime-weights] $*"; }
error() { echo "[refresh-runtime-weights] ERROR: $*" >&2; }

info "Building x3-chain-node with runtime-benchmarks feature"
build_log="$(mktemp -t x3-refresh-weights.XXXXXX.log)"
if ! cargo build --release --features runtime-benchmarks -p x3-chain-node 2> >(tee "$build_log" >&2); then
  error "Build failed. See $build_log for details."
  exit 1
fi

if [[ ! -x "$NODE_BIN" ]]; then
  error "Benchmark binary not found at $NODE_BIN"
  exit 1
fi

info "Generating pallet_svm weights"
"$NODE_BIN" benchmark pallet \
  --chain=dev \
  --pallet=pallet_svm \
  --extrinsic='*' \
  --steps=50 \
  --repeat=20 \
  --output=pallets/svm-runtime/src/weights.rs

info "Generating pallet_swarm weights"
"$NODE_BIN" benchmark pallet \
  --chain=dev \
  --pallet=pallet_swarm \
  --extrinsic='*' \
  --steps=50 \
  --repeat=20 \
  --output=pallets/swarm/src/weights.rs

info "Done: refreshed weights for pallet_svm and pallet_swarm."
