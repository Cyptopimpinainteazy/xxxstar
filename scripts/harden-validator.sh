#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# harden-validator.sh — Security hardening for X3 validator hosts
#
# Apply system-level security measures for a bare-metal validator node.
# Should be run AFTER install-validator.sh but BEFORE starting the service.
#
# Usage: sudo bash scripts/harden-validator.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  echo "ERROR: This script must be run as root."
  exit 1
fi

echo "==> X3 Validator Security Hardening"

# ── 1. Firewall: allow only P2P and SSH ─────────────────────────────────
echo "[1/6] Configuring firewall..."
if command -v ufw &>/dev/null; then
  ufw --force reset
  ufw default deny incoming
  ufw default allow outgoing
  ufw allow ssh
  ufw allow 30333/tcp comment 'X3 P2P'
  ufw limit ssh
  ufw --force enable
  echo "    ufw: enabled with P2P (30333) + SSH"
elif command -v firewall-cmd &>/dev/null; then
  firewall-cmd --permanent --add-port=30333/tcp
  firewall-cmd --permanent --remove-service=ssh --add-rich-rule='rule family="ipv4" source address="YOUR-MGMT-CIDR" service name="ssh" accept'
  firewall-cmd --reload
  echo "    firewalld: configured for P2P (30333)"
else
  echo "    WARNING: No firewall tool found. Install ufw or firewalld."
fi

# ── 2. Kernel hardening (sysctl) ────────────────────────────────────────
echo "[2/6] Applying kernel hardening..."
cat >> /etc/sysctl.d/90-x3-validator.conf <<'EOF'
# X3 Validator — kernel hardening

# IP spoofing protection
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1

# Ignore ICMP redirects
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv6.conf.all.accept_redirects = 0

# Ignore source-routed packets
net.ipv4.conf.all.accept_source_route = 0
net.ipv6.conf.all.accept_source_route = 0

# Disable ICMP redirect sending
net.ipv4.conf.all.send_redirects = 0

# SYN flood protection
net.ipv4.tcp_syncookies = 1
net.ipv4.tcp_syn_retries = 2

# Increase ephemeral port range (validators make many outbound P2P connections)
net.ipv4.ip_local_port_range = 16384 65535

# Increase backlog for high-traffic nodes
net.core.somaxconn = 65536
net.ipv4.tcp_max_syn_backlog = 65536

# Reduce TIME_WAIT sockets
net.ipv4.tcp_fin_timeout = 15
EOF
sysctl -p /etc/sysctl.d/90-x3-validator.conf
echo "    sysctl: hardening applied"

# ── 3. Disable unnecessary services ─────────────────────────────────────
echo "[3/6] Disabling unnecessary services..."
for svc in avahi-daemon cups bluetooth postfix nfs-server rpcbind; do
  systemctl disable --now "$svc" 2>/dev/null || true
done
echo "    Unnecessary services disabled"

# ── 4. Filesystem mount hardening ───────────────────────────────────────
echo "[4/6] Checking filesystem mounts..."
if grep -q " /tmp " /proc/mounts 2>/dev/null; then
  # Remount /tmp with noexec,nosuid,nodev if not already
  mount -o remount,noexec,nosuid,nodev /tmp 2>/dev/null || true
  echo "    /tmp remounted with security options"
fi

# ── 5. SSH hardening ────────────────────────────────────────────────────
echo "[5/6] Hardening SSH..."
SSHD_CONFIG="/etc/ssh/sshd_config"
if [[ -f "$SSHD_CONFIG" ]]; then
  sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' "$SSHD_CONFIG"
  sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' "$SSHD_CONFIG"
  sed -i 's/^#\?ChallengeResponseAuthentication.*/ChallengeResponseAuthentication no/' "$SSHD_CONFIG"
  sed -i 's/^#\?UsePAM.*/UsePAM no/' "$SSHD_CONFIG"
  sed -i 's/^#\?MaxAuthTries.*/MaxAuthTries 3/' "$SSHD_CONFIG"
  sed -i 's/^#\?ClientAliveInterval.*/ClientAliveInterval 300/' "$SSHD_CONFIG"
  sed -i 's/^#\?ClientAliveCountMax.*/ClientAliveCountMax 2/' "$SSHD_CONFIG"

  # Only allow SSH key auth
  if ! grep -q "^AuthenticationMethods" "$SSHD_CONFIG"; then
    echo "AuthenticationMethods publickey" >> "$SSHD_CONFIG"
  fi

  systemctl restart sshd
  echo "    SSH hardened: key-only auth, no root password login"
fi

# ── 6. Logging and auditing ─────────────────────────────────────────────
echo "[6/6] Configuring log rotation..."
cat > /etc/logrotate.d/x3-validator <<'EOF'
/var/log/x3/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0640 x3 x3
    sharedscripts
    postrotate
        systemctl kill -s USR1 x3-validator.service 2>/dev/null || true
    endscript
}
EOF
echo "    Log rotation configured at /etc/logrotate.d/x3-validator"

# ── Summary ─────────────────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  Security Hardening Complete                     ║"
echo "╠══════════════════════════════════════════════════╣"
echo "║  Firewall: P2P (30333) + SSH only                ║"
echo "║  Kernel:   anti-spoof, TCP hardening              ║"
echo "║  SSH:      key-only, no passwords                 ║"
echo "║  Logs:     rotate daily, 30-day retention         ║"
echo "╚══════════════════════════════════════════════════╝"