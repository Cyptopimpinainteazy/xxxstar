#!/usr/bin/env bash
# Join a fresh Ubuntu machine to the X3 build cluster. Run it ON the new machine, as the
# user who will own the workers (not root), with sudo available:
#
#   curl -fsSL https://raw.githubusercontent.com/Cyptopimpinainteazy/xxxstar/master/scripts/x3-cluster/x3-join.sh \
#     | bash -s -- --role gpu --name x3gpu2
#
# It does only the privileged, one-time parts: hostname, base packages, sshd, the control
# node's public key, linger, a LAN-only firewall, and for --role gpu the NVIDIA driver and
# Ollama. Everything else (tooling, checkout, per-GPU workers, models, router registration,
# benchmark, first job) is done by the control node, which finds this machine on its next
# `x3cluster.py discover --onboard` sweep (every 5 minutes) without further input.
#
# Re-running is safe. Nothing is deleted; no disk, RAID or partition is touched.
set -euo pipefail

# Keep in sync with inventory.json (test_x3jobs.py checks this).
LAN="192.168.0.0/24"
CONTROL_IP="192.168.0.70"
CONTROL_KEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJEW3gU29rVC93o1TVD5i6yA09ewbV9eo+upTr3pw14S x3star1-to-x3gpu1"
ROLES="gpu build sim data net ops"
WORKER_PORTS="11434:11449"

role="" name="" dry=0
while [ $# -gt 0 ]; do
    case "$1" in
        --role) role="${2:-}"; shift 2 ;;
        --name) name="${2:-}"; shift 2 ;;
        --dry-run) dry=1; shift ;;
        -h|--help) sed -n '2,14p' "$0" 2>/dev/null || true; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
case " $ROLES " in *" $role "*) ;; *) echo "--role must be one of: $ROLES" >&2; exit 2 ;; esac
[ "$(id -u)" -ne 0 ] || { echo "run as the normal user (it uses sudo itself), not as root" >&2; exit 2; }

do_() { echo "+ $*"; [ "$dry" = 1 ] || "$@"; }
need_reboot=0

# 1. Stable hostname: the control node only accepts a machine whose hostname matches inventory.json.
if [ -n "$name" ] && [ "$(hostname)" != "$name" ]; then
    do_ sudo hostnamectl set-hostname "$name"
    if grep -q '^127\.0\.1\.1' /etc/hosts; then
        do_ sudo sed -i "s/^127\.0\.1\.1.*/127.0.1.1\t$name/" /etc/hosts
    else
        echo "+ add 127.0.1.1 $name to /etc/hosts"; [ "$dry" = 1 ] || echo -e "127.0.1.1\t$name" | sudo tee -a /etc/hosts >/dev/null
    fi
fi

# 2. Base packages. packagekitd (desktop updater) often holds the apt lock; apt waits instead of failing.
do_ sudo systemctl stop packagekit 2>/dev/null || true
do_ sudo apt-get -o DPkg::Lock::Timeout=600 update
do_ sudo apt-get -o DPkg::Lock::Timeout=600 install -y openssh-server git python3 curl ca-certificates \
    tmux jq iperf3 ethtool ufw pciutils
do_ sudo systemctl enable --now ssh

# 3. Let the control node in with its key (public key only; nothing secret is copied anywhere).
mkdir -p ~/.ssh && chmod 700 ~/.ssh
touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys
if ! grep -qF "$CONTROL_KEY" ~/.ssh/authorized_keys; then
    echo "+ authorize x3star1's public key"; [ "$dry" = 1 ] || echo "$CONTROL_KEY" >> ~/.ssh/authorized_keys
fi

# 4. User services (the per-GPU workers) must survive logout and start at boot.
do_ sudo loginctl enable-linger "$USER"

# 5. Firewall: default deny inbound; SSH from the LAN; GPU worker ports only from the control node.
#    SSH is allowed before ufw is enabled, so this cannot lock out the session running it.
do_ sudo ufw allow from "$LAN" to any port 22 proto tcp
if [ "$role" = gpu ]; then
    do_ sudo ufw allow from "$CONTROL_IP" to any port "$WORKER_PORTS" proto tcp
fi
do_ sudo ufw default deny incoming
do_ sudo ufw default allow outgoing
do_ sudo ufw --force enable

# 6. GPU: driver and Ollama. Workers themselves are created per GPU by the control node.
if [ "$role" = gpu ]; then
    if ! command -v nvidia-smi >/dev/null || ! nvidia-smi -L >/dev/null 2>&1; then
        if lspci | grep -qi nvidia; then
            do_ sudo apt-get -o DPkg::Lock::Timeout=600 install -y ubuntu-drivers-common
            do_ sudo ubuntu-drivers install
            need_reboot=1
        else
            echo "WARNING: no NVIDIA device on the PCI bus; this machine cannot be a GPU worker" >&2
        fi
    fi
    if ! command -v ollama >/dev/null; then
        echo "+ install Ollama (https://ollama.com/install.sh)"
        [ "$dry" = 1 ] || curl -fsSL https://ollama.com/install.sh | sh
    fi
    # The stock service would take :11434 on every GPU; the cluster runs one pinned worker per GPU instead.
    if systemctl list-unit-files ollama.service >/dev/null 2>&1 && systemctl is-enabled ollama.service >/dev/null 2>&1; then
        do_ sudo systemctl disable --now ollama.service
    fi
fi

echo
echo "joined as $(hostname) role=$role ip=$(hostname -I | awk '{print $1}')"
if [ "$need_reboot" = 1 ]; then
    echo "REBOOT NEEDED for the NVIDIA driver: sudo reboot. The control node onboards it after it comes back."
else
    echo "The control node ($CONTROL_IP) will find and onboard this machine within ~5 minutes."
fi
