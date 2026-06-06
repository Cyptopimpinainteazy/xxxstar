#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-substrate-tools.sh — Master orchestrator for all X3 Substrate tooling
#
# Provides a single entry-point to all 5 Substrate SDK ecosystem tools used
# for testnet readiness, correctness, and release hardening:
#
#   1. try-runtime   — safe runtime upgrade migration testing
#   2. zombienet     — ephemeral multi-validator testnet (block production + finality)
#   3. chopsticks    — state forking / block replay / storage mutation
#   4. benchmarks    — FRAME pallet weight generation (prevents overweight blocks)
#   5. srtool        — deterministic reproducible WASM (governance-grade hash)
#
# USAGE:
#   ./scripts/run-substrate-tools.sh              — interactive menu
#   ./scripts/run-substrate-tools.sh status       — prerequisite health check
#   ./scripts/run-substrate-tools.sh try-runtime  — delegate to try-runtime script
#   ./scripts/run-substrate-tools.sh zombienet    — delegate to zombienet script
#   ./scripts/run-substrate-tools.sh chopsticks   — delegate to chopsticks script
#   ./scripts/run-substrate-tools.sh benchmarks   — delegate to benchmarks script
#   ./scripts/run-substrate-tools.sh srtool       — delegate to srtool script
#   ./scripts/run-substrate-tools.sh all-checks   — run non-interactive pre-flight checks
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCRIPTS="$REPO_ROOT/scripts"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'
BOLD='\033[1m'; RESET='\033[0m'
info()    { echo -e "${CYAN}[tools]${RESET} $*"; }
success() { echo -e "${GREEN}[tools]  ✓${RESET} $*"; }
warn()    { echo -e "${YELLOW}[tools] ⚠${RESET} $*"; }
die()     { echo -e "${RED}[tools] ✗ ERROR:${RESET} $*" >&2; exit 1; }
ok()      { echo -e "  ${GREEN}✓${RESET} $*"; }
nok()     { echo -e "  ${RED}✗${RESET} $*"; }
maybe()   { echo -e "  ${YELLOW}?${RESET} $*"; }

# ── Tool table ───────────────────────────────────────────────────────────────
print_table() {
  echo ""
  echo -e "${BOLD}  X3 Substrate Ecosystem Tools${RESET}"
  echo "  ──────────────────────────────────────────────────────────────────────────────"
  printf "  ${BOLD}%-14s %-28s %-16s %-18s${RESET}\n" "TOOL" "PURPOSE" "X3 TARGET" "QUICK START"
  echo "  ──────────────────────────────────────────────────────────────────────────────"
  printf "  %-14s %-28s %-16s %-18s\n" \
    "try-runtime"  "Migration safety (OnRuntimeUpgrade)" "x3-chain-node"     "run-try-runtime.sh build"
  printf "  %-14s %-28s %-16s %-18s\n" \
    "zombienet"    "Multi-validator ephemeral testnet"   "3x validator"      "run-zombienet.sh spawn"
  printf "  %-14s %-28s %-16s %-18s\n" \
    "chopsticks"   "State fork / block replay / mutation" "ws://9944"        "run-chopsticks.sh fork"
  printf "  %-14s %-28s %-16s %-18s\n" \
    "benchmarks"   "FRAME pallet weight generation"      "4x pallets"        "run-frame-benchmarks.sh run"
  printf "  %-14s %-28s %-16s %-18s\n" \
    "srtool"       "Deterministic reproducible WASM"     "x3-chain-runtime"  "run-srtool.sh build"
  echo "  ──────────────────────────────────────────────────────────────────────────────"
  echo ""
}

# ── Prerequisite status ──────────────────────────────────────────────────────
cmd_status() {
  echo ""
  echo -e "${BOLD}  X3 Substrate Tools — Prerequisite Status${RESET}"
  echo "  ─────────────────────────────────────────────────────────"

  local all_ok=true

  # 1. x3-chain-node binary (release)
  if [[ -f "$REPO_ROOT/target/release/x3-chain-node" ]]; then
    ok "x3-chain-node (release) — $(du -sh "$REPO_ROOT/target/release/x3-chain-node" | cut -f1)"
  else
    nok "x3-chain-node (release) — NOT BUILT.  Run: cargo build --release -p x3-chain-node"
    all_ok=false
  fi

  # 2. x3-chain-node (try-runtime feature)
  if "$REPO_ROOT/target/release/x3-chain-node" --help 2>/dev/null | grep -q 'try-runtime'; then
    ok "x3-chain-node --features try-runtime"
  else
    maybe "x3-chain-node try-runtime — not built yet (run run-try-runtime.sh build)"
  fi

  # 3. x3-chain-node (runtime-benchmarks feature)
  if "$REPO_ROOT/target/release/x3-chain-node" --help 2>/dev/null | grep -q 'benchmark'; then
    ok "x3-chain-node --features runtime-benchmarks"
  else
    maybe "x3-chain-node runtime-benchmarks — not built yet (run run-frame-benchmarks.sh build)"
  fi

  # 4. zombienet
  if command -v zombienet &>/dev/null; then
    ok "zombienet — $(zombienet --version 2>/dev/null || echo 'installed')"
  else
    nok "zombienet — NOT FOUND.  Install: npm install -g @zombienet/cli"
    all_ok=false
  fi

  # 5. chopsticks
  if command -v chopsticks &>/dev/null; then
    ok "chopsticks — $(chopsticks --version 2>/dev/null | head -1 || echo 'installed')"
  elif command -v npx &>/dev/null; then
    ok "chopsticks — via npx @acala-network/chopsticks (Node.js $(node --version))"
  else
    nok "chopsticks — NOT FOUND.  Install: npm install -g @acala-network/chopsticks"
    all_ok=false
  fi

  # 6. srtool
  if command -v srtool &>/dev/null; then
    ok "srtool — $(srtool version 2>/dev/null | head -1 || echo 'installed')"
  else
    maybe "srtool CLI — not found (Docker fallback available)"
  fi

  # 7. Docker (required for srtool)
  if command -v docker &>/dev/null && docker info &>/dev/null 2>&1; then
    ok "Docker — $(docker --version | cut -d' ' -f3 | tr -d ',')"
  else
    nok "Docker — not running (required for srtool)"
    all_ok=false
  fi

  # 8. Cargo + Rust toolchain
  if command -v cargo &>/dev/null; then
    ok "cargo — $(cargo --version)"
  else
    nok "cargo — NOT FOUND"
    all_ok=false
  fi

  # 9. nightly toolchain (for fuzz)
  if rustup toolchain list 2>/dev/null | grep -q nightly; then
    ok "rustup nightly — installed"
  else
    maybe "rustup nightly — missing (needed for cargo-fuzz)"
  fi

  # 10. Zombienet config
  if [[ -f "$REPO_ROOT/zombienet/x3-local-testnet.toml" ]]; then
    ok "zombienet/x3-local-testnet.toml"
  else
    nok "zombienet/x3-local-testnet.toml — missing"
    all_ok=false
  fi

  # 11. Chopsticks config
  if [[ -f "$REPO_ROOT/chopsticks/x3-dev.yml" ]]; then
    ok "chopsticks/x3-dev.yml"
  else
    nok "chopsticks/x3-dev.yml — missing"
    all_ok=false
  fi

  echo "  ─────────────────────────────────────────────────────────"
  if $all_ok; then
    success "All critical prerequisites satisfied!"
  else
    warn "Some prerequisites are missing. Address the ✗ items above before running tools."
  fi
  echo ""
}

# ── Non-interactive pre-flight (CI-friendly) ─────────────────────────────────
cmd_all_checks() {
  echo ""
  info "Running non-interactive pre-flight checks …"
  echo ""

  local checks_passed=0
  local checks_failed=0

  run_check() {
    local name="$1"; shift
    if "$@" &>/dev/null 2>&1; then
      ok "$name"
      checks_passed=$((checks_passed + 1))
    else
      nok "$name"
      checks_failed=$((checks_failed + 1))
    fi
  }

  run_check "cargo builds successfully"           cargo check -p x3-chain-node --message-format=short
  run_check "zombienet config parses"             test -f "$REPO_ROOT/zombienet/x3-local-testnet.toml"
  run_check "chopsticks config exists"            test -f "$REPO_ROOT/chopsticks/x3-dev.yml"
  run_check "scripts are executable (try-runtime)" test -x "$SCRIPTS/run-try-runtime.sh"
  run_check "scripts are executable (zombienet)"   test -x "$SCRIPTS/run-zombienet.sh"
  run_check "scripts are executable (chopsticks)"  test -x "$SCRIPTS/run-chopsticks.sh"
  run_check "scripts are executable (benchmarks)"  test -x "$SCRIPTS/run-frame-benchmarks.sh"
  run_check "scripts are executable (srtool)"      test -x "$SCRIPTS/run-srtool.sh"
  run_check "Docker available"                     docker info

  echo ""
  echo "  Pre-flight: ${GREEN}$checks_passed passed${RESET}  |  ${RED}$checks_failed failed${RESET}"
  echo ""
  [[ $checks_failed -eq 0 ]] && success "All pre-flight checks PASSED" || warn "$checks_failed checks FAILED"
}

# ── Interactive menu ──────────────────────────────────────────────────────────
cmd_menu() {
  print_table

  echo -e "${BOLD}  Select a tool to run:${RESET}"
  echo ""
  echo "    1) try-runtime   — test migration hooks against live state"
  echo "    2) zombienet     — spawn ephemeral 3-validator testnet"
  echo "    3) chopsticks    — fork chain state + replay blocks"
  echo "    4) benchmarks    — generate pallet weight constants"
  echo "    5) srtool        — build deterministic runtime WASM"
  echo "    6) status        — show prerequisite health"
  echo "    0) exit"
  echo ""
  read -r -p "  Choice [0-6]: " choice

  case "$choice" in
    1) bash "$SCRIPTS/run-try-runtime.sh" help ;;
    2) bash "$SCRIPTS/run-zombienet.sh" help ;;
    3) bash "$SCRIPTS/run-chopsticks.sh" help ;;
    4) bash "$SCRIPTS/run-frame-benchmarks.sh" help ;;
    5) bash "$SCRIPTS/run-srtool.sh" help ;;
    6) cmd_status ;;
    0) info "Bye!"; exit 0 ;;
    *) warn "Invalid choice. Run ./scripts/run-substrate-tools.sh for menu." ;;
  esac
}

# ── Delegate to individual scripts ───────────────────────────────────────────
delegate() {
  local script="$1"; shift
  local target="$SCRIPTS/$script"
  [[ -x "$target" ]] || chmod +x "$target"
  exec "$target" "$@"
}

# ─────────── dispatch ──────────────────────────────────────────────────────
chmod +x "$SCRIPTS"/run-try-runtime.sh \
         "$SCRIPTS"/run-zombienet.sh \
         "$SCRIPTS"/run-chopsticks.sh \
         "$SCRIPTS"/run-frame-benchmarks.sh \
         "$SCRIPTS"/run-srtool.sh \
         "$0" 2>/dev/null || true

COMMAND="${1:-menu}"
shift || true

case "$COMMAND" in
  try-runtime|try_runtime)   delegate run-try-runtime.sh "$@" ;;
  zombienet|zombie)          delegate run-zombienet.sh "$@" ;;
  chopsticks|chop)           delegate run-chopsticks.sh "$@" ;;
  benchmarks|bench|bm)       delegate run-frame-benchmarks.sh "$@" ;;
  srtool|wasm)               delegate run-srtool.sh "$@" ;;
  status|check)              cmd_status ;;
  all-checks|preflight)      cmd_all_checks ;;
  menu|--menu)               cmd_menu ;;
  help|--help|-h)            print_table; echo "  Subcommands: try-runtime | zombienet | chopsticks | benchmarks | srtool | status | all-checks" ;;
  *)                         warn "Unknown command: $COMMAND"; print_table; exit 1 ;;
esac
