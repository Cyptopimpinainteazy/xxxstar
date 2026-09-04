#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RECEIPTS_DIR="$ROOT_DIR/proof/receipts/claims"
PLAN_FILE="$ROOT_DIR/proof/reports/receipt_migration_plan.md"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required to inspect receipts." >&2
  exit 2
fi

mkdir -p "$(dirname "$PLAN_FILE")"

{
  echo "# Receipt Migration Plan"
  echo
  echo "Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo
  echo "Legacy receipts require full re-proof to produce cryptographically bound ProofForge receipts."
  echo "This script does not fabricate proof data. It generates a re-proof action plan only."
  echo
  echo "## Legacy Receipts"
  echo
  echo "| Claim ID | Legacy Verifier | Legacy Date | Re-proof Command |"
  echo "|---|---|---|---|"

  found_any=0
  while IFS= read -r -d '' file; do
    if jq -e 'has("claim_id") and has("status") and has("date") and has("verifier") and has("hash") and (has("repo_commit_hash") | not)' "$file" >/dev/null; then
      claim_id="$(jq -r '.claim_id' "$file")"
      verifier="$(jq -r '.verifier // "unknown"' "$file")"
      date_value="$(jq -r '.date // "unknown"' "$file")"
      cmd="cargo run -p proof-forge -- verify $claim_id --strict"
      echo "| $claim_id | $verifier | $date_value | $cmd |"
      found_any=1
    fi
  done < <(find "$RECEIPTS_DIR" -type f -name '*.json' -print0 | sort -z)

  if [[ "$found_any" -eq 0 ]]; then
    echo "| none | n/a | n/a | n/a |"
  fi

  echo
  echo "## Next Steps"
  echo
  echo "1. Run each re-proof command in a clean workspace state."
  echo "2. Ensure commands regenerate structured receipts under proof/receipts/claims."
  echo "3. Run scripts/proof/verify-receipts.sh and resolve all failures."
} > "$PLAN_FILE"

echo "Wrote migration plan: $PLAN_FILE"
