#!/usr/bin/env bash
# check-runtime-variants.sh — migration dry-run across every runtime variant.
#
# FEATURE_REGISTRY's `triforge_runtime` entry records the gap: "No automated
# migration-dry-run across all 6 construct_runtime! variants". The standalone
# `try-runtime` CLI does not exist in the pinned Polkadot SDK, so the dry-run is
# in-process: `runtime_upgrade_rehearsal` executes the same OnRuntimeUpgrade hooks
# an upgrade would (AllPalletsWithSystem + the Migrations tuple) for whichever
# variant the active feature set selects, and asserts the work fits in a block.
#
# One `cargo test` per variant is what makes it cover all of them: the variant is
# chosen by features, so each run compiles and exercises a different
# construct_runtime!.
#
# Usage: scripts/check-runtime-variants.sh [variant-name ...]
#   No arguments runs every variant. Unknown names are an error, not a skip.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

LOG_DIR="$ROOT/.ai/runlogs"
mkdir -p "$LOG_DIR"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

# name|extra-features — the six combinations the runtime supports (five
# construct_runtime! blocks, plus the testnet config toggle on the full set).
VARIANTS=(
  "full|"
  "dev|dev"
  "dev+frontier|dev,frontier"
  "frontier|frontier"
  "mainnet-rc1|mainnet-rc1"
  "testnet|testnet"
)

if [ "$#" -gt 0 ]; then
  SELECTED=("$@")
else
  SELECTED=()
  for entry in "${VARIANTS[@]}"; do SELECTED+=("${entry%%|*}"); done
fi

declare -a NAMES=() RESULTS=() SECS=()
FAILED=0

run_variant() {
  local name="$1" feats="$2"
  local log="$LOG_DIR/runtime-variant-$STAMP-$name.log"
  # `tuples-96` used to be a feature of `x3-chain-runtime`. TICKET-093 moved it
  # onto the runtime's `frame-support` dependency — the runtime cannot be built
  # without it, and a feature that has to be on for the build to succeed is not
  # a feature. Passing it here made all six variants fail with
  # "the package 'x3-chain-runtime' does not contain this feature: tuples-96".
  local feature_list="std"
  [ -n "$feats" ] && feature_list="$feature_list,$feats"

  echo ""
  echo "=== runtime variant: $name (features: $feature_list) ==="
  local start=$SECONDS
  if SKIP_WASM_BUILD="${SKIP_WASM_BUILD:-1}" cargo test -p x3-chain-runtime \
      --no-default-features --features "$feature_list" \
      runtime_upgrade_rehearsal > "$log" 2>&1; then
    echo "PASS ($((SECONDS - start))s) — $log"
    RESULTS+=("PASS")
  else
    RESULTS+=("FAIL")
    FAILED=1
    echo "FAIL ($((SECONDS - start))s) — $log"
    grep -E "^test |^error|panicked|test result: FAILED" "$log" | head -12
  fi
  NAMES+=("$name")
  SECS+=("$((SECONDS - start))")
}

for want in "${SELECTED[@]}"; do
  found=""
  for entry in "${VARIANTS[@]}"; do
    name="${entry%%|*}"; feats="${entry#*|}"
    if [ "$name" = "$want" ]; then
      run_variant "$name" "$feats"
      found=1
      break
    fi
  done
  if [ -z "$found" ]; then
    echo "unknown variant: $want" >&2
    echo "known: $(for e in "${VARIANTS[@]}"; do printf '%s ' "${e%%|*}"; done)" >&2
    exit 2
  fi
done

echo ""
echo "──────── runtime variant dry-run summary ($STAMP) ────────"
printf '%-18s %-6s %s\n' "VARIANT" "RESULT" "SECONDS"
for i in "${!NAMES[@]}"; do
  printf '%-18s %-6s %s\n' "${NAMES[$i]}" "${RESULTS[$i]}" "${SECS[$i]}"
done

if [ "$FAILED" = 1 ]; then
  echo ""
  echo "runtime variant dry-run: FAILED"
  exit 1
fi
echo ""
echo "runtime variant dry-run: all variants passed"
