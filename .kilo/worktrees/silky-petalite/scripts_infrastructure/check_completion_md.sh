#!/usr/bin/env bash
set -euo pipefail

FILE="X3_COMPLETION.md"

if ! [ -f "$FILE" ]; then
  echo "ERROR: $FILE missing"
  exit 2
fi

# Detect null bytes (binary corruption)
if grep -qP '\x00' "$FILE"; then
  echo "ERROR: $FILE appears corrupted (contains null bytes)."
  echo "Attempting to restore from the last commit where it was text..."

  # Find most recent commit where file has no null bytes
  for rev in $(git rev-list --max-count=20 HEAD -- "$FILE"); do
    git show "$rev:$FILE" | grep -qP '\x00' && continue
    git show "$rev:$FILE" > "$FILE"
    echo "Restored $FILE from $rev"
    exit 0
  done

  echo "Failed to find a non-binary version in the last 20 commits. Manual recovery required."
  exit 1
fi

echo "OK: $FILE is text and contains no null bytes."
