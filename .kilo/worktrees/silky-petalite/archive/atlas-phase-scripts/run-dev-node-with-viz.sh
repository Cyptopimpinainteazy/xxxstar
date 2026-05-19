#!/bin/bash
# X3 Chain Dev Node with Live Block Visualization
# Starts the node and displays blocks in real-time with our milestone system
# Usage: ./run-dev-node-with-viz.sh

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NODE_SCRIPT="$PROJECT_ROOT/run-dev-node.sh"
MONITOR_SCRIPT="$PROJECT_ROOT/scripts/monitor_blocks.sh"
LOG_FILE="$PROJECT_ROOT/.x3-dev.log"

# Colors
BOLD='\033[1m'
GREEN='\033[92m'
CYAN='\033[96m'
RESET='\033[0m'

cleanup() {
    echo -e "\n${CYAN}Shutting down...${RESET}"
    jobs -p | xargs kill 2>/dev/null || true
    wait 2>/dev/null || true
}

trap cleanup SIGINT SIGTERM EXIT

echo -e "${BOLD}${GREEN}🚀 X3 CHAIN DEVELOPMENT NODE WITH LIVE BLOCK VISUALIZATION${RESET}"
echo ""
echo "Starting node and block monitor..."
echo ""

# Remove old log file
rm -f "$LOG_FILE"

# Start node in background, sending output to log
"$NODE_SCRIPT" > "$LOG_FILE" 2>&1 &

# Give node time to start
sleep 3

# Start monitoring (will tail the log and display blocks)
echo "Monitoring blocks..."
echo ""
bash "$MONITOR_SCRIPT" "$LOG_FILE"

echo -e "${BLUE}Node stopped${NC}"
