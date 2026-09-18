#!/usr/bin/env bash
# check-script-syntax.sh - syntax-check every script entry point in the tree.
#
# Why this exists: the workflows, the make targets, the hooks, and the operator
# runbooks all call into scripts/, and a shell typo or a Python indentation slip
# only shows up when that exact step runs. On a repo whose hosted CI cannot
# execute, that can be never. This gate parses every shell, Python and tooling
# JS file without executing any of them, so it is safe and takes seconds.
#
# Scope comes from git (tracked + untracked, ignored files skipped), so build
# output and vendored trees never enter the scan.
#
# Usage: scripts/check-script-syntax.sh [--quiet]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

QUIET=0
[ "${1:-}" = "--quiet" ] && QUIET=1

EXCLUDES=(
  ':(exclude)target/**' ':(exclude)**/target/**'
  ':(exclude)node_modules/**' ':(exclude)**/node_modules/**'
  ':(exclude)vendor/**' ':(exclude)**/vendor/**'
  ':(exclude).venv/**' ':(exclude)**/.venv/**'
  ':(exclude)**/forge-std/**' ':(exclude)**/lib/**'
  ':(exclude)**/dist/**' ':(exclude)**/build/**'
  ':(exclude)**/__pycache__/**'
  ':(exclude).toolchain/**'
  ':(exclude)**/tauri-vendor/**'
  ':(exclude)**/third_party/**'
)

collect() { # collect <array-name> <pathspec...>
  local -n out="$1"
  shift
  mapfile -d '' -t out < <(
    git ls-files -z --cached --others --exclude-standard -- "$@" "${EXCLUDES[@]}" 2>/dev/null
  )
}

shell_files=()
python_files=()
js_files=()

collect shell_files '*.sh' '*.bash'
collect python_files '*.py'
collect js_files 'scripts/**/*.js' 'scripts/**/*.cjs' 'scripts/**/*.mjs' 'tools/**/*.js'

failures=0

for file in "${shell_files[@]}"; do
  if ! message=$(bash -n "$file" 2>&1); then
    printf 'x bash -n %s\n%s\n' "$file" "$message"
    failures=$((failures + 1))
  fi
done

# One interpreter for all of them: 500 process spawns cost more than the compile.
# compile() instead of py_compile: no __pycache__ writes into the tree.
if [ "${#python_files[@]}" -gt 0 ]; then
  python_out=$(python3 - "${python_files[@]}" <<'PY'
import pathlib
import sys

bad = 0
for name in sys.argv[1:]:
    try:
        compile(pathlib.Path(name).read_bytes(), name, "exec")
    except SyntaxError as exc:
        bad += 1
        print(f"x python {name}:{exc.lineno}: {exc.msg}")
    except ValueError as exc:  # embedded NUL, bad encoding
        bad += 1
        print(f"x python {name}: {exc}")
sys.exit(1 if bad else 0)
PY
  ) || {
    printf '%s\n' "$python_out"
    failures=$((failures + $(printf '%s\n' "$python_out" | grep -c '^x python')))
  }
fi

if command -v node >/dev/null 2>&1; then
  for file in "${js_files[@]}"; do
    if ! message=$(node --check "$file" 2>&1); then
      printf 'x node %s\n%s\n' "$file" "$message"
      failures=$((failures + 1))
    fi
  done
elif [ "$QUIET" -eq 0 ]; then
  echo "  note: node not found - skipped ${#js_files[@]} JS file(s)"
fi

if [ "$QUIET" -eq 0 ]; then
  echo "script syntax check"
  echo "  shell files:  ${#shell_files[@]}"
  echo "  python files: ${#python_files[@]}"
  echo "  js files:     ${#js_files[@]}"
  echo "  failures:     $failures"
fi

if [ "$failures" -gt 0 ]; then
  echo "FAIL - $failures script(s) do not parse"
  exit 1
fi

[ "$QUIET" -eq 0 ] && echo "OK - every script parses"
exit 0
