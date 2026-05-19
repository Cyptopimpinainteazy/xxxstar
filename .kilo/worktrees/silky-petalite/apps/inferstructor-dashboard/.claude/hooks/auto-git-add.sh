#!/bin/bash
# Auto-git-add: Automatically stage modified files after editing
file_path=$(jq -r '.tool_input.file_path // empty')
if [[ -n "$file_path" ]] && git rev-parse --git-dir > /dev/null 2>&1; then
  git add "$file_path" 2>/dev/null || true
fi
