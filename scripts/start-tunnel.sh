#!/bin/bash
# Start Cloudflare tunnel to expose local X3 services to x3star.net

TUNNEL_ID="6c118620-18cf-4795-80a8-6d44d37aecaa"
TUNNEL_NAME="atlas-sphere"
CONFIG_FILE="$HOME/.cloudflared/config.yml"
if [ -f "$(pwd)/infra/cloudflare-tunnel/config.yml" ]; then
  CONFIG_FILE="$(pwd)/infra/cloudflare-tunnel/config.yml"
fi

echo "🌐 Starting Cloudflare tunnel: $TUNNEL_NAME"
echo "   Tunnel ID: $TUNNEL_ID"
echo "   Config: $CONFIG_FILE"
echo ""
echo "Routing:"
echo "  ws.x3star.net  → localhost:9944 (WebSocket RPC)"
echo "  rpc.x3star.net → localhost:9933 (HTTP RPC)"
echo "  x3star.net     → localhost:4174 (HTML site)"
echo "  x3star.org     → localhost:5173 (Desktop app)"
echo ""

exec cloudflared --config "$CONFIG_FILE" tunnel run "$TUNNEL_NAME"
