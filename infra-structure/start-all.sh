#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════
#  X3 Chain — Infra-Structure: Bare Metal Start All
#  Starts every service directly on the host (no Docker)
#
#  Services:
#    1. Chain DB API        (port 7070)  — 62,500+ blockchain registry
#    2. Blockchain TPS      (port 3010)  — TPS benchmarking & demo runner
#    3. GPU Validator        (Python)     — Cross-chain GPU validation
#    4. Dashboard (Vite)     (port 5174)  — Inferstructor UI
#
#  Usage:
#    ./start-all.sh           — start everything
#    ./start-all.sh --no-ui   — services only (headless)
#    ./start-all.sh --seed    — re-seed chain DB then start
# ═══════════════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

LOG_DIR="$SCRIPT_DIR/logs"
PID_FILE="$SCRIPT_DIR/.infra.pids"
mkdir -p "$LOG_DIR"

# ── Helpers ───────────────────────────────────────────────────────────
log()  { echo -e "${BLUE}[infra]${NC} $*"; }
ok()   { echo -e "${GREEN}  ✓${NC} $*"; }
warn() { echo -e "${YELLOW}  ⚠${NC} $*"; }
err()  { echo -e "${RED}  ✗${NC} $*"; }

save_pid() {
  echo "$1:$2" >> "$PID_FILE"
}

check_port() {
  local port=$1 name=$2
  if lsof -i :"$port" -sTCP:LISTEN -t &>/dev/null; then
    warn "Port $port already in use — $name may already be running"
    return 1
  fi
  return 0
}

check_cmd() {
  if ! command -v "$1" &>/dev/null; then
    err "$1 not found — please install it"
    return 1
  fi
}

# ── Parse args ────────────────────────────────────────────────────────
NO_UI=false
SEED=false
for arg in "$@"; do
  case "$arg" in
    --no-ui)  NO_UI=true ;;
    --seed)   SEED=true ;;
    --help|-h)
      echo "Usage: $0 [--no-ui] [--seed] [--help]"
      echo "  --no-ui   Start services only (no dashboard)"
      echo "  --seed    Re-seed the chain database before starting"
      exit 0
      ;;
  esac
done

# ── Banner ────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${CYAN}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║     X3 Chain — Infra-Structure (Bare Metal)  ║${NC}"
echo -e "${BOLD}${CYAN}╚═══════════════════════════════════════════════════╝${NC}"
echo ""

# ── Prerequisites ─────────────────────────────────────────────────────
log "Checking prerequisites..."
check_cmd node
check_cmd npm
check_cmd python3
ok "node $(node --version), npm $(npm --version), python3 $(python3 --version 2>&1 | awk '{print $2}')"

# Clear old PID file
> "$PID_FILE"

# ── 0. Seed chain DB if needed ────────────────────────────────────────
DB_FILE="$SCRIPT_DIR/db/chains.db"
if [ "$SEED" = true ] || [ ! -f "$DB_FILE" ]; then
  log "Seeding chain database (62,500+ blockchains)..."
  python3 "$SCRIPT_DIR/db/seed/seed_chains.py" --db "$DB_FILE" --count 62000
  ok "Chain DB seeded at $DB_FILE"
else
  CHAIN_COUNT=$(python3 -c "import sqlite3; c=sqlite3.connect('$DB_FILE'); print(c.execute('SELECT COUNT(*) FROM chains').fetchone()[0]); c.close()" 2>/dev/null || echo "0")
  ok "Chain DB exists: $CHAIN_COUNT chains"
fi

# ── 1. Chain DB API (port 7070) ───────────────────────────────────────
log "Starting Chain DB API on port 7070..."
if check_port 7070 "Chain DB API"; then
  cd "$SCRIPT_DIR/services/chain-db"
  if [ ! -d node_modules ]; then
    log "  Installing chain-db dependencies..."
    npm install --silent 2>&1 | tail -1
  fi
  CHAIN_DB_PATH="$DB_FILE" CHAIN_DB_PORT=7070 \
    node server.js > "$LOG_DIR/chain-db.log" 2>&1 &
  save_pid "chain-db" "$!"
  sleep 1
  ok "Chain DB API (PID $!) — http://localhost:7070"
  cd "$SCRIPT_DIR"
fi

# ── 2. Blockchain TPS Service (port 3010) ─────────────────────────────
log "Starting Blockchain TPS service on port 3010..."
if check_port 3010 "Blockchain TPS"; then
  cd "$SCRIPT_DIR/services/blockchain-tps"
  if [ ! -d node_modules ]; then
    log "  Installing blockchain-tps dependencies..."
    npm install --silent 2>&1 | tail -1
  fi
  PORT=3010 node server.js > "$LOG_DIR/blockchain-tps.log" 2>&1 &
  save_pid "blockchain-tps" "$!"
  sleep 1
  ok "Blockchain TPS (PID $!) — http://localhost:3010"
  cd "$SCRIPT_DIR"
fi

# ── 3. GPU Validator (Python) ─────────────────────────────────────────
log "Setting up GPU Validator..."
cd "$SCRIPT_DIR/validator"
if [ -d .venv ]; then
  ok "Validator virtualenv exists"
else
  warn "No virtualenv found — run: cd validator && python3 -m venv .venv && pip install -e ."
fi
cd "$SCRIPT_DIR"

# ── 4. Dashboard (Vite dev server, port 5174) ─────────────────────────
if [ "$NO_UI" = false ]; then
  log "Starting Inferstructor Dashboard on port 5174..."
  if check_port 5174 "Dashboard"; then
    cd "$SCRIPT_DIR/dashboard"
    if [ ! -d node_modules ]; then
      log "  Installing dashboard dependencies..."
      npm install --silent 2>&1 | tail -1
    fi
    npx vite --port 5174 > "$LOG_DIR/dashboard.log" 2>&1 &
    save_pid "dashboard" "$!"
    sleep 2
    ok "Dashboard (PID $!) — http://localhost:5174"
    cd "$SCRIPT_DIR"
  fi
fi

# ── 5. RPC Crawler Daemon (background) ────────────────────────────────
log "Starting RPC Crawler Daemon (background)..."
CRAWLER_DIR="$SCRIPT_DIR/services/rpc-crawler"
if [ -f "$CRAWLER_DIR/crawler_daemon.py" ]; then
  CHAIN_DB_PATH="$DB_FILE" \
  CRAWLER_STATE_FILE="$CRAWLER_DIR/crawler_state.json" \
  CRAWLER_LOG_FILE="$LOG_DIR/rpc-crawler.log" \
    python3 "$CRAWLER_DIR/crawler_daemon.py" \
      --interval 15 --aggressive \
      > "$LOG_DIR/rpc-crawler.log" 2>&1 &
  save_pid "rpc-crawler" "$!"
  ok "RPC Crawler (PID $!) — logs: $LOG_DIR/rpc-crawler.log"
else
  warn "RPC Crawler not found at $CRAWLER_DIR/crawler_daemon.py"
fi
cd "$SCRIPT_DIR"

# ── 6. Faucet Auto-Claimer (background) ───────────────────────────────
log "Starting Faucet Auto-Claimer (background)..."
if [ -f "$CRAWLER_DIR/faucet_claimer.py" ]; then
  CHAIN_DB_PATH="$DB_FILE" \
    python3 "$CRAWLER_DIR/faucet_claimer.py" \
      --interval 60 --seed-wallets \
      > "$LOG_DIR/faucet-claimer.log" 2>&1 &
  save_pid "faucet-claimer" "$!"
  ok "Faucet Claimer (PID $!) — logs: $LOG_DIR/faucet-claimer.log"
else
  warn "Faucet Claimer not found at $CRAWLER_DIR/faucet_claimer.py"
fi

# ── Summary ───────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  All services started!${NC}"
echo ""
echo -e "  ${CYAN}Chain DB API${NC}       http://localhost:${BOLD}7070${NC}"
echo -e "  ${CYAN}Blockchain TPS${NC}     http://localhost:${BOLD}3010${NC}"
if [ "$NO_UI" = false ]; then
echo -e "  ${CYAN}Dashboard${NC}          http://localhost:${BOLD}5174${NC}"
fi
echo -e "  ${CYAN}RPC Crawler${NC}        background (15min cycles, aggressive)"
echo -e "  ${CYAN}Faucet Claimer${NC}     background (60min cycles, auto-seed)"
echo ""
echo -e "  Logs:  ${YELLOW}$LOG_DIR/${NC}"
echo -e "  PIDs:  ${YELLOW}$PID_FILE${NC}"
echo -e "  Stop:  ${BOLD}./stop-all.sh${NC}"
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════════${NC}"
echo ""
