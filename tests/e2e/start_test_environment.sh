#!/bin/bash

# X3-X3-Sphere E2E Test Environment Startup Script
# This script starts the complete test environment with monitoring and mock services

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
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

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    log_error "Docker is not running. Please start Docker and try again."
    exit 1
fi

# Check if Docker Compose is available
if ! command -v docker-compose > /dev/null 2>&1 && ! docker compose version > /dev/null 2>&1; then
    log_error "Docker Compose is not available. Please install Docker Compose and try again."
    exit 1
fi

# Set up environment variables
export TEST_ENVIRONMENT=testnet
export X3_NODE_ENV=testnet
export DOCKER_BUILDKIT=1

# Load deterministic configuration if in CI or explicitly enabled
if [ "${CI:-false}" = "true" ] || [ "${E2E_DETERMINISTIC_TRIPLE_RUN:-0}" = "1" ]; then
  if [ -f "$SCRIPT_DIR/fixtures/deterministic_config.toml" ]; then
    export X3_E2E_DETERMINISTIC_SEED="x3-e2e-deterministic-seed-001"
    export X3_E2E_GENESIS_TIMESTAMP="1707388800"
    export X3_E2E_BLOCK_TIME_MILLIS="6000"
    log_info "Deterministic mode enabled: seed=$X3_E2E_DETERMINISTIC_SEED timestamp=$X3_E2E_GENESIS_TIMESTAMP"
  fi
fi

# Load deterministic configuration if running in deterministic mode
if [ "${E2E_DETERMINISTIC_TRIPLE_RUN:-0}" = "1" ]; then
    echo "Loading deterministic configuration..."
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    FIXTURE_FILE="$SCRIPT_DIR/fixtures/deterministic_config.toml"
    if [ -f "$FIXTURE_FILE" ]; then
        # Export deterministic settings as environment variables
        export X3_E2E_DETERMINISTIC_SEED="0x1234567890abcdef1234567890abcdef"
        export X3_E2E_GENESIS_TIMESTAMP="1707340800"
        export X3_E2E_BLOCK_TIME_MS="6000"
        export X3_E2E_SESSION_LENGTH="6"
        export X3_E2E_NUM_VALIDATORS="3"
        export X3_E2E_GAS_LIMIT="8000000"
        export X3_E2E_STRUCTURED_LOGGING="true"
        export X3_E2E_DIAGNOSTIC_MODE="true"
        log_info "Deterministic config loaded from $FIXTURE_FILE"
    else
        log_warning "Deterministic config requested but fixture not found at $FIXTURE_FILE"
    fi
fi

log_info "Starting X3-X3-Sphere E2E Test Environment..."
log_info "Test Environment: $TEST_ENVIRONMENT"
log_info "Project Root: $PROJECT_ROOT"

# Clean up any existing containers
log_info "Cleaning up existing containers..."
docker-compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true

# Create necessary directories
log_info "Creating necessary directories..."
mkdir -p "$SCRIPT_DIR/logs"
mkdir -p "$SCRIPT_DIR/test-results"
mkdir -p "$SCRIPT_DIR/monitoring/data"

# Build and start services
log_info "Building and starting services..."
docker-compose -f "$COMPOSE_FILE" up -d --build

# Wait for services to be healthy
log_info "Waiting for services to be healthy..."

# Function to wait for service using specialized checks
wait_for_rpc_service() {
    local service_name=$1
    local url=$2
    local method=${3:-system_health}
    local timeout=${4:-300}

    log_info "Waiting for $service_name RPC ($method) at $url"
    if tests/e2e/wait_for_rpc.sh "$url" "$method" "$timeout"; then
        log_success "$service_name is ready!"
        return 0
    else
        log_error "$service_name RPC did not become ready"
        return 1
    fi
}

wait_for_port() {
    local service_name=$1
    local host=$2
    local port=$3
    local timeout=${4:-120}

    log_info "Waiting for $service_name on $host:$port"
    local start=$(date +%s)
    local interval=1
    while true; do
        if (echo > /dev/tcp/$host/$port) >/dev/null 2>&1; then
            log_success "$service_name port $port is reachable"
            return 0
        fi
        now=$(date +%s)
        elapsed=$((now - start))
        if [ $elapsed -ge $timeout ]; then
            log_error "$service_name failed to start within $timeout seconds"
            return 1
        fi
        sleep $interval
        # exponential backoff with cap
        if [ $interval -lt 8 ]; then
            interval=$((interval * 2))
            if [ $interval -gt 8 ]; then interval=8; fi
        fi
    done
}

# Wait for critical services
echo ""
# For X3 Node use JSON-RPC health check
wait_for_rpc_service "X3 Node" "http://localhost:9933/" system_health 300
# For Redis and PostgreSQL, wait for port availability
wait_for_port "Redis" "localhost" 6379 120
wait_for_port "PostgreSQL" "localhost" 5432 120

# Wait for mock services (optional)
echo ""
log_info "Waiting for mock services..."
docker-compose -f "$COMPOSE_FILE" ps

# Show service status
log_info "Service Status:"
docker-compose -f "$COMPOSE_FILE" ps

# Display access information
echo ""
log_success "🎉 E2E Test Environment is ready!"
echo ""
echo "🔗 Service Access Points:"
echo "  📊 Grafana Dashboard: http://localhost:3000 (admin/admin)"
echo "  📈 Prometheus: http://localhost:9090"
echo "  🚨 AlertManager: http://localhost:9093"
echo "  🔗 X3 Node RPC: http://localhost:9933"
echo "  🔌 WebSocket: ws://localhost:9944"
echo "  💾 Redis: localhost:6379"
echo "  🗄️  PostgreSQL: localhost:5432 (x3_testnet/testuser/testpass)"
echo ""
echo "🚀 To run tests:"
echo "  cd $SCRIPT_DIR"
echo "  ./run_e2e_tests.sh"
echo ""
echo "🧹 To stop environment:"
echo "  ./stop_test_environment.sh"
echo ""

# Save PID for cleanup
echo $$ > "$SCRIPT_DIR/.test_env.pid"
log_success "Test environment started successfully!"
