#!/usr/bin/env bash
# Reports whether PR-Agent can run: binary, GitHub access, local GPU model and
# the DeepSeek key. Uses the repository's verification vocabulary.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_DIR="${PR_AGENT_VENV:-$HOME/.venvs/pr-agent}"
ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/pr-agent/env"
OLLAMA_BASE="${OLLAMA_BASE:-http://127.0.0.1:11434}"
LOCAL_MODEL="${PR_AGENT_LOCAL_MODEL:-ollama/qwen3:8b}"
DEEPSEEK_BASE="${DEEPSEEK_BASE:-https://api.deepseek.com}"
failures=0
blocked=0

report() { printf '%-22s %s\n' "$1" "$2"; }
note() { printf '%-22s %s\n' "  " "$1"; }

if [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

if [ -x "$VENV_DIR/bin/pr-agent" ]; then
  report "pr-agent" "PASS $("$VENV_DIR/bin/pr-agent" --version 2>/dev/null | tail -1)"
else
  report "pr-agent" "FAIL not installed in $VENV_DIR"
  failures=$((failures + 1))
fi

if [ -f "$ROOT_DIR/.pr_agent.toml" ]; then
  report "repo config" "PASS $ROOT_DIR/.pr_agent.toml"
else
  report "repo config" "FAIL .pr_agent.toml missing"
  failures=$((failures + 1))
fi

token="${GITHUB_TOKEN:-$(gh auth token 2>/dev/null || true)}"
if [ -n "$token" ] && GH_TOKEN="$token" gh api repos/Cyptopimpinainteazy/xxxstar --jq .full_name >/dev/null 2>&1; then
  report "github token" "PASS repo reachable"
else
  report "github token" "FAIL export GITHUB_TOKEN or run 'gh auth login'"
  failures=$((failures + 1))
fi

if curl -sf -m 5 "$OLLAMA_BASE/api/tags" >/dev/null 2>&1; then
  local_name="${LOCAL_MODEL#ollama/}"
  if curl -sf -m 5 "$OLLAMA_BASE/api/tags" | grep -q "\"${local_name%%:*}"; then
    report "local gpu model" "PASS $LOCAL_MODEL served at $OLLAMA_BASE"
  else
    report "local gpu model" "FAIL $local_name not pulled"
    note "run: ollama pull $local_name"
    failures=$((failures + 1))
  fi
else
  report "local gpu model" "BLOCKED ollama not reachable at $OLLAMA_BASE"
  note "run: systemctl --user start ollama-worker-b"
  blocked=$((blocked + 1))
fi

if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
  report "deepseek" "BLOCKED DEEPSEEK_API_KEY is empty in $ENV_FILE"
  note "set it: sed -i 's|^DEEPSEEK_API_KEY=.*|DEEPSEEK_API_KEY=sk-...|' $ENV_FILE"
  note "ids the router config declares: deepseek-flash, deepseek-v4-pro"
  note "local reviews still work: services/pr-agent/review.sh --local <pr>"
  blocked=$((blocked + 1))
else
  models="$(curl -sf -m 20 -H "Authorization: Bearer $DEEPSEEK_API_KEY" "$DEEPSEEK_BASE/models")"
  if [ -n "$models" ]; then
    report "deepseek" "PASS key accepted"
    note "available: $(printf '%s' "$models" | python3 -c 'import json,sys; print(", ".join(m["id"] for m in json.load(sys.stdin).get("data", [])))' 2>/dev/null)"
  else
    report "deepseek" "FAIL key rejected or API unreachable"
    failures=$((failures + 1))
  fi
fi

echo
if [ "$failures" -gt 0 ]; then
  echo "$failures check(s) failed"
elif [ "$blocked" -gt 0 ]; then
  echo "$blocked dependency/dependencies blocked; reviews run only on the paths still PASS"
else
  echo "ready: cloud reviews via DeepSeek, local reviews via the GPUs"
fi
exit "$failures"
