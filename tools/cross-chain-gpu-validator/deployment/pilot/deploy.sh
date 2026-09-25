#!/usr/bin/env bash
# ── X3 Chain – Pilot Deployment Script ───────────────────────────────────
# Deploy multi-chain X3 GPU validator to Threadripper core + secondary fleet
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KERNELS_DIR="$PROJECT_ROOT/cross-chain-gpu-validator/kernels"
BUILD_DIR="$KERNELS_DIR/build"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'

banner() { echo -e "\n${CYAN}═══ $1 ═══${NC}"; }
ok()     { echo -e "${GREEN}✓ $1${NC}"; }
warn()   { echo -e "${YELLOW}⚠ $1${NC}"; }
fail()   { echo -e "${RED}✗ $1${NC}"; exit 1; }

# ── Pre-flight checks ────────────────────────────────────────────────────────
banner "Pre-flight checks"

command -v nvidia-smi >/dev/null 2>&1 || fail "nvidia-smi not found — NVIDIA driver required"
command -v docker      >/dev/null 2>&1 || fail "docker not found"
command -v cargo        >/dev/null 2>&1 || fail "cargo not found"

GPU_COUNT=$(nvidia-smi --list-gpus 2>/dev/null | wc -l)
echo "  Detected $GPU_COUNT GPU(s)"
[[ "$GPU_COUNT" -ge 1 ]] || fail "No GPUs detected"

# ── Build CUDA kernels ───────────────────────────────────────────────────────
banner "Building CUDA kernels"

if [[ ! -f "$BUILD_DIR/libsecp256k1_batch.so" ]]; then
    pushd "$KERNELS_DIR" >/dev/null
    bash build.sh
    popd >/dev/null
    ok "All 5 GPU kernels compiled"
else
    ok "Kernels already built (use --rebuild to force)"
    if [[ "${1:-}" == "--rebuild" ]]; then
        pushd "$KERNELS_DIR" >/dev/null
        bash build.sh
        popd >/dev/null
        ok "Kernels rebuilt"
    fi
fi

# Verify all .so files
for lib in libsecp256k1_batch.so libkeccak256_batch.so libsha256_batch.so libed25519_batch.so libstream_pipeline.so; do
    [[ -f "$BUILD_DIR/$lib" ]] || fail "Missing $BUILD_DIR/$lib"
done
ok "All 5 kernel libraries verified"

# ── Build Rust workspace ─────────────────────────────────────────────────────
banner "Building Rust workspace"

pushd "$PROJECT_ROOT" >/dev/null
cargo build --release -p x3-bench -p x3-vm -p x3-backend 2>&1 | tail -5
popd >/dev/null
ok "Rust workspace built"

# ── Deploy core node (local Threadripper) ────────────────────────────────────
banner "Deploying core node (local)"

DEPLOY_TARGET="/opt/x3/pilot"
sudo mkdir -p "$DEPLOY_TARGET/kernels" "$DEPLOY_TARGET/config" "$DEPLOY_TARGET/logs"

# Copy kernel libraries
sudo cp "$BUILD_DIR"/*.so "$DEPLOY_TARGET/kernels/"
ok "Kernel libraries installed to $DEPLOY_TARGET/kernels/"

# Copy configs
sudo cp "$SCRIPT_DIR/threadripper.toml" "$DEPLOY_TARGET/config/"
ok "Core node config installed"

# Copy Rust binaries
if [[ -f "$PROJECT_ROOT/target/release/x3-bench" ]]; then
    sudo cp "$PROJECT_ROOT/target/release/x3-bench" "$DEPLOY_TARGET/"
    ok "x3-bench binary installed"
fi

# Set LD_LIBRARY_PATH in environment file
cat <<EOF | sudo tee "$DEPLOY_TARGET/env.sh" >/dev/null
export LD_LIBRARY_PATH="$DEPLOY_TARGET/kernels:\${LD_LIBRARY_PATH:-}"
export X3_CONFIG="$DEPLOY_TARGET/config/threadripper.toml"
export X3_LOG_DIR="$DEPLOY_TARGET/logs"
export CUDA_VISIBLE_DEVICES=0,1,2
EOF
ok "Environment file created"

# ── Docker Compose for pilot ─────────────────────────────────────────────────
banner "Starting pilot services (Docker Compose)"

COMPOSE_FILE="$SCRIPT_DIR/docker-compose.pilot.yml"
if [[ -f "$COMPOSE_FILE" ]]; then
    docker compose -f "$COMPOSE_FILE" up -d --build 2>&1 | tail -10
    ok "Pilot services started"
else
    warn "docker-compose.pilot.yml not found — skipping container deployment"
    echo "  Run manually: docker compose -f $COMPOSE_FILE up -d"
fi

# ── Deploy secondary nodes (SSH) ─────────────────────────────────────────────
banner "Deploying secondary nodes"

SECONDARY_CONF="$SCRIPT_DIR/secondary_nodes.toml"
if [[ -f "$SECONDARY_CONF" ]]; then
    # Extract node hosts from TOML
    HOSTS=$(grep '^host' "$SECONDARY_CONF" | sed 's/.*= *"\(.*\)"/\1/')
    for HOST in $HOSTS; do
        echo -n "  Deploying to $HOST... "
        if ssh -o ConnectTimeout=5 "x3@$HOST" "mkdir -p /opt/x3/pilot/kernels /opt/x3/pilot/config" 2>/dev/null; then
            scp -q "$BUILD_DIR"/*.so "x3@$HOST:/opt/x3/pilot/kernels/"
            scp -q "$SECONDARY_CONF" "x3@$HOST:/opt/x3/pilot/config/"
            ok "$HOST"
        else
            warn "Cannot reach $HOST — skipping"
        fi
    done
else
    warn "secondary_nodes.toml not found — skipping fleet deployment"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
banner "Pilot Deployment Complete"
echo ""
echo "  Core node:       local (Threadripper, $GPU_COUNT GPUs)"
echo "  Kernel dir:      $DEPLOY_TARGET/kernels/"
echo "  Config:          $DEPLOY_TARGET/config/threadripper.toml"
echo "  Dashboard:       http://localhost:8080"
echo "  Prometheus:      http://localhost:9100/metrics"
echo ""
echo "  Quick test:      source $DEPLOY_TARGET/env.sh && x3-bench --gpu"
echo ""
