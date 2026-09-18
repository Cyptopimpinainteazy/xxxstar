#!/usr/bin/env bash
# Local CI — run the gates that actually execute, on this machine.
#
# GitHub-hosted CI for this repository is blocked (account billing lock), so the
# self-hosted runner and this script are the only places these gates run. Every
# gate below is a real command with a real exit code; nothing is swallowed.
#
# Usage:
#   scripts/local-ci.sh                 # fast: format, guards, check, lint, unit tests
#   scripts/local-ci.sh --live          # + the two self-contained contract gates (anvil/SVM)
#   scripts/local-ci.sh --cross         # + the X3-native and X3VM<->EVM/SVM cross-domain lifecycles
#   scripts/local-ci.sh --release       # + the release gate (make mainnet-check)
#   scripts/local-ci.sh --variants      # + the runtime migration dry-run for all six variants
#   scripts/local-ci.sh --all           # everything
#   scripts/local-ci.sh --list          # show the gate list without running it
#
# Results are written to .ai/runlogs/local-ci-<timestamp>.log and a summary table
# is printed at the end. Exit status is non-zero if any gate failed.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

RUN_LIVE=0
RUN_CROSS=0
RUN_RELEASE=0
RUN_VARIANTS=0
LIST_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --live) RUN_LIVE=1 ;;
    --cross) RUN_LIVE=1; RUN_CROSS=1 ;;
    --release) RUN_RELEASE=1 ;;
    --variants) RUN_VARIANTS=1 ;;
    --all) RUN_LIVE=1; RUN_CROSS=1; RUN_RELEASE=1; RUN_VARIANTS=1 ;;
    --list) LIST_ONLY=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG_DIR="$ROOT/.ai/runlogs"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/local-ci-$STAMP.log"

declare -a GATE_NAMES=() GATE_STATUS=() GATE_SECS=() GATE_LOGS=()
FAILED=0

# run_gate <name> <log-suffix> <command...>
run_gate() {
  local name="$1"; shift
  local slug="$1"; shift
  local gate_log="$LOG_DIR/local-ci-$STAMP-$slug.log"
  GATE_NAMES+=("$name"); GATE_LOGS+=("$gate_log")
  echo ""
  echo "=== $name ==="
  local start=$SECONDS
  if "$@" > "$gate_log" 2>&1; then
    GATE_STATUS+=("PASS")
    echo "PASS ($((SECONDS - start))s) — $gate_log"
  else
    GATE_STATUS+=("FAIL")
    FAILED=1
    echo "FAIL ($((SECONDS - start))s) — $gate_log"
    tail -15 "$gate_log"
  fi
  GATE_SECS+=("$((SECONDS - start))")
}

# The repository's own gates.
GATES_FAST=(
  "format check:cargo fmt --all -- --check"
  "agent guards:make guard"
  "readiness consistency:bash scripts/check-readiness-consistency.sh"
  "workspace check:env SKIP_WASM_BUILD=1 cargo check --workspace"
  "clippy workspace:cargo clippy --workspace --all-targets -- -D warnings"
  "clippy runtime rc1:cargo clippy -p x3-chain-runtime --all-targets --no-default-features --features std,mainnet-rc1 -- -D warnings"
  "clippy node rc1:cargo clippy -p x3-chain-node --all-targets --features mainnet-rc1 -- -D warnings"
  "test x3-lang:cargo test --manifest-path x3-lang/Cargo.toml"
  "test atomic-kernel:cargo test -p pallet-x3-atomic-kernel"
  "test atomic-swap std:cargo test -p x3-atomic-swap --features std"
  "test settlement-engine:cargo test -p pallet-x3-settlement-engine"
  "test node:env SKIP_WASM_BUILD=1 cargo test -p x3-chain-node"
  "test cross-vm-coordinator:cargo test --manifest-path crates/cross-vm-coordinator/Cargo.toml"
)

# Gates that boot real chains (anvil / solana-test-validator / x3 dev node).
GATES_LIVE=(
  "EVM contract lifecycle:X3-contracts/evm/test-live-lifecycle.sh"
  "SVM contract lifecycle:programs/svm/x3_atomic_swap/test-live-lifecycle.sh"
)

describe_all() {
  echo "fast gates:"
  printf '  - %s\n' "${GATES_FAST[@]%%:*}" | grep -v '^  - $'
  if [ "$RUN_LIVE" = 1 ]; then
    echo "live gates:"
    printf '  - %s\n' "${GATES_LIVE[@]%%:*}"
  fi
  if [ "$RUN_CROSS" = 1 ]; then
    echo "cross-domain gates:"
    cat <<'EOF'
  - X3-native local-node lifecycles (cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored)
  - X3VM<->EVM cross-domain lifecycles (needs anvil + deployed AtlasHTLC; see .ai/runlogs for the runner recipe)
  - X3VM<->SVM cross-domain lifecycles (needs solana-test-validator + SBF program)
EOF
  fi
  if [ "$RUN_RELEASE" = 1 ]; then
    echo "release gate:"
    echo "  - make mainnet-check  (docs, release build + runtime WASM, chain specs, critical suites, srtool, secret scan)"
  fi
  if [ "$RUN_VARIANTS" = 1 ]; then
    echo "runtime variants:"
    echo "  - scripts/check-runtime-variants.sh  (migration dry-run per construct_runtime! variant)"
  fi
}

if [ "$LIST_ONLY" = 1 ]; then
  describe_all
  exit 0
fi

echo "local-ci $STAMP — root=$ROOT"
echo "log: $LOG"
{
  echo "local-ci $STAMP"
  echo "root=$ROOT"
  echo "live=$RUN_LIVE cross=$RUN_CROSS"
} > "$LOG"

for spec in "${GATES_FAST[@]}"; do
  name="${spec%%:*}"; cmd="${spec#*:}"
  slug="$(echo "$name" | tr ' ' '-')"
  # shellcheck disable=SC2086
  run_gate "$name" "$slug" env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" bash -c "$cmd"
done

# Live gates need a few tools and a free port; fail loudly rather than skip silently.
if [ "$RUN_LIVE" = 1 ]; then
  for spec in "${GATES_LIVE[@]}"; do
    name="${spec%%:*}"; cmd="${spec#*:}"
    slug="$(echo "$name" | tr ' ' '-')"
    # shellcheck disable=SC2086
    run_gate "$name" "$slug" env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" bash -c "$cmd"
  done
fi

if [ "$RUN_CROSS" = 1 ]; then
  run_gate "X3-native lifecycles" "x3-native-lifecycles" \
    env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" \
    bash -lc "env -u SKIP_WASM_BUILD cargo test -p x3-chain-node --test x3vm_live_lifecycle -- --ignored --nocapture --test-threads=1"
  echo ""
  echo "NOTE: the X3VM<->EVM and X3VM<->SVM cross-domain gates need anvil /"
  echo "      solana-test-validator wired up first; run them with"
  echo "      the recipes recorded in .ai/memory/agent-memory.md."
fi

# The release gate: docs, release build + runtime WASM, chain specs, the critical
# pallet/runtime suites, srtool/docker reproducibility prerequisites and the
# forbidden-secret scan. Needs `srtool` on PATH (see the memory notes for the
# pinned install command) or it fails loudly on section 5.
if [ "$RUN_RELEASE" = 1 ]; then
  run_gate "release gate (mainnet-check)" "release-gate" \
    env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" bash -c "make mainnet-check"
fi

# The migration dry-run covers every construct_runtime! variant; each variant is a
# separate feature set, so this compiles the runtime six times.
if [ "$RUN_VARIANTS" = 1 ]; then
  run_gate "runtime variant dry-runs" "runtime-variants" \
    env CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}" bash scripts/check-runtime-variants.sh
fi

echo ""
echo "──────── local-ci summary ($STAMP) ────────"
printf '%-34s %-6s %s\n' "GATE" "RESULT" "SECONDS"
for i in "${!GATE_NAMES[@]}"; do
  printf '%-34s %-6s %s\n' "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}"
done
{
  echo ""
  echo "──────── summary ────────"
  printf '%-34s %-6s %s\n' "GATE" "RESULT" "SECONDS"
  for i in "${!GATE_NAMES[@]}"; do
    printf '%-34s %-6s %s\n' "${GATE_NAMES[$i]}" "${GATE_STATUS[$i]}" "${GATE_SECS[$i]}"
  done
} >> "$LOG"

if [ "$FAILED" = 1 ]; then
  echo ""
  echo "local-ci: FAILED — see the per-gate logs listed above"
  exit 1
fi
echo ""
echo "local-ci: all gates passed"
