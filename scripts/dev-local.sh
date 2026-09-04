#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

echo "Starting X3 local development frontends..."

# Desktop frontend
(
  cd apps/x3-desktop
  echo "x3-desktop => http://localhost:5173"
  npm run dev
) &
DESKTOP_PID=$!

# HTML frontend
(
  cd x3fronend
  echo "x3fronend => http://127.0.0.1:4174"
  PORT=4174 npm run server
) &
X3FRONEND_PID=$!

echo "Started x3-desktop (PID=$DESKTOP_PID) and x3fronend (PID=$X3FRONEND_PID)"

echo "Use Ctrl+C to stop both"
wait
