#!/bin/bash
# Launch the ARBITRAGE Intelligence Dashboard (X3 Intelligence)

APP_DIR="/home/lojak/Desktop/x3-chain-master/apps/x3-intelligence"
LOG_FILE="/tmp/x3-intelligence.log"
PORT=3005

cd "$APP_DIR" || exit 1
npm install --silent > /dev/null 2>&1 || true
PORT=$PORT npm run dev > "$LOG_FILE" 2>&1 &
DASH_PID=$!

sleep 2

if ps -p $DASH_PID > /dev/null; then
  echo "✓ ARBITRAGE Intelligence Dashboard launched at http://localhost:$PORT/"
  echo "  Logs: tail -f $LOG_FILE"
else
  echo "⚠ Failed to launch ARBITRAGE Intelligence Dashboard. Check $LOG_FILE for details."
fi
