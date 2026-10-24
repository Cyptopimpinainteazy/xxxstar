#!/usr/bin/env bash
# Installs PR-Agent into an isolated venv and prepares the secrets file.
# Idempotent: re-running installs the pinned version and leaves an existing env
# file alone. The pin matters: dry_run.py reads pr-agent internals.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENV_DIR="${PR_AGENT_VENV:-$HOME/.venvs/pr-agent}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/pr-agent"
ENV_FILE="$CONFIG_DIR/env"
PYTHON_BIN="${PYTHON:-/usr/bin/python3}"
# dry_run.py depends on this release's internals; bump both together.
PR_AGENT_VERSION="0.48.0"

log() { printf '==> %s\n' "$*"; }

is_py312() { "$1" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 12) else 1)' 2>/dev/null; }

if ! is_py312 "$PYTHON_BIN"; then
  echo "error: pr-agent requires Python >= 3.12, $PYTHON_BIN reports $("$PYTHON_BIN" -V 2>&1)" >&2
  exit 1
fi

# A venv left by an older interpreter would pass the check above and still run
# pr-agent on that older Python; rebuild it rather than reuse it.
if [ -x "$VENV_DIR/bin/python" ] && ! is_py312 "$VENV_DIR/bin/python"; then
  log "venv at $VENV_DIR is $("$VENV_DIR/bin/python" -V 2>&1); recreating"
  rm -rf -- "$VENV_DIR"
fi

if [ ! -x "$VENV_DIR/bin/python" ]; then
  log "creating venv at $VENV_DIR"
  "$PYTHON_BIN" -m venv "$VENV_DIR"
fi

log "installing pr-agent[github] into $VENV_DIR"
"$VENV_DIR/bin/python" -m pip install --quiet --upgrade pip
"$VENV_DIR/bin/python" -m pip install --quiet "pr-agent[github]==$PR_AGENT_VERSION"

mkdir -p "$CONFIG_DIR"
if [ -f "$ENV_FILE" ]; then
  log "keeping existing secrets file $ENV_FILE"
else
  log "writing secrets template $ENV_FILE"
  cat > "$ENV_FILE" <<'TEMPLATE'
# Sourced by services/pr-agent/review.sh. Never commit this file.
# DeepSeek keys: https://platform.deepseek.com/api_keys
DEEPSEEK_API_KEY=
# Optional. Left empty, review.sh falls back to `gh auth token`.
GITHUB_TOKEN=
TEMPLATE
  chmod 600 "$ENV_FILE"
fi

log "pr-agent $("$VENV_DIR/bin/pr-agent" --version 2>/dev/null | tail -1) installed"
log "repo config: $ROOT_DIR/.pr_agent.toml"
log "next: put your key in $ENV_FILE, then run services/pr-agent/check.sh"

if ! grep -qE '^DEEPSEEK_API_KEY=.+' "$ENV_FILE"; then
  echo
  echo "DEEPSEEK_API_KEY is empty, so cloud reviews fail until it is set."
  echo "Local GPU reviews work now:  services/pr-agent/review.sh --local <pr>"
fi
