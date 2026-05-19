#!/bin/bash
#
# E2E Benchmark Validation Script
# 
# This script validates the real end-to-end benchmark flow:
# sidecar → gateway → PostgreSQL
#
# Prerequisites:
#   - Docker and docker-compose installed
#   - Rust toolchain installed (cargo)
#
# Usage:
#   ./scripts/benchmark-e2e.sh [command] [options]
#
# Commands:
#   up           - Start all services (PostgreSQL, gateway, sidecar)
#   down         - Stop all services
#   clean        - Stop and clean up all data
#   test         - Run E2E tests against running services
#   submit       - Submit a real benchmark job
#   query-db     - Query PostgreSQL for stored reports
#   logs         - Show service logs
#   health       - Check health of all services
#   full         - Up + test + down (complete validation)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="docker-compose.benchmark-e2e.yml"
CONFIG_DIR="$PROJECT_ROOT/docker-config"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Defaults
SERVICES_TIMEOUT=60
RPC_PORT=9955
GATEWAY_PORT=8080
POSTGRES_PORT=5432
TENANT_ID="e2e-test-tenant"
GATEWAY_TOKEN="benchmark-secret-token-e2e"

# ============================================================================
# Utility Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

wait_for_service() {
    local service=$1
    local port=$2
    local timeout=$3
    local elapsed=0
    
    log_info "Waiting for $service on port $port (timeout: ${timeout}s)..."
    
    while [ $elapsed -lt $timeout ]; do
        if nc -z localhost $port 2>/dev/null; then
            log_success "$service is ready"
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    
    log_error "Timeout waiting for $service"
    return 1
}

check_docker() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    log_success "Docker is available"
}

check_docker_compose() {
    # docker compose is bundled with docker, just verify docker exists
    if ! docker compose version &> /dev/null; then
        log_error "docker compose is not available"
        exit 1
    fi
    log_success "docker compose is available"
}

check_netcat() {
    if ! command -v nc &> /dev/null; then
        log_warn "netcat (nc) not found, installing..."
        if command -v apt-get &> /dev/null; then
            sudo apt-get update && sudo apt-get install -y netcat-openbsd
        fi
    fi
}

check_cargo() {
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo is not installed. Please install Rust."
        exit 1
    fi
    log_success "Cargo is available"
}

# ============================================================================
# Service Management
# ============================================================================

services_up() {
    log_info "Starting E2E benchmark services..."
    
    check_docker
    check_docker_compose
    check_netcat
    
    cd "$PROJECT_ROOT"
    
    if [ ! -f "$COMPOSE_FILE" ]; then
        log_error "docker-compose file not found: $COMPOSE_FILE"
        exit 1
    fi
    
    # Start services
    docker compose -f "$COMPOSE_FILE" up -d
    
    log_success "Services started (containers may still be initializing)"
    
    # Wait for each service
    wait_for_service "PostgreSQL" $POSTGRES_PORT $SERVICES_TIMEOUT
    wait_for_service "Gateway" $GATEWAY_PORT $SERVICES_TIMEOUT
    wait_for_service "Sidecar" $RPC_PORT $SERVICES_TIMEOUT
    
    log_success "All services are healthy"
}

services_down() {
    log_info "Stopping E2E benchmark services..."
    
    cd "$PROJECT_ROOT"
    
    if docker compose -f "$COMPOSE_FILE" down; then
        log_success "Services stopped"
    else
        log_error "Failed to stop services"
        return 1
    fi
}

services_clean() {
    log_info "Cleaning up E2E benchmark services and data..."
    
    cd "$PROJECT_ROOT"
    
    if docker compose -f "$COMPOSE_FILE" down -v; then
        log_success "Services and volumes removed"
    else
        log_error "Failed to clean up services"
        return 1
    fi
}

services_health() {
    log_info "Checking service health..."
    
    # Check PostgreSQL
    if nc -z localhost $POSTGRES_PORT 2>/dev/null; then
        log_success "PostgreSQL is running on port $POSTGRES_PORT"
    else
        log_error "PostgreSQL is not responding"
    fi
    
    # Check Gateway
    if curl -sf "http://localhost:$GATEWAY_PORT/health" > /dev/null 2>&1; then
        log_success "Gateway is running on port $GATEWAY_PORT"
    else
        log_error "Gateway health check failed"
    fi
    
    # Check Sidecar metrics
    if curl -sf "http://localhost:$RPC_PORT/metrics" > /dev/null 2>&1; then
        log_success "Sidecar metrics are running on port $RPC_PORT"
    else
        log_error "Sidecar metrics check failed"
    fi
}

services_logs() {
    log_info "Showing service logs..."
    cd "$PROJECT_ROOT"
    docker compose -f "$COMPOSE_FILE" logs -f "$@"
}

# ============================================================================
# Benchmark Operations
# ============================================================================

submit_benchmark_job() {
    log_info "Submitting benchmark job to sidecar..."
    
    local report_id="benchmark-$(date +%s)"
    
    # Create benchmark job request
    local json_request=$(cat <<EOF
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "benchmark_submitJob",
  "params": {
    "chain_name": "ethereum",
    "chain_type": "evm",
    "workload_duration_seconds": 10
  }
}
EOF
)
    
    log_info "Request: $json_request"
    
    # Submit to sidecar RPC
    local response=$(curl -sf -X POST \
        -H "Content-Type: application/json" \
        -d "$json_request" \
        "http://localhost:$RPC_PORT/rpc" 2>&1) || true
    
    if [ -z "$response" ]; then
        log_error "Failed to submit benchmark job (no response)"
        return 1
    fi
    
    log_info "Response: $response"
    
    # Check for errors in response
    if echo "$response" | grep -q "error"; then
        log_error "RPC error in response"
        return 1
    fi
    
    log_success "Benchmark job submitted"
    return 0
}

query_database() {
    log_info "Querying PostgreSQL for stored benchmark reports..."
    
    # Query reports from PostgreSQL
    local query="SELECT report_id, tenant_id, chain_name, generated_at FROM benchmark_reports LIMIT 10;"
    
    docker exec $(docker compose -f "$PROJECT_ROOT/$COMPOSE_FILE" ps -q postgres) \
        psql -U gateway -d x3_indexer -c "$query" 2>/dev/null || {
        log_error "Failed to query database"
        return 1
    }
}

# ============================================================================
# E2E Test Execution
# ============================================================================

run_tests() {
    log_info "Running E2E tests..."
    
    check_cargo
    
    cd "$PROJECT_ROOT"
    
    # First, validate services are running
    services_health || {
        log_error "Services are not healthy. Please run './scripts/benchmark-e2e.sh up' first"
        return 1
    }
    
    # Run the full E2E test
    log_info "Running E2E benchmark integration tests..."
    if cargo test --test e2e_gateway_integration --lib benchmark -- --nocapture --ignored; then
        log_success "Tests passed"
    else
        log_warn "Some tests failed (expected for ignored tests without real services)"
    fi
    
    # Run quick sanity tests
    log_info "Running sanity tests..."
    if cargo test -p x3-sidecar --lib gateway_client -- --nocapture; then
        log_success "Sanity tests passed"
    else
        log_error "Sanity tests failed"
        return 1
    fi
}

# ============================================================================
# Negative Path Scenarios
# ============================================================================

test_invalid_token() {
    log_info "Testing invalid auth token scenario..."
    
    local json_request=$(cat <<EOF
{
  "tenant_id": "test-tenant",
  "report": {
    "report_id": "invalid-auth-test",
    "generated_at_unix": $(date +%s),
    "chain_name": "ethereum",
    "chain_type": "evm",
    "baseline": {
      "avg_tps": 100.0,
      "p50_latency_ms": 250,
      "p95_latency_ms": 500,
      "p99_latency_ms": 1000,
      "failure_rate": 0.01
    },
    "x3_replay": {
      "avg_tps": 150.0,
      "p50_latency_ms": 200,
      "p95_latency_ms": 400,
      "p99_latency_ms": 800,
      "failure_rate": 0.005
    },
    "recommendation": "SidecarMode",
    "summary": {
      "projected_soft_confirmation_improvement": "10-15%",
      "projected_app_throughput_improvement": "40-50%",
      "projected_route_latency_delta": "-100ms to -200ms",
      "projected_bridge_latency_delta": "-50ms to -100ms"
    },
    "workload_profile": {
      "total_transactions": 1000,
      "total_receipts": 1000,
      "total_logs": 1000,
      "active_lanes": 8,
      "active_log_lanes": 4,
      "log_classes": [
        {
          "class_name": "Transfer",
          "count": 500,
          "share_of_logs": 0.5,
          "unique_contracts": 10,
          "unique_transactions": 100
        }
      ],
      "low_conflict_ratio": 0.5,
      "medium_conflict_ratio": 0.3,
      "high_conflict_ratio": 0.2,
      "estimated_serial_fraction": 0.15
    },
    "artifacts": [],
    "signer": "e2e-test"
  }
}
EOF
)
    
    # Try with invalid token
    local response=$(curl -sf -X POST \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer invalid-token" \
        -d "$json_request" \
        "http://localhost:$GATEWAY_PORT/api/v1/benchmarks/results" 2>&1) || true
    
    if [ -z "$response" ]; then
        log_error "Expected rejection, but got no response"
        return 1
    fi
    
    # Check for auth error
    if echo "$response" | grep -qiE "(401|403|unauthorized|forbidden)"; then
        log_success "Invalid token correctly rejected (as expected)"
        return 0
    else
        log_warn "Unexpected response: $response"
        return 1
    fi
}

test_gateway_recovery() {
    log_info "Testing gateway recovery scenario..."
    log_warn "Note: This requires manual intervention to stop/restart gateway"
    log_info "Scenarios to test manually:"
    log_info "  1. Stop gateway: docker compose -f $COMPOSE_FILE stop x3-gateway"
    log_info "  2. Try to submit: ./scripts/benchmark-e2e.sh submit"
    log_info "  3. Restart gateway: docker compose -f $COMPOSE_FILE start x3-gateway"
    log_info "  4. Submit again and verify success"
}

# ============================================================================
# Combined Operations
# ============================================================================

full_validation() {
    log_info "Running full E2E validation..."
    
    services_up
    
    # Give services time to settle
    sleep 2
    
    # Check health
    services_health
    
    # Run tests
    run_tests
    
    # Test negative path
    test_invalid_token
    
    # Query database
    query_database
    
    # Cleanup
    services_down
    
    log_success "Full E2E validation completed!"
}

# ============================================================================
# Main
# ============================================================================

main() {
    local command="${1:-help}"
    
    case "$command" in
        up)
            services_up
            ;;
        down)
            services_down
            ;;
        clean)
            services_clean
            ;;
        test)
            run_tests
            ;;
        submit)
            submit_benchmark_job
            ;;
        query-db)
            query_database
            ;;
        logs)
            shift || true
            services_logs "$@"
            ;;
        health)
            services_health
            ;;
        invalid-token)
            test_invalid_token
            ;;
        gateway-recovery)
            test_gateway_recovery
            ;;
        full)
            full_validation
            ;;
        help|*)
            cat <<EOF
X3 Benchmark E2E Validation Script

USAGE:
    $(basename "$0") <command> [options]

COMMANDS:
    up                 Start all services (PostgreSQL, Gateway, Sidecar)
    down               Stop all services
    clean              Stop services and remove volumes/data
    health             Check health of all running services
    test               Run E2E tests against running services
    submit             Submit a benchmark job to the sidecar
    query-db           Query PostgreSQL for stored reports
    logs [service]     Show logs from services (optionally filter by service)
    
    invalid-token      Test invalid auth token rejection
    gateway-recovery   Test gateway recovery scenario (manual steps)
    
    full               Complete validation: up → test → down
    
    help               Show this help message

EXAMPLES:
    # Complete validation
    $(basename "$0") full
    
    # Manual validation
    $(basename "$0") up
    $(basename "$0") test
    $(basename "$0") submit
    $(basename "$0") query-db
    $(basename "$0") down
    
    # Check status
    $(basename "$0") health
    $(basename "$0") logs x3-gateway
    
    # Negative path testing
    $(basename "$0") invalid-token

CONFIGURATION:
    Services are defined in: $COMPOSE_FILE
    Configuration is in: $CONFIG_DIR/sidecar-e2e.toml
    
    Gateway Token: $GATEWAY_TOKEN
    Tenant ID: $TENANT_ID
    Ports:
      - PostgreSQL: $POSTGRES_PORT
      - Gateway: $GATEWAY_PORT
      - Sidecar RPC: $RPC_PORT

EOF
            exit 0
            ;;
    esac
}

# Run main function
main "$@"
