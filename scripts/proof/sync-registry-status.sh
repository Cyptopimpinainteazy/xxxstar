#!/usr/bin/env bash
# sync-registry-status.sh
# Read all v2 receipts in proof/receipts/claims/ and sync claim statuses
# in proof/claims/registry.yml based on whether valid v2 receipts exist.
#
# Rules:
#   - verified/passed/proof_complete/green                  → VERIFIED
#   - partial                                               → PARTIAL
#   - failed/unverified/blocked/red/unknown                → UNVERIFIED
#   - legacy receipt only                                   → UNVERIFIED (legacy)
#   - no receipt                                            → UNVERIFIED (no receipt)
#
# Does NOT fabricate status. Only marks VERIFIED when a passing receipt is present.

set -euo pipefail

RECEIPTS_DIR="proof/receipts/claims"
REGISTRY="proof/claims/registry.yml"

if [[ ! -f "$REGISTRY" ]]; then
  echo "ERROR: registry not found at $REGISTRY" >&2
  exit 1
fi

if [[ ! -d "$RECEIPTS_DIR" ]]; then
  echo "ERROR: receipts dir not found at $RECEIPTS_DIR" >&2
  exit 1
fi

verified=0
partial=0
unverified_with_receipt=0
unverified_failed=0
unverified_missing=0

# Collect all claim IDs from the registry
claim_ids=()
while IFS= read -r line; do
  if [[ "$line" =~ ^[[:space:]]{2}(x3\.[a-zA-Z0-9_.]+): ]]; then
    claim_ids+=("${BASH_REMATCH[1]}")
  fi
done < "$REGISTRY"

echo "Syncing registry status for ${#claim_ids[@]} claims..."
echo ""

for claim_id in "${claim_ids[@]}"; do
  receipt_file="$RECEIPTS_DIR/${claim_id}.receipt.json"

  if [[ ! -f "$receipt_file" ]]; then
    # No receipt at all - ensure UNVERIFIED
    new_status="UNVERIFIED"
    reason="no receipt"
    unverified_missing=$((unverified_missing + 1))
  else
    # Check if it's a v2 receipt (has repo_commit_hash field)
    if ! python3 -c "import json,sys; d=json.load(open('$receipt_file')); sys.exit(0 if 'repo_commit_hash' in d else 1)" 2>/dev/null; then
      new_status="UNVERIFIED"
      reason="legacy receipt format"
      unverified_with_receipt=$((unverified_with_receipt + 1))
    else
      # v2 receipt - check result status
      result_status=$(python3 -c "
import json, sys
d = json.load(open('$receipt_file'))
r = d.get('result', {})
# result may be a ProofResult object or a simple string
if isinstance(r, dict):
    print(r.get('status', 'unknown'))
else:
    print(str(r))
" 2>/dev/null || echo "unknown")

      case "$result_status" in
        verified|passed|proof_complete|green)
          new_status="VERIFIED"
          reason="v2 receipt: $result_status"
          verified=$((verified + 1))
          ;;
        partial)
          new_status="PARTIAL"
          reason="v2 receipt status: $result_status"
          partial=$((partial + 1))
          ;;
        failed|unverified|blocked|red|unknown)
          new_status="UNVERIFIED"
          reason="v2 receipt status: $result_status"
          unverified_failed=$((unverified_failed + 1))
          ;;
        *)
          new_status="UNVERIFIED"
          reason="v2 receipt status: $result_status (unrecognized)"
          unverified_failed=$((unverified_failed + 1))
          ;;
      esac
    fi
  fi

  # Update the registry in-place using sed
  # Registry format:  <claim_id>:
  #                     ...
  #                     status: UNVERIFIED
  # We update the status line that comes after the claim_id header.
  # Use Python for reliable YAML line editing.
  python3 - "$REGISTRY" "$claim_id" "$new_status" <<'EOF'
import sys, re

registry_path = sys.argv[1]
claim_id = sys.argv[2]
new_status = sys.argv[3]

with open(registry_path, 'r') as f:
    lines = f.readlines()

in_claim = False
updated = False
out = []

for i, line in enumerate(lines):
    stripped = line.strip()
    # Detect entry for this claim
    if stripped == f"{claim_id}:":
        in_claim = True
        out.append(line)
        continue

    if in_claim:
        # Next top-level claim or end of file terminates the block
        if re.match(r'^  x3\.', line) and not line.startswith('    '):
            in_claim = False
        elif stripped.startswith('status:'):
            indent = len(line) - len(line.lstrip())
            line = ' ' * indent + f'status: {new_status}\n'
            updated = True
            in_claim = False  # stop after first status line

    out.append(line)

with open(registry_path, 'w') as f:
    f.writelines(out)

if not updated:
    sys.exit(1)
EOF

  # Map status symbol for display
  if [[ "$new_status" == "VERIFIED" || "$new_status" == "PARTIAL" ]]; then
    symbol="✓"
  else
    symbol="✗"
  fi
  printf "  %s  %-50s  %s\n" "$symbol" "$claim_id" "$reason"
done

echo ""
echo "Sync complete:"
echo "  VERIFIED   : $verified"
echo "  PARTIAL    : $partial"
echo "  UNVERIFIED : $((unverified_with_receipt + unverified_failed + unverified_missing))"
echo "    - with passing v2 receipt but failed : $unverified_failed"
echo "    - legacy receipt only                : $unverified_with_receipt"
echo "    - no receipt                         : $unverified_missing"
