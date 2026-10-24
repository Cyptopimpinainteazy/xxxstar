#!/usr/bin/env bash
# Reports whether PR-Agent can run, per execution mode.
#
#   check.sh            # cloud: DeepSeek is the required provider
#   check.sh local      # local: the GPU model is required, DeepSeek is not
#   check.sh router     # router: x3-ai-router is required, providers are its job
#
# A provider that the selected mode does not use is reported as INFO and never
# fails the check: a rejected cloud key must not block a --local run, and a
# missing local model must not block a cloud review.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_DIR="${PR_AGENT_VENV:-$HOME/.venvs/pr-agent}"
ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/pr-agent/env"
OLLAMA_BASE="${OLLAMA_BASE:-http://127.0.0.1:11434}"
LOCAL_MODEL="${PR_AGENT_LOCAL_MODEL:-ollama/qwen3:8b}"
DEEPSEEK_BASE="${DEEPSEEK_BASE:-https://api.deepseek.com}"
ROUTER_BASE="${X3_ROUTER_BASE:-http://127.0.0.1:11435/v1}"
MODE="${1:-${PR_AGENT_MODE:-cloud}}"
failures=0

case "$MODE" in
  cloud|local|router) : ;;
  *) echo "usage: check.sh [cloud|local|router]" >&2; exit 2 ;;
esac

report() { printf '%-22s %s\n' "$1" "$2"; }
note() { printf '%-22s %s\n' "  " "$1"; }
pass() { report "$1" "PASS $2"; }
fail() { report "$1" "FAIL $2"; failures=$((failures + 1)); }
info() { report "$1" "INFO $2"; }

# The secrets file must not blank credentials the environment already carries.
load_env_file() {
  local file="$1" name
  [ -f "$file" ] || return 0
  local -A preset=()
  while read -r name; do
    [ -n "${!name:-}" ] && preset["$name"]="${!name}"
  done < <(sed -nE 's/^([A-Za-z_][A-Za-z0-9_]*)=.*/\1/p' "$file")
  set -a
  # shellcheck disable=SC1090
  . "$file"
  set +a
  for name in "${!preset[@]}"; do
    printf -v "$name" '%s' "${preset[$name]}"
    export "${name?}"
  done
}

router_curl() {
  if [ -n "${X3_ROUTER_TOKEN:-}" ]; then
    curl -H "Authorization: Bearer $X3_ROUTER_TOKEN" "$@"
  else
    curl "$@"
  fi
}

load_env_file "$ENV_FILE"

report "mode" "$MODE"

if [ -x "$VENV_DIR/bin/pr-agent" ]; then
  pass "pr-agent" "$("$VENV_DIR/bin/pr-agent" --version 2>/dev/null | tail -1)"
else
  fail "pr-agent" "not installed in $VENV_DIR"
  note "run: services/pr-agent/install.sh"
fi

if [ -f "$ROOT_DIR/.pr_agent.toml" ]; then
  pass "repo config" "$ROOT_DIR/.pr_agent.toml"
else
  fail "repo config" ".pr_agent.toml missing"
fi

# A PR review posts a comment, so it needs a token in every mode.
token="${GITHUB_TOKEN:-$(gh auth token 2>/dev/null || true)}"
if [ -n "$token" ] && GH_TOKEN="$token" gh api repos/Cyptopimpinainteazy/xxxstar --jq .full_name >/dev/null 2>&1; then
  pass "github token" "repo reachable"
else
  fail "github token" "export GITHUB_TOKEN or run 'gh auth login'"
fi

# ── local GPU model ───────────────────────────────────────────────────────
local_required=$([ "$MODE" = "local" ] && echo 1 || echo 0)
local_name="${LOCAL_MODEL#ollama/}"
if curl -sf -m 5 "$OLLAMA_BASE/api/tags" >/dev/null 2>&1; then
  if curl -sf -m 5 "$OLLAMA_BASE/api/tags" | grep -q "\"${local_name%%:*}"; then
    pass "local gpu model" "$LOCAL_MODEL served at $OLLAMA_BASE"
  elif [ "$local_required" = 1 ]; then
    fail "local gpu model" "$local_name not pulled"
    note "run: ollama pull $local_name"
  else
    info "local gpu model" "$local_name not pulled; only --local needs it"
  fi
elif [ "$local_required" = 1 ]; then
  fail "local gpu model" "ollama not reachable at $OLLAMA_BASE"
  note "run: systemctl --user start ollama-worker-b"
else
  info "local gpu model" "ollama not reachable at $OLLAMA_BASE; only --local needs it"
fi

# ── router ────────────────────────────────────────────────────────────────
router_required=$([ "$MODE" = "router" ] && echo 1 || echo 0)
router_code="$(router_curl -sS -m 5 -o /dev/null -w '%{http_code}' "${ROUTER_BASE%/v1}/health" 2>/dev/null || true)"
case "$router_code" in
  200) pass "x3-ai-router" "healthy at $ROUTER_BASE" ;;
  401|403)
    if [ "$router_required" = 1 ]; then
      fail "x3-ai-router" "HTTP $router_code; set X3_ROUTER_TOKEN"
    else
      info "x3-ai-router" "HTTP $router_code; set X3_ROUTER_TOKEN to use --via-router"
    fi
    ;;
  000|"")
    if [ "$router_required" = 1 ]; then
      fail "x3-ai-router" "not answering at $ROUTER_BASE"
      note "run: systemctl --user start x3-ai-router"
    else
      info "x3-ai-router" "not answering; only --via-router needs it"
    fi
    ;;
  *)
    if [ "$router_required" = 1 ]; then
      fail "x3-ai-router" "health returned HTTP $router_code"
    else
      info "x3-ai-router" "health returned HTTP $router_code"
    fi
    ;;
esac

# ── deepseek ──────────────────────────────────────────────────────────────
deepseek_required=$([ "$MODE" = "cloud" ] && echo 1 || echo 0)
if [ -z "${DEEPSEEK_API_KEY:-}" ]; then
  if [ "$deepseek_required" = 1 ]; then
    fail "deepseek" "DEEPSEEK_API_KEY is empty in $ENV_FILE"
    note "set it: sed -i 's|^DEEPSEEK_API_KEY=.*|DEEPSEEK_API_KEY=sk-...|' $ENV_FILE"
    note "ids the router config declares: deepseek-flash, deepseek-v4-pro"
    note "without it: services/pr-agent/review.sh --local <pr>"
  else
    info "deepseek" "no key; not used by --local or --via-router"
  fi
else
  models="$(curl -sf -m 20 -H "Authorization: Bearer $DEEPSEEK_API_KEY" "$DEEPSEEK_BASE/models")"
  if [ -n "$models" ]; then
    pass "deepseek" "key accepted"
    note "available: $(printf '%s' "$models" | python3 -c 'import json,sys; print(", ".join(m["id"] for m in json.load(sys.stdin).get("data", [])))' 2>/dev/null)"
  elif [ "$deepseek_required" = 1 ]; then
    fail "deepseek" "key rejected or API unreachable"
  else
    info "deepseek" "key rejected or API unreachable; not used in $MODE mode"
  fi
fi

echo
if [ "$failures" -gt 0 ]; then
  echo "$MODE mode: $failures required dependency/dependencies FAILED"
else
  echo "$MODE mode: ready"
fi
exit "$failures"
