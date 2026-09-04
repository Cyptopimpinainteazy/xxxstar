#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════
#  X3 Chain — Infra-Structure: Install Dependencies
#  One-shot setup for all bare-metal services
# ═══════════════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

log()  { echo -e "${BLUE}[setup]${NC} $*"; }
ok()   { echo -e "${GREEN}  ✓${NC} $*"; }
warn() { echo -e "${YELLOW}  ⚠${NC} $*"; }
err()  { echo -e "${RED}  ✗${NC} $*"; }

echo ""
echo -e "${BOLD}${CYAN}╔═══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║  X3 Chain — Infra-Structure: Install All     ║${NC}"
echo -e "${BOLD}${CYAN}╚═══════════════════════════════════════════════════╝${NC}"
echo ""

# ── 1. Chain DB API deps ──────────────────────────────────────────────
log "Installing Chain DB API dependencies..."
cd "$SCRIPT_DIR/services/chain-db"
npm install --silent
ok "chain-db (better-sqlite3, express, cors)"
cd "$SCRIPT_DIR"

# ── 2. Blockchain TPS deps ───────────────────────────────────────────
log "Installing Blockchain TPS dependencies..."
cd "$SCRIPT_DIR/services/blockchain-tps"
npm install --silent
ok "blockchain-tps (express, socket.io, redis)"
cd "$SCRIPT_DIR"

# ── 3. Dashboard deps ────────────────────────────────────────────────
log "Installing Dashboard dependencies..."
cd "$SCRIPT_DIR/dashboard"
npm install --silent
ok "dashboard (react, vite, recharts, tailwind, lucide)"
cd "$SCRIPT_DIR"

# ── 4. GPU Validator Python setup ─────────────────────────────────────
log "Setting up GPU Validator Python environment..."
cd "$SCRIPT_DIR/validator"
if [ ! -d .venv ]; then
  python3 -m venv .venv
  ok "Created virtualenv at validator/.venv"
else
  ok "Virtualenv already exists"
fi
source .venv/bin/activate
pip install -e . --quiet 2>&1 | tail -1
ok "Installed cross-chain-gpu-validator package"
deactivate
cd "$SCRIPT_DIR"

# ── 5. Seed chain database ───────────────────────────────────────────
DB_FILE="$SCRIPT_DIR/db/chains.db"
if [ ! -f "$DB_FILE" ]; then
  log "Seeding chain database (62,500+ blockchains)..."
  python3 db/seed/seed_chains.py --db "$DB_FILE" --count 62000
  ok "Chain DB seeded"
else
  CHAIN_COUNT=$(python3 -c "import sqlite3; c=sqlite3.connect('$DB_FILE'); print(c.execute('SELECT COUNT(*) FROM chains').fetchone()[0]); c.close()" 2>/dev/null || echo "?")
  ok "Chain DB already exists ($CHAIN_COUNT chains)"
fi

# ── Done ──────────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Setup complete!${NC}"
echo ""
echo -e "  Next steps:"
echo -e "    ${BOLD}./start-all.sh${NC}       — Start all services"
echo -e "    ${BOLD}./status.sh${NC}          — Check service health"
echo -e "    ${BOLD}./stop-all.sh${NC}        — Stop everything"
echo -e "${BOLD}${GREEN}═══════════════════════════════════════════════════${NC}"
echo ""
