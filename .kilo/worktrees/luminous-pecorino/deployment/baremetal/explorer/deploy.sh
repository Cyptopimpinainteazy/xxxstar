#!/usr/bin/env bash
set -euo pipefail

# Example deploy script for bare-metal server(s).
# - Builds app locally, rsyncs app directory to remote host, runs npm ci & npm run build there,
#   and restarts systemd service `explorer.service`.
# Usage: ./deploy.sh <user@host> <path=/opt/explorer>

REMOTE=${1:-}
DEST=${2:-/opt/explorer}

if [ -z "$REMOTE" ]; then
  echo "Usage: $0 <user@host> [dest]"
  exit 1
fi

echo "Building explorer locally..."
cd apps/explorer
npm ci
npm run build
cd -

echo "Uploading build to ${REMOTE}:${DEST}"
rsync -avz --delete apps/explorer/ ${REMOTE}:${DEST}/

echo "Installing dependencies on remote and restarting service"
ssh ${REMOTE} bash -lc "cd ${DEST} && npm ci --production && sudo systemctl daemon-reload || true && sudo systemctl restart explorer.service"

echo "Deploy complete. Tail logs with: ssh ${REMOTE} 'journalctl -u explorer -f'"