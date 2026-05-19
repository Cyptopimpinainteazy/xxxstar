#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${ARTIFACT_DIR:-$ROOT_DIR/artifacts/node}"
CONTAINER_IMAGE="${CONTAINER_IMAGE:-rust:1.92.0-bookworm}"
CONTAINER_CARGO_HOME="${CONTAINER_CARGO_HOME:-/usr/local/cargo}"
LOG_DIR="${BUILD_LOG_DIR:-$ROOT_DIR/artifacts/logs}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_FILE="${BUILD_LOG_FILE:-$LOG_DIR/build-node-artifact-container-$TIMESTAMP.log}"

mkdir -p "$ARTIFACT_DIR" "$LOG_DIR"

# Persist build logs while still streaming to stdout/stderr.
exec > >(tee -a "$LOG_FILE") 2>&1
echo "Build log: $LOG_FILE"

# Build x3-chain-node in an isolated container and export the release binary.
docker run --rm --name x3-node-artifact-build \
  -v "$ROOT_DIR:/work" \
  -v "$HOME/.cargo/registry:$CONTAINER_CARGO_HOME/registry" \
  -v "$HOME/.cargo/git:$CONTAINER_CARGO_HOME/git" \
  -e "CARGO_HOME=$CONTAINER_CARGO_HOME" \
  -w /work \
  "$CONTAINER_IMAGE" \
  bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive

    apt-get update
    apt-get install -y --no-install-recommends \
      ca-certificates \
      clang \
      cmake \
      curl \
      g++ \
      gcc \
      git \
      libssl-dev \
      lld \
      make \
      pkg-config \
      protobuf-compiler

    if ! command -v rustup >/dev/null 2>&1; then
      curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
    fi

    # Ensure rustup/cargo bins are on PATH regardless of install location.
    if [ -f "$CARGO_HOME/env" ]; then
      . "$CARGO_HOME/env"
    elif [ -f /usr/local/cargo/env ]; then
      . /usr/local/cargo/env
    elif [ -f "$HOME/.cargo/env" ]; then
      . "$HOME/.cargo/env"
    fi

    rustup target add wasm32-unknown-unknown

    # Always use all available container cores to avoid single-threaded wasm-opt bottlenecks.
    export CARGO_BUILD_JOBS="$(nproc)"
    export CARGO_INCREMENTAL=0
    export RUST_MIN_STACK=268435456

    cargo build -p x3-chain-node --release --target-dir /work/target-ci

    cp /work/target-ci/release/x3-chain-node /work/artifacts/node/x3-chain-node
    sha256sum /work/artifacts/node/x3-chain-node > /work/artifacts/node/x3-chain-node.sha256
  '

ls -lh "$ARTIFACT_DIR/x3-chain-node" "$ARTIFACT_DIR/x3-chain-node.sha256"
echo "Build log saved: $LOG_FILE"
echo "Artifact ready: $ARTIFACT_DIR/x3-chain-node"