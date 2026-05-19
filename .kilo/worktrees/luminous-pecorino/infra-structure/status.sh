#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════════
#  X3 Chain — Infra-Structure: Status Check
#  Shows the status of all bare-metal services
# ═══════════════════════════════════════════════════════════════════════
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/.infra.pids"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

echo ""
echo -e "${BOLD}${CYAN}X3 Chain — Infra-Structure Status${NC}"
echo -e "${CYAN}──────────────────────────────────────${NC}"
echo ""

check_service() {
  local name=$1 port=$2 url=$3
  local pid=""
  pid=$(lsof -i :"$port" -sTCP:LISTEN -t 2>/dev/null || true)

  if [ -n "$pid" ]; then
    # Try HTTP health check
    local status_code=""
    status_code=$(curl -s -o /dev/null -w "%{http_code}" --connect-timeout 2 "$url" 2>/dev/null || echo "000")
    if [ "$status_code" = "200" ]; then
      echo -e "  ${GREEN}●${NC} ${BOLD}$name${NC}"
      echo -e "    Port: $port  PID: $pid  HTTP: ${GREEN}$status_code${NC}  URL: $url"
    else
      echo -e "  ${YELLOW}●${NC} ${BOLD}$name${NC}"
      echo -e "    Port: $port  PID: $pid  HTTP: ${YELLOW}$status_code${NC}  URL: $url"
    fi
  else
    echo -e "  ${RED}●${NC} ${BOLD}$name${NC}"
    echo -e "    Port: $port  ${RED}NOT RUNNING${NC}"
  fi
}

check_service "Chain DB API"       7070 "http://localhost:7070/health"
check_service "Blockchain TPS"     3010 "http://localhost:3010/"
check_service "Dashboard (Vite)"   5174 "http://localhost:5174/"

echo ""

# Chain DB stats
if curl -s --connect-timeout 2 http://localhost:7070/health &>/dev/null; then
  CHAIN_COUNT=$(curl -s http://localhost:7070/health 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('chains_loaded', '?'))" 2>/dev/null || echo "?")
  echo -e "  ${BLUE}📊${NC} Chain DB: ${BOLD}$CHAIN_COUNT${NC} blockchains loaded"
fi

# Validator venv check
if [ -d "$SCRIPT_DIR/validator/.venv" ]; then
  echo -e "  ${GREEN}🐍${NC} GPU Validator virtualenv: ${GREEN}present${NC}"
else
  echo -e "  ${YELLOW}🐍${NC} GPU Validator virtualenv: ${YELLOW}not found${NC}"
fi

echo ""

# Log sizes
if [ -d "$SCRIPT_DIR/logs" ]; then
  echo -e "  ${CYAN}Logs:${NC}"
  for f in "$SCRIPT_DIR/logs"/*.log; do
    [ -f "$f" ] || continue
    local_name=$(basename "$f")
    local_size=$(du -h "$f" 2>/dev/null | cut -f1)
    echo -e "    $local_name  ($local_size)"
  done
  echo ""
fi
