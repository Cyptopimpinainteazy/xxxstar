#!/usr/bin/env bash
set -euo pipefail
# x3-update-proof-ledger.sh — append a proof entry to docs/X3_PROOF_LEDGER.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LEDGER="$REPO_ROOT/docs/X3_PROOF_LEDGER.md"
PROOF_LOG="$REPO_ROOT/.x3/proof/latest-proof.log"

# Ensure ledger exists
if [ ! -f "$LEDGER" ]; then
    mkdir -p "$(dirname "$LEDGER")"
    cat > "$LEDGER" << 'LEDGEREOF'
# X3 Proof Ledger

## Proof History

LEDGEREOF
fi

# Determine proof result
PROOF_RESULT="UNKNOWN"
if [ -f "$PROOF_LOG" ]; then
    if grep -q 'OVERALL: FAIL' "$PROOF_LOG" 2>/dev/null; then
        PROOF_RESULT="FAIL"
    elif grep -q 'OVERALL: PASS' "$PROOF_LOG" 2>/dev/null; then
        PROOF_RESULT="PASS"
    fi
fi

# Get changed files
CHANGED_FILES="UNKNOWN"
if git -C "$REPO_ROOT" rev-parse --git-dir &>/dev/null 2>&1; then
    CHANGED_FILES=$(git -C "$REPO_ROOT" diff --name-only HEAD 2>/dev/null | tr '\n' ', ' | sed 's/,$//')
    [ -z "$CHANGED_FILES" ] && CHANGED_FILES="(none)"
fi

# Get branch
BRANCH="UNKNOWN"
if git -C "$REPO_ROOT" rev-parse --git-dir &>/dev/null 2>&1; then
    BRANCH=$(git -C "$REPO_ROOT" branch --show-current 2>/dev/null || echo "detached")
fi

DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Prepend entry after header
TMP=$(mktemp)
cat "$LEDGER" | while IFS= read -r line; do
    echo "$line"
    if [[ "$line" == "## Proof History" ]]; then
        echo ""
        echo "## Proof Run - $DATE"
        echo ""
        echo "- Area: $AREA"
        echo "- Claim: $CLAIM"
        echo "- Branch: $BRANCH"
        echo "- Commands run: scripts/x3-proof-check.sh"
        echo "- Result: $PROOF_RESULT"
        echo "- Files changed: $CHANGED_FILES"
        echo "- Evidence log: .x3/proof/latest-proof.log"
        echo "- Remaining gaps: See docs/X3_COMPLETION_STATUS.md"
        echo "- Next best task: See docs/X3_NEXT_TASKS.md"
        echo ""
    fi
done > "$TMP"
mv "$TMP" "$LEDGER"

echo "Proof ledger updated: $LEDGER"
echo "Result: $PROOF_RESULT"