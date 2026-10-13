#!/usr/bin/env bash
# Real Postgres integration gate for x3-gateway.
#
# Owns a disposable Postgres container, waits for readiness, runs the ignored
# Rust integration test against the production Database::connect() path, and
# always removes the container on exit.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

IMAGE="${X3_GATEWAY_POSTGRES_IMAGE:-postgres:16-alpine}"
NAME="x3-gateway-postgres-$PPID-$$"
DB_USER="x3"
DB_PASSWORD="x3-test-only"
DB_NAME="x3_gateway_test"
HOST_PORT="$(python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
)"

if ! command -v docker >/dev/null 2>&1; then
  echo "gateway-postgres-integration: docker is required; no Postgres test container was started" >&2
  exit 2
fi

if ! docker info >/dev/null 2>&1; then
  echo "gateway-postgres-integration: docker daemon is not reachable" >&2
  exit 2
fi

cleanup() {
  status=$?
  trap - EXIT
  if [ "$status" -ne 0 ]; then
    echo "gateway-postgres-integration: Postgres logs follow because the gate failed" >&2
    docker logs "$NAME" >&2 2>/dev/null || true
  fi
  docker rm -f "$NAME" >/dev/null 2>&1 || true
  exit "$status"
}
trap cleanup EXIT

echo "gateway-postgres-integration: starting $IMAGE as $NAME"
docker run --detach --rm \
  --name "$NAME" \
  -e "POSTGRES_USER=$DB_USER" \
  -e "POSTGRES_PASSWORD=$DB_PASSWORD" \
  -e "POSTGRES_DB=$DB_NAME" \
  -p "127.0.0.1:$HOST_PORT:5432" \
  "$IMAGE" >/dev/null

ready=0
for _ in $(seq 1 60); do
  if docker exec "$NAME" pg_isready -U "$DB_USER" -d "$DB_NAME" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 1
done

if [ "$ready" -ne 1 ]; then
  echo "gateway-postgres-integration: Postgres did not become ready within 60 seconds" >&2
  exit 1
fi

export X3_GATEWAY_TEST_DATABASE_URL="postgres://$DB_USER:$DB_PASSWORD@127.0.0.1:$HOST_PORT/$DB_NAME"

echo "gateway-postgres-integration: running x3-gateway database integration test on 127.0.0.1:$HOST_PORT"
env SKIP_WASM_BUILD=1 \
  cargo test --locked -p x3-gateway --test postgres_integration -- \
  --ignored --nocapture --test-threads=1
