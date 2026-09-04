#!/usr/bin/env bash
# Prove internal X3 cross-VM router execution in a clean Rust container.
#
# Default proof target:
#   test_x3_native_evm_svm_roundtrip_preserves_supply
#
# This is the binding chain-layer proof for the internal X3Native -> X3Evm ->
# X3Svm -> X3Native transfer path. It executes the real Substrate router and
# supply-ledger pallet test harness, then writes machine- and human-readable
# evidence under reports/cross-vm/.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REPORT_DIR="${REPORT_DIR:-$ROOT_DIR/reports/cross-vm}"
DOCKER_IMAGE="${DOCKER_IMAGE:-rust:1.90-slim-bookworm}"
TEST_NAME="${TEST_NAME:-test_x3_native_evm_svm_roundtrip_preserves_supply}"
MODE="focused"

usage() {
  printf '%s\n' \
    "Usage: $0 [--full] [--test TEST_NAME]" \
    "" \
    "Options:" \
    "  --full            Run the full pallet-x3-cross-vm-router lib suite." \
    "  --test NAME       Run one router test by name." \
    "" \
    "Environment:" \
    "  DOCKER_IMAGE      Rust Docker image (default: rust:1.90-slim-bookworm)." \
    "  REPORT_DIR        Evidence output directory (default: reports/cross-vm)." \
    "  CACHE_ROOT        Host cache directory for Docker cargo/target caches."
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --full)
      MODE="full"
      shift
      ;;
    --test)
      [[ $# -ge 2 ]] || { echo "ERROR: --test requires a value" >&2; exit 2; }
      TEST_NAME="$2"
      MODE="focused"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v docker >/dev/null 2>&1; then
  echo "ERROR: docker is required for this proof script" >&2
  exit 127
fi

mkdir -p "$REPORT_DIR"

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SAFE_TEST_NAME="${TEST_NAME//[^A-Za-z0-9_.-]/_}"
LOG_FILE="$REPORT_DIR/router-proof-${MODE}-${SAFE_TEST_NAME}-${STAMP}.log"
JSON_FILE="$REPORT_DIR/router-proof-${MODE}-${SAFE_TEST_NAME}-${STAMP}.json"
MD_FILE="$REPORT_DIR/router-proof-${MODE}-${SAFE_TEST_NAME}-${STAMP}.md"
LATEST_LOG="$REPORT_DIR/router-proof-latest.log"
LATEST_JSON="$REPORT_DIR/router-proof-latest.json"
LATEST_MD="$REPORT_DIR/router-proof-latest.md"

CACHE_ROOT="${CACHE_ROOT:-${TMPDIR:-/tmp}/x3-cross-vm-router-proof}"
CARGO_HOME_HOST="$CACHE_ROOT/cargo-home"
TARGET_HOST="$CACHE_ROOT/target"
mkdir -p "$CARGO_HOME_HOST" "$TARGET_HOST"

if [[ "$MODE" == "full" ]]; then
  CARGO_TEST_ARGS=(test -p pallet-x3-cross-vm-router --lib -- --nocapture)
  TEST_LABEL="full router lib suite"
else
  CARGO_TEST_ARGS=(test -p pallet-x3-cross-vm-router --lib "$TEST_NAME" -- --nocapture)
  TEST_LABEL="$TEST_NAME"
fi

COMMAND_TEXT="cargo ${CARGO_TEST_ARGS[*]}"

echo "==> X3 cross-VM router proof"
echo "    mode:        $MODE"
echo "    test:        $TEST_LABEL"
echo "    docker:      $DOCKER_IMAGE"
echo "    log:         $LOG_FILE"
echo "    cargo cache: $CARGO_HOME_HOST"
echo "    target:      $TARGET_HOST"

set +e
docker run --rm \
  -v "$ROOT_DIR":/x3 \
  -v "$CARGO_HOME_HOST":/cargo-home \
  -v "$TARGET_HOST":/x3-target \
  -w /x3 \
  "$DOCKER_IMAGE" \
  sh -lc "export PATH=/usr/local/cargo/bin:\$PATH RUSTUP_TOOLCHAIN=1.90.0 CARGO_HOME=/cargo-home CARGO_TARGET_DIR=/x3-target CC=clang HOST_CC=clang CXX=clang++; \
    rm -rf /var/lib/apt/lists/*; \
    apt-get update >/dev/null; \
    apt-get install -y --no-install-recommends clang cmake git libssl-dev pkg-config protobuf-compiler >/dev/null; \
    CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo ${CARGO_TEST_ARGS[*]}" \
  2>&1 | tee "$LOG_FILE"
STATUS=${PIPESTATUS[0]}
set -e

if [[ "$STATUS" -eq 0 ]]; then
  RESULT="PASS"
else
  RESULT="FAIL"
fi

PASSED_COUNT="$(grep -Eo 'test result: ok\. [0-9]+ passed' "$LOG_FILE" | tail -1 | grep -Eo '[0-9]+' | tail -1 || true)"
FAILED_COUNT="$(grep -Eo '[0-9]+ failed' "$LOG_FILE" | tail -1 | grep -Eo '[0-9]+' | tail -1 || true)"
PASSED_COUNT="${PASSED_COUNT:-0}"
FAILED_COUNT="${FAILED_COUNT:-unknown}"

python3 - "$JSON_FILE" "$RESULT" "$STATUS" "$MODE" "$TEST_LABEL" "$DOCKER_IMAGE" "$COMMAND_TEXT" "$LOG_FILE" "$PASSED_COUNT" "$FAILED_COUNT" <<'PY'
import json
import sys
from datetime import datetime, timezone

path, result, status, mode, test_label, image, command, log_file, passed, failed = sys.argv[1:]
payload = {
    "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
    "result": result,
    "exit_code": int(status),
    "mode": mode,
    "test": test_label,
    "docker_image": image,
    "command": command,
    "log": log_file,
    "passed": int(passed) if passed.isdigit() else passed,
    "failed": int(failed) if failed.isdigit() else failed,
    "proof_surface": "pallet-x3-cross-vm-router Substrate router + supply-ledger test harness",
    "claim": "Internal X3Native/X3Evm/X3Svm cross-VM router execution preserves canonical supply and drains pending supply.",
}
with open(path, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2)
    handle.write("\n")
PY

{
  printf '# X3 Cross-VM Router Proof\n\n'
  printf '## Verdict\n\n'
  printf -- '- Result: %s\n' "$RESULT"
  printf -- '- Mode: %s\n' "$MODE"
  printf -- '- Test: %s\n' "$TEST_LABEL"
  printf -- '- Docker image: %s\n' "$DOCKER_IMAGE"
  printf -- '- Exit code: %s\n' "$STATUS"
  printf -- '- Passed count: %s\n' "$PASSED_COUNT"
  printf -- '- Failed count: %s\n\n' "$FAILED_COUNT"
  printf '## Proof Surface\n\n'
  printf 'This proof runs the `pallet-x3-cross-vm-router` Substrate pallet test harness. The focused default test executes the router and supply-ledger path for X3Native -> X3Evm, X3Evm -> X3Svm, and X3Svm -> X3Native, then verifies canonical supply conservation, pending supply drain, and VM-adapter origin enforcement.\n\n'
  printf '## Command\n\n'
  printf '```bash\n%s\n```\n\n' "$COMMAND_TEXT"
  printf '## Evidence\n\n'
  printf -- '- Log: %s\n' "$LOG_FILE"
  printf -- '- JSON: %s\n\n' "$JSON_FILE"
  printf '## Known Constraint\n\n'
  printf 'The local host Rust toolchains have shown rustc SIGSEGV/ICE failures while compiling third-party dependencies before X3 code. This script intentionally uses a clean Docker Rust environment so the proof is not dependent on the broken host compiler state.\n'
} > "$MD_FILE"

cp "$LOG_FILE" "$LATEST_LOG"
cp "$JSON_FILE" "$LATEST_JSON"
cp "$MD_FILE" "$LATEST_MD"

echo "==> Result: $RESULT"
echo "==> Wrote:  $JSON_FILE"
echo "==> Wrote:  $MD_FILE"
echo "==> Latest: $LATEST_JSON"
echo "==> Latest: $LATEST_MD"

exit "$STATUS"