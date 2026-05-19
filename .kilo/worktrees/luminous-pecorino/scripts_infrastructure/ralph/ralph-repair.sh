#!/bin/bash
# Repair common issues after Ralph run: line endings, file paths, etc.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "[ralph-repair] Checking git status..."
if ! git diff --exit-code; then
  echo "[ralph-repair] Found uncommitted changes. Fixing line endings..."
  # Fix line endings
  find . -name "*.rs" -type f -exec sed -i 's/\r$//' {} \;
  find . -name "*.md" -type f -exec sed -i 's/\r$//' {} \;

  # Check again
  if git diff --exit-code; then
    echo "[ralph-repair] Fixed line endings. No changes remain."
  else
    echo "[ralph-repair] Still have changes after fixing line endings."
    git diff --name-only
  fi
else
  echo "[ralph-repair] No uncommitted changes found."
fi

echo "[ralph-repair] Repair complete."