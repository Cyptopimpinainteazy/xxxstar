#!/usr/bin/env bash
# Self-test for the gate failure-packet wrapper.
#
# Proves both directions: a failing gate produces a packet whose extracted
# file/line/error match the failure, and a passing gate produces no packet at
# all. Without the second assertion the wrapper could pass by writing packets
# unconditionally, which would make every future packet worthless.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

WORK="$(mktemp -d -t x3-failure-packet-test.XXXXXX)"
trap 'rm -rf "$WORK"' EXIT

fail=0

# Run a command, capturing its status without letting errexit (which is not
# enabled) or a leaked `set -e` abort the rest of the test.
run() {
  local status=0
  "$@" >/dev/null 2>&1 || status=$?
  printf '%s' "$status"
}

echo "test_x3_failure_packet: failing gate must produce a packet"
fixture='printf "running 1 test\n"; printf "thread %s panicked at pallets/x3-supply-ledger/src/lib.rs:412:9:\nassertion \`left == right\` failed\n" "main"; exit 101'
status="$(run bash scripts/x3-failure-packet.sh --label selftest-fail --packet-dir "$WORK/fail-packets" -- \
  bash -c "$fixture")"

if [ "$status" -ne 101 ]; then
  echo "  FAIL: wrapper exit was $status, expected the wrapped command's 101" >&2
  fail=1
fi

packet="$(find "$WORK/fail-packets" -name '*.json' | head -1)"
if [ -z "$packet" ]; then
  echo "  FAIL: no packet written for a failing gate" >&2
  fail=1
else
  python3 - "$packet" <<'PY' || fail=1
import json
import sys
from pathlib import Path

packet = json.load(open(sys.argv[1]))
assert packet["schema"] == "x3-gate-failure-packet-v1", packet["schema"]
assert packet["exit_code"] == 101, packet["exit_code"]
assert packet["suspected_file"] == "pallets/x3-supply-ledger/src/lib.rs", packet["suspected_file"]
assert 412 in packet["suspected_lines"], packet["suspected_lines"]
assert "panicked at" in packet["first_error"], packet["first_error"]
assert packet["commit"], "the packet must pin the commit it ran on"
assert isinstance(packet["duration_seconds"], int), packet["duration_seconds"]
assert packet["replay_command"].startswith("cd "), packet["replay_command"]
assert "pallets/x3-supply-ledger/src/lib.rs" in packet["first_error"]
markdown = Path(sys.argv[1]).with_suffix(".md").read_text()
assert "## Log excerpt (tail)" in markdown, "the markdown must carry the excerpt it points at"
assert "panicked at" in markdown, "the markdown excerpt must contain the failure"
print("  packet fields verified")
PY
fi

echo "test_x3_failure_packet: replay lines quote arguments a shell must re-split"
status=0
bash scripts/x3-failure-packet.sh --label selftest-quoting --packet-dir "$WORK/quoting-packets" -- \
  bash -c 'printf "thread main panicked at x.rs:1:1\n"; exit 101' -- "arg with spaces" \
  >"$WORK/quoting.out" 2>"$WORK/quoting.err" || status=$?
if [ "$status" -ne 101 ]; then
  echo "  FAIL: quoting fixture exit was $status, expected 101" >&2
  fail=1
fi
if ! grep -Fq 'arg\ with\ spaces' "$WORK/quoting.out"; then
  echo "  FAIL: the running line does not quote 'arg with spaces': $(cat "$WORK/quoting.out")" >&2
  fail=1
fi
if grep -Fq -- "-- arg with spaces" "$WORK/quoting.out"; then
  echo "  FAIL: the running line re-splits 'arg with spaces'" >&2
  fail=1
fi
if ! grep -Fq 'arg\ with\ spaces' "$WORK/quoting.err"; then
  echo "  FAIL: the replay line does not quote 'arg with spaces': $(cat "$WORK/quoting.err")" >&2
  fail=1
fi
if grep -Fq -- "-- arg with spaces" "$WORK/quoting.err"; then
  echo "  FAIL: the replay line re-splits 'arg with spaces'" >&2
  fail=1
fi
packet="$(find "$WORK/quoting-packets" -name '*.json' | head -1)"
if [ -z "$packet" ]; then
  echo "  FAIL: quoting fixture wrote no packet" >&2
  fail=1
else
  python3 - "$packet" <<'PY' || fail=1
import json
import sys

packet = json.load(open(sys.argv[1]))
assert "'arg with spaces'" in packet["command"], packet["command"]
assert "'arg with spaces'" in packet["replay_command"], packet["replay_command"]
print("  replay quoting verified")
PY
fi

echo "test_x3_failure_packet: a label that names no file is a usage error"
status="$(run bash scripts/x3-failure-packet.sh --label bad/name --packet-dir "$WORK/slash" -- bash -c 'exit 101')"
if [ "$status" -ne 2 ]; then
  echo "  FAIL: slash label exit was $status, expected 2" >&2
  fail=1
fi
if [ -d "$WORK/slash" ]; then
  echo "  FAIL: a refused label still created a packet directory" >&2
  fail=1
fi

echo "test_x3_failure_packet: a backtick line cannot break the markdown fence"
cat > "$WORK/fence-fixture.sh" <<'FIXTURE'
printf 'before\n'
printf '```\n'
printf 'thread main panicked at x.rs:1:1\n'
exit 101
FIXTURE
status="$(run bash scripts/x3-failure-packet.sh --label selftest-fence --packet-dir "$WORK/fence-packets" -- \
  bash "$WORK/fence-fixture.sh")"
if [ "$status" -ne 101 ]; then
  echo "  FAIL: fence fixture exit was $status, expected 101" >&2
  fail=1
fi
packet="$(find "$WORK/fence-packets" -name '*.json' | head -1)"
if [ -z "$packet" ]; then
  echo "  FAIL: no packet written for the fence fixture" >&2
  fail=1
else
  python3 - "$packet" <<'PY' || fail=1
import sys
from pathlib import Path

lines = Path(sys.argv[1]).with_suffix(".md").read_text().splitlines()
start = lines.index("````text")
end = lines.index("````", start + 1)
excerpt = lines[start + 1 : end]
assert "```" in "\n".join(excerpt), "the excerpt keeps its own backticks"
print("  fence escaped")
PY
fi

echo "test_x3_failure_packet: passing gate must write no packet"
status="$(run bash scripts/x3-failure-packet.sh --label selftest-pass --packet-dir "$WORK/pass-packets" -- \
  bash -c 'exit 0')"
if [ "$status" -ne 0 ]; then
  echo "  FAIL: passing gate exit was $status, expected 0" >&2
  fail=1
fi
if [ -d "$WORK/pass-packets" ] && [ -n "$(find "$WORK/pass-packets" -type f 2>/dev/null)" ]; then
  echo "  FAIL: a passing gate wrote a packet" >&2
  fail=1
fi

echo "test_x3_failure_packet: a broken packet builder must not replace the gate status"
status="$(run bash scripts/x3-failure-packet.sh --label selftest-broken-dir --packet-dir /dev/null/not-a-dir -- \
  bash -c 'exit 101')"
if [ "$status" -ne 101 ]; then
  echo "  FAIL: wrapper exit was $status, expected the gate's 101 despite the packet failure" >&2
  fail=1
fi

echo "test_x3_failure_packet: a silent failing gate still yields a dispatchable packet"
status="$(run bash scripts/x3-failure-packet.sh --label selftest-silent --packet-dir "$WORK/silent-packets" -- \
  bash -c 'exit 7')"
if [ "$status" -ne 7 ]; then
  echo "  FAIL: silent gate exit was $status, expected 7" >&2
  fail=1
fi
packet="$(find "$WORK/silent-packets" -name '*.json' | head -1)"
if [ -z "$packet" ]; then
  echo "  FAIL: a silent failure wrote no packet" >&2
  fail=1
else
  # The producer and the dispatcher must agree: run the dispatcher's own
  # validator over the packet the wrapper just wrote.
  python3 - "$packet" <<'PY' || fail=1
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path("crates/x3-sim/scripts")))
import root_cause  # noqa: E402

packet = json.load(open(sys.argv[1]))
assert packet["first_error"].strip(), "a silent failure still needs a first_error"
assert "7" in packet["first_error"], packet["first_error"]
root_cause.load_packet(Path(sys.argv[1]))  # exits non-zero if not dispatchable
print("  silent-failure packet is dispatchable")
PY
fi

echo "test_x3_failure_packet: an unusable checkout is refused before the gate runs"
marker="$WORK/gate-ran"
status="$(X3_FAILURE_PACKET_ROOT="$WORK/missing-checkout" run bash scripts/x3-failure-packet.sh \
  --label selftest-bad-root --packet-dir "$WORK/bad-root" -- bash -c "touch '$marker'; exit 0")"
if [ "$status" -ne 2 ]; then
  echo "  FAIL: bad checkout exit was $status, expected 2" >&2
  fail=1
fi
if [ -e "$marker" ]; then
  echo "  FAIL: the gate ran despite an unusable checkout" >&2
  fail=1
fi

echo "test_x3_failure_packet: usage error must be exit 2"
status="$(run bash scripts/x3-failure-packet.sh --label missing-command)"
if [ "$status" -ne 2 ]; then
  echo "  FAIL: usage error exit was $status, expected 2" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "test_x3_failure_packet: FAILED" >&2
  exit 1
fi
echo "test_x3_failure_packet: OK"
