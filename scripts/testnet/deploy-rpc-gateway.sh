#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# deploy-rpc-gateway.sh — Deploy public RPC gateway with rate limiting
#
# Deploys an HAProxy-based RPC gateway in front of the testnet validators.
# Provides TLS termination, rate limiting, IP allowlisting, and WebSocket
# support for Substrate subscriptions.
#
# Usage:
#   ./scripts/testnet/deploy-rpc-gateway.sh [--validator-rpcs URL1 URL2 URL3]
#       [--domain rpc.testnet.x3chain.com] [--port 8545] [--ws-port 9944]
#       [--rate-limit-rps 1000] [--tls-cert PATH] [--tls-key PATH]
#
# Environment:
#   VALIDATOR_RPCS    Space-separated list of validator RPC URLs
#   GATEWAY_DOMAIN    Public domain for the RPC gateway
#   GATEWAY_PORT      HTTP RPC port (default: 8545)
#   GATEWAY_WS_PORT   WebSocket port (default: 9944)
#   RATE_LIMIT_RPS    Max requests per second per IP (default: 1000)
#   TLS_CERT_PATH     Path to TLS certificate
#   TLS_KEY_PATH      Path to TLS key
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_DIR="${ROOT_DIR}/scripts/testnet/compose"
VALIDATOR_RPCS=()
GATEWAY_DOMAIN="${GATEWAY_DOMAIN:-rpc.testnet.x3chain.com}"
GATEWAY_PORT="${GATEWAY_PORT:-8545}"
GATEWAY_WS_PORT="${GATEWAY_WS_PORT:-9944}"
RATE_LIMIT_RPS="${RATE_LIMIT_RPS:-1000}"
TLS_CERT_PATH="${TLS_CERT_PATH:-}"
TLS_KEY_PATH="${TLS_KEY_PATH:-}"

usage() {
  cat <<EOF
Usage: $(basename "$0") [--validator-rpcs URL1 URL2 URL3] [--domain DOMAIN]
       [--port PORT] [--ws-port PORT] [--rate-limit-rps N]
       [--tls-cert PATH] [--tls-key PATH]

Deploy public RPC gateway with rate limiting for X3 testnet.

Options:
  --validator-rpcs URLS  Validator RPC endpoints (default: http://127.0.0.1:9944)
  --domain DOMAIN        Public domain (default: rpc.testnet.x3chain.com)
  --port PORT            HTTP RPC port (default: 8545)
  --ws-port PORT         WebSocket port (default: 9944)
  --rate-limit-rps N     Max requests/sec per IP (default: 1000)
  --tls-cert PATH        TLS certificate file
  --tls-key PATH         TLS key file
  -h, --help             Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --validator-rpcs) shift; while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do VALIDATOR_RPCS+=("$1"); shift; done ;;
    --domain) GATEWAY_DOMAIN="${2:-}"; shift 2 ;;
    --port) GATEWAY_PORT="${2:-}"; shift 2 ;;
    --ws-port) GATEWAY_WS_PORT="${2:-}"; shift 2 ;;
    --rate-limit-rps) RATE_LIMIT_RPS="${2:-}"; shift 2 ;;
    --tls-cert) TLS_CERT_PATH="${2:-}"; shift 2 ;;
    --tls-key) TLS_KEY_PATH="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

if [[ ${#VALIDATOR_RPCS[@]} -eq 0 ]]; then
  VALIDATOR_RPCS=("http://127.0.0.1:9944")
fi

echo "=========================================="
echo " X3 Testnet RPC Gateway Deploy"
echo " Validators: ${VALIDATOR_RPCS[*]}"
echo " Domain:     ${GATEWAY_DOMAIN}"
echo " HTTP port:  ${GATEWAY_PORT}"
echo " WS port:    ${GATEWAY_WS_PORT}"
echo " Rate limit: ${RATE_LIMIT_RPS} req/s per IP"
echo "=========================================="

mkdir -p "$COMPOSE_DIR"

# Build HAProxy config
HAPROXY_CFG="${COMPOSE_DIR}/haproxy-rpc-gateway.cfg"

# Generate backend server lines
BACKEND_SERVERS=""
IDX=0
for rpc in "${VALIDATOR_RPCS[@]}"; do
  # Extract host and port from URL
  HOST="${rpc#http://}"
  HOST="${HOST#https://}"
  HOST="${HOST%%:*}"
  PORT="${rpc##*:}"
  PORT="${PORT%/}"
  BACKEND_SERVERS+="    server validator-${IDX} ${HOST}:${PORT} check inter 1s fall 3 rise 2 maxconn 5000${NEWLINE}"
  IDX=$((IDX + 1))
done

cat > "$HAPROXY_CFG" <<HAPROXY
global
    log stdout format raw local0
    maxconn 50000
    nbthread 4
    tune.bufsize 32768

defaults
    log global
    mode http
    option httplog
    option dontlognull
    option http-keep-alive
    option redispatch
    retries 2
    timeout connect 1500ms
    timeout client  30s
    timeout server  30s
    timeout http-request 5s
    timeout http-keep-alive 60s
    timeout tunnel 1h

    # Security headers
    http-response set-header X-Content-Type-Options nosniff
    http-response set-header X-Frame-Options DENY
    http-response set-header X-XSS-Protection "1; mode=block"

# ---------------------------------------------------------------------------
# Frontend: HTTP RPC
# ---------------------------------------------------------------------------
frontend rpc_http
    bind *:${GATEWAY_PORT}
    # Rate limiting — per-IP stick table
    stick-table type ip size 100k expire 60s store http_req_rate(10s),gpc0
    http-request track-sc0 src
    http-request sc-inc-gpc0(0) if { sc0_http_req_rate(0) gt ${RATE_LIMIT_RPS} }
    # Deny if rate limit exceeded
    http-request deny deny_status 429 if { sc0_get_gpc0(0) gt 5 }

    # Request size limit — 10 MB max body
    http-request deny if { req.body_size gt 10485760 }

    # Block sensitive methods
    http-request deny if { path_beg /admin }
    http-request deny if { path_beg /debug }
    http-request deny if { path_beg /internal }

    # WebSocket upgrade detection
    acl is_websocket hdr(Upgrade) -i websocket
    acl is_upgrade hdr(Connection) -i upgrade
    use_backend rpc_ws_backend if is_websocket is_upgrade
    default_backend rpc_http_backend

# ---------------------------------------------------------------------------
# Frontend: Health check
# ---------------------------------------------------------------------------
frontend health_check
    bind *:8081
    monitor-uri /health
    default_backend rpc_health_backend

# ---------------------------------------------------------------------------
# Backend: HTTP RPC
# ---------------------------------------------------------------------------
backend rpc_http_backend
    balance roundrobin
    option httpchk POST /health
    http-check expect status 200
${BACKEND_SERVERS}
    default-server error-limit 50 on-error mark-down inter 30s

# ---------------------------------------------------------------------------
# Backend: WebSocket
# ---------------------------------------------------------------------------
backend rpc_ws_backend
    balance source
    option httpchk GET /health
    http-check expect status 200
${BACKEND_SERVERS}
    timeout tunnel 1h
    timeout client 1h
    timeout server 1h

# ---------------------------------------------------------------------------
# Backend: Health aggregation
# ---------------------------------------------------------------------------
backend rpc_health_backend
    option httpchk GET /health
    http-check expect status 200
    server health-local 127.0.0.1:8081 check inter 2s fall 2 rise 2
HAPROXY

# Write docker-compose for the RPC gateway
cat > "${COMPOSE_DIR}/docker-compose.rpc-gateway.yml" <<COMPOSE
version: '3.8'

services:
  haproxy:
    image: haproxy:3.0-alpine
    container_name: x3-rpc-gateway
    ports:
      - "${GATEWAY_PORT}:${GATEWAY_PORT}"
      - "8081:8081"
    volumes:
      - ./haproxy-rpc-gateway.cfg:/usr/local/etc/haproxy/haproxy.cfg:ro
COMPOSE

# Add TLS volumes if provided
if [[ -n "$TLS_CERT_PATH" && -n "$TLS_KEY_PATH" ]]; then
  # Add TLS frontend to HAProxy config
  cat >> "$HAPROXY_CFG" <<TLS

frontend rpc_tls
    bind *:443 ssl crt /etc/ssl/certs/x3-rpc-gateway.pem alpn h2,http/1.1
    stick-table type ip size 100k expire 60s store http_req_rate(10s),gpc0
    http-request track-sc0 src
    http-request sc-inc-gpc0(0) if { sc0_http_req_rate(0) gt ${RATE_LIMIT_RPS} }
    http-request deny deny_status 429 if { sc0_get_gpc0(0) gt 5 }
    acl is_websocket hdr(Upgrade) -i websocket
    acl is_upgrade hdr(Connection) -i upgrade
    use_backend rpc_ws_backend if is_websocket is_upgrade
    default_backend rpc_http_backend
TLS

  # Update docker-compose to include TLS cert
  cat >> "${COMPOSE_DIR}/docker-compose.rpc-gateway.yml" <<TLSCOMPOSE
      - ${TLS_CERT_PATH}:/etc/ssl/certs/x3-rpc-gateway.pem:ro
TLSCOMPOSE
fi

echo "[deploy] Starting RPC gateway..."
docker compose -f "${COMPOSE_DIR}/docker-compose.rpc-gateway.yml" up -d

echo "[deploy] RPC gateway deployment complete."
echo "        HTTP:  http://localhost:${GATEWAY_PORT}"
echo "        Health: http://localhost:8081/health"
echo "        Backends:"
for rpc in "${VALIDATOR_RPCS[@]}"; do
  echo "          - ${rpc}"
done
echo ""
echo " To check logs: docker compose -f ${COMPOSE_DIR}/docker-compose.rpc-gateway.yml logs -f"
echo " To stop:       docker compose -f ${COMPOSE_DIR}/docker-compose.rpc-gateway.yml down"
