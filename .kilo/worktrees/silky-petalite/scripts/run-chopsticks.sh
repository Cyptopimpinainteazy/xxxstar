#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# run-chopsticks.sh — Fork, replay, and mutate X3 chain state with Chopsticks
#
# Chopsticks forks the X3 node at a block (live or snapshot) into an in-process
# SQLite store and serves a full JSON-RPC endpoint locally.  You can then:
#   • replay extrinsics                          (dev_newBlock + sendExtrinsic)
#   • mutate storage slots without consensus     (dev_setStorage)
#   • test runtime upgrades                      (--wasm-override)
#   • run polkadot.js / subxt scripts offline    (against port 8000)
#
# Modes:
#   ./scripts/run-chopsticks.sh fork       — fork live node, start RPC at :8000
#   ./scripts/run-chopsticks.sh replay     — replay a block by number / hash
#   ./scripts/run-chopsticks.sh mutate     — inject storage overrides example
#   ./scripts/run-chopsticks.sh upgrade    — fork + swap runtime WASM
#   ./scripts/run-chopsticks.sh help       — show this help
#
# Dependencies: npx / chopsticks, running x3-chain-node at ws://127.0.0.1:9944
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG="$REPO_ROOT/chopsticks/x3-dev.yml"
DB_DIR="$REPO_ROOT/.chopsticks-db"
LOG_FILE="$REPO_ROOT/chopsticks.log"
WASM="$REPO_ROOT/target/release/wbuild/x3-chain-runtime/x3_chain_runtime.compact.compressed.wasm"

CHOPSTICKS_BIN="${CHOPSTICKS_BIN:-$(which chopsticks 2>/dev/null || echo 'npx @acala-network/chopsticks@latest')}"
ENDPOINT="${CHOPSTICKS_ENDPOINT:-ws://127.0.0.1:9944}"
PORT="${CHOPSTICKS_PORT:-8000}"

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${CYAN}[chopsticks]${NC} $*"; }
success() { echo -e "${GREEN}[chopsticks]  ✓${NC} $*"; }
warn()    { echo -e "${YELLOW}[chopsticks] ⚠${NC} $*"; }
die()     { echo -e "${RED}[chopsticks] ✗ ERROR:${NC} $*" >&2; exit 1; }

print_help() {
  cat <<EOF

  X3 Chopsticks — fork/replay/mutate chain state locally

  USAGE:
    ./scripts/run-chopsticks.sh [COMMAND] [OPTIONS]

  COMMANDS:
    fork           Fork live node at HEAD. Serves RPC at localhost:$PORT
    replay BLOCK   Fork at BLOCK (number or 0xhash) and replay one new block
    mutate         Fork + inject storage example (shows dev_setStorage call)
    upgrade        Fork + swap in new WASM build (tests runtime upgrade offline)
    help           Show this message

  ENVIRONMENT VARIABLES:
    CHOPSTICKS_ENDPOINT   Source node WS (default: $ENDPOINT)
    CHOPSTICKS_PORT       Local RPC port  (default: $PORT)

  EXAMPLES:
    # Reproduce a bug at block 12345
    ./scripts/run-chopsticks.sh replay 12345

    # Test new WASM without a live upgrade proposal
    cargo build --release -p x3-chain-node
    ./scripts/run-chopsticks.sh upgrade

  Once running, connect any Substrate client to ws://127.0.0.1:$PORT
    polkadot-js-api  rpc.chain.getBlock
    subxt            Endpoint::Url("ws://127.0.0.1:$PORT")

EOF
}

check_deps() {
  # Validate chopsticks is reachable
  if [[ "$CHOPSTICKS_BIN" == *"npx"* ]]; then
    command -v npx &>/dev/null || die "npx not found. Install Node.js 18+."
  else
    command -v "$CHOPSTICKS_BIN" &>/dev/null || die "chopsticks not found at $CHOPSTICKS_BIN"
  fi
}

cmd_fork() {
  check_deps
  mkdir -p "$DB_DIR"

  info "Forking X3 chain from $ENDPOINT at HEAD …"
  info "Local RPC → ws://127.0.0.1:$PORT"
  info "SQLite DB → $DB_DIR/x3-dev.db"
  info "Press Ctrl-C to stop"
  echo ""
  info "Connect polkadot.js at: https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:$PORT"
  echo ""

  $CHOPSTICKS_BIN \
    --config "$CONFIG" \
    --endpoint "$ENDPOINT" \
    --port "$PORT" \
    --db "$DB_DIR/x3-dev.db" \
    2>&1 | tee "$LOG_FILE"
}

cmd_replay() {
  local block="${1:-latest}"
  check_deps
  mkdir -p "$DB_DIR"

  info "Forking at block $block and replaying one new block …"

  $CHOPSTICKS_BIN \
    --config "$CONFIG" \
    --endpoint "$ENDPOINT" \
    --port "$PORT" \
    --db "$DB_DIR/x3-replay-$block.db" \
    --block "$block" \
    2>&1 | tee "$LOG_FILE" &

  local chopsticks_pid=$!
  sleep 5  # wait for chopsticks to start

  info "Chopsticks running (pid $chopsticks_pid). Triggering dev_newBlock …"
  curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"id":1,"jsonrpc":"2.0","method":"dev_newBlock","params":[{"count":1}]}' \
    "http://127.0.0.1:$PORT" | python3 -m json.tool || true

  echo ""
  success "Block replay triggered. Chopsticks still running at :$PORT"
  info "Press Ctrl-C or kill $chopsticks_pid to stop"
  wait "$chopsticks_pid" 2>/dev/null || true
}

cmd_mutate() {
  check_deps
  mkdir -p "$DB_DIR"

  info "Starting fork + storage mutation example …"
  info "This shows how to use dev_setStorage to inject test state"
  echo ""

  $CHOPSTICKS_BIN \
    --config "$CONFIG" \
    --endpoint "$ENDPOINT" \
    --port "$PORT" \
    --db "$DB_DIR/x3-mutate.db" \
    2>&1 | tee "$LOG_FILE" &

  local chopsticks_pid=$!
  sleep 5

  info "Injecting storage override via dev_setStorage …"
  info "(Example: sets Alice's free balance to 9999 tokens)"
  curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{
      "id":1,"jsonrpc":"2.0","method":"dev_setStorage",
      "params":[{
        "System": {
          "Account": [
            ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
             {"nonce":0,"consumers":0,"providers":1,"sufficients":0,
              "data":{"free":9999000000000000,"reserved":0,"miscFrozen":0,"feeFrozen":0}}]
          ]
        }
      }]
    }' \
    "http://127.0.0.1:$PORT" | python3 -m json.tool || true

  echo ""
  info "Producing one block to commit the storage change …"
  curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"id":2,"jsonrpc":"2.0","method":"dev_newBlock","params":[{"count":1}]}' \
    "http://127.0.0.1:$PORT" | python3 -m json.tool || true

  echo ""
  success "Storage mutated. Verify with: system_account(Alice) → free: 9999 tokens"
  info "Chopsticks still running at :$PORT — Ctrl-C to stop"
  wait "$chopsticks_pid" 2>/dev/null || true
}

cmd_upgrade() {
  check_deps
  [[ -f "$WASM" ]] || die "Compressed WASM not found at $WASM. Run: cargo build --release -p x3-chain-node"
  mkdir -p "$DB_DIR"

  info "Forking with WASM override to test runtime upgrade …"
  info "WASM → $WASM"
  info "This tests migration hooks without submitting a governance proposal"

  $CHOPSTICKS_BIN \
    --config "$CONFIG" \
    --endpoint "$ENDPOINT" \
    --port "$PORT" \
    --db "$DB_DIR/x3-upgrade.db" \
    --wasm-override "$WASM" \
    2>&1 | tee "$LOG_FILE"
}

# ─────────────────────────────────────────────────────────────────────────────
# test — CI-friendly non-interactive validation suite (no live node required)
#   1. Chopsticks binary / npx reachable
#   2. Config YAML exists and has required fields
#   3. WASM binary exists (for upgrade mode)
#   4. DB dir is writable
#   5. curl available (used by mutate/replay)
# ─────────────────────────────────────────────────────────────────────────────
cmd_test() {
  local PASS=0 FAIL=0

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo "  Chopsticks — CI Test Suite"
  echo "══════════════════════════════════════════════════════"
  echo ""

  _check() {
    local label="$1"; shift
    echo -n "  Checking: $label … "
    if "$@" &>/dev/null 2>&1; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC}"
      FAIL=$((FAIL + 1))
    fi
  }

  _check_grep() {
    local label="$1"; local file="$2"; local pattern="$3"
    echo -n "  Checking: $label … "
    if grep -qE "$pattern" "$file" 2>/dev/null; then
      echo -e "${GREEN}✓ PASS${NC}"
      PASS=$((PASS + 1))
    else
      echo -e "${RED}✗ FAIL${NC} (pattern '$pattern' not found in $file)"
      FAIL=$((FAIL + 1))
    fi
  }

  # 1. Chopsticks binary / npx reachable
  if [[ "$CHOPSTICKS_BIN" == *"npx"* ]]; then
    _check "npx available" command -v npx
  else
    _check "chopsticks binary found" command -v "$CHOPSTICKS_BIN"
  fi

  # 2. Config YAML exists with expected content
  _check "Config YAML exists" test -f "$CONFIG"
  _check_grep "Config has endpoint"    "$CONFIG" 'endpoint'
  _check_grep "Config has port"        "$CONFIG" 'port'
  _check_grep "Config has 9944/local"  "$CONFIG" '9944|localhost|127\.0\.0\.1'

  # 3. WASM binary exists (required for upgrade mode)
  _check "Compressed WASM exists (upgrade mode)" test -f "$WASM"

  # 4. DB dir writeable
  mkdir -p "$DB_DIR"
  _check "DB directory writable" test -w "$DB_DIR"

  # 5. curl available (used for RPC calls in mutate/replay)
  _check "curl available (for RPC calls)" command -v curl

  # 6. python3 available (used for JSON pretty-print)
  _check "python3 available (JSON formatting)" command -v python3

  echo ""
  echo "══════════════════════════════════════════════════════"
  echo -e "  Results: ${GREEN}✓ $PASS PASSED${NC}  |  ${RED}✗ $FAIL FAILED${NC}"
  echo "══════════════════════════════════════════════════════"
  echo ""

  [[ $FAIL -eq 0 ]] && success "All Chopsticks CI tests passed." || die "$FAIL test(s) failed."
}

# ─────────── dispatch ──────────────────────────────────────────────────────
COMMAND="${1:-help}"
case "$COMMAND" in
  fork)           cmd_fork ;;
  replay)         cmd_replay "${2:-latest}" ;;
  mutate)         cmd_mutate ;;
  upgrade)        cmd_upgrade ;;
  test)           cmd_test ;;
  help|--help|-h) print_help ;;
  *)              warn "Unknown command: $COMMAND"; print_help; exit 1 ;;
esac
