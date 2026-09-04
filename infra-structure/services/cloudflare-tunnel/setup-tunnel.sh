#!/usr/bin/env bash
# ============================================================================
# Cloudflare Tunnel Setup — x3star.net
# ============================================================================
# Prerequisites:
#   1. Free Cloudflare account  → https://dash.cloudflare.com/sign-up
#   2. Add x3star.net to Cloudflare (free plan)
#   3. Change nameservers at Squarespace to the ones Cloudflare gives you
#
# This script:
#   • Installs cloudflared (if missing)
#   • Authenticates with Cloudflare
#   • Creates the tunnel
#   • Writes DNS CNAME records
#   • Installs as a systemd service (auto-start on boot)
# ============================================================================
set -euo pipefail

DOMAIN="x3star.net"
TUNNEL_NAME="x3-chain"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_TEMPLATE="${SCRIPT_DIR}/config.yml"
CONFIG_DEST="${HOME}/.cloudflared/config.yml"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
info()  { echo -e "${GREEN}[INFO]${NC}  $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# ── Step 1: Install cloudflared ─────────────────────────────────────────────
install_cloudflared() {
  if command -v cloudflared &>/dev/null; then
    info "cloudflared already installed: $(cloudflared --version)"
    return 0
  fi

  info "Installing cloudflared…"
  if command -v apt-get &>/dev/null; then
    # Debian / Ubuntu
    curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg \
      | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
    echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared $(lsb_release -cs) main" \
      | sudo tee /etc/apt/sources.list.d/cloudflared.list
    sudo apt-get update && sudo apt-get install -y cloudflared
  elif command -v dnf &>/dev/null; then
    sudo dnf install -y cloudflared || {
      # Fallback to binary
      curl -fsSL -o /tmp/cloudflared https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
      sudo install -m 0755 /tmp/cloudflared /usr/local/bin/cloudflared
    }
  elif command -v pacman &>/dev/null; then
    sudo pacman -Sy --noconfirm cloudflared || {
      curl -fsSL -o /tmp/cloudflared https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
      sudo install -m 0755 /tmp/cloudflared /usr/local/bin/cloudflared
    }
  else
    info "Downloading cloudflared binary…"
    curl -fsSL -o /tmp/cloudflared https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64
    sudo install -m 0755 /tmp/cloudflared /usr/local/bin/cloudflared
  fi

  info "cloudflared installed: $(cloudflared --version)"
}

# ── Step 2: Authenticate ────────────────────────────────────────────────────
authenticate() {
  if [[ -f "${HOME}/.cloudflared/cert.pem" ]]; then
    info "Already authenticated with Cloudflare."
    return 0
  fi

  info "Opening browser to authenticate with Cloudflare…"
  info "Select '${DOMAIN}' when prompted."
  cloudflared tunnel login
}

# ── Step 3: Create tunnel ───────────────────────────────────────────────────
create_tunnel() {
  # Check if tunnel already exists
  if cloudflared tunnel list | grep -q "${TUNNEL_NAME}"; then
    info "Tunnel '${TUNNEL_NAME}' already exists."
    TUNNEL_ID=$(cloudflared tunnel list --output json | python3 -c "
import json, sys
tunnels = json.load(sys.stdin)
for t in tunnels:
    if t['name'] == '${TUNNEL_NAME}':
        print(t['id'])
        break
")
  else
    info "Creating tunnel '${TUNNEL_NAME}'…"
    TUNNEL_ID=$(cloudflared tunnel create "${TUNNEL_NAME}" 2>&1 | grep -oP '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}')
  fi

  if [[ -z "${TUNNEL_ID:-}" ]]; then
    error "Failed to get tunnel ID. Check output above."
    exit 1
  fi

  info "Tunnel ID: ${TUNNEL_ID}"
  export TUNNEL_ID
}

# ── Step 4: Write config ────────────────────────────────────────────────────
write_config() {
  mkdir -p "${HOME}/.cloudflared"

  info "Writing config to ${CONFIG_DEST}…"
  sed "s/TUNNEL_ID/${TUNNEL_ID}/g" "${CONFIG_TEMPLATE}" > "${CONFIG_DEST}"

  info "Config written. Review: cat ${CONFIG_DEST}"
}

# ── Step 5: Create DNS records ──────────────────────────────────────────────
create_dns() {
  local subdomains=("" "www" "rpc" "ws" "explorer" "api")

  for sub in "${subdomains[@]}"; do
    local fqdn
    if [[ -z "$sub" ]]; then
      fqdn="${DOMAIN}"
    else
      fqdn="${sub}.${DOMAIN}"
    fi

    info "Creating DNS CNAME: ${fqdn} → ${TUNNEL_ID}.cfargotunnel.com"
    cloudflared tunnel route dns "${TUNNEL_NAME}" "${fqdn}" 2>/dev/null || {
      warn "DNS record for ${fqdn} may already exist (that's OK)."
    }
  done
}

# ── Step 6: Install systemd service ─────────────────────────────────────────
install_service() {
  info "Installing cloudflared as a systemd service…"
  sudo cloudflared service install 2>/dev/null || {
    warn "Service may already be installed."
  }

  sudo systemctl enable cloudflared
  sudo systemctl restart cloudflared
  info "cloudflared service started and enabled on boot."
}

# ── Step 7: Validate ────────────────────────────────────────────────────────
validate() {
  echo ""
  info "════════════════════════════════════════════════════════════════"
  info "  Cloudflare Tunnel for ${DOMAIN} is ready!"
  info "════════════════════════════════════════════════════════════════"
  info ""
  info "  Tunnel ID:    ${TUNNEL_ID}"
  info "  Tunnel name:  ${TUNNEL_NAME}"
  info "  Config:       ${CONFIG_DEST}"
  info ""
  info "  Routes:"
  info "    https://${DOMAIN}            → localhost:5173  (X3 Desktop)"
  info "    https://www.${DOMAIN}        → localhost:5173  (X3 Desktop)"
  info "    https://rpc.${DOMAIN}        → localhost:9933  (Node HTTP RPC)"
  info "    https://ws.${DOMAIN}         → localhost:9944  (Node WebSocket)"
  info "    https://explorer.${DOMAIN}   → localhost:4000  (Explorer)"
  info "    https://api.${DOMAIN}        → localhost:8080  (API)"
  info ""
  info "  Status:  sudo systemctl status cloudflared"
  info "  Logs:    sudo journalctl -u cloudflared -f"
  info "  Stop:    sudo systemctl stop cloudflared"
  info ""
  info "  DNS propagation can take 1-5 minutes."
  info "════════════════════════════════════════════════════════════════"
}

# ── Main ─────────────────────────────────────────────────────────────────────
main() {
  echo ""
  info "Setting up Cloudflare Tunnel for ${DOMAIN}"
  echo ""

  install_cloudflared
  authenticate
  create_tunnel
  write_config
  create_dns
  install_service
  validate
}

main "$@"
