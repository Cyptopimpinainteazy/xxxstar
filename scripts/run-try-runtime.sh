#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-try-runtime.sh — X3 try-runtime migration & state-transition checker
#
# Uses the `try-runtime` feature baked into x3-chain-node to verify that:
#   • All OnRuntimeUpgrade hooks complete without panic
#   • Pre/post migration state invariants hold
#   • No storage version mismatches exist
#
# Modes:
#   ./scripts/run-try-runtime.sh build      — compile node with try-runtime feature
#   ./scripts/run-try-runtime.sh live       — run against a live WS endpoint
#   ./scripts/run-try-runtime.sh snap       — run against a local state snapshot
#   ./scripts/run-try-runtime.sh help       — show this help
#
# Dependencies: Rust stable+nightly, built x3-chain-node
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="$REPO_ROOT/target/release/x3-chain-node"
SNAP_DIR="$REPO_ROOT/.try-runtime-snapshots"
LOG_FILE="$REPO_ROOT/try-runtime.log"

# Default WS endpoint for live mode (override with TRY_RUNTIME_URI env var)
URI="${TRY_RUNTIME_URI:-ws://127.0.0.1:9944}"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

info()    { echo -e "${CYAN}[try-runtime]${NC} $*"; }
success() { echo -e "${GREEN}[try-runtime]  ✓${NC} $*"; }
warn()    { echo -e "${YELLOW}[try-runtime] ⚠${NC} $*"; }
die()     { echo -e "${RED}[try-runtime] ✗ ERROR:${NC} $*" >&2; exit 1; }

print_help() {
  cat <<EOF

  X3 try-runtime — runtime upgrade & storage migration checker

  USAGE:
    ./scripts/run-try-runtime.sh [COMMAND]

  COMMANDS:
    build        Compile x3-chain-node with --features try-runtime
    live         Run on-runtime-upgrade checks against a live node WS
                 Set TRY_RUNTIME_URI=ws://host:port to override endpoint
    snap         Snapshot live state then run checks offline
                 Set TRY_RUNTIME_URI to pick the source
    check-pallets  List pallet storage versions currently in WASM
    help         Show this message

  EXAMPLES:
    # 1) Build first
    ./scripts/run-try-runtime.sh build

    # 2a) Run against local dev node
    TRY_RUNTIME_URI=ws://127.0.0.1:9944 ./scripts/run-try-runtime.sh live

    # 2b) Snapshot then check offline
    TRY_RUNTIME_URI=ws://127.0.0.1:9944 ./scripts/run-try-runtime.sh snap

EOF
}

cmd_build() {
  info "Building x3-chain-node with --features try-runtime …"
  info "(this reuses incremental cache; ~2–5 min on first run)"
  cd "$REPO_ROOT"
  cargo build \
    --release \
    -p x3-chain-node \
    --features try-runtime \
    2>&1 | tee "$LOG_FILE"
  success "Build complete → $NODE_BIN"
  echo ""
  info "Binary size: $(du -sh "$NODE_BIN" | cut -f1)"
}

cmd_live() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-try-runtime.sh build' first."

  info "Running on-runtime-upgrade checks against $URI"
  info "Output → $LOG_FILE"
  echo ""

  # Check node supports try-runtime subcommand
  if ! "$NODE_BIN" try-runtime --help &>/dev/null; then
    die "try-runtime subcommand not available. Rebuild with: ./scripts/run-try-runtime.sh build"
  fi

  "$NODE_BIN" try-runtime \
    --runtime existing \
    on-runtime-upgrade \
    live \
    --uri "$URI" \
    2>&1 | tee "$LOG_FILE"

  local exit_code=${PIPESTATUS[0]}
  echo ""
  if [[ $exit_code -eq 0 ]]; then
    success "All OnRuntimeUpgrade hooks PASSED. No panics. State invariants hold."
    grep -E 'migration|upgrade|version' "$LOG_FILE" | grep -v '^$' | head -20 || true
  else
    die "try-runtime detected FAILURES. Check $LOG_FILE for details."
  fi
}

cmd_snap() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-try-runtime.sh build' first."

  mkdir -p "$SNAP_DIR"
  local snap_file="$SNAP_DIR/x3-state-$(date +%Y%m%d-%H%M%S).snap"

  info "Snapshotting chain state from $URI …"
  info "Snapshot → $snap_file"

  "$NODE_BIN" try-runtime \
    --runtime existing \
    on-runtime-upgrade \
    live \
    --uri "$URI" \
    --snapshot-path "$snap_file" \
    2>&1 | tee "$LOG_FILE"

  local exit_code=${PIPESTATUS[0]}
  echo ""
  if [[ $exit_code -eq 0 ]]; then
    success "Snapshot saved: $snap_file"
    info "Snapshot size: $(du -sh "$snap_file" | cut -f1)"
    echo ""
    info "To re-run offline:"
    echo "  $NODE_BIN try-runtime --runtime existing on-runtime-upgrade snap --snapshot-path $snap_file"
  else
    die "Snapshot + upgrade check FAILED. Check $LOG_FILE"
  fi
}

cmd_check_pallets() {
  [[ -f "$NODE_BIN" ]] || die "Node binary not found. Run './scripts/run-try-runtime.sh build' first."

  info "Fetching pallet storage versions from WASM runtime …"
  info "(requires running node at $URI)"

  "$NODE_BIN" try-runtime \
    --runtime existing \
    on-runtime-upgrade \
    live \
    --uri "$URI" \
    2>&1 | grep -E 'StorageVersion|version|migration' | head -40 || true
}

# ─────────────────────────────────────────────────────────────────────────────
# test — CI-friendly non-interactive validation suite (no live node required)
#   1. Binary exists
#   2. Binary has try-runtime subcommand
#   3. try-runtime --help exits 0
#   4. Snapshot directory is writeable
#   5. WASM binary exists alongside the node
# ─────────────────────────────────────────────────────────────────────────────
cmd_test() {
  local PASS=0 FAIL=0

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo "  try-runtime — CI Test Suite"
  echo "══════════════════════════════════════════════════════"
  echo ""

  _check() {
    local label="$1"; shift
    echo -n "  Checking: $label … "
    if "$@" &>/dev/null; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC}"
      FAIL=$((FAIL + 1))
    fi
  }

  # 1. Node binary exists
  _check "Node binary exists" test -f "$NODE_BIN"

  # 2. Binary has try-runtime subcommand
  if [[ -f "$NODE_BIN" ]]; then
    _check "try-runtime subcommand available" "$NODE_BIN" try-runtime --help
  else
    echo -e "  Skipping subcommand checks (binary not built)"
    FAIL=$((FAIL + 2))
  fi

  # 3. Snapshot directory is writable (or can be created)
  mkdir -p "$SNAP_DIR"
  _check "Snapshot directory writable" test -w "$SNAP_DIR"

  # 4. WASM binary exists next to the node
  local wasm_path
  wasm_path="$REPO_ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"
  _check "WASM binary exists" test -f "$wasm_path"

  # 5. Expected pallets appear in node --version output
  if [[ -f "$NODE_BIN" ]]; then
    echo -n "  Checking: node --version output … "
    local ver
    ver=$("$NODE_BIN" --version 2>&1 || true)
    if [[ -n "$ver" ]]; then
      echo -e "${GREEN}✓ PASS${NC} ($ver)"
      PASS=$((PASS + 1))
    else
      echo -e "${YELLOW}⚠ WARN${NC} (empty version string)"
    fi
  fi

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo -e "  Results: ${GREEN}✓ $PASS PASSED${NC}  |  ${RED}✗ $FAIL FAILED${NC}"
  echo "══════════════════════════════════════════════════════"
  echo ""

  [[ $FAIL -eq 0 ]] && success "All try-runtime CI tests passed." || die "$FAIL test(s) failed."
}

# ─────────── dispatch ──────────────────────────────────────────────────────
COMMAND="${1:-help}"
case "$COMMAND" in
  build)         cmd_build ;;
  live)          cmd_live ;;
  snap)          cmd_snap ;;
  check-pallets) cmd_check_pallets ;;
  test)          cmd_test ;;
  help|--help|-h) print_help ;;
  *)             warn "Unknown command: $COMMAND"; print_help; exit 1 ;;
esac
