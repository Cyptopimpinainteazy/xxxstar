#!/bin/bash
# Apply patch instructions from Ralph structured output.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/ralph-last.out"
PATCH_FILE="$SCRIPT_DIR/ralph.patch"

if [ ! -f "$OUTPUT_FILE" ]; then
  echo "[ralph-apply] output file missing: $OUTPUT_FILE" >&2
  exit 1
fi

# Extract patch from JSON
PATCH_CONTENT=$(jq -r '.patch // empty' "$OUTPUT_FILE" 2>/dev/null || echo "")
if [ -n "$PATCH_CONTENT" ] && [ "$PATCH_CONTENT" != "null" ]; then
  echo "$PATCH_CONTENT" > "$PATCH_FILE"
else
  # Fallback to old format
  awk 'BEGIN {p=0} /^PATCH:/ {p=1; next} /^ENDPATCH/ {p=0; exit} p {print}' "$OUTPUT_FILE" > "$PATCH_FILE"
fi

if [ ! -s "$PATCH_FILE" ]; then
  echo "[ralph-apply] no patch found in output. Nothing to apply."
  exit 0
fi

# Ensure patch seems safe
if ! grep -q '^\+\+\+\|^---\|^@@' "$PATCH_FILE"; then
  echo "[ralph-apply] patch file does not look like a diff block. Aborting." >&2
  exit 1
fi

echo "[ralph-apply] patch extracted to $PATCH_FILE"

git apply --check "$PATCH_FILE"

# Apply patch
git apply "$PATCH_FILE"

echo "[ralph-apply] Applied patch. Running validation checks..."

pushd "$SCRIPT_DIR/../.." >/dev/null

# Run test commands from structured output if available
TEST_COMMANDS=$(jq -r '.test_commands[] // empty' "$OUTPUT_FILE" 2>/dev/null | tr '\n' '\0' | xargs -0 -n1 echo || echo "")
if [ -n "$TEST_COMMANDS" ]; then
  echo "[ralph-apply] Running test commands from structured output..."
  while IFS= read -r cmd; do
    if [ -n "$cmd" ]; then
      echo "[ralph-apply] Running: $cmd"
      eval "$cmd" || {
        echo "[ralph-apply] Test command failed: $cmd" >&2
        exit 1
      }
    fi
  done <<< "$TEST_COMMANDS"
else
  # Fallback to default tests
  echo "[ralph-apply] Running default tests..."
  cargo test --workspace
fi

popd >/dev/null

# Extract task ID and summary for commit message
TASK_ID=$(jq -r '.task_id // empty' "$OUTPUT_FILE" 2>/dev/null || echo "")
SUMMARY=$(jq -r '.summary // empty' "$OUTPUT_FILE" 2>/dev/null || echo "")
if [ -n "$TASK_ID" ] && [ -n "$SUMMARY" ]; then
  COMMIT_MSG="feat: $TASK_ID - $SUMMARY"
else
  COMMIT_MSG="feat: ralph auto-commit"
fi

# Commit the changes
git add -A
git commit -m "$COMMIT_MSG"

echo "[ralph-apply] patch application complete, tests run, committed as: $COMMIT_MSG"
