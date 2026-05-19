#!/usr/bin/env bash
set -euo pipefail

# Collect basic Ollama + Copilot Gateway diagnostics
# Usage: ./scripts/ollama_diag_collect.sh [BASE_URL] [OUT_DIR]

BASE_URL=${1:-http://127.0.0.1:11434}
OUT_DIR=${2:-./diagnostics}
mkdir -p "$OUT_DIR"
TS=$(date -u +"%Y%m%dT%H%M%SZ")
OUT="$OUT_DIR/ollama_diag_$TS.txt"

echo "Ollama diagnostics - $TS" > "$OUT"

echo "--- Models (GET /v1/models) ---" >> "$OUT"
curl -sS "$BASE_URL/v1/models" >> "$OUT" || echo "ERROR: curl /v1/models failed" >> "$OUT"
echo >> "$OUT"

echo "--- Chat smoke test (POST /v1/chat/completions) ---" >> "$OUT"
curl -sS -X POST "$BASE_URL/v1/chat/completions" -H "Content-Type: application/json" -d '{"model":"codellama:7b","messages":[{"role":"user","content":"diagnostic test: reply with hello"}]}' >> "$OUT" || echo "ERROR: chat completion failed" >> "$OUT"
echo >> "$OUT"

echo "--- Workspace settings (.vscode/settings.json) ---" >> "$OUT"
if [ -f .vscode/settings.json ]; then
  sed -n '1,500p' .vscode/settings.json >> "$OUT"
else
  echo "No .vscode/settings.json found in workspace root" >> "$OUT"
fi
echo >> "$OUT"

echo "--- VS Code extensions (code --list-extensions) ---" >> "$OUT"
if command -v code >/dev/null 2>&1; then
  code --list-extensions --show-versions >> "$OUT" 2>/dev/null || echo "ERROR: 'code --list-extensions' failed" >> "$OUT"
else
  echo "'code' CLI not found; run: code --list-extensions --show-versions" >> "$OUT"
fi
echo >> "$OUT"

echo "--- Extension logs template ---" >> "$OUT"
cat >> "$OUT" <<'TEMPLATE'
Please paste the contents of the VS Code Output panel for the following channels:

- GitHub Copilot LLM Gateway
- GitHub Copilot

Steps to collect logs:
1. In VS Code: View → Output
2. Select 'GitHub Copilot LLM Gateway' from the dropdown and copy recent messages
3. Paste logs below this line and keep any timestamps/errors

--- BEGIN PASTE ---

<PASTE LOGS HERE>

--- END PASTE ---

Warning: Do NOT paste private keys or secrets. Remove any API keys before sharing.
TEMPLATE

echo "Diagnostics collected to: $OUT"
echo "Open $OUT to view contents."
