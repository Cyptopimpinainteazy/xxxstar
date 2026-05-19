#!/usr/bin/env bash
set -euo pipefail

# Get script directory, handle both direct execution and sourcing
if [[ -n "${BASH_SOURCE[0]:-}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR="$(pwd)"
fi
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$ROOT_DIR"

echo "Checking Tauri capability scopes..."
if [[ -d "apps" ]]; then
  CAP_FILES="$(find apps -path '*/src-tauri/capabilities/*.json' 2>/dev/null || true)"
else
  CAP_FILES=""
fi
if [[ -n "${CAP_FILES}" ]]; then
  if echo "$CAP_FILES" | xargs grep -l '"shell:allow-(execute|spawn|stdin-write)"' 2>/dev/null || false; then
    echo "ERROR: Broad Tauri shell execution permissions detected in capability files."
    exit 1
  fi
fi

echo "Checking CSP for unsafe-eval in production config..."
TAURI_FILE="apps/x3-desktop/src-tauri/tauri.conf.json"
HTML_FILE="apps/x3-desktop/index.html"

if [[ -f "$TAURI_FILE" ]] && grep -q "unsafe-eval" "$TAURI_FILE"; then
  echo "ERROR: unsafe-eval is forbidden in desktop production CSP (tauri.conf.json)."
  exit 1
fi

if [[ -f "$HTML_FILE" ]] && grep -q "unsafe-eval" "$HTML_FILE"; then
  echo "ERROR: unsafe-eval is forbidden in desktop production CSP (index.html)."
  exit 1
fi

echo "Desktop security guardrails passed."
