#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEMO_DIR="$REPO_ROOT/third_party/threejs-avatar-demo"
HUNYUAN_SCRIPT="$REPO_ROOT/scripts/run-hunyuan3d-ollama.sh"

API_PORT="${API_PORT:-8082}"
DEMO_PORT="${DEMO_PORT:-8090}"

if [[ ! -d "$DEMO_DIR" ]]; then
  echo "Demo not found at $DEMO_DIR"
  exit 1
fi

if [[ ! -x "$HUNYUAN_SCRIPT" ]]; then
  echo "Missing $HUNYUAN_SCRIPT"
  exit 1
fi

cleanup() {
  if [[ -n "${API_PID:-}" ]] && kill -0 "$API_PID" 2>/dev/null; then
    kill "$API_PID" || true
  fi
  if [[ -n "${HTTP_PID:-}" ]] && kill -0 "$HTTP_PID" 2>/dev/null; then
    kill "$HTTP_PID" || true
  fi
}
trap cleanup EXIT INT TERM

"$HUNYUAN_SCRIPT" api &
API_PID=$!

cd "$DEMO_DIR"
python3 -m http.server "$DEMO_PORT" &
HTTP_PID=$!

printf "\nOllama demo ready:\n  http://127.0.0.1:%s/ollama.html\n\n" "$DEMO_PORT"

wait "$API_PID"
