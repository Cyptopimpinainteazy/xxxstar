#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

# Check formatting across the whole workspace (fast, no compile)
cargo fmt --check

# Skip WASM build during check to avoid wasm-opt-sys C++ compilation.
# The release build below will do the full WASM build.
export SKIP_WASM_BUILD=1

# Check chain-critical packages only.
# apps/analytics pulls brotli-8.0.2 (rustc 1.88 const-eval ICE) — excluded.
# x3-sidecar/x3-gateway pull h2-0.3.27 (rustc 1.88 borrow-check ICE) — excluded.
# x3-chain-runtime causes rustc 1.88 SIGSEGV on metadata generation — excluded from check, included in build.
CHAIN_PKGS=(
  x3-chain-node
  pallet-x3-supply-ledger
  pallet-x3-cross-vm-router
  pallet-x3-asset-registry
  pallet-x3-account-registry
  pallet-x3-atomic-kernel
  pallet-x3-settlement-engine
  x3-proof
  x3-ixl
  x3-cli
)

for pkg in "${CHAIN_PKGS[@]}"; do
  echo "==> cargo check -p $pkg"
  cargo check -p "$pkg"
done

# Full release builds (WASM build re-enabled for node binary)
unset SKIP_WASM_BUILD
cargo build --release -p x3-chain-node
cargo build --release -p x3-cli
cargo build --release -p x3-proof

echo "fresh_build_check: PASS"
