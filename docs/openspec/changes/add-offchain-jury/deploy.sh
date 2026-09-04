#!/bin/bash
# Quick deployment script for X3 Chain Jury Service
# Usage: ./deploy.sh [dev|staging|prod] [cpu|gpu]

set -euo pipefail

ENVIRONMENT=${1:-dev}
GPU_MODE=${2:-cpu}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'  # No Color

# Functions
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
    exit 1
}

# Validation
if [[ ! "$ENVIRONMENT" =~ ^(dev|staging|prod)$ ]]; then
    log_error "Invalid environment. Use: dev|staging|prod"
fi

if [[ ! "$GPU_MODE" =~ ^(cpu|gpu)$ ]]; then
    log_error "Invalid GPU mode. Use: cpu|gpu"
fi

log_info "Deploying Jury Service"
log_info "Environment: $ENVIRONMENT"
log_info "GPU Mode: $GPU_MODE"

# Check prerequisites
log_info "Checking prerequisites..."

if ! command -v docker &> /dev/null; then
    log_error "Docker not found. Please install Docker."
fi

if ! command -v docker-compose &> /dev/null; then
    log_error "Docker Compose not found. Please install Docker Compose."
fi

log_success "Docker and Docker Compose found"

# Create environment file
log_info "Setting up environment configuration..."
ENV_FILE="$SCRIPT_DIR/.env-$ENVIRONMENT"

if [[ ! -f "$ENV_FILE" ]]; then
    log_info "Creating $ENV_FILE from template..."
    cp "$SCRIPT_DIR/jury.env.example" "$ENV_FILE"
    log_warn "Please edit $ENV_FILE with your configuration"
    nano "$ENV_FILE" || log_warn "Could not open editor. Edit $ENV_FILE manually."
fi

# Build Docker image
log_info "Building Docker image..."
if [[ "$GPU_MODE" == "gpu" ]]; then
    log_warn "Building GPU-enabled image. This may take longer..."
    docker build \
        --build-arg PYTHON_VERSION=3.11 \
        --build-arg WITH_GPU=true \
        -t jury-service:$ENVIRONMENT-gpu \
        -f "$SCRIPT_DIR/Dockerfile" \
        "$REPO_ROOT" || log_error "Docker build failed"
else
    docker build \
        --build-arg PYTHON_VERSION=3.11 \
        --build-arg WITH_GPU=false \
        -t jury-service:$ENVIRONMENT \
        -f "$SCRIPT_DIR/Dockerfile" \
        "$REPO_ROOT" || log_error "Docker build failed"
fi

log_success "Docker image built successfully"

# Deploy with docker-compose
log_info "Deploying with docker-compose..."
cd "$SCRIPT_DIR"

if [[ "$ENVIRONMENT" == "prod" ]]; then
    log_warn "Production deployment detected. Using production settings..."
    docker-compose \
        -f docker-compose.yml \
        --env-file .env-$ENVIRONMENT \
        up -d jury-db jury-cache jury-service

    log_success "Jury service deployed in production mode"
    log_info "Waiting for service to become healthy..."
    sleep 10
else
    docker-compose \
        --env-file .env-$ENVIRONMENT \
        up -d

    log_success "Jury service deployed in $ENVIRONMENT mode"
fi

# Verify deployment
log_info "Verifying deployment..."
MAX_ATTEMPTS=30
ATTEMPT=1

while [[ $ATTEMPT -le $MAX_ATTEMPTS ]]; do
    if curl -f http://localhost:8000/health &> /dev/null; then
        log_success "Jury service is healthy!"
        break
    fi
    
    if [[ $ATTEMPT -eq $MAX_ATTEMPTS ]]; then
        log_error "Service failed to become healthy after $MAX_ATTEMPTS attempts"
    fi
    
    log_info "Waiting for service to be ready... (attempt $ATTEMPT/$MAX_ATTEMPTS)"
    sleep 2
    ((ATTEMPT++))
done

# Display status
log_info "Service Status:"
docker-compose ps

# Final instructions
log_info ""
log_success "Deployment complete!"
log_info ""
log_info "Next steps:"
log_info "1. Create a jury session:"
log_info "   curl -X POST http://localhost:8000/api/jury/session \\"
log_info "     -H 'Content-Type: application/json' \\"
log_info "     -d '{\"task_ids\": [...], \"members\": [...]}'"
log_info ""
log_info "2. View logs:"
log_info "   docker-compose logs -f jury-service"
log_info ""
log_info "3. Access the database:"
log_info "   docker exec -it x3-jury-db psql -U jury_admin -d jury_audit"
log_info ""

if [[ "$ENVIRONMENT" == "dev" ]]; then
    log_info "Development mode: Prometheus metrics available at http://localhost:9090"
    log_info ""
    log_warn "To enable observability stack:"
    log_warn "  docker-compose --profile observability up -d jury-metrics"
fi
