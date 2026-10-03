#!/usr/bin/env bash
# Install the dual-GPU AI stack as services that survive logout and reboot.
#   deploy/install.sh           user units: router + GTX 1070 worker
#   deploy/install.sh --system  also pin the system ollama.service to the RTX (sudo)
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
units="$HOME/.config/systemd/user"
mkdir -p "$units" "$HOME/.config/x3-router" "$HOME/.local/share/x3-router"
env_file="$HOME/.config/x3-router/env"
if [ ! -f "$env_file" ]; then
    printf '# DEEPSEEK_API_KEY=\n# X3_ROUTER_TOKEN=\n# X3_ROUTER_HOST=127.0.0.1\n' > "$env_file"
fi
chmod 600 "$env_file"
install -m 644 "$here/ollama-worker-b.service" "$here/x3-ai-router.service" "$units/"
systemctl --user daemon-reload
systemctl --user enable --now ollama-worker-b.service x3-ai-router.service
loginctl show-user "$USER" -p Linger | grep -q yes || echo "WARN: run 'sudo loginctl enable-linger $USER' so user services start at boot"
if [ "${1:-}" = "--system" ]; then
    sudo install -D -m 644 "$here/ollama-rtx.conf" /etc/systemd/system/ollama.service.d/override.conf
    sudo systemctl daemon-reload
    sudo systemctl restart ollama.service
fi
