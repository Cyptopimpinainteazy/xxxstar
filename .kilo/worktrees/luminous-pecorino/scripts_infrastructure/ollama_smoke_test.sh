#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/ollama_smoke_test.sh [BASE_URL]
BASE_URL=${1:-http://127.0.0.1:11434}

echo "Listing models from ${BASE_URL}"
curl -sS "${BASE_URL}/v1/models" || true
echo

echo "Running chat completion smoke test against codellama:7b"
curl -sS -X POST "${BASE_URL}/v1/chat/completions" -H "Content-Type: application/json" -d @- <<'JSON'
{
  "model": "codellama:7b",
  "messages": [
    {"role": "user", "content": "Run a quick smoke test: reply with the text hello from codellama and then output a short JSON object {\"ok\":true} on a new line."}
  ],
  "max_tokens": 128
}
JSON

echo
