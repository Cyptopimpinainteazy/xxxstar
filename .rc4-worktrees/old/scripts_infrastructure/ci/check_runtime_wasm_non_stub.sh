#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

if [[ -n "${SKIP_WASM_BUILD:-}" ]]; then
  echo "::error::SKIP_WASM_BUILD is set. Mainnet gate requires a real runtime wasm artifact."
  exit 1
fi

cargo build -p x3-chain-runtime --release

mapfile -t wasm_binary_files < <(find target/release/build -path '*/out/wasm_binary.rs' -print)
if [[ ${#wasm_binary_files[@]} -eq 0 ]]; then
  echo "::error::No generated wasm_binary.rs found under target/release/build"
  exit 1
fi

stub_hits=0
for f in "${wasm_binary_files[@]}"; do
  if grep -q "WASM_BINARY: Option<&\[u8\]> = None" "$f"; then
    echo "::error file=${f}::Runtime wasm build output is stubbed (WASM_BINARY=None)."
    stub_hits=$((stub_hits + 1))
  fi
done

if [[ $stub_hits -ne 0 ]]; then
  exit 1
fi

if ! find target/release/wbuild -name '*.wasm' -type f -size +1k | grep -q .; then
  echo "::error::No non-trivial wasm artifact found in target/release/wbuild"
  exit 1
fi

echo "Runtime wasm non-stub gate passed"
