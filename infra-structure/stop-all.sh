#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════
#  X3 Chain — Infra-Structure: Bare Metal Stop All
#  Cleanly stops all services started by start-all.sh
# ═══════════════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/.infra.pids"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'

log()  { echo -e "${BLUE}[infra]${NC} $*"; }
ok()   { echo -e "${GREEN}  ✓${NC} $*"; }
warn() { echo -e "${YELLOW}  ⚠${NC} $*"; }

echo ""
echo -e "${BOLD}Stopping X3 Chain Infra-Structure services...${NC}"
echo ""

stopped=0

if [ -f "$PID_FILE" ]; then
  while IFS=: read -r name pid; do
    if [ -z "$pid" ]; then continue; fi
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null && ok "Stopped $name (PID $pid)" || warn "Failed to stop $name (PID $pid)"
      ((stopped++))
    else
      warn "$name (PID $pid) already stopped"
    fi
  done < "$PID_FILE"
  rm -f "$PID_FILE"
else
  warn "No PID file found — trying port-based cleanup"
fi

# Fallback: kill by well-known ports
for port_info in "7070:chain-db" "3010:blockchain-tps" "5174:dashboard"; do
  port="${port_info%%:*}"
  name="${port_info##*:}"
  pid=$(lsof -i :"$port" -sTCP:LISTEN -t 2>/dev/null || true)
  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null && ok "Stopped $name on port $port (PID $pid)" || true
    ((stopped++))
  fi
done

# Kill any orphaned crawler processes
crawler_pids=$(pgrep -f "crawler_daemon.py" 2>/dev/null || true)
if [ -n "$crawler_pids" ]; then
  echo "$crawler_pids" | xargs kill 2>/dev/null && ok "Stopped RPC crawler daemon" || true
  ((stopped++))
fi

# Kill any orphaned faucet claimer processes
claimer_pids=$(pgrep -f "faucet_claimer.py" 2>/dev/null || true)
if [ -n "$claimer_pids" ]; then
  echo "$claimer_pids" | xargs kill 2>/dev/null && ok "Stopped Faucet claimer" || true
  ((stopped++))
fi

if [ "$stopped" -eq 0 ]; then
  log "No running services found"
else
  echo ""
  ok "Stopped $stopped service(s)"
fi
echo ""
