#!/usr/bin/env bash
# Re-record `docs/reports/runtime-wasm-hashes.json` from a reproducible build.
#
# Stage 6b of the mainnet release gate rebuilds the runtime and fails when any
# value differs from that record, so every change that alters the runtime's
# bytes has to re-attest it. Doing that by hand is three commands and a
# copy-paste of eight hex strings; this does it once, and refuses to write
# anything unless two independent builds agree.
#
#   ./scripts/update-runtime-hashes.sh          # builds twice, then writes
#   ./scripts/update-runtime-hashes.sh --check  # builds twice, writes nothing
#
# "Two independent builds" means the second run is not the first run's cache:
# srtool's target directory under `$RUNTIME_DIR/target/` is removed in between
# (inside a throwaway container, because the image writes those files as its own
# uid). If the two builds disagree, nothing is written and the exit status is
# non-zero — that is a reproducibility failure, not a stale record.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RECORD="$ROOT/docs/reports/runtime-wasm-hashes.json"
RUNTIME_DIR="${SRTOOL_RUNTIME_DIR:-runtime}"
SRTOOL_IMAGE="${SRTOOL_IMAGE:-paritytech/srtool:1.93.0-0.18.4}"
CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

info() { printf '[runtime-hashes] %s\n' "$*"; }

clear_srtool_target() {
  # The container user owns those files, so remove them as root in a container.
  docker run --rm -u 0:0 -v "$ROOT":/build alpine \
    rm -rf "/build/$RUNTIME_DIR/target/srtool" >/dev/null 2>&1 || true
}

build_once() {
  SRTOOL_IMAGE="$SRTOOL_IMAGE" bash "$ROOT/scripts/run-srtool.sh" build
}

# srtool's output is verbose; hand the two logs over as files rather than as
# environment variables (the pair does not fit in ARG_MAX).
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
OUT1_FILE="$TMP_DIR/build1.log"
OUT2_FILE="$TMP_DIR/build2.log"

info "build 1/2"
build_once >"$OUT1_FILE" 2>&1
info "clearing srtool's target directory"
clear_srtool_target
info "build 2/2"
build_once >"$OUT2_FILE" 2>&1

OUT1_FILE="$OUT1_FILE" OUT2_FILE="$OUT2_FILE" RECORD="$RECORD" ROOT="$ROOT" \
CHECK_ONLY="$CHECK_ONLY" python3 - <<'PY'
import datetime
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys

root = pathlib.Path(os.environ["ROOT"])
record_path = pathlib.Path(os.environ["RECORD"])

# Reuse the gate's parser so the two can never disagree about what srtool said.
spec = importlib.util.spec_from_file_location(
    "mainnet_release_gate", root / "scripts" / "mainnet_release_gate.py"
)
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)

first = gate._srtool_values(pathlib.Path(os.environ["OUT1_FILE"]).read_text(errors="replace"))
second = gate._srtool_values(pathlib.Path(os.environ["OUT2_FILE"]).read_text(errors="replace"))

if not first or not second:
    print("[runtime-hashes] could not read a hash block from a build", file=sys.stderr)
    sys.exit(1)

# `version` and `metadata` come out of the same srtool block as the hashes, and
# they are compared between the two builds and written into the record: a record
# whose top-level `runtime_version` drifts behind the wasm it describes is the
# same kind of stale claim as a hash that no longer matches.
fields = ("size", "set_code", "authorize_upgrade", "ipfs", "blake2_256", "version", "metadata")
disagreements = [
    f"{runtime}.{field}: build1={first.get(runtime, {}).get(field)} "
    f"build2={second.get(runtime, {}).get(field)}"
    for runtime in sorted(set(first) | set(second))
    for field in fields
    if second.get(runtime, {}).get(field) != first.get(runtime, {}).get(field)
]
if disagreements:
    print("[runtime-hashes] the two builds disagree — this is a reproducibility "
          "failure, not a stale record:", file=sys.stderr)
    for line in disagreements:
        print(f"  {line}", file=sys.stderr)
    sys.exit(1)

revision = subprocess.run(
    ["git", "rev-parse", "--short", "HEAD"], cwd=root, capture_output=True, text=True, check=True
).stdout.strip()

record = json.loads(record_path.read_text())
previous = record.get("recorded_revision")
previous_runtimes = record.get("runtimes", {})
record["recorded_revision"] = revision
record["runtimes"] = {k: {f: first[k][f] for f in fields} for k in sorted(first)}

# The top-level fields describe the same artifact as the hashes. `runtime_version`
# and `recorded_at` used to be carried over from whatever the record said before,
# which left the record claiming "x3-chain-11" for a wasm that reports
# "x3-chain-12" after the BTC proof-of-work fix bumped the spec version.
compact_version = first.get("compact", {}).get("version")
if not compact_version:
    print("[runtime-hashes] srtool reported no Version line; refusing to leave the "
          "record's runtime_version stale", file=sys.stderr)
    sys.exit(1)
record["runtime_version"] = compact_version
record["recorded_at"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")

print(f"[runtime-hashes] two builds of {revision} agree ({previous} -> {revision})")
for runtime in sorted(first):
    print(f"  {runtime:<10} {first[runtime]['size']} bytes  {first[runtime]['blake2_256']}")

# The report quotes the same values in prose. Print the pairs so updating it is a
# copy-paste rather than a diff hunt.
doc_pairs = [
    (previous_runtimes.get(runtime, {}).get(field), first[runtime][field])
    for runtime in sorted(first)
    for field in fields
    if previous_runtimes.get(runtime, {}).get(field)
    and previous_runtimes[runtime][field] != first[runtime][field]
]
if doc_pairs and previous:
    print(
        "[runtime-hashes] docs/reports/runtime-wasm-reproducibility.md quotes the "
        "previous values; replace:"
    )
    for old, new in doc_pairs:
        print(f"  {old} -> {new}")
    if previous:
        print(f"  {previous} -> {revision}")

if os.environ["CHECK_ONLY"] == "1":
    print("[runtime-hashes] --check: nothing written")
    sys.exit(0)

record_path.write_text(json.dumps(record, indent=2) + "\n")
print(f"[runtime-hashes] wrote {record_path.relative_to(root)}")
print("[runtime-hashes] update docs/reports/runtime-wasm-reproducibility.md too if the "
      "hashes are quoted there, then run `make mainnet-check`.")
PY
