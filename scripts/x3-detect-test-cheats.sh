#!/usr/bin/env bash
set -euo pipefail
# x3-detect-test-cheats.sh — detect test weakening/skipping/removal patterns

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Test-Cheat Detection — $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="

CHEATS_FOUND=0

# Check for ignored/skipped tests
echo "--- Searching for skipped/ignored tests ---"
for pattern in '#\[ignore\]' 'describe\.skip' 'it\.skip' 'test\.skip' '\.skip(' '@unstable' '@skip'; do
    FOUND=$(find "$REPO_ROOT" \
        -type f \
        \( -name '*.rs' -o -name '*.ts' -o -name '*.tsx' -o -name '*.js' -o -name '*.move' -o -name '*.cairo' -o -name '*.sol' -o -name '*.py' \) \
        -not -path '*/.git/*' \
        -not -path '*/target/*' \
        -not -path '*/node_modules/*' \
        -not -path '*/.venv/*' \
        -not -path '*/forge-std/*' \
        -not -path '*/vendor/*' \
        -exec grep -l -n "$pattern" {} \; 2>/dev/null | head -20)
    if [ -n "$FOUND" ]; then
        echo "$FOUND" | while IFS= read -r line; do
            echo "[WARN] Skipped/ignored test: $line"
            CHEATS_FOUND=$((CHEATS_FOUND + 1))
        done
    fi
done

# Git-aware checks
if git -C "$REPO_ROOT" rev-parse --git-dir &>/dev/null 2>&1; then
    echo ""
    echo "--- Checking git diff for suspicious test changes ---"
    
    # Check if tests were deleted without corresponding source changes
    TEST_DELETIONS=$(git -C "$REPO_ROOT" diff --name-only --diff-filter=D HEAD 2>/dev/null | grep -E '(test|spec|_test)\.(rs|ts|tsx|js|py|sol)$' || true)
    if [ -n "$TEST_DELETIONS" ]; then
        echo "[WARN] Test files deleted:"
        echo "$TEST_DELETIONS"
        CHEATS_FOUND=$((CHEATS_FOUND + $(echo "$TEST_DELETIONS" | wc -l)))
    fi
    
    # Check for assertion removals in unstaged changes
    ASSERT_DIFF=$(git -C "$REPO_ROOT" diff -- test/ tests/ integration-tests/ \
        -E '-(.*assert|.*expect|.*should|.*must|.*require)' 2>/dev/null | head -20 || true)
    if [ -n "$ASSERT_DIFF" ]; then
        echo "[WARN] Assertions removed from tests:"
        echo "$ASSERT_DIFF"
        CHEATS_FOUND=$((CHEATS_FOUND + 1))
    fi
else
    echo ""
    echo "--- Git not available — skipping diff-based checks ---"
fi

echo ""
echo "=== Test-Cheat Detection Summary ==="
echo "Suspicious patterns found: $CHEATS_FOUND"

if [ "$CHEATS_FOUND" -gt 0 ]; then
    echo "VERDICT: WARN — $CHEATS_FOUND suspicious test patterns detected."
    echo "Review each finding. Do not proceed without understanding."
    exit 0  # Warning, not hard fail by default — but blocks commit via pre-commit hook
else
    echo "VERDICT: PASS — No suspicious test patterns detected."
    exit 0
fi