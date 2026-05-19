#!/usr/bin/env bash
###############################################################################
# fix-gpu-drivers.sh — Fix NVIDIA GPU driver stack for x3-chain swarm
#
# Machine: 3x GTX 1070, Ubuntu 22.04, kernel 5.15.0-168
# Current: Driver 535.288.01 (working but half-configured), CUDA 12.2
#
# This script:
#   1. Fixes half-installed nvidia packages (iU state)
#   2. Purges old driver remnants (470, 590 leftovers)
#   3. Installs nvidia-container-toolkit for Docker GPU passthrough
#   4. Configures Docker to use the nvidia runtime
#   5. Enables GPU persistence mode
#   6. Validates everything works
###############################################################################
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()  { echo -e "${GREEN}[FIX-GPU]${NC} $*"; }
warn() { echo -e "${YELLOW}[FIX-GPU]${NC} $*"; }
err()  { echo -e "${RED}[FIX-GPU]${NC} $*" >&2; }

if [[ $EUID -ne 0 ]]; then
    err "This script must be run as root. Use: sudo bash $0"
    exit 1
fi

###############################################################################
# Step 1: Fix half-installed NVIDIA packages
###############################################################################
log "Step 1/6: Fixing half-installed NVIDIA packages..."

# Configure any unpacked-but-not-configured packages
dpkg --configure -a 2>&1 || true

# Fix broken dependencies
apt-get -f install -y 2>&1

# Verify the problematic packages are now fully installed
for pkg in nvidia-driver-535 xserver-xorg-video-nvidia-535; do
    status=$(dpkg-query -W -f='${Status}' "$pkg" 2>/dev/null || echo "not-installed")
    if [[ "$status" == "install ok installed" ]]; then
        log "  $pkg: OK"
    else
        warn "  $pkg still not clean ($status), attempting reinstall..."
        apt-get install --reinstall -y "$pkg" 2>&1 || true
    fi
done

###############################################################################
# Step 2: Purge old driver remnants
###############################################################################
log "Step 2/6: Purging old driver remnants (470, 590 leftovers)..."

# Get list of removed-but-config-remaining nvidia packages
old_pkgs=$(dpkg -l | grep -E '^rc.*nvidia|^rc.*libnvidia' | awk '{print $2}' || true)
if [[ -n "$old_pkgs" ]]; then
    echo "$old_pkgs" | xargs dpkg --purge 2>&1 || true
    log "  Purged old config files"
else
    log "  No old remnants to purge"
fi

# Also clean up any 590 leftovers that are cluttering things
for pkg in libnvidia-cfg1 libnvidia-gpucomp nvidia-firmware; do
    installed_ver=$(dpkg-query -W -f='${Version}' "$pkg" 2>/dev/null || true)
    if [[ "$installed_ver" == *"590"* ]]; then
        warn "  Removing 590-series leftover: $pkg ($installed_ver)"
        apt-get remove -y "$pkg" 2>&1 || true
    fi
done

###############################################################################
# Step 3: Install nvidia-container-toolkit
###############################################################################
log "Step 3/6: Installing nvidia-container-toolkit..."

# Check if already installed
if dpkg -l nvidia-container-toolkit 2>/dev/null | grep -q '^ii'; then
    log "  nvidia-container-toolkit already installed"
else
    # Add NVIDIA container toolkit repository
    if [[ ! -f /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg ]]; then
        log "  Adding NVIDIA container toolkit repository..."
        curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | \
            gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg

        curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
            sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
            tee /etc/apt/sources.list.d/nvidia-container-toolkit.list > /dev/null
    fi

    apt-get update -qq 2>&1
    apt-get install -y nvidia-container-toolkit 2>&1
    log "  nvidia-container-toolkit installed"
fi

###############################################################################
# Step 4: Configure Docker nvidia runtime
###############################################################################
log "Step 4/6: Configuring Docker nvidia runtime..."

# Use nvidia-ctk to configure Docker
nvidia-ctk runtime configure --runtime=docker 2>&1

# Ensure the daemon.json has nvidia as default runtime for swarm containers
DAEMON_JSON="/etc/docker/daemon.json"
if [[ -f "$DAEMON_JSON" ]]; then
    # Check if default-runtime is set
    if ! grep -q '"default-runtime"' "$DAEMON_JSON"; then
        log "  Setting nvidia as default Docker runtime..."
        # Use python3 to safely merge JSON
        python3 -c "
import json
with open('$DAEMON_JSON') as f:
    config = json.load(f)
config['default-runtime'] = 'nvidia'
with open('$DAEMON_JSON', 'w') as f:
    json.dump(config, f, indent=2)
" 2>&1
    else
        log "  Default runtime already configured"
    fi
else
    # Create from scratch
    cat > "$DAEMON_JSON" << 'EOF'
{
  "default-runtime": "nvidia",
  "runtimes": {
    "nvidia": {
      "args": [],
      "path": "nvidia-container-runtime"
    }
  }
}
EOF
fi

log "  Restarting Docker daemon..."
systemctl restart docker 2>&1
sleep 2

###############################################################################
# Step 5: Enable GPU persistence mode
###############################################################################
log "Step 5/6: Enabling GPU persistence mode..."

# Enable persistence mode on all GPUs (reduces first-access latency)
nvidia-smi -pm 1 2>&1 || warn "  Could not enable persistence mode (non-critical)"

# Enable the persistence daemon for boot
if systemctl is-enabled nvidia-persistenced &>/dev/null; then
    log "  nvidia-persistenced already enabled"
else
    # Install if missing
    if ! command -v nvidia-persistenced &>/dev/null; then
        apt-get install -y nvidia-persistenced 2>&1 || true
    fi
    systemctl enable nvidia-persistenced 2>&1 || true
    systemctl start nvidia-persistenced 2>&1 || true
    log "  nvidia-persistenced enabled and started"
fi

###############################################################################
# Step 6: Validate everything
###############################################################################
log "Step 6/6: Validating GPU stack..."

echo ""
echo "=========================================="
echo "  NVIDIA Driver Validation"
echo "=========================================="

# Driver
echo -n "  Driver:            "
nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1

# GPU count
gpu_count=$(nvidia-smi --query-gpu=name --format=csv,noheader | wc -l)
echo "  GPUs detected:     $gpu_count"

# CUDA
echo -n "  CUDA version:      "
nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1 > /dev/null
nvcc --version 2>/dev/null | grep "release" | awk '{print $6}' | tr -d ',' || echo "nvcc not in PATH (runtime libs OK)"

# Persistence
echo -n "  Persistence mode:  "
nvidia-smi --query-gpu=persistence_mode --format=csv,noheader | head -1

# Container toolkit
echo -n "  Container toolkit: "
nvidia-ctk --version 2>/dev/null || echo "NOT INSTALLED"

# Docker GPU test
echo -n "  Docker GPU access: "
if docker run --rm --gpus all nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1; then
    echo -e "  ${GREEN}Docker can see GPUs!${NC}"
else
    # Try with runtime flag
    if docker run --rm --runtime=nvidia nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | head -1; then
        echo -e "  ${GREEN}Docker can see GPUs (via runtime flag)!${NC}"
    else
        err "  Docker CANNOT access GPUs. Check: docker run --rm --gpus all nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi"
    fi
fi

# Package health
echo ""
echo "  Package health:"
broken=$(dpkg -l | grep -E '^(iF|iU|iW)' | grep -iE 'nvidia|cuda' | wc -l)
if [[ "$broken" -eq 0 ]]; then
    echo -e "    ${GREEN}All NVIDIA/CUDA packages healthy${NC}"
else
    echo -e "    ${RED}$broken broken package(s) remain:${NC}"
    dpkg -l | grep -E '^(iF|iU|iW)' | grep -iE 'nvidia|cuda' | awk '{print "      "$2, $3}'
fi

echo ""
echo "=========================================="
echo ""

# Per-GPU summary
log "Per-GPU summary:"
nvidia-smi --query-gpu=index,name,memory.total,persistence_mode,power.draw --format=csv,noheader 2>/dev/null | while IFS=',' read -r idx name mem persist power; do
    echo "  GPU $idx:$name |$mem | Persist:$persist | Power:$power"
done

echo ""
log "Done! Your 3x GTX 1070 setup should now be ready for the swarm."
log ""
log "Next steps:"
log "  1. Test: docker run --rm --gpus all nvidia/cuda:12.2.0-base-ubuntu22.04 nvidia-smi"
log "  2. Start swarm: docker compose -f crates/gpu-swarm/docker/docker-compose.yml up"
log "  3. Set GPU_BACKEND=cuda in your environment"
