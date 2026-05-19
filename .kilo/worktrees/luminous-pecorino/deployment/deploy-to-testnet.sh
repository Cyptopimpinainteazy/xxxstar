#!/usr/bin/env bash

# X3 Chain v1.1 Testnet Deployment Automation
# Orchestrates release package extraction, verification, and node startup
# 
# Usage:
#   ./deploy-to-testnet.sh <testnet-host> [--validator] [--bootnode <peer-id>]
#
# Examples:
#   # Bootstrap validator on fresh testnet
#   ./deploy-to-testnet.sh testnet.x3-chain.io --validator
#
#   # Full-sync node joining existing testnet
#   ./deploy-to-testnet.sh validator-2.testnet.x3-chain.io
#
#   # Validator with bootnode peer
#   ./deploy-to-testnet.sh validator-3.testnet.x3-chain.io --validator \
#     --bootnode "/ip4/192.168.1.1/tcp/30333/p2p/12D3KooW..."

set -euo pipefail

# ============ COLORS ============
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ============ CONFIGURATION ============
RELEASE_NAME="x3-chain-v1.1"
RELEASE_TARBALL="${RELEASE_NAME}-release.tar.gz"
CHECKSUMS_FILE="CHECKSUMS.sha256"
SIGNATURE_FILE="CHECKSUMS.sha256.asc"

TARGET_HOST="${1:-}"
VALIDATOR_MODE=false
BOOTNODE=""
DEPLOY_PATH="/opt/x3-chain"
SERVICE_NAME="x3-chain-node"
REMOTE_STAGE="/tmp/x3-chain-v1.1-deploy"

# ============ FUNCTIONS ============

log_info() {
  echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
  echo -e "${GREEN}[OK]${NC} $*"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $*"
}

usage() {
  cat << EOF
X3 Chain v1.1 Testnet Deployment

Usage: $0 <target-host> [OPTIONS]

Positional:
  <target-host>     SSH host to deploy to (e.g., validator-1.testnet)

Options:
  --validator       Start as validator (requires registration on chain)
  --bootnode <addr> Connect to existing validator as peer
  --path <dir>      Deployment directory (default: /opt/x3-chain)
  --help            Show this message

Prerequisites:
  - SSH access to target host with sudo privileges
  - Release artifacts (tarball, checksums, signature) in current directory
  - GPG installed on target host

Examples:
  # Bootstrap validator
  $0 validator-1.testnet.x3-chain.io --validator

  # Join existing network
  $0 validator-2.testnet.x3-chain.io --validator \\
    --bootnode "/ip4/10.0.0.1/tcp/30333/p2p/12D3KooW..."
EOF
  exit 1
}

# ============ ARGUMENT PARSING ============
if [[ -z "$TARGET_HOST" ]]; then
  usage
fi

while [[ $# -gt 1 ]]; do
  case $2 in
    --validator)
      VALIDATOR_MODE=true
      shift
      ;;
    --bootnode)
      BOOTNODE="$3"
      shift 2
      ;;
    --path)
      DEPLOY_PATH="$3"
      shift 2
      ;;
    --help)
      usage
      ;;
    *)
      log_error "Unknown option: $2"
      usage
      ;;
  esac
done

# ============ PRE-FLIGHT CHECKS ============
log_info "Performing pre-flight checks..."

# Check local artifacts
for file in "$RELEASE_TARBALL" "$CHECKSUMS_FILE" "$SIGNATURE_FILE"; do
  if [[ ! -f "$file" ]]; then
    log_error "Missing local artifact: $file"
    exit 1
  fi
done
log_success "All release artifacts present locally"

# Verify checksums locally
log_info "Verifying local checksums..."
if ! sha256sum -c "$CHECKSUMS_FILE" > /dev/null 2>&1; then
  log_error "Checksum verification failed"
  exit 1
fi
log_success "Checksums verified"

# Verify GPG signature locally
log_info "Verifying GPG signature..."
if ! gpg --verify "$SIGNATURE_FILE" "$CHECKSUMS_FILE" > /dev/null 2>&1; then
  log_error "GPG signature verification failed"
  exit 1
fi
log_success "GPG signature verified"

# Test SSH connectivity
log_info "Testing SSH connectivity to $TARGET_HOST..."
if ! ssh -o ConnectTimeout=5 "$TARGET_HOST" "echo 'SSH connection OK'" > /dev/null 2>&1; then
  log_error "Cannot connect to $TARGET_HOST via SSH"
  exit 1
fi
log_success "SSH connectivity confirmed"

log_info "Creating remote staging directory..."
ssh "$TARGET_HOST" "rm -rf '$REMOTE_STAGE' && mkdir -p '$REMOTE_STAGE'"

log_info "Copying release artifacts to remote host..."
scp "$RELEASE_TARBALL" "$CHECKSUMS_FILE" "$SIGNATURE_FILE" "$TARGET_HOST:$REMOTE_STAGE/" > /dev/null
log_success "Artifacts copied to $TARGET_HOST:$REMOTE_STAGE"

# ============ REMOTE PRE-FLIGHT ============
log_info "Preparing remote deployment environment..."

ssh "$TARGET_HOST" bash << 'REMOTE_PREFLIGHT'
set -euo pipefail

# Check Rust/Cargo
if ! command -v cargo &> /dev/null; then
  echo "ERROR: cargo not found on remote host" >&2
  exit 1
fi

# Check GPG
if ! command -v gpg &> /dev/null; then
  echo "ERROR: gpg not found on remote host" >&2
  exit 1
fi

# Check systemctl (for service management)
if ! command -v systemctl &> /dev/null; then
  echo "WARNING: systemctl not found; manual service management required" >&2
fi

echo "Remote pre-flight checks passed"
REMOTE_PREFLIGHT

log_success "Remote environment validated"

# ============ DEPLOYMENT ============
log_info "Deploying X3 Chain v1.1 to $TARGET_HOST..."

# Create deployment script
DEPLOY_SCRIPT=$(mktemp)
cat > "$DEPLOY_SCRIPT" << DEPLOY_COMMANDS
#!/usr/bin/env bash
set -euo pipefail

DEPLOY_PATH="$DEPLOY_PATH"
RELEASE_TARBALL="$RELEASE_TARBALL"
CHECKSUMS_FILE="$CHECKSUMS_FILE"
SIGNATURE_FILE="$SIGNATURE_FILE"
VALIDATOR_MODE=$VALIDATOR_MODE
BOOTNODE="$BOOTNODE"
SERVICE_NAME="$SERVICE_NAME"
REMOTE_STAGE="$REMOTE_STAGE"

EXTRA_ARGS=""
if [[ "$VALIDATOR_MODE" == "true" ]]; then
  EXTRA_ARGS="$EXTRA_ARGS --validator"
fi
if [[ -n "$BOOTNODE" ]]; then
  EXTRA_ARGS="$EXTRA_ARGS --bootnodes $BOOTNODE"
fi

mkdir -p "\$DEPLOY_PATH"
cd "\$REMOTE_STAGE"
echo "[OK] Using staged artifacts in \$REMOTE_STAGE"

# Verify integrity on remote
if ! sha256sum -c "\$CHECKSUMS_FILE" > /dev/null 2>&1; then
  echo "Error: Remote checksum verification failed" >&2
  exit 1
fi
echo "[OK] Checksums verified on remote"

if ! gpg --verify "\$SIGNATURE_FILE" "\$CHECKSUMS_FILE" > /dev/null 2>&1; then
  echo "Error: GPG signature verification failed on remote" >&2
  exit 1
fi
echo "[OK] GPG signature verified on remote"

# Extract release
rm -rf "\$DEPLOY_PATH"/*
tar -xzf "\$RELEASE_TARBALL" -C "\$DEPLOY_PATH"
echo "[OK] Release package extracted"

# Verify extracted binary
if [[ ! -f "\$DEPLOY_PATH/x3-chain-node" ]] || [[ ! -x "\$DEPLOY_PATH/x3-chain-node" ]]; then
  echo "Error: Invalid or missing binary in release package" >&2
  exit 1
fi
chmod +x "\$DEPLOY_PATH/x3-chain-node" "\$DEPLOY_PATH"/scripts/*.sh
echo "[OK] Binary verified and scripts executable"

if ! id -u x3 > /dev/null 2>&1; then
  sudo useradd --system --create-home --home-dir /var/lib/x3-chain --shell /usr/sbin/nologin x3
  echo "[OK] Created system user x3"
fi

sudo mkdir -p /var/lib/x3-chain
sudo chown -R x3:x3 /var/lib/x3-chain "\$DEPLOY_PATH"

# Create systemd service (if systemctl available)
if command -v systemctl &> /dev/null; then
  sudo tee /etc/systemd/system/\$SERVICE_NAME.service > /dev/null <<SYSTEMD_UNIT
[Unit]
Description=X3 Chain Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=x3
WorkingDirectory=/opt/x3-chain
ExecStart=/opt/x3-chain/x3-chain-node \\
  --chain=testnet \\
  --base-path=/var/lib/x3-chain \\
  --rpc-port=9944 \\
  --rpc-external \\
  --prometheus-external $EXTRA_ARGS
ExecStop=/bin/kill -SIGTERM \$MAINPID
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
SYSTEMD_UNIT
  sudo systemctl daemon-reload
  echo "[OK] Systemd service installed"
fi

echo ""
echo "==== DEPLOYMENT COMPLETE ===="
echo "Location: \$DEPLOY_PATH"
echo "Binary: \$DEPLOY_PATH/x3-chain-node"
echo "Start: systemctl start $SERVICE_NAME"
echo ""

rm -rf "\$REMOTE_STAGE"

DEPLOY_COMMANDS

# Execute deployment on remote
log_info "Executing remote deployment..."
if ! scp "$DEPLOY_SCRIPT" "$TARGET_HOST:/tmp/deploy.sh" > /dev/null 2>&1; then
  log_error "Failed to copy deployment script to remote"
  rm "$DEPLOY_SCRIPT"
  exit 1
fi

ssh "$TARGET_HOST" bash /tmp/deploy.sh

log_success "Deployment complete!"

# ============ POST-DEPLOYMENT ============
log_info "Running post-deployment validation..."

ssh "$TARGET_HOST" bash << 'POST_VALIDATION'
set -euo pipefail

DEPLOY_PATH="/opt/x3-chain"

# Health check
if command -v bash &> /dev/null && [[ -f "$DEPLOY_PATH/scripts/x3_node_healthcheck.sh" ]]; then
  echo "[INFO] Running health check..."
  NODE_NAME=testnet-validator bash "$DEPLOY_PATH/scripts/x3_node_healthcheck.sh" --mode prod
  echo "[OK] Health check completed"
fi

# Start service (if systemctl available)
if command -v systemctl &> /dev/null; then
  echo "[INFO] Starting x3-chain-node service..."
  sudo systemctl start x3-chain-node || echo "Note: Service start may require manual intervention"
  sleep 2
  if sudo systemctl is-active x3-chain-node > /dev/null 2>&1; then
    echo "[OK] Service is running"
  else
    echo "[WARN] Service failed to start; check logs with: journalctl -u x3-chain-node -f"
  fi
fi

echo ""
echo "==== POST-DEPLOYMENT CHECKS ===="
echo "View logs: journalctl -u x3-chain-node -f"
echo "RPC health: curl http://localhost:9944/health"
echo "Operator SOP: $DEPLOY_PATH/docs/X3_OPERATOR_SOP.md"
echo ""

POST_VALIDATION

log_success "X3 Chain v1.1 successfully deployed to $TARGET_HOST"

# Cleanup
rm "$DEPLOY_SCRIPT"

echo ""
echo "Next steps:"
echo "  1. Monitor node startup: ssh $TARGET_HOST journalctl -u x3-chain-node -f"
echo "  2. Verify consensus: curl http://$TARGET_HOST:9944/system_health"
echo "  3. For multi-validator setup, configure bootnodes per X3_OPERATOR_SOP.md"
