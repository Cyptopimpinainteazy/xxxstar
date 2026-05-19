#!/usr/bin/env bash
set -euo pipefail

# Fails if unresolved launch placeholder tokens exist in Phase 13f docs.
# Use inline marker PHASE13F_PLACEHOLDER_OK on a line to suppress that line.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

docs=(
  "crates/x3-relayer/PHASE_13F_MAINNET_LAUNCH_RUNBOOK.md"
  "crates/x3-relayer/MAINNET_INCIDENT_RESPONSE.md"
  "crates/x3-relayer/RPC_FAILOVER_PROCEDURES.md"
  "crates/x3-relayer/VALIDATOR_OPERATIONS.md"
  "crates/x3-relayer/MAINNET_PERFORMANCE_BASELINE.md"
  "crates/x3-relayer/GPU_VALIDATOR_TROUBLESHOOTING.md"
  "crates/x3-relayer/PHASE_13F_MASTER_INDEX.md"
)

for doc in "${docs[@]}"; do
  if [[ ! -f "$doc" ]]; then
    echo "ERROR: expected doc missing: $doc" >&2
    exit 2
  fi
done

# Keep this strict for launch-readiness gating.
patterns=(
  'YOUR_[A-Z0-9_]+'
  '0xYOUR_[A-Z0-9_]+'
  '\[Name\]'
  '\[email\]'
  '\[phone\]'
  '\[[xXyYzZ]\]'
)

tmp_out="$(mktemp)"
trap 'rm -f "$tmp_out"' EXIT

rg_args=(--line-number --with-filename --color never)
for p in "${patterns[@]}"; do
  rg_args+=(-e "$p")
done

if ! rg "${rg_args[@]}" "${docs[@]}" > "$tmp_out" 2>/dev/null; then
  echo "PASS: no unresolved Phase 13f placeholders found"
  exit 0
fi

filtered="$(grep -v 'PHASE13F_PLACEHOLDER_OK' "$tmp_out" || true)"

if [[ -z "$filtered" ]]; then
  echo "PASS: placeholder hits are explicitly suppressed"
  exit 0
fi

echo "FAIL: unresolved Phase 13f placeholders detected"
echo
echo "$filtered"
echo
echo "Fix by replacing placeholder values, or add PHASE13F_PLACEHOLDER_OK on intentionally illustrative lines."
exit 1
