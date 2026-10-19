#!/usr/bin/env bash
# Run a cargo gate unchanged and, when it fails, publish its first errors as GitHub check-run
# annotations (`::error ...`), so a failure on the self-hosted runners can be read through the
# checks API when the job log itself cannot be downloaded.
#
# The gate's exit status is the script's exit status: this changes only how a failure is
# reported, never whether one is.
#
#   bash scripts/ci/cargo-annotate.sh cargo clippy --locked --workspace --all-targets \
#       --message-format=short -- -D warnings
set -uo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <command> [args...]" >&2
  exit 2
fi

log="$(mktemp "${RUNNER_TEMP:-/tmp}/cargo-annotate.XXXXXX")"
"$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
# Cargo colours its output on a terminal-like runner; the patterns below match plain text.
sed -i -E 's/\x1b\[[0-9;]*m//g' "$log"

if [ "$status" -ne 0 ]; then
  # GitHub keeps at most 10 error annotations per step; the first ones are the actionable ones.
  {
    # `--message-format=short` diagnostics: path:line:col: error[CODE]: message
    grep -E '^[^ :]+:[0-9]+:[0-9]+: error' "$log" |
      sed -E 's/^([^:]+):([0-9]+):([0-9]+): (error[^:]*): (.*)$/::error file=\1,line=\2,col=\3::\4: \5/'
    # A panicking test: thread '<name>' panicked at path:line:col:
    grep -E "^thread '.*' (\([0-9]+\) )?panicked at [^ :]+:[0-9]+:[0-9]+" "$log" |
      sed -E "s/^thread '([^']*)' (\([0-9]+\) )?panicked at ([^:]+):([0-9]+):([0-9]+):?.*$/::error file=\3,line=\4,col=\5::test \1 panicked/"
    # Failures with no source location: a failed crate build, a failed test binary.
    grep -E '^error: (could not compile|test failed|failed to)' "$log" | sed -E 's/^/::error::/'
    # Anything else cargo reported as an error (a lockfile, a toolchain or a disk problem), so a
    # failure never reaches the checks API without saying why.
    grep -E '^error(\[[A-Z0-9]+\])?: ' "$log" | sed -E 's/^/::error::/'
    # The last lines, when nothing above matched at all.
    if ! grep -qE '^(error|thread .* panicked)' "$log"; then
      tail -n 5 "$log" | sed -E 's/^/::error::(tail) /'
    fi
  } | awk '!seen[$0]++' | head -10
fi

rm -f "$log"
exit "$status"
