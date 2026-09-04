#!/bin/bash

# X3-X3-Sphere E2E Test Environment Shutdown Script
# This script stops and cleans up the complete test environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose.test.yml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if PID file exists
PID_FILE="$SCRIPT_DIR/.test_env.pid"
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        log_info "Stopping test environment (PID: $PID)..."
        kill "$PID" 2>/dev/null || true
    fi
    rm -f "$PID_FILE"
fi

# Stop and remove containers
log_info "Stopping Docker containers..."
docker-compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || {
    log_warning "Some containers may not have stopped cleanly"
}

# Clean up networks
log_info "Cleaning up Docker networks..."
docker network prune -f 2>/dev/null || true

# Clean up volumes if requested
if [ "$1" = "--clean" ] || [ "$1" = "-c" ]; then
    log_info "Cleaning up volumes and images..."
    docker volume prune -f 2>/dev/null || true
    docker image prune -f 2>/dev/null || true
    
    # Remove project-specific volumes
    docker volume ls -q | grep "e2e\|testnet\|x3" | xargs -r docker volume rm 2>/dev/null || true
fi

# Clean up temporary files
log_info "Cleaning up temporary files..."
rm -rf "$SCRIPT_DIR/logs"/* 2>/dev/null || true
rm -rf "$SCRIPT_DIR/test-results"/* 2>/dev/null || true
rm -rf "$SCRIPT_DIR/monitoring/data"/* 2>/dev/null || true

# Clean up any remaining processes
log_info "Cleaning up remaining processes..."
pkill -f "x3-node" 2>/dev/null || true
pkill -f "prometheus" 2>/dev/null || true
pkill -f "grafana" 2>/dev/null || true
pkill -f "alertmanager" 2>/dev/null || true

# Show final status
log_info "Final container status:"
docker ps -a --filter "name=x3" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null || echo "No X3 containers found"

log_success "🧹 E2E Test Environment shutdown complete!"
echo ""
echo "🔄 To restart the environment:"
echo "  ./start_test_environment.sh"
echo ""
echo "🗑️  To clean up volumes and images:"
echo "  ./stop_test_environment.sh --clean"
echo ""

# Verify cleanup
REMAINING_CONTAINERS=$(docker ps -a --filter "name=x3" -q | wc -l)
if [ "$REMAINING_CONTAINERS" -eq 0 ]; then
    log_success "All X3 test containers cleaned up successfully"
else
    log_warning "$REMAINING_CONTAINERS X3 containers still exist. Manual cleanup may
