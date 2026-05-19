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

mkdir -p "$DRPC_DIR"

if [[ ! -f "$DRPC_DIR/dshackle.yaml" || ! -f "$DRPC_DIR/nodecore.yml" ]]; then
  echo "Missing config templates in $DRPC_DIR"
  exit 1
fi

echo "Starting dRPC stack from $COMPOSE_FILE"
(
  cd "$DRPC_DIR"
  "${COMPOSE_CMD[@]}" -f "$COMPOSE_FILE" up -d
)

echo "Waiting for nodecore (:9090)"
for _ in $(seq 1 30); do
  if curl -sS http://127.0.0.1:9090/ >/dev/null 2>&1 || curl -sS http://127.0.0.1:9090/queries/ethereum >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

echo "dRPC stack launched"
echo "- dshackle proxy: http://127.0.0.1:8545/eth"
echo "- nodecore query: http://127.0.0.1:9090/queries/ethereum"
