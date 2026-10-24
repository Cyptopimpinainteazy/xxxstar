#!/usr/bin/env bash
# Runs a PR-Agent command against a GitHub PR, a local diff, or stdin.
#
#   review.sh [--local] [--dry-run] <pr-number|pr-url> [command] [args...]
#   review.sh [--local] [--dry-run] --diff <file> [command] [args...]
#   git diff | review.sh [--local] [--dry-run] --stdin [command] [args...]
#
# The default model comes from .pr_agent.toml (DeepSeek). --local swaps in the
# Ollama model served by the two local GPUs, which needs no API key. --via-router
# routes through services/x3-ai-router so spend is recorded in /v1/usage.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_DIR="${PR_AGENT_VENV:-$HOME/.venvs/pr-agent}"
ENV_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/pr-agent/env"
# qwen2.5-coder:7b fails PR-Agent's review schema on this box; qwen3:8b returns
# valid output. See README "Which model" for the measurement.
LOCAL_MODEL="${PR_AGENT_LOCAL_MODEL:-ollama/qwen3:8b}"
ROUTER_BASE="${X3_ROUTER_BASE:-http://127.0.0.1:11435/v1}"
ROUTER_MODEL="${PR_AGENT_ROUTER_MODEL:-x3-deep}"

usage() {
  cat <<'EOF'
Usage:
  review.sh [--local] [--dry-run] <pr-number|pr-url> [command] [args...]
  review.sh [--local] [--dry-run] --diff <file> [command] [args...]
  git diff | review.sh [--local] [--dry-run] --stdin [command] [args...]

Flags:
  --local        run on the local GPUs through Ollama instead of DeepSeek/cloud
  --via-router   route through services/x3-ai-router so spend lands in /v1/usage
  --dry-run      build the review but do not publish comments to the PR
  --model M      override the model id for this run
  -h, --help     this text

Commands: review (default), describe, improve, ask, update_changelog, ...

Examples:
  ./review.sh 672
  ./review.sh 672 review --pr_reviewer.extra_instructions="focus on pallets/"
  ./review.sh --local 672
  ./review.sh --via-router --model x3-code 672 improve
  git diff | ./review.sh --local --stdin review
EOF
}

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

[ -x "$VENV_DIR/bin/pr-agent" ] || die "pr-agent is not installed in $VENV_DIR; run services/pr-agent/install.sh"

local_model=0
via_router=0
dry_run=0
model_override=""
target_kind=""
target=""

while [ $# -gt 0 ]; do
  case "$1" in
    --local) local_model=1; shift ;;
    --via-router) via_router=1; shift ;;
    --dry-run) dry_run=1; shift ;;
    --model) [ $# -ge 2 ] || die "--model needs a value"; model_override="$2"; shift 2 ;;
    --model=*) model_override="${1#*=}"; shift ;;
    --diff) [ $# -ge 2 ] || die "--diff needs a file"; target_kind="diff"; target="$2"; shift 2 ;;
    --stdin) target_kind="stdin"; shift ;;
    -h|--help) usage; exit 0 ;;
    --*) die "unknown flag $1" ;;
    *) break ;;
  esac
done

if [ -z "$target_kind" ]; then
  [ $# -gt 0 ] || { usage; exit 1; }
  target_kind="pr"
  target="$1"
  case "$target" in
    *://*) : ;;
    *[!0-9]*) die "'$target' is neither a PR number nor a PR URL" ;;
    *)
      repo_slug="$(git -C "$ROOT_DIR" remote get-url origin 2>/dev/null \
        | sed -E 's#^.*github\.com[:/]##; s#\.git$##')"
      [ -n "$repo_slug" ] || die "could not derive the repo slug from origin; pass the full PR URL"
      target="https://github.com/$repo_slug/pull/$target"
      ;;
  esac
  shift
fi

command_name="${1:-review}"
[ $# -gt 0 ] && shift

if [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

args=()
overrides=()
case "$target_kind" in
  pr)
    : "${GITHUB_TOKEN:=$(gh auth token 2>/dev/null || true)}"
    [ -n "${GITHUB_TOKEN:-}" ] || die "no GitHub token: export GITHUB_TOKEN or run 'gh auth login'"
    export GITHUB_TOKEN
    # PR-Agent reads settings, not $GITHUB_TOKEN; dynaconf maps this env var to github.user_token.
    export GITHUB__USER_TOKEN="$GITHUB_TOKEN"
    args+=("--pr_url=$target")
    ;;
  diff) args+=("--diff-file=$target") ;;
  stdin) args+=("--stdin") ;;
esac

# dynaconf's env loader is off, so pass the repo config explicitly: this also
# applies it in plain-diff mode, where no git provider exists to read it from.
if [ -f "$ROOT_DIR/.pr_agent.toml" ] && [ "${PR_AGENT_NO_EXTRA_CONFIG:-0}" != 1 ]; then
  args+=("--extra_config_url=$ROOT_DIR/.pr_agent.toml")
fi

if [ "$local_model" = 1 ] && [ "$via_router" = 1 ]; then
  die "--local and --via-router are mutually exclusive; the router already reaches Ollama"
fi

if [ "$local_model" = 1 ]; then
  overrides+=("--config.model=$LOCAL_MODEL")
  # PR-Agent's default ai_timeout is 120 s. An 8B on a reviewed diff regularly
  # needs more: PR #672 timed out at exactly 120 s before the retry answered, so
  # the first attempt was wasted. The cloud path keeps the default.
  overrides+=("--config.ai_timeout=${PR_AGENT_LOCAL_TIMEOUT:-600}")
elif [ "$via_router" = 1 ]; then
  # litellm treats an `openai/`-prefixed id as OpenAI-wire, so the router's
  # OpenAI-compatible surface is the transport and its policy name is the model.
  export OPENAI_API_BASE="$ROUTER_BASE"
  export OPENAI_API_KEY="${X3_ROUTER_TOKEN:-local-router}"
  overrides+=("--config.model=openai/${model_override:-$ROUTER_MODEL}")
elif [ -n "$model_override" ]; then
  overrides+=("--config.model=$model_override")
fi

if [ "$dry_run" = 1 ]; then
  overrides+=("--config.publish_output=false")
fi

if [ "$via_router" = 1 ]; then
  curl -sf -m 5 -o /dev/null "${ROUTER_BASE%/v1}/health" \
    || die "x3-ai-router is not answering at $ROUTER_BASE; start it with: systemctl --user start x3-ai-router"
fi

# pr-agent only accepts settings overrides after the command name.
exec "$VENV_DIR/bin/pr-agent" "${args[@]}" "$command_name" "${overrides[@]}" "$@"
