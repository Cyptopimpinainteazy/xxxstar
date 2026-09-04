#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REGISTRY_FILE="$ROOT_DIR/proof/claims/registry.yml"
RECEIPTS_DIR="$ROOT_DIR/proof/receipts/claims"

if ! command -v jq >/dev/null 2>&1; then
  echo "ERROR: jq is required." >&2
  exit 2
fi

if [[ ! -f "$REGISTRY_FILE" ]]; then
  echo "ERROR: Missing registry file: $REGISTRY_FILE" >&2
  exit 2
fi

declare -A actual_status
declare -A expected_status
declare -A seen_claim

while IFS='|' read -r claim status; do
  [[ -z "${claim:-}" ]] && continue
  actual_status["$claim"]="$status"
  expected_status["$claim"]="UNVERIFIED"
done < <(
  awk '
    /^claims:/ { in_claims=1; next }
    in_claims && /^[^[:space:]]/ { in_claims=0 }
    in_claims && /^  x3\.[A-Za-z0-9_.-]+:/ {
      claim=$1
      sub(/:$/, "", claim)
    }
    in_claims && /^    status:/ {
      status=$2
      if (claim != "") print claim "|" status
    }
  ' "$REGISTRY_FILE"
)

while IFS= read -r -d '' file; do
  claim_id="$(jq -r '.result.claim_id // .claim_id // empty' "$file")"
  [[ -z "${claim_id:-}" ]] && continue

  if ! jq -e '
    has("repo_commit_hash") and
    has("command_run") and
    has("artifact_hash") and
    has("policy_hash") and
    has("relevant_files") and
    has("timestamp") and
    has("result") and
    has("limitations") and
    has("binding_hash")
  ' "$file" >/dev/null; then
    continue
  fi

  result_status="$(jq -r '.result.status // .status // ""' "$file" | tr '[:upper:]' '[:lower:]')"
  seen_claim["$claim_id"]=1
  if [[ "$result_status" == "verified" || "$result_status" == "pass" || "$result_status" == "passed" ]]; then
    expected_status["$claim_id"]="VERIFIED"
  elif [[ "$result_status" == "partial" ]]; then
    expected_status["$claim_id"]="PARTIAL"
  else
    expected_status["$claim_id"]="UNVERIFIED"
  fi
done < <(find "$RECEIPTS_DIR" -type f -name '*.json' -print0 | sort -z)

echo "Checking claim registry status consistency..."
mismatch_count=0

for claim in "${!actual_status[@]}"; do
  actual="${actual_status[$claim]}"
  expected="${expected_status[$claim]:-UNVERIFIED}"
  if [[ "$actual" != "$expected" ]]; then
    mismatch_count=$((mismatch_count + 1))
    echo "MISMATCH  $claim  actual=$actual  expected=$expected"
  fi
done

if (( mismatch_count > 0 )); then
  echo ""
  echo "Claim status consistency FAILED: $mismatch_count mismatches"
  echo "Update proof/claims/registry.yml statuses to match structured receipts."
  exit 1
fi

echo "Claim status consistency PASSED"
