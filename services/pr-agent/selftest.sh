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

# ── the secrets file must not blank injected credentials ──────────────────
# install.sh writes empty `DEEPSEEK_API_KEY=` / `GITHUB_TOKEN=` placeholders.
# A plain `. file` would overwrite a runner's injected secrets with those.
injected_env="$tmp/injected-env"
cat > "$injected_env" <<'TEMPLATE'
DEEPSEEK_API_KEY=
GITHUB_TOKEN=
SOME_OTHER_KEY=
TEMPLATE
# review.sh resolves ENV_FILE from XDG_CONFIG_HOME, so point it at a temp dir.
mkdir -p "$tmp/xdg/pr-agent"
cp "$injected_env" "$tmp/xdg/pr-agent/env"

probe="$(cd "$tmp" && XDG_CONFIG_HOME="$tmp/xdg" DEEPSEEK_API_KEY=injected-token \
  bash -c 'set -a; source <(sed -n "/^load_env_file()/,/^}/p" "$1"); set +a
           load_env_file "$2"; printf "%s" "${DEEPSEEK_API_KEY:-MISSING}"' _ \
  "$ROOT_DIR/services/pr-agent/review.sh" "$tmp/xdg/pr-agent/env")"
if [ "$probe" = "injected-token" ]; then
  pass "an injected credential survives the empty template"
else
  fail "an injected credential survives the empty template (got '${probe}')"
fi

# The file's own value must still apply when nothing was injected.
probe="$(cd "$tmp" && env -u DEEPSEEK_API_KEY XDG_CONFIG_HOME="$tmp/xdg" bash -c '
  set -a; source <(sed -n "/^load_env_file()/,/^}/p" "$1"); set +a
  printf "REAL_KEY=from-file\n" > "$2"
  load_env_file "$2"; printf "%s" "${REAL_KEY:-MISSING}"' _ \
  "$ROOT_DIR/services/pr-agent/review.sh" "$tmp/xdg/pr-agent/env2")"
if [ "$probe" = "from-file" ]; then
  pass "the secrets file supplies a variable nothing else set"
else
  fail "the secrets file supplies a variable nothing else set (got '${probe}')"
fi

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
printf '%s' "${CONFIG__FALLBACK_MODELS:-unset}" > "$PR_AGENT_ARGV_LOG.fallback"
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
expect_argv "--local selects the GPU model and lengthens the timeout" \
  "--diff-file=/tmp/x.diff --extra_config_url=$CONFIG review --config.model=ollama/qwen3:8b --config.ai_timeout=600" \
  --local --diff /tmp/x.diff

expect_fallback() { # <label> <expected> [wrapper args...]
  local label="$1" expected="$2"; shift 2
  if run_wrapper "$@"; then :; fi
  local actual
  actual="$(cat "$argv_log.fallback" 2>/dev/null)"
  if [ "$actual" = "$expected" ]; then
    pass "$label"
  else
    fail "$label (expected '$expected', got '$actual')"
  fi
}

expect_fallback "--local disables cloud fallback so no diff can leave the box" "[]" --local --diff /tmp/x.diff
expect_fallback "the cloud path keeps the configured fallbacks" "unset" --diff /tmp/x.diff
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

# ── --via-router overrides and router authentication ──────────────────────
# A one-shot HTTP server stands in for x3-ai-router: it answers /health and
# records the Authorization header it received, so both the argv and the auth
# path can be asserted without the real service.
stub_port="$(( 20000 + RANDOM % 20000 ))"
stub_seen="$tmp/probe-auth"
python3 - "$stub_port" "$stub_seen" <<'PY' &
import http.server, sys
port, seen = int(sys.argv[1]), sys.argv[2]

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get("Authorization"):
            open(seen, "w").write(self.headers["Authorization"])
        self.send_response(200)
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"ok")

    def log_message(self, *args):
        pass

http.server.HTTPServer(("127.0.0.1", port), Handler).serve_forever()
PY
stub_pid=$!
trap 'rm -rf "$tmp"; kill "$stub_pid" 2>/dev/null' EXIT
for _ in 1 2 3 4 5 6 7 8 9 10; do
  if (exec 3<>"/dev/tcp/127.0.0.1/$stub_port") 2>/dev/null; then exec 3<&- 3>&-; break; fi
  sleep 0.2
done

: > "$argv_log"
X3_ROUTER_TOKEN=probe-token PR_AGENT_VENV="$fake_venv" PR_AGENT_ARGV_LOG="$argv_log" \
  X3_ROUTER_BASE="http://127.0.0.1:$stub_port/v1" GITHUB_TOKEN="test-token" \
  bash "$ROOT_DIR/services/pr-agent/review.sh" --via-router --diff /tmp/x.diff review >"$tmp/out" 2>&1 || true
actual="$(tr '\n' ' ' < "$argv_log" | sed 's/ $//')"
expected="--diff-file=/tmp/x.diff --extra_config_url=$CONFIG review --config.model=openai/x3-deep"
if [ "$actual" = "$expected" ]; then
  pass "--via-router routes through the router policy with no direct fallback"
else
  fail "--via-router routes through the router policy with no direct fallback"
  printf '        expected: %s\n        actual:   %s\n' "$expected" "$actual"
fi

if [ "$(cat "$argv_log.fallback" 2>/dev/null)" = "[]" ]; then
  pass "--via-router disables fallback so retries cannot bypass the router"
else
  fail "--via-router disables fallback so retries cannot bypass the router"
fi

if [ "$(cat "$stub_seen" 2>/dev/null)" = "Bearer probe-token" ]; then
  pass "the router health probe sends the bearer token"
else
  fail "the router health probe sends the bearer token (got '$(cat "$stub_seen" 2>/dev/null)')"
fi

# ── check.sh gates only the providers the mode uses ───────────────────────
stub_bin="$tmp/stubbin"
mkdir -p "$stub_bin" "$tmp/fakevenv/bin"
cat > "$tmp/fakevenv/bin/pr-agent" <<'SHIM'
#!/usr/bin/env bash
echo "pr-agent 0.48.0"
SHIM
cat > "$stub_bin/curl" <<'SHIM'
#!/usr/bin/env bash
url="${*: -1}"; want_code=0
for arg in "$@"; do case "$arg" in *'%{http_code}'*) want_code=1 ;; esac; done
case "$url" in
  # FAIL_OLLAMA=1 simulates a box with no local model.
  *"/api/tags") [ "${FAIL_OLLAMA:-0}" = 1 ] && exit 22
                echo '{"models":[{"name":"qwen3:8b"}]}'; exit 0 ;;
  *"/health")   [ "$want_code" = 1 ] && echo 200; exit 0 ;;
  # A rejected DeepSeek key: curl -f exits nonzero and prints nothing.
  *"/models")   exit 22 ;;
esac
exit 0
SHIM
cat > "$stub_bin/gh" <<'SHIM'
#!/usr/bin/env bash
exit 0
SHIM
chmod +x "$stub_bin/curl" "$stub_bin/gh" "$tmp/fakevenv/bin/pr-agent"

run_check() { # <mode> [VAR=value ...]
  local mode="$1"; shift
  env "$@" PATH="$stub_bin:$PATH" PR_AGENT_VENV="$tmp/fakevenv" GITHUB_TOKEN=test-token \
    bash "$ROOT_DIR/services/pr-agent/check.sh" "$mode" >"$tmp/check.out" 2>&1
}

if run_check local DEEPSEEK_API_KEY=rejected-key; then
  pass "check.sh local ignores a rejected cloud key"
else
  fail "check.sh local ignores a rejected cloud key (exited nonzero)"
fi

if run_check cloud DEEPSEEK_API_KEY=rejected-key; then
  fail "check.sh cloud fails on a rejected cloud key (exited 0)"
else
  pass "check.sh cloud fails on a rejected cloud key"
fi

if run_check local DEEPSEEK_API_KEY=rejected-key FAIL_OLLAMA=1; then
  fail "check.sh local fails when the GPU model is missing (exited 0)"
else
  pass "check.sh local fails when the GPU model is missing"
fi

if run_check cloud DEEPSEEK_API_KEY=rejected-key FAIL_OLLAMA=1; then
  fail "check.sh cloud ignores a missing GPU model (exited nonzero)"
else
  pass "check.sh cloud ignores a missing GPU model"
fi

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
