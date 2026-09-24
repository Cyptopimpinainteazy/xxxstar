#!/bin/bash
# X3 Chain TPS Testing Orchestration Script
# 
# Usage:
#   ./scripts/run-tps-tests.sh [up|down|logs|status]
#
# Examples:
#   ./scripts/run-tps-tests.sh up          # Start TPS tracking
#   ./scripts/run-tps-tests.sh down        # Stop all services
#   ./scripts/run-tps-tests.sh logs        # Show service logs
#   ./scripts/run-tps-tests.sh status      # Check service status

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TPS_COMPOSE="${PROJECT_ROOT}/tests/perf/docker-compose.tps.yml"
RPC_URL="${RPC_URL:-http://127.0.0.1:9944}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        return 1
    fi
    
    if ! docker ps &> /dev/null; then
        log_error "Docker daemon is not running"
        return 1
    fi
    
    log_success "Docker is available"
    return 0
}

check_cargo() {
    if ! command -v cargo &> /dev/null; then
        log_error "Rust toolchain is not installed"
        return 1
    fi
    
    log_success "Rust toolchain is available"
    return 0
}

check_rpc() {
    log_info "Checking RPC connection at $RPC_URL..."
    
    if curl -s -f -X POST "$RPC_URL" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"system_health","params":[]}' \
        > /dev/null 2>&1; then
        log_success "RPC connection verified"
        return 0
    else
        log_warn "RPC connection failed (services may still work)"
        return 1
    fi
}

build_tps_tracker() {
    log_info "Building TPS tracker..."
    
    if cargo build --release \
        --manifest-path "${PROJECT_ROOT}/crates/tps-tracker/Cargo.toml" \
        2>&1 | grep -E "(Compiling|Finished)"; then
        log_success "TPS tracker built"
        return 0
    else
        log_error "TPS tracker build failed"
        return 1
    fi
}

start_services() {
    log_info "Starting Docker services..."
    
    if RPC_URL="$RPC_URL" docker-compose \
        -f "$TPS_COMPOSE" \
        up -d --build 2>&1 | grep -E "(Creating|Starting|done)"; then
        log_success "Services started"
        return 0
    else
        log_error "Failed to start services"
        return 1
    fi
}

stop_services() {
    log_info "Stopping services..."
    
    if docker-compose -f "$TPS_COMPOSE" down 2>&1 | grep -E "(Stopping|Removing|done)"; then
        log_success "Services stopped"
        return 0
    else
        log_warn "Services may already be stopped"
        return 0
    fi
}

show_logs() {
    log_info "Showing service logs (Press Ctrl+C to exit)...\n"
    docker-compose -f "$TPS_COMPOSE" logs -f || true
}

show_status() {
    log_info "Service status:\n"
    
    echo "Docker containers:"
    docker-compose -f "$TPS_COMPOSE" ps || log_warn "No containers running"
    
    echo ""
    echo "Service health:"
    
    if curl -s -f http://localhost:8086/health > /dev/null 2>&1; then
        log_success "InfluxDB is running (http://localhost:8086)"
    else
        log_warn "InfluxDB is not responding"
    fi
    
    if curl -s -f http://localhost:8501 > /dev/null 2>&1; then
        log_success "Dashboard is running (http://localhost:8501)"
    else
        log_warn "Dashboard is not responding"
    fi
    
    echo ""
    log_info "Check logs with: $0 logs"
}

open_dashboard() {
    log_info "Opening dashboard..."
    
    # Try different browsers
    if command -v xdg-open &> /dev/null; then
        xdg-open "http://localhost:8501" || true
    elif command -v open &> /dev/null; then
        open "http://localhost:8501" || true
    else
        log_info "Open http://localhost:8501 in your browser"
    fi
}

up() {
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  X3 Chain TPS Testing Infrastructure${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"
    
    # Check prerequisites
    check_docker || return 1
    check_cargo || return 1
    check_rpc || true
    
    # Build and start
    build_tps_tracker || return 1
    start_services || return 1
    
    # Wait for services to be ready
    log_info "Waiting for services to initialize..."
    sleep 5
    
    # Show status
    show_status
    
    echo ""
    log_info "Opening dashboard in browser..."
    open_dashboard
    
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Dashboard: http://localhost:8501${NC}"
    echo -e "${GREEN}  InfluxDB:  http://localhost:8086${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}\n"
    
    log_info "View logs with: $0 logs"
    log_info "Stop services with: $0 down"
}

down() {
    echo -e "${YELLOW}Shutting down TPS testing infrastructure...${NC}\n"
    stop_services
    echo ""
    log_success "All services stopped"
}

if [[ $# -eq 0 ]]; then
    log_error "Missing command"
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  up       Start TPS testing infrastructure (default)"
    echo "  down     Stop all services"
    echo "  logs     Show service logs (Ctrl+C to exit)"
    echo "  status   Show service status"
    echo "  help     Show this help message"
    exit 1
fi

case "${1:-up}" in
    up)
        up
        ;;
    down)
        down
        ;;
    logs)
        show_logs
        ;;
    status)
        show_status
        ;;
    help|--help|-h)
        echo "X3 Chain TPS Testing Script"
        echo ""
        echo "Usage: $0 [command]"
        echo ""
        echo "Commands:"
        echo "  up      Start TPS testing infrastructure"
        echo "  down    Stop all services"
        echo "  logs    Show service logs (Ctrl+C to exit)"
        echo "  status  Show service status"
        echo "  help    Show this help message"
        echo ""
        echo "Environment variables:"
        echo "  RPC_URL  X3 Chain RPC endpoint (default: http://127.0.0.1:9944)"
        ;;
    *)
        log_error "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac
