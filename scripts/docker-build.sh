#!/bin/bash
# Build Docker images for X3 Chain infrastructure
# Usage: ./scripts/docker-build.sh [validator|indexer|all] [version] [registry]
#
# Examples:
#   ./scripts/docker-build.sh all latest docker.io/x3-chain
#   ./scripts/docker-build.sh validator v1.0.0 ghcr.io/x3-chain
#   ./scripts/docker-build.sh indexer latest localhost:5000

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# ── Configuration ────────────────────────────────────────────────────────
TARGET="${1:-all}"
VERSION="${2:-latest}"
REGISTRY="${3:-docker.io/x3-chain}"
PLATFORMS="linux/amd64,linux/arm64"  # Multi-arch support

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# ── Validation ───────────────────────────────────────────────────────────
if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
    log_error "Cargo.toml not found. Run this script from workspace root."
    exit 1
fi

if ! command -v docker &> /dev/null; then
    log_error "docker not found. Install Docker to build images."
    exit 1
fi

log_info "Building X3 Chain Docker images"
log_info "Target: $TARGET | Version: $VERSION | Registry: $REGISTRY"

# ── Build Functions ──────────────────────────────────────────────────────
build_validator() {
    local image_tag="${REGISTRY}/x3-chain-node:${VERSION}"
    log_info "Building validator image: $image_tag"
    
    docker build \
        -f "$PROJECT_ROOT/Dockerfile.validator" \
        -t "$image_tag" \
        --build-arg RUST_VERSION=1.80 \
        --platform "$PLATFORMS" \
        "$PROJECT_ROOT"
    
    # Also tag as 'latest'
    docker tag "$image_tag" "${REGISTRY}/x3-chain-node:latest"
    
    log_info "Validator image built successfully"
    docker images | grep x3-chain-node
}

build_indexer() {
    local image_tag="${REGISTRY}/x3-indexer:${VERSION}"
    log_info "Building indexer image: $image_tag"
    
    docker build \
        -f "$PROJECT_ROOT/Dockerfile.indexer" \
        -t "$image_tag" \
        --build-arg RUST_VERSION=1.80 \
        --platform "$PLATFORMS" \
        "$PROJECT_ROOT"
    
    # Also tag as 'latest'
    docker tag "$image_tag" "${REGISTRY}/x3-indexer:latest"
    
    log_info "Indexer image built successfully"
    docker images | grep x3-indexer
}

push_images() {
    log_info "Pushing images to registry: $REGISTRY"
    
    docker push "${REGISTRY}/x3-chain-node:${VERSION}"
    docker push "${REGISTRY}/x3-chain-node:latest"
    
    docker push "${REGISTRY}/x3-indexer:${VERSION}"
    docker push "${REGISTRY}/x3-indexer:latest"
    
    log_info "All images pushed successfully"
}

# ── Execution ────────────────────────────────────────────────────────────
case "$TARGET" in
    validator)
        build_validator
        ;;
    indexer)
        build_indexer
        ;;
    all)
        build_validator
        build_indexer
        ;;
    *)
        log_error "Invalid target: $TARGET. Use: validator | indexer | all"
        exit 1
        ;;
esac

log_info "Build complete. Images ready for deployment."
log_info "Next: Update Kubernetes manifests with image tags:"
log_info "  - Validator: ${REGISTRY}/x3-chain-node:${VERSION}"
log_info "  - Indexer:   ${REGISTRY}/x3-indexer:${VERSION}"
