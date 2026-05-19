#!/bin/bash
# X3 Infrastructure Control - Manages all services for x3star.net

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SERVICES=(
  "x3-desktop.service"
  "x3fronend.service"
  "x3-chain-node.service"
  "cloudflared-tunnel.service"
)

# Helper functions
print_header() {
  echo -e "${BLUE}=== $1 ===${NC}"
}

print_ok() {
  echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
  echo -e "${RED}✗ $1${NC}"
}

print_warn() {
  echo -e "${YELLOW}⚠ $1${NC}"
}

service_status() {
  local svc=$1
  systemctl --user is-active "$svc" >/dev/null 2>&1 && echo "running" || echo "stopped"
}

# Commands
cmd_status() {
  print_header "X3 Infrastructure Status"
  echo ""

  local all_ok=true
  for svc in "${SERVICES[@]}"; do
    local status=$(service_status "$svc")
    local display_name=$(echo "$svc" | sed 's/.service//' | tr '-' ' ')
    if [ "$status" = "running" ]; then
      print_ok "$display_name: $status"
      systemctl --user status "$svc" --no-pager 2>&1 | grep -E "Active:|Restart|cpu|memory" | head -3 | sed 's/^/  /'
    else
      print_error "$display_name: $status"
      all_ok=false
    fi
  done

  echo ""
  print_header "Service Health Checks"

  # Check desktop
  if curl -s http://localhost:5173 -I >/dev/null 2>&1; then
    print_ok "Desktop (port 5173): responding"
  else
    print_warn "Desktop (port 5173): not responding"
  fi

  # Check node RPC
  if curl -s http://localhost:9933 -d '{"id":1,"jsonrpc":"2.0","method":"system_name","params":[]}' \
       -H 'Content-Type: application/json' 2>/dev/null | grep -q "result"; then
    print_ok "Node RPC (port 9933): responding"
  else
    print_warn "Node RPC (port 9933): not responding"
  fi

  # Check tunnel
  if curl -s -I https://x3star.net 2>/dev/null | grep -q "200\|302\|404"; then
    print_ok "Tunnel (x3star.net): responding"
  else
    print_warn "Tunnel (x3star.net): not responding"
  fi

  if curl -s -I https://x3star.org 2>/dev/null | grep -q "200\|302\|404"; then
    print_ok "Tunnel (x3star.org): responding"
  else
    print_warn "Tunnel (x3star.org): not responding"
  fi

  # Check HTML frontend service
  if curl -s http://127.0.0.1:4174 -I >/dev/null 2>&1; then
    print_ok "HTML frontend (port 4174): responding"
  else
    print_warn "HTML frontend (port 4174): not responding"
  fi

  echo ""
  if [ "$all_ok" = true ]; then
    print_ok "All services running"
  else
    print_warn "Some services stopped"
  fi
}

cmd_start() {
  print_header "Starting X3 Infrastructure"
  for svc in "${SERVICES[@]}"; do
    echo "Starting $svc..."
    systemctl --user start "$svc" || print_warn "Failed to start $svc"
  done
  print_ok "Services started"
  sleep 3
  cmd_status
}

cmd_stop() {
  print_header "Stopping X3 Infrastructure"
  # Stop in reverse order
  for ((i=${#SERVICES[@]}-1; i>=0; i--)); do
    echo "Stopping ${SERVICES[$i]}..."
    systemctl --user stop "${SERVICES[$i]}" || true
  done
  print_ok "Services stopped"
}

cmd_restart() {
  cmd_stop
  sleep 2
  cmd_start
}

cmd_logs() {
  if [ -n "$1" ]; then
    case "$1" in
      desktop)
        journalctl --user -u x3-desktop.service -n 50 -f
        ;;
      node)
        journalctl --user -u x3-chain-node.service -n 50 -f
        ;;
      tunnel)
        journalctl --user -u cloudflared-tunnel.service -n 50 -f
        ;;
      *)
        print_error "Unknown service: $1"
        echo "Usage: $0 logs {desktop|node|tunnel}"
        exit 1
        ;;
    esac
  else
    print_header "All Service Logs (Ctrl+C to exit)"
    journalctl --user -n 100 -f \
      -u x3-desktop.service \
      -u x3-chain-node.service \
      -u cloudflared-tunnel.service
  fi
}

cmd_enable() {
  print_header "Enabling Auto-Start at Boot"
  loginctl enable-linger "$USER" || print_warn "Linger already enabled"
  for svc in "${SERVICES[@]}"; do
    systemctl --user enable "$svc"
  done
  print_ok "Services enabled for auto-start"
}

cmd_disable() {
  print_header "Disabling Auto-Start at Boot"
  for svc in "${SERVICES[@]}"; do
    systemctl --user disable "$svc"
  done
  print_ok "Services disabled"
}

cmd_boot_test() {
  print_header "Simulating System Boot"
  echo "This will stop and restart all services to test boot behavior"
  read -p "Continue? (y/n) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    cmd_stop
    sleep 5
    cmd_start
  fi
}

# Main
case "${1:-status}" in
  status)
    cmd_status
    ;;
  start)
    cmd_start
    ;;
  stop)
    cmd_stop
    ;;
  restart)
    cmd_restart
    ;;
  logs)
    cmd_logs "$2"
    ;;
  enable)
    cmd_enable
    ;;
  disable)
    cmd_disable
    ;;
  boot-test)
    cmd_boot_test
    ;;
  *)
    echo "X3 Infrastructure Manager"
    echo ""
    echo "Usage: $0 {status|start|stop|restart|logs|enable|disable|boot-test}"
    echo ""
    echo "Commands:"
    echo "  status          Show service status and health checks"
    echo "  start           Start all services"
    echo "  stop            Stop all services"
    echo "  restart         Restart all services"
    echo "  logs [svc]      Stream logs (optional: desktop|node|tunnel)"
    echo "  enable          Enable auto-start at boot"
    echo "  disable         Disable auto-start at boot"
    echo "  boot-test       Simulate system boot (stop/start)"
    echo ""
    echo "Services managed:"
    for svc in "${SERVICES[@]}"; do
      echo "  - $svc"
    done
    exit 1
    ;;
esac
