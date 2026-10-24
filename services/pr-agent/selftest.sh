#!/usr/bin/env bash
# Offline self-test for the PR-Agent wiring: no network, no LLM call.
# Verifies the repo config parses, the scripts are syntactically valid, and
# review.sh builds the argv PR-Agent expects. Used by the local-ci gate.
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONFIG="$ROOT_DIR/.pr_agent.toml"
failures=0

pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# ── config ────────────────────────────────────────────────────────────────
if [ ! -f "$CONFIG" ]; then
  fail ".pr_agent.toml exists"
else
  if python3 - "$CONFIG" <<'PY'
import sys, tomllib
path = sys.argv[1]
with open(path, "rb") as handle:
    cfg = tomllib.load(handle)
required_sections = ("config", "ollama", "pr_reviewer", "pr_code_suggestions", "pr_description")
missing = [name for name in required_sections if name not in cfg]
if missing:
    sys.exit(f"missing sections: {', '.join(missing)}")
model = cfg["config"].get("model", "")
if not model:
    sys.exit("config.model is empty")
if not any(name in model for name in ("deepseek", "ollama", "x3-")):
    sys.exit(f"config.model {model!r} names no configured provider")
if not cfg["pr_reviewer"].get("extra_instructions", "").strip():
    sys.exit("pr_reviewer.extra_instructions is empty; the repo invariants would not reach the model")
PY
  then pass ".pr_agent.toml parses with the required sections and a routed model"
  else fail ".pr_agent.toml parses with the required sections and a routed model"
  fi
fi

# ── syntax ────────────────────────────────────────────────────────────────
for script in install.sh review.sh check.sh selftest.sh usage.sh; do
  if bash -n "$ROOT_DIR/services/pr-agent/$script" 2>/dev/null; then
    pass "$script is valid bash"
  else
    fail "$script is valid bash"
  fi
done

# ── review.sh argv contract, against a recording shim ─────────────────────
fake_venv="$tmp/venv"
mkdir -p "$fake_venv/bin"
argv_log="$tmp/argv"
cat > "$fake_venv/bin/pr-agent" <<'SHIM'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$PR_AGENT_ARGV_LOG"
SHIM
chmod +x "$fake_venv/bin/pr-agent"

run_wrapper() {
  : > "$argv_log"
  PR_AGENT_VENV="$fake_venv" PR_AGENT_ARGV_LOG="$argv_log" \
    GITHUB_TOKEN="test-token" X3_ROUTER_BASE="http://127.0.0.1:1/v1" \
    bash "$ROOT_DIR/services/pr-agent/review.sh" "$@" >"$tmp/out" 2>&1
  return $?
}

expect_argv() {
  local label="$1"; shift
  local expected="$1"; shift
  if run_wrapper "$@"; then :; fi
  local actual
  actual="$(tr '\n' ' ' < "$argv_log" | sed 's/ $//')"
  if [ "$actual" = "$expected" ]; then
    pass "$label"
  else
    fail "$label"
    printf '        expected: %s\n        actual:   %s\n' "$expected" "$actual"
  fi
}

expect_dies() {
  local label="$1"; shift
  if run_wrapper "$@" >/dev/null 2>&1; then
    fail "$label (wrapper exited 0)"
  elif grep -q 'error:' "$tmp/out"; then
    pass "$label"
  else
    fail "$label (no diagnostic on stderr)"
  fi
}

repo_slug="$(git -C "$ROOT_DIR" remote get-url origin 2>/dev/null | sed -E 's#^.*github\.com[:/]##; s#\.git$##')"
[ -n "$repo_slug" ] || repo_slug="Cyptopimpinainteazy/xxxstar"

expect_argv "bare PR number expands to a URL and uses the repo config" \
  "--pr_url=https://github.com/$repo_slug/pull/672 --extra_config_url=$CONFIG review" \
  672
expect_argv "explicit PR URL passes through" \
  "--pr_url=https://github.com/$repo_slug/pull/7 --extra_config_url=$CONFIG review" \
  "https://github.com/$repo_slug/pull/7"
expect_argv "--local selects the GPU model and a longer timeout" \
  "--diff-file=/tmp/x.diff --extra_config_url=$CONFIG review --config.model=ollama/qwen3:8b --config.ai_timeout=600" \
  --local --diff /tmp/x.diff
expect_argv "--dry-run suppresses publishing" \
  "--pr_url=https://github.com/$repo_slug/pull/672 --extra_config_url=$CONFIG describe --config.publish_output=false" \
  --dry-run 672 describe
expect_argv "overrides land after the command name" \
  "--pr_url=https://github.com/$repo_slug/pull/672 --extra_config_url=$CONFIG improve --config.publish_output=false --pr_code_suggestions.num_code_suggestions_per_chunk=5" \
  --dry-run 672 improve --pr_code_suggestions.num_code_suggestions_per_chunk=5
expect_argv "--model overrides the configured model" \
  "--diff-file=/tmp/x.diff --extra_config_url=$CONFIG review --config.model=deepseek/deepseek-flash" \
  --model deepseek/deepseek-flash --diff /tmp/x.diff
expect_dies "an unreachable router fails loudly" --via-router --diff /tmp/x.diff
expect_dies "--local and --via-router are rejected together" --local --via-router 672
expect_dies "a non-numeric target is rejected" not-a-pr

# ── usage.sh formatting, against a saved router snapshot ──────────────────
fixture="$ROOT_DIR/services/pr-agent/fixtures/usage.json"
if [ ! -r "$fixture" ]; then
  fail "usage fixture exists"
else
  expect_usage() {
    local label="$1" expected="$2"; shift 2
    local actual
    actual="$(PR_AGENT_USAGE_FILE="$fixture" bash "$ROOT_DIR/services/pr-agent/usage.sh" "$@" 2>&1)"
    if [ "$actual" = "$expected" ]; then
      pass "$label"
    else
      fail "$label"
      printf '        expected: %s\n        actual:   %s\n' "$expected" "$actual"
    fi
  }

  expect_usage "usage.sh totals a day and marks zero-cost providers free" \
"2026-10-24:
  deepseek_flash     2 req  \$0.0012
  ollama_qwen3       1 req  \$0.0000 (free)
  total: \$0.0012" 2026-10-24

  expect_usage "usage.sh filters to one provider" \
"2026-10-24:
  deepseek_flash     2 req  \$0.0012
  total: \$0.0012" 2026-10-24 deepseek_flash

  expect_usage "usage.sh says so when a day has no rows" \
    "no usage recorded for 2026-01-01" 2026-01-01
fi

# ── summary ───────────────────────────────────────────────────────────────
echo
if [ "$failures" -eq 0 ]; then
  echo "pr-agent self-test: PASS"
else
  echo "pr-agent self-test: FAIL ($failures check(s))"
fi
exit "$failures"
