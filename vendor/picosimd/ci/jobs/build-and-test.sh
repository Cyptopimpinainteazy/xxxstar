#!/bin/bash

set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
cd ../..

echo ">> cargo test --features std,ops (default toolchain)"
cargo test --features std,ops

echo ">> cargo test --features std,ops (Rust 1.89)"
rustup run 1.89 cargo test --features std,ops

echo ">> cargo check --features std,ops (wasm32-unknown-unknown)"
cargo check --target=wasm32-unknown-unknown --features std,ops
