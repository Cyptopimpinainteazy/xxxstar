#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# install-validator.sh — Install X3 validator binary and systemd service
#
# Intended for bare-metal or VPS validator hosts.
# Does NOT use Docker. Produces a systemd-managed validator.
#
# Usage:
#   sudo bash scripts/install-validator.sh [--version v0.4.0-rc.1]
#
# Options:
#   --version TAG    GitHub release tag to download (default: latest)
#   --chain FILE     Path to chain-spec JSON (default: auto-detect from release)
#   --help           Show this help
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

REPO="Cyptopimpinainteazy/xxxstar"
DEFAULT_VERSION="latest"
BINARY="x3-chain-node"
INSTALL_DIR="/usr/local/bin"
DATA_DIR="/var/lib/x3"
CONFIG_DIR="/etc/x3"
LOG_DIR="/var/log/x3"
USER="x3"

# ── Parse arguments ──────────────────────────────────────────────────────
VERSION="${DEFAULT_VERSION}"
CHAIN_SPEC=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --chain)   CHAIN_SPEC="$2"; shift 2 ;;
    --help)    grep "^#" "$0" | grep -v "^#!" | sed 's/^# //'; exit 0 ;;
    *)         echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ── Pre-flight checks ───────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
  echo "ERROR: This script must be run as root."
  exit 1
fi

# ── Resolve latest version if needed ────────────────────────────────────
if [[ "$VERSION" == "latest" ]]; then
  echo "==> Resolving latest release from GitHub..."
  VERSION=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
  if [[ -z "$VERSION" ]]; then
    echo "ERROR: Could not resolve latest release. Specify --version explicitly."
    exit 1
  fi
  echo "    Latest release: ${VERSION}"
fi

echo "==> Installing X3 validator ${VERSION}..."

# ── Create user if missing ──────────────────────────────────────────────
if ! id -u "${USER}" &>/dev/null; then
  echo "==> Creating system user '${USER}'..."
  useradd --system --no-create-home --shell /sbin/nologin "${USER}"
fi

# ── Create directories ──────────────────────────────────────────────────
mkdir -p "${INSTALL_DIR}" "${DATA_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"

# ── Download binary ─────────────────────────────────────────────────────
BINARY_URL="https://github.com/${REPO}/releases/download/${VERSION}/${BINARY}"
CHECKSUM_URL="${BINARY_URL}.sha256"

echo "==> Downloading binary..."
curl -L -o "${INSTALL_DIR}/${BINARY}" "${BINARY_URL}" --fail --silent --show-error
chmod +x "${INSTALL_DIR}/${BINARY}"

echo "==> Verifying checksum..."
curl -L -o "/tmp/${BINARY}.sha256" "${CHECKSUM_URL}" --fail --silent --show-error 2>/dev/null || true
if [[ -f "/tmp/${BINARY}.sha256" ]]; then
  (cd "${INSTALL_DIR}" && sha256sum -c "/tmp/${BINARY}.sha256")
  rm -f "/tmp/${BINARY}.sha256"
  echo "    Checksum: PASS"
else
  echo "    WARNING: No checksum file found. Skipping verification."
fi

echo "    Binary installed: ${INSTALL_DIR}/${BINARY}"

# ── Download chain spec if not provided ────────────────────────────────
if [[ -z "$CHAIN_SPEC" ]]; then
  SPEC_URL="https://github.com/${REPO}/releases/download/${VERSION}/x3-mainnet-raw.json"
  echo "==> Downloading chain spec..."
  curl -L -o "${CONFIG_DIR}/chain-spec.json" "${SPEC_URL}" --fail --silent --show-error || {
    echo "    WARNING: No chain spec in release. You must manually place one at ${CONFIG_DIR}/chain-spec.json"
  }
else
  echo "==> Using provided chain spec: ${CHAIN_SPEC}"
  cp "${CHAIN_SPEC}" "${CONFIG_DIR}/chain-spec.json"
fi

# ── Install systemd service ─────────────────────────────────────────────
SERVICE_FILE="packaging/systemd/x3-validator.service"
if [[ -f "${SERVICE_FILE}" ]]; then
  echo "==> Installing systemd service..."
  cp "${SERVICE_FILE}" /etc/systemd/system/x3-validator.service
  systemctl daemon-reload
  echo "    Service installed: x3-validator.service"
else
  echo "    WARNING: Service file ${SERVICE_FILE} not found. Skipping systemd installation."
fi

# ── Set ownership ───────────────────────────────────────────────────────
chown -R "${USER}:${USER}" "${DATA_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"

# ── Summary ─────────────────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  X3 Validator Installation Complete              ║"
echo "╠══════════════════════════════════════════════════╣"
echo "║  Binary:     ${INSTALL_DIR}/${BINARY}"
echo "║  Version:    ${VERSION}"
echo "║  Data:       ${DATA_DIR}"
echo "║  Config:     ${CONFIG_DIR}/chain-spec.json"
echo "║  Service:    x3-validator.service"
echo "║                                                  ║"
echo "║  Start:   systemctl start x3-validator           ║"
echo "║  Enable:  systemctl enable x3-validator          ║"
echo "║  Status:  systemctl status x3-validator          ║"
echo "║  Logs:    journalctl -fu x3-validator            ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""
echo "Before starting:"
echo "  1. Generate session keys: ${INSTALL_DIR}/${BINARY} key generate"
echo "  2. Configure firewall: ufw allow 30333/tcp"
echo "  3. Mount NVMe storage at ${DATA_DIR} if available"
echo "  4. Run hardening: bash scripts/harden-validator.sh"