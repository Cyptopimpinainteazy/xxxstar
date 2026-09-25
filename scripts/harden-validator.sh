#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# harden-validator.sh — security hardening for X3 validator hosts
#
# Apply system-level security measures for a bare-metal validator node. Run it
# after install-validator.sh and before starting the service.
#
#   sudo bash scripts/harden-validator.sh --mgmt-cidr 203.0.113.7/32
#   bash scripts/harden-validator.sh --check --mgmt-cidr 203.0.113.7/32
#
# `--check` needs no root, changes nothing, prints what each step would do, and
# exits non-zero if an input the plan needs is missing. It is what
# scripts/mainnet/harden_validator_gate.sh runs, because an operator script that
# nothing exercises is a script whose failure the first operator discovers.
#
# Two things this script used to do that it no longer does on its own:
#
#   * `ufw --force reset` ran unconditionally, discarding whatever firewall the
#     host already had. It now needs --reset-firewall (or X3_RESET_FIREWALL=1);
#     without it the firewall rules are added to the existing configuration.
#
#   * the firewalld branch passed `source address="YOUR-MGMT-CIDR"` — a literal
#     placeholder — to `firewall-cmd`. The management CIDR is required now
#     (--mgmt-cidr or X3_MGMT_CIDR), and its absence is a refusal, not a rule.
#
# Environment:
#   X3_MGMT_CIDR        management network allowed to reach SSH (firewalld path)
#   X3_P2P_PORT         libp2p port to open (default 30333)
#   X3_RESET_FIREWALL   1 to allow `ufw --force reset`
#   X3_FIREWALL_TOOL    auto (default) | ufw | firewalld | none — `auto` picks
#                       whichever is installed; the others force a branch, which
#                       is how the gate exercises all four on one machine
#   X3_HARDEN_ROOT      prefix for every path this script reads or writes
#                       (default empty = the real filesystem). The gate sets it
#                       to a temporary directory so "wrote nothing" is
#                       something it can assert rather than assume.
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

CHECK_ONLY=0
MGMT_CIDR="${X3_MGMT_CIDR:-}"
RESET_FIREWALL="${X3_RESET_FIREWALL:-0}"
FIREWALL_TOOL="${X3_FIREWALL_TOOL:-auto}"
ROOT_PREFIX="${X3_HARDEN_ROOT:-}"
P2P_PORT="${X3_P2P_PORT:-30333}"

SYSCTL_FILE="$ROOT_PREFIX/etc/sysctl.d/90-x3-validator.conf"
SSHD_CONFIG="$ROOT_PREFIX/etc/ssh/sshd_config"
LOGROTATE_FILE="$ROOT_PREFIX/etc/logrotate.d/x3-validator"

usage() {
  sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --check) CHECK_ONLY=1 ;;
    --mgmt-cidr) MGMT_CIDR="${2:-}"; shift ;;
    --mgmt-cidr=*) MGMT_CIDR="${1#*=}" ;;
    --reset-firewall) RESET_FIREWALL=1 ;;
    --p2p-port) P2P_PORT="${2:-}"; shift ;;
    --p2p-port=*) P2P_PORT="${1#*=}" ;;
    -h|--help) usage; exit 0 ;;
    *) echo "ERROR: unknown argument '$1'" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

if [ "$CHECK_ONLY" != 1 ] && [ "${EUID:-$(id -u)}" -ne 0 ]; then
  echo "ERROR: This script must be run as root (or use --check, which needs no root)." >&2
  exit 1
fi

note() { printf '    %s\n' "$*"; }
step() { printf '[%s] %s\n' "$1" "$2"; }

# ── resolve the firewall branch and refuse an incomplete plan ────────────────
case "$FIREWALL_TOOL" in
  auto)
    if command -v ufw >/dev/null 2>&1; then FIREWALL_TOOL=ufw
    elif command -v firewall-cmd >/dev/null 2>&1; then FIREWALL_TOOL=firewalld
    else FIREWALL_TOOL=none
    fi ;;
  ufw|firewalld|none) ;;
  *) echo "ERROR: X3_FIREWALL_TOOL must be auto, ufw, firewalld or none" >&2; exit 2 ;;
esac

if [ "$FIREWALL_TOOL" = firewalld ] && [ -z "$MGMT_CIDR" ]; then
  cat >&2 <<'EOF'
ERROR: the firewalld path needs the management network, and none was given.

  --mgmt-cidr <cidr>   (or X3_MGMT_CIDR)

It is required because the SSH rule is scoped to that network. This script used
to pass the literal string "YOUR-MGMT-CIDR" to `firewall-cmd`, which would have
created a rule matching nothing — a host locked out of its own SSH, or a
placeholder that looked configured and was not.
EOF
  exit 1
fi

if [ -n "$MGMT_CIDR" ] && ! printf '%s' "$MGMT_CIDR" | grep -Eq '^[0-9A-Fa-f:.]+/[0-9]{1,3}$'; then
  echo "ERROR: --mgmt-cidr '$MGMT_CIDR' is not a CIDR (expected e.g. 203.0.113.7/32)" >&2
  exit 1
fi

if [ "$CHECK_ONLY" = 1 ]; then
  echo "==> X3 Validator Security Hardening (check only — nothing will be written)"
else
  echo "==> X3 Validator Security Hardening"
fi
note "firewall tool: $FIREWALL_TOOL, P2P port: $P2P_PORT$( [ -n "$MGMT_CIDR" ] && printf ', management CIDR: %s' "$MGMT_CIDR" )"
note "paths under: ${ROOT_PREFIX:-/}"

# ── 1. Firewall: allow only P2P and SSH ─────────────────────────────────────
step "1/6" "Configuring firewall..."
case "$FIREWALL_TOOL" in
  ufw)
    if [ "$RESET_FIREWALL" = 1 ]; then
      if [ "$CHECK_ONLY" = 1 ]; then
        note "would reset the existing ufw rules (\`ufw --force reset\`) because --reset-firewall was given"
      else
        ufw --force reset
      fi
    else
      note "keeping the existing ufw rules (pass --reset-firewall to replace them)"
    fi
    if [ "$CHECK_ONLY" = 1 ]; then
      note "would set: default deny incoming, default allow outgoing"
      note "would allow: ssh (rate limited), ${P2P_PORT}/tcp (X3 P2P)"
      note "would enable ufw"
    else
      ufw default deny incoming
      ufw default allow outgoing
      ufw allow ssh
      ufw allow "${P2P_PORT}/tcp" comment 'X3 P2P'
      ufw limit ssh
      ufw --force enable
      note "ufw: enabled with P2P (${P2P_PORT}) + SSH"
    fi ;;
  firewalld)
    RICH_RULE="rule family=\"ipv4\" source address=\"${MGMT_CIDR}\" service name=\"ssh\" accept"
    if [ "$CHECK_ONLY" = 1 ]; then
      note "would run: firewall-cmd --permanent --add-port=${P2P_PORT}/tcp"
      note "would run: firewall-cmd --permanent --remove-service=ssh --add-rich-rule='${RICH_RULE}'"
      note "would reload firewalld"
    else
      firewall-cmd --permanent --add-port="${P2P_PORT}/tcp"
      firewall-cmd --permanent --remove-service=ssh \
        --add-rich-rule="rule family=\"ipv4\" source address=\"${MGMT_CIDR}\" service name=\"ssh\" accept"
      firewall-cmd --reload
      note "firewalld: configured for P2P (${P2P_PORT}) and SSH from ${MGMT_CIDR}"
    fi ;;
  none)
    note "WARNING: no firewall tool found (installed: neither ufw nor firewall-cmd)."
    note "         Nothing was configured. Install one and re-run, or set X3_FIREWALL_TOOL." ;;
esac

# ── 2. Kernel hardening (sysctl) ────────────────────────────────────────────
step "2/6" "Applying kernel hardening..."
if [ "$CHECK_ONLY" = 1 ]; then
  note "would write $SYSCTL_FILE (anti-spoof, ICMP redirects, SYN cookies, port range, backlogs)"
  note "would apply it with \`sysctl -p $SYSCTL_FILE\`"
else
  mkdir -p "$(dirname "$SYSCTL_FILE")"
  cat >> "$SYSCTL_FILE" <<'EOF'
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
  sysctl -p "$SYSCTL_FILE" >/dev/null
  note "sysctl: hardening applied"
fi

# ── 3. Disable unnecessary services ─────────────────────────────────────────
step "3/6" "Disabling unnecessary services..."
UNNEEDED="avahi-daemon cups bluetooth postfix nfs-server rpcbind"
for svc in $UNNEEDED; do
  if [ "$CHECK_ONLY" = 1 ]; then
    if systemctl list-unit-files "$svc.service" >/dev/null 2>&1; then
      note "would disable --now $svc"
    fi
  else
    systemctl disable --now "$svc" 2>/dev/null || true
  fi
done
[ "$CHECK_ONLY" = 1 ] && note "would disable: $UNNEEDED (whichever are present)"
note "Unnecessary services handled"

# ── 4. Filesystem mount hardening ───────────────────────────────────────────
step "4/6" "Checking filesystem mounts..."
if grep -q " /tmp " "$ROOT_PREFIX/proc/mounts" 2>/dev/null || grep -q " /tmp " /proc/mounts 2>/dev/null; then
  if [ "$CHECK_ONLY" = 1 ]; then
    note "would remount /tmp with noexec,nosuid,nodev"
  else
    mount -o remount,noexec,nosuid,nodev /tmp 2>/dev/null || true
    note "/tmp remounted with security options"
  fi
else
  note "/tmp is not a separate mount; nothing to remount"
fi

# ── 5. SSH hardening ────────────────────────────────────────────────────────
step "5/6" "Hardening SSH..."
if [[ -f "$SSHD_CONFIG" ]] || [ "$CHECK_ONLY" = 1 ]; then
  if [ "$CHECK_ONLY" = 1 ]; then
    note "would set in $SSHD_CONFIG: PermitRootLogin prohibit-password, PasswordAuthentication no,"
    note "  ChallengeResponseAuthentication no, UsePAM no, MaxAuthTries 3,"
    note "  ClientAliveInterval 300, ClientAliveCountMax 2, AuthenticationMethods publickey"
    note "would restart sshd"
  else
    sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' "$SSHD_CONFIG"
    sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' "$SSHD_CONFIG"
    sed -i 's/^#\?ChallengeResponseAuthentication.*/ChallengeResponseAuthentication no/' "$SSHD_CONFIG"
    sed -i 's/^#\?UsePAM.*/UsePAM no/' "$SSHD_CONFIG"
    sed -i 's/^#\?MaxAuthTries.*/MaxAuthTries 3/' "$SSHD_CONFIG"
    sed -i 's/^#\?ClientAliveInterval.*/ClientAliveInterval 300/' "$SSHD_CONFIG"
    sed -i 's/^#\?ClientAliveCountMax.*/ClientAliveCountMax 2/' "$SSHD_CONFIG"
    if ! grep -q "^AuthenticationMethods" "$SSHD_CONFIG"; then
      echo "AuthenticationMethods publickey" >> "$SSHD_CONFIG"
    fi
    systemctl restart sshd
    note "SSH hardened: key-only auth, no root password login"
  fi
else
  note "no $SSHD_CONFIG on this host; SSH hardening skipped"
fi

# ── 6. Logging and auditing ─────────────────────────────────────────────────
step "6/6" "Configuring log rotation..."
if [ "$CHECK_ONLY" = 1 ]; then
  note "would write $LOGROTATE_FILE (daily, 30 kept, compressed, x3:x3, restart x3-validator.service)"
else
  mkdir -p "$(dirname "$LOGROTATE_FILE")"
  cat > "$LOGROTATE_FILE" <<'EOF'
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
  note "Log rotation configured at $LOGROTATE_FILE"
fi

echo ""
if [ "$CHECK_ONLY" = 1 ]; then
  echo "Check complete — every step above is a plan, and nothing was written."
else
  echo "╔══════════════════════════════════════════════════╗"
  echo "║  Security Hardening Complete                     ║"
  echo "╠══════════════════════════════════════════════════╣"
  printf '║  Firewall: P2P (%s) + SSH only%-*s║\n' "$P2P_PORT" $((20 - ${#P2P_PORT})) ""
  echo "║  Kernel:   anti-spoof, TCP hardening              ║"
  echo "║  SSH:      key-only, no passwords                 ║"
  echo "║  Logs:     rotate daily, 30-day retention         ║"
  echo "╚══════════════════════════════════════════════════╝"
fi
