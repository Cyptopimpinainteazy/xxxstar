#!/bin/bash

# X3 Desktop - Start All Applications
# This script launches all required apps for the X3 Desktop environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APPS_DIR="$SCRIPT_DIR/apps"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to check if port is in use
port_in_use() {
  lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null 2>&1
  return $?
}

# Function to start an app
start_app() {
  local app_name=$1
  local port=$2
  local app_dir=$3
  
  echo -e "${BLUE}Starting $app_name on port $port...${NC}"
  
  if port_in_use $port; then
    echo -e "${YELLOW}⚠ Port $port is already in use. Skipping $app_name.${NC}"
    return 1
  fi
  
  if [ ! -d "$APPS_DIR/$app_dir" ]; then
    echo -e "${RED}✗ Directory not found: $app_dir${NC}"
    return 1
  fi
  
  cd "$APPS_DIR/$app_dir"
  
  # Install dependencies if node_modules don't exist
  if [ ! -d "node_modules" ]; then
    echo -e "${BLUE}Installing dependencies...${NC}"
    npm install --legacy-peer-deps 2>/dev/null || true
  fi
  
  # Start the app in background
  npm run dev -- -p $port > /tmp/${app_name}.log 2>&1 &
  local pid=$!
  
  echo -e "${GREEN}✓ $app_name started (PID: $pid)${NC}"
  sleep 2
}

echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${BLUE}X3 Desktop - Starting All Apps${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo ""

# Start Tauri dev server if not already running
if ! port_in_use 5173; then
  echo -e "${BLUE}Starting Tauri dev server...${NC}"
  cd "$APPS_DIR/x3-desktop"
  npm run tauri:dev > /tmp/tauri-dev.log 2>&1 &
  echo -e "${GREEN}✓ Tauri dev server started${NC}"
  sleep 3
else
  echo -e "${YELLOW}⚠ Tauri dev server already running on port 5173${NC}"
fi

echo ""
echo -e "${BLUE}Starting backend apps...${NC}"
echo ""

# Start each app
start_app "explorer" 3001 "explorer" || true
start_app "wallet" 3002 "wallet" || true
start_app "dex" 3003 "dex" || true
start_app "x3-intelligence" 3007 "x3-intelligence" || true

echo ""
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${GREEN}✓ X3 Desktop Setup Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}Access the dashboard at:${NC}"
echo -e "  ${GREEN}http://localhost:5173${NC}"
echo ""
echo -e "${BLUE}Running applications:${NC}"
for port in 3001 3002 3003 5173 3007; do
  if port_in_use $port; then
    echo -e "  ${GREEN}✓ Port $port${NC}"
  fi
done

echo ""
echo -e "${BLUE}View logs with:${NC}"
echo -e "  tail -f /tmp/tauri-dev.log"
echo -e "  tail -f /tmp/explorer.log"
echo -e "  tail -f /tmp/wallet.log"
echo -e "  tail -f /tmp/dex.log"
echo -e "  tail -f /tmp/x3-intelligence.log"
echo ""
echo -e "${YELLOW}Note: Press Ctrl+C to stop this script (apps will keep running in background)${NC}"
echo ""

# Keep script running
wait
