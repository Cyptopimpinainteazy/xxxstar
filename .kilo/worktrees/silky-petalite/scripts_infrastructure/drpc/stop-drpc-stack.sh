#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DRPC_DIR="${DRPC_DIR:-$ROOT_DIR/infra/drpc}"
COMPOSE_FILE="$DRPC_DIR/docker-compose.drpc.yml"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required"
  exit 1
fi

# Prefer docker compose (v2 plugin), fallback to docker-compose (v1 binary).
COMPOSE_CMD=()
if docker compose version >/dev/null 2>&1; then
  COMPOSE_CMD=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD=(docker-compose)
else
  echo "docker compose is required (install docker compose plugin or docker-compose)"
  exit 1
fi

(
  cd "$DRPC_DIR"
  "${COMPOSE_CMD[@]}" -f "$COMPOSE_FILE" down
)

echo "dRPC stack stopped"
