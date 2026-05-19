#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo build --release -p x3-chain-node
cargo build --release -p x3-cli
cargo build --release -p x3-proof

echo "fresh_build_check: PASS"
