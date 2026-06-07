#!/usr/bin/env bash

set -euo pipefail

cd "${0%/*}/"
cd ../..

export RUSTFLAGS="-D warnings"

echo ">> cargo clippy (default toolchain)"
cargo clippy

echo ">> cargo clippy (Rust 1.89)"
rustup run 1.89 cargo clippy
