#!/bin/bash
# Build-on-change: Auto-build when source files change
file_path=$(jq -r '.tool_input.file_path // empty')
if [[ "$file_path" =~ \.(ts|tsx|js|jsx)$ ]]; then
  npm run build 2>/dev/null || true
fi
