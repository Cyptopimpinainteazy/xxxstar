#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# deploy-indexer.sh — Deploy indexer + PostgreSQL for X3 testnet
#
# Deploys the x3-indexer service with a PostgreSQL backend for indexing
# blockchain events, blocks, and extrinsics. Exposes a GraphQL endpoint.
#
# Usage:
#   ./scripts/testnet/deploy-indexer.sh [--rpc-urls URL1 URL2 URL3]
#       [--db-password PASSWORD] [--db-name NAME] [--port 4000]
#
# Environment:
#   RPC_URLS         Space-separated list of RPC endpoints (default: localhost:9944)
#   DB_PASSWORD      PostgreSQL password (default: auto-generated)
#   DB_NAME          Database name (default: x3_indexer)
#   INDEXER_PORT     GraphQL port (default: 4000)
#   POSTGRES_PORT    PostgreSQL port (default: 5432)
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_DIR="${ROOT_DIR}/scripts/testnet/compose"
DB_PASSWORD="${DB_PASSWORD:-$(openssl rand -hex 16)}"
DB_NAME="${DB_NAME:-x3_indexer}"
DB_USER="${DB_USER:-indexer}"
INDEXER_PORT="${INDEXER_PORT:-4000}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
RPC_URLS=()

usage() {
  cat <<EOF
Usage: $(basename "$0") [--rpc-urls URL1 URL2 URL3] [--db-password PASSWORD]
       [--db-name NAME] [--port PORT] [--pg-port PORT]

Deploy indexer + PostgreSQL for X3 testnet.

Options:
  --rpc-urls URLS     Space-separated RPC endpoints (default: http://127.0.0.1:9944)
  --db-password PASS  PostgreSQL password (default: auto-generated)
  --db-name NAME      Database name (default: x3_indexer)
  --port PORT         GraphQL endpoint port (default: 4000)
  --pg-port PORT      PostgreSQL port (default: 5432)
  -h, --help          Show this help.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --rpc-urls) shift; while [[ $# -gt 0 && ! "$1" =~ ^-- ]]; do RPC_URLS+=("$1"); shift; done ;;
    --db-password) DB_PASSWORD="${2:-}"; shift 2 ;;
    --db-name) DB_NAME="${2:-}"; shift 2 ;;
    --port) INDEXER_PORT="${2:-}"; shift 2 ;;
    --pg-port) POSTGRES_PORT="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown argument: $1"; usage; exit 2 ;;
  esac
done

if [[ ${#RPC_URLS[@]} -eq 0 ]]; then
  RPC_URLS=("http://127.0.0.1:9944")
fi

echo "=========================================="
echo " X3 Testnet Indexer + PostgreSQL Deploy"
echo " RPCs:   ${RPC_URLS[*]}"
echo " DB:     ${DB_NAME}@localhost:${POSTGRES_PORT}"
echo " GraphQL port: ${INDEXER_PORT}"
echo "=========================================="

mkdir -p "$COMPOSE_DIR"

# Write docker-compose for indexer + PostgreSQL
cat > "${COMPOSE_DIR}/docker-compose.indexer.yml" <<COMPOSE
version: '3.8'

services:
  postgres:
    image: postgres:16-alpine
    container_name: x3-indexer-postgres
    ports:
      - "${POSTGRES_PORT}:5432"
    environment:
      POSTGRES_USER: ${DB_USER}
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: ${DB_NAME}
      PGDATA: /var/lib/postgresql/data/pgdata
    volumes:
      - x3-indexer-pgdata:/var/lib/postgresql/data
      - ./init-indexer-schema.sql:/docker-entrypoint-initdb.d/01-init-schema.sql:ro
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${DB_USER} -d ${DB_NAME}"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 15s
    restart: unless-stopped
    networks:
      - x3-testnet

  indexer:
    image: docker.io/x3-chain/x3-indexer:latest
    container_name: x3-indexer
    ports:
      - "${INDEXER_PORT}:4000"
    environment:
      LISTEN_ADDR: 0.0.0.0:4000
      LOG_LEVEL: info
      DB_HOST: postgres
      DB_PORT: 5432
      DB_USER: ${DB_USER}
      DB_PASSWORD: ${DB_PASSWORD}
      DB_NAME: ${DB_NAME}
    command: >
      /usr/local/bin/x3-indexer
      --listen 0.0.0.0:4000
      --rpc-urls $(printf '%s ' "${RPC_URLS[@]}")
      --db-host postgres
      --db-user ${DB_USER}
      --db-password ${DB_PASSWORD}
      --log-level info
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "-s", "-X", "POST", "http://localhost:4000/graphql",
             "-H", "Content-Type: application/json",
             "-d", '{"query":"{ __typename }"}']
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 20s
    restart: unless-stopped
    networks:
      - x3-testnet

networks:
  x3-testnet:
    driver: bridge

volumes:
  x3-indexer-pgdata:
COMPOSE

# Write PostgreSQL init schema
cat > "${COMPOSE_DIR}/init-indexer-schema.sql" <<SQL
-- X3 Indexer Database Schema
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS blocks (
  id SERIAL PRIMARY KEY,
  block_number BIGINT UNIQUE NOT NULL,
  block_hash VARCHAR(255) NOT NULL,
  parent_hash VARCHAR(255),
  timestamp TIMESTAMP DEFAULT NOW(),
  validator VARCHAR(255),
  state_root VARCHAR(255),
  extrinsics_root VARCHAR(255),
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS events (
  id SERIAL PRIMARY KEY,
  block_number BIGINT NOT NULL,
  event_index INTEGER NOT NULL,
  section VARCHAR(255) NOT NULL,
  method VARCHAR(255) NOT NULL,
  data JSONB,
  created_at TIMESTAMP DEFAULT NOW(),
  FOREIGN KEY (block_number) REFERENCES blocks(block_number),
  UNIQUE(block_number, event_index)
);

CREATE TABLE IF NOT EXISTS extrinsics (
  id SERIAL PRIMARY KEY,
  block_number BIGINT NOT NULL,
  extrinsic_index INTEGER NOT NULL,
  hash VARCHAR(255) NOT NULL,
  method VARCHAR(255),
  section VARCHAR(255),
  signer VARCHAR(255),
  nonce BIGINT,
  signature VARCHAR(255),
  success BOOLEAN,
  created_at TIMESTAMP DEFAULT NOW(),
  FOREIGN KEY (block_number) REFERENCES blocks(block_number),
  UNIQUE(block_number, extrinsic_index)
);

-- Bridge-specific tables for testnet verification
CREATE TABLE IF NOT EXISTS bridge_transfers (
  id SERIAL PRIMARY KEY,
  block_number BIGINT NOT NULL,
  extrinsic_index INTEGER NOT NULL,
  source_chain VARCHAR(255) NOT NULL,
  target_chain VARCHAR(255) NOT NULL,
  sender VARCHAR(255) NOT NULL,
  recipient VARCHAR(255) NOT NULL,
  asset_id VARCHAR(255),
  amount NUMERIC(78, 0),
  status VARCHAR(50) DEFAULT 'pending',
  tx_hash VARCHAR(255),
  created_at TIMESTAMP DEFAULT NOW(),
  FOREIGN KEY (block_number) REFERENCES blocks(block_number)
);

CREATE TABLE IF NOT EXISTS cross_vm_calls (
  id SERIAL PRIMARY KEY,
  block_number BIGINT NOT NULL,
  extrinsic_index INTEGER NOT NULL,
  source_vm VARCHAR(50) NOT NULL,
  target_vm VARCHAR(50) NOT NULL,
  caller VARCHAR(255) NOT NULL,
  payload_hash VARCHAR(255),
  success BOOLEAN,
  gas_used BIGINT,
  created_at TIMESTAMP DEFAULT NOW(),
  FOREIGN KEY (block_number) REFERENCES blocks(block_number)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_block_number ON events(block_number);
CREATE INDEX IF NOT EXISTS idx_events_method ON events(method);
CREATE INDEX IF NOT EXISTS idx_extrinsics_block_number ON extrinsics(block_number);
CREATE INDEX IF NOT EXISTS idx_extrinsics_signer ON extrinsics(signer);
CREATE INDEX IF NOT EXISTS idx_bridge_transfers_status ON bridge_transfers(status);
CREATE INDEX IF NOT EXISTS idx_bridge_transfers_block ON bridge_transfers(block_number);
CREATE INDEX IF NOT EXISTS idx_cross_vm_calls_block ON cross_vm_calls(block_number);
SQL

echo "[deploy] Starting PostgreSQL and indexer..."
docker compose -f "${COMPOSE_DIR}/docker-compose.indexer.yml" up -d

echo "[deploy] Waiting for PostgreSQL to be healthy..."
sleep 5
docker compose -f "${COMPOSE_DIR}/docker-compose.indexer.yml" exec -T postgres \
  pg_isready -U "${DB_USER}" -d "${DB_NAME}" || true

echo "[deploy] Indexer deployment complete."
echo "        GraphQL: http://localhost:${INDEXER_PORT}/graphql"
echo "        PostgreSQL: localhost:${POSTGRES_PORT} (user=${DB_USER}, db=${DB_NAME})"
echo ""
echo " To check logs: docker compose -f ${COMPOSE_DIR}/docker-compose.indexer.yml logs -f"
echo " To stop:       docker compose -f ${COMPOSE_DIR}/docker-compose.indexer.yml down"
