#!/usr/bin/env bash

# X3 Chain Node Startup Health Check
# Validates prerequisites and environment before launching dev/prod node

set -euo pipefail

# ============ COLORS & FORMATTING ============
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============ COUNTERS ============
PASS=0
FAIL=0
WARN=0

# ============ CONFIGURATION ============
MODE="${MODE:-dev}"
STRICT="${STRICT:-false}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# ============ HELPER FUNCTIONS ============

pass() {
  local msg="$1"
  echo -e "${GREEN}✓${NC} $msg"
  ((PASS++)) || true
}

warn() {
  local msg="$1"
  echo -e "${YELLOW}⚠${NC} $msg"
  ((WARN++)) || true
}

fail() {
  local msg="$1"
  echo -e "${RED}✗${NC} $msg"
  ((FAIL++)) || true
}

section() {
  local title="$1"
  echo ""
  echo -e "${BLUE}═══ $title ═══${NC}"
  echo ""
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --mode)
        MODE="$2"
        shift 2
        ;;
      --strict)
        STRICT="true"
        shift
        ;;
      --help)
        print_help
        exit 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        print_help
        exit 1
        ;;
    esac
  done

  # Validate mode
  if [[ ! "$MODE" =~ ^(dev|prod)$ ]]; then
    fail "Invalid mode '$MODE'. Use 'dev' or 'prod'."
    exit 1
  fi
}

print_help() {
  cat << EOF
X3 Chain Node Health Check

Usage: bash scripts/x3_node_healthcheck.sh [OPTIONS]

Options:
  --mode dev|prod          Mode to check (default: dev)
  --strict                 Exit with code 2 if warnings present
  --help                   Show this help message

Examples:
  bash scripts/x3_node_healthcheck.sh
  bash scripts/x3_node_healthcheck.sh --mode prod
  bash scripts/x3_node_healthcheck.sh --mode dev --strict

Exit Codes:
  0  - All checks passed
  1  - One or more failures
  2  - Warnings present and --strict enabled
EOF
}

check_command() {
  local cmd="$1"
  local required="${2:-required}"

  if command -v "$cmd" >/dev/null 2>&1; then
    pass "Found command: $cmd"
    return 0
  fi

  if [[ "$required" == "required" ]]; then
    fail "Missing required command: $cmd"
    return 1
  else
    warn "Missing optional command: $cmd (has fallback)"
    return 0
  fi
}

check_file() {
  local filepath="$1"
  local description="$2"
  local required="${3:-required}"

  if [[ -f "$filepath" ]]; then
    pass "Found: $description"
    return 0
  fi

  if [[ "$required" == "required" ]]; then
    fail "Missing: $description at $filepath"
    return 1
  else
    warn "Missing (optional): $description at $filepath"
    return 0
  fi
}

check_port_free() {
  local port="$1"
  local label="$2"
  local pid=""

  # Try lsof first (most portable)
  if command -v lsof >/dev/null 2>&1; then
    pid=$(lsof -ti :"$port" 2>/dev/null || true)
  # Fallback to netstat
  elif command -v netstat >/dev/null 2>&1; then
    pid=$(netstat -tulpn 2>/dev/null | grep ":$port " | awk '{print $NF}' | cut -d'/' -f1 || true)
  # Fallback to ss (modern Linux)
  elif command -v ss >/dev/null 2>&1; then
    pid=$(ss -tulpn 2>/dev/null | grep ":$port " | awk '{print $NF}' | cut -d'"' -f2 || true)
  fi

  if [[ -z "$pid" ]]; then
    pass "Port $port ($label) is free"
    return 0
  else
    # Get process name if possible
    local pname=""
    if [[ -n "$pid" && "$pid" =~ ^[0-9]+$ ]]; then
      pname=$(ps -p "$pid" -o comm= 2>/dev/null || echo "unknown")
    fi
    warn "Port $port ($label) is occupied by PID $pid ($pname)"
    return 0
  fi
}

# ============ MAIN CHECKS ============

main() {
  parse_args "$@"

  echo "╔════════════════════════════════════════════════╗"
  echo "║   X3 CHAIN NODE STARTUP HEALTH CHECK          ║"
  echo "╚════════════════════════════════════════════════╝"
  echo ""
  echo "Mode: $MODE"
  echo "Strict: $STRICT"
  echo "Working directory: $PROJECT_ROOT"
  echo ""

  # ========== TASK 2: PREREQUISITE CHECKS ==========
  section "REQUIRED COMMANDS"

  check_command "cargo" "required"
  check_command "bash" "required"
  check_command "curl" "required"

  section "OPTIONAL TOOLS (helpful for diagnostics)"

  check_command "lsof" "optional"
  check_command "netstat" "optional"
  check_command "ss" "optional"
  check_command "nc" "optional"

  # ========== TASK 2: BINARY CHECK ==========
  section "NODE BINARY"

  if [[ -f "$PROJECT_ROOT/target/release/x3-chain-node" ]]; then
    pass "Binary found at target/release/x3-chain-node"
  else
    warn "Binary not found at target/release/x3-chain-node"
    echo "  Recommendation: Run 'cargo build --release' to build the node binary"
  fi

  # ========== TASK 2: PRODUCTION MODE CHECKS ==========
  if [[ "$MODE" == "prod" ]]; then
    section "PRODUCTION MODE REQUIREMENTS"

    # Check NODE_NAME environment variable
    if [[ -n "${NODE_NAME:-}" ]]; then
      pass "NODE_NAME is set: $NODE_NAME"
    else
      fail "NODE_NAME environment variable is required for production mode"
    fi

    # Check that script is not running as root
    if [[ $EUID -ne 0 ]]; then
      pass "Not running as root (production requirement)"
    else
      fail "Node must not run as root in production mode"
    fi
  fi

  # ========== TASK 3: APP ENVIRONMENT FILES ==========
  section "APP ENVIRONMENT CONFIGURATION"

  local apps=("explorer" "wallet" "dex" "x3-intelligence")
  local missing_envs=0

  for app in "${apps[@]}"; do
    local env_file="$PROJECT_ROOT/apps/$app/.env.local"
    if [[ -f "$env_file" ]]; then
      pass "Found $app/.env.local"
    else
      warn "Missing $app/.env.local"
      ((missing_envs++)) || true
    fi
  done

  if [[ $missing_envs -gt 0 ]]; then
    echo ""
    echo "  Recommendation: Run './setup-app-env.sh' to generate missing .env.local files"
  fi

  # ========== TASK 4: PORT AVAILABILITY ==========
  section "PORT AVAILABILITY"

  if [[ "$MODE" == "dev" ]]; then
    # Dev ports from run-dev-node.sh
    check_port_free "${RPC_PORT:-9944}" "RPC"
    check_port_free "${WS_PORT:-9945}" "WebSocket"
    check_port_free "${P2P_PORT:-30333}" "P2P"
    check_port_free "${PROMETHEUS_PORT:-9615}" "Prometheus"
  else
    # Prod typically uses same ports
    check_port_free "${RPC_PORT:-9944}" "RPC"
    check_port_free "${P2P_PORT:-30333}" "P2P"
    check_port_free "${PROMETHEUS_PORT:-9615}" "Prometheus"
  fi

  # ========== TASK 4: LIVE HEALTH PROBE (if node already running) ==========
  section "LIVE NODE HEALTH (if running)"

  if lsof -ti:9944 >/dev/null 2>/dev/null; then
    if curl -s http://127.0.0.1:9944/health >/dev/null 2>&1; then
      pass "Node health endpoint responding"
    else
      warn "Node running on port 9944 but /health endpoint not responding"
    fi
  else
    echo "Node not currently running on port 9944 (expected for preflight check)"
  fi

  # ========== SUMMARY ==========
  section "SUMMARY"

  echo "  ✓ Passed: $PASS"
  echo "  ⚠ Warnings: $WARN"
  echo "  ✗ Failed: $FAIL"
  echo ""

  # ========== EXIT LOGIC ==========
  if [[ $FAIL -gt 0 ]]; then
    echo -e "${RED}Health check FAILED${NC}"
    return 1
  fi

  if [[ "$STRICT" == "true" && $WARN -gt 0 ]]; then
    echo -e "${YELLOW}Health check passed with warnings (strict mode enabled)${NC}"
    return 2
  fi

  echo -e "${GREEN}Health check PASSED${NC}"
  return 0
}

# Run main function
main "$@"
