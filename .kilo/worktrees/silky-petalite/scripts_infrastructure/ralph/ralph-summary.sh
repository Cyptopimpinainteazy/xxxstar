#!/bin/bash
# Print summary of last Ralph output and next task hints.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/ralph-last.out"
PRD_FILE="$SCRIPT_DIR/prd.json"

echo "=== Ralph Summary ==="
echo ""

if [ -f "$OUTPUT_FILE" ]; then
  echo "Last Output Summary:"
  SUMMARY=$(jq -r '.summary // empty' "$OUTPUT_FILE" 2>/dev/null || grep "SUMMARY:" "$OUTPUT_FILE" | head -1 | sed 's/SUMMARY: //')
  if [ -n "$SUMMARY" ] && [ "$SUMMARY" != "null" ]; then
    echo "- $SUMMARY"
  fi

  STATUS=$(jq -r '.status // empty' "$OUTPUT_FILE" 2>/dev/null || grep "STATUS:" "$OUTPUT_FILE" | head -1 | sed 's/STATUS: //')
  if [ -n "$STATUS" ] && [ "$STATUS" != "null" ]; then
    echo "- Status: $STATUS"
  fi

  ERRORS=$(jq -r '.errors // empty' "$OUTPUT_FILE" 2>/dev/null || grep "ERRORS:" "$OUTPUT_FILE" | head -1 | sed 's/ERRORS: //')
  if [ -n "$ERRORS" ] && [ "$ERRORS" != "none" ] && [ "$ERRORS" != "null" ]; then
    echo "- Errors: $ERRORS"
  fi
  echo ""
else
  echo "No previous output found."
  echo ""
fi

if [ -f "$PRD_FILE" ]; then
  echo "Next Task:"
  # Find first story with passes: false
  NEXT_STORY=$(jq -r '.userStories[] | select(.passes == false) | .id + " - " + .title' "$PRD_FILE" | head -1)
  if [ -n "$NEXT_STORY" ]; then
    echo "- $NEXT_STORY"
  else
    echo "- All stories completed!"
  fi
  echo ""
fi

echo "Available Commands:"
echo "- ./ralph.sh: Run next iteration"
echo "- ./ralph-auto.sh: Auto-run with Ollama"
echo "- ./ralph-auto.sh --strict: Strict mode"
echo "- ./ralph-apply.sh: Apply last patch"
echo "- ./ralph-check-coverage.sh: Run coverage checks"
echo "- ./ralph-repair.sh: Fix common issues"
echo ""