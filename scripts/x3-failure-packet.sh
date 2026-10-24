#!/usr/bin/env bash
# Wrap any X3 gate and turn a failure into a failure packet.
#
# The deterministic simulator emits packets for simulated failures. The same
# pattern has to cover the subsystems it cannot simulate yet — the atomic
# kernel, the settlement engine, the supply ledger, the runtime — where the
# failure arrives as a failing cargo test instead of a violated invariant.
#
# This wrapper runs the command, and on a non-zero exit extracts what a
# root-cause agent needs: the first error, the failing tests, the file/line
# each panic points at, the exact commit, and the replay command. Nothing is
# invented: a passing gate writes no packet, and fields that cannot be
# extracted are absent rather than guessed.
#
# Usage:
#   scripts/x3-failure-packet.sh --label pallet-x3-settlement-engine -- \
#       cargo test -p pallet-x3-settlement-engine
#   scripts/x3-failure-packet.sh --label runtime --root-cause -- \
#       cargo check -p x3-chain-runtime
#
# Exit code is the wrapped command's exit code, so this drops into CI directly.
set -uo pipefail

LABEL=""
ROOT_CAUSE=0
PACKET_DIR="${X3_FAILURE_PACKET_DIR:-failure-packets}"

while [ $# -gt 0 ]; do
  case "$1" in
    --label)
      if [ $# -lt 2 ]; then echo "x3-failure-packet: --label needs a value" >&2; exit 2; fi
      LABEL="$2"; shift 2 ;;
    --packet-dir)
      if [ $# -lt 2 ]; then echo "x3-failure-packet: --packet-dir needs a value" >&2; exit 2; fi
      PACKET_DIR="$2"; shift 2 ;;
    --root-cause) ROOT_CAUSE=1; shift ;;
    --) shift; break ;;
    -h|--help) sed -n '2,26p' "$0"; exit 0 ;;
    *) echo "x3-failure-packet: unknown argument '$1'" >&2; exit 2 ;;
  esac
done

if [ -z "$LABEL" ] || [ $# -eq 0 ]; then
  echo "x3-failure-packet: usage: $0 --label <name> [--root-cause] -- <command...>" >&2
  exit 2
fi

# A label names a file, so refuse one that cannot be part of a filename. The
# packet builder slugifies, but refusing early keeps the caller honest.
case "$LABEL" in
  */*|*\\*) echo "x3-failure-packet: --label must not contain '/' or '\\'" >&2; exit 2 ;;
esac

# The command as a shell would need to see it again. `$*` loses the quoting,
# so a line an operator copies back into a shell can silently run a different
# command when an argument contains spaces.
QUOTED_COMMAND=""
for arg in "$@"; do
  QUOTED_COMMAND+="$(printf '%q ' "$arg")"
done
QUOTED_COMMAND="${QUOTED_COMMAND% }"

# `X3_FAILURE_PACKET_ROOT` lets the wrapper run a gate in another checkout
# (for example a warm main tree while the wrapper itself is being reviewed in
# a worktree). Defaults to the repository containing the script.
ROOT="${X3_FAILURE_PACKET_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
if ! cd "$ROOT"; then
  echo "x3-failure-packet: cannot enter checkout '$ROOT' (set X3_FAILURE_PACKET_ROOT to a real checkout)" >&2
  exit 2
fi

LOG="$(mktemp -t x3-gate-log.XXXXXX)"
trap 'rm -f "$LOG"' EXIT

echo "x3-failure-packet: running [$LABEL]: $QUOTED_COMMAND"
started="$(date +%s)"
set +e
"$@" 2>&1 | tee "$LOG"
status="${PIPESTATUS[0]}"
set -e
elapsed="$(( $(date +%s) - started ))"

if [ "$status" -eq 0 ]; then
  echo "x3-failure-packet: [$LABEL] PASS in ${elapsed}s"
  exit 0
fi

if ! mkdir -p "$PACKET_DIR"; then
  echo "x3-failure-packet: [$LABEL] FAIL (exit $status) — cannot create $(printf '%q' "$PACKET_DIR"); the gate result stands" >&2
  exit "$status"
fi
if ! PACKET_PATH="$(python3 - "$LABEL" "$status" "$LOG" "$PACKET_DIR" "$ROOT" "$elapsed" "$@" <<'PY'
import hashlib
import json
import re
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

label, status, log_path, packet_dir, root, elapsed, *command = sys.argv[1:]
slug = re.sub(r"[^A-Za-z0-9._-]+", "-", label).strip("-") or "gate"
log = Path(log_path).read_text(errors="replace")
lines = log.splitlines()

def git(*args):
    try:
        return subprocess.run(["git", *args], cwd=root, capture_output=True, text=True, timeout=10).stdout.strip()
    except Exception:
        return ""

first_error = next(
    (line.strip() for line in lines
     if re.search(r"panicked at|^error(\[|:)| assertion|left == right|test result: FAILED", line)),
    "",
)
if not first_error:
    first_error = lines[-1].strip() if lines else ""
if not first_error:
    # A silent failure is still a failure: the dispatcher requires a
    # non-empty first_error, so record the one fact that always exists.
    first_error = f"{label} exited {status} with no output"
failing_tests = sorted({
    match.group(1)
    for line in lines
    for match in [re.search(r"^test (.*) \.\.\. FAILED", line), re.search(r"^---- (.*) stdout ----", line)]
    if match
})
locations = []
for line in lines:
    for pattern in (r"panicked at ([^:]+):(\d+):(\d+)", r"--> ([^:]+):(\d+):(\d+)"):
        match = re.search(pattern, line)
        if match:
            locations.append({"file": match.group(1), "line": int(match.group(2))})
# Keep log order: the first entry is where the failure actually surfaced, and
# it is a better `suspected_file` than an alphabetical accident.
unique = []
seen_locations = set()
for item in locations:
    key = (item["file"], item["line"])
    if key not in seen_locations:
        seen_locations.add(key)
        unique.append(item)
locations = unique

identity = f"{label}|{status}|{command}|{first_error}|{locations}|{failing_tests}"
failure_id = hashlib.sha256(identity.encode()).hexdigest()[:16]

packet = {
    "schema": "x3-gate-failure-packet-v1",
    "failure_id": failure_id,
    "gate": label,
    "producer": "scripts/x3-failure-packet.sh",
    "commit": git("rev-parse", "HEAD"),
    "branch": git("branch", "--show-current"),
    "worktree_dirty": bool(git("status", "--porcelain")),
    "stored_at": datetime.now(timezone.utc).isoformat(),
    # Saved with shell quoting, so the replay line runs the failing command
    # exactly even when an argument contains spaces.
    "command": shlex.join(command),
    "exit_code": int(status),
    "duration_seconds": int(elapsed),
    "first_error": first_error,
    "failing_tests": failing_tests,
    "suspected_file": locations[0]["file"] if locations else "",
    "suspected_lines": sorted({item["line"] for item in locations}),
    "locations": locations,
    "log_excerpt": "\n".join(lines[-200:]),
    # The wrapper runs its command after `cd "$ROOT"`; a replay from another
    # directory would exercise different code, so the directory is part of the
    # reproduction.
    "replay_command": f"cd {shlex.quote(root)} && " + shlex.join(command),
    "next_step": "Verify the cause before changing code: rerun the replay command, confirm the failure, then run the same command after the fix.",
}

stem = f"{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{slug}-{failure_id}"
json_path = Path(packet_dir) / f"{stem}.json"
json_path.write_text(json.dumps(packet, indent=2) + "\n")

# Gate output is arbitrary text: a line of backticks must not close the
# excerpt fence, so the fence is one backtick longer than the longest run in
# the excerpt (at least three).
excerpt = packet["log_excerpt"]
fence = "`" * max([3] + [len(run) + 1 for run in re.findall(r"`+", excerpt)])

markdown = [
    f"# Gate failure packet {failure_id}",
    "",
    f"- gate: `{label}`",
    f"- command: `{packet['command']}`",
    f"- exit code: {status}",
    f"- commit: `{packet['commit']}` on `{packet['branch']}` (dirty: {packet['worktree_dirty']})",
    f"- first error: {first_error}",
    "",
    "## Failing tests",
    "",
    *([f"- `{test}`" for test in failing_tests] or ["- (none parsed; see the log excerpt)"]),
    "",
    "## Suspected locations",
    "",
    *([f"- `{item['file']}:{item['line']}`" for item in locations] or ["- (no file:line in the output)"]),
    "",
    "## Repro",
    "",
    "```bash",
    packet["replay_command"],
    "```",
    "",
    "## Log excerpt (tail)",
    "",
    f"{fence}text",
    excerpt,
    fence,
]
(Path(packet_dir) / f"{stem}.md").write_text("\n".join(markdown) + "\n")
print(json_path)
PY
)"; then
  # The gate status is the contract: a broken packet builder must not replace
  # a test failure with a tooling failure.
  echo "x3-failure-packet: [$LABEL] FAIL (exit $status) — packet generation failed; the gate result stands" >&2
  exit "$status"
fi

echo "x3-failure-packet: [$LABEL] FAIL (exit $status) — packet: $PACKET_PATH" >&2
echo "x3-failure-packet: replay: $QUOTED_COMMAND" >&2

if [ "$ROOT_CAUSE" -eq 1 ]; then
  if [ ! -f "$ROOT/crates/x3-sim/scripts/root_cause.py" ]; then
    echo "x3-failure-packet: root-cause dispatcher missing; packet kept, nothing dispatched" >&2
    exit "$status"
  fi
  python3 "$ROOT/crates/x3-sim/scripts/root_cause.py" "$PACKET_PATH" \
    --out "${X3_ROOT_CAUSE_DIR:-reports/root-cause}" || \
    echo "x3-failure-packet: root-cause dispatch failed; the packet above is still the evidence" >&2
fi

exit "$status"
