#!/usr/bin/env bash
# Install the dual-GPU AI stack as user services.
#   deploy/install.sh           user units: router + GTX 1070 worker
#   deploy/install.sh --system  also pin the system ollama.service to the RTX (sudo)
#
# The units in this directory are templates: @ROUTER_DIR@ becomes this checkout's
# router directory and @OLLAMA@ the ollama binary on PATH, so the services work
# from wherever the repository and Ollama actually are.
# User services survive logout and reboot only with linger enabled; this script
# warns when it is not, it does not enable it (that needs sudo).
set -euo pipefail
usage() { echo "usage: $0 [--system]" >&2; exit 2; }
system=0
# Validate every argument before touching anything: a typo such as --sytem must
# not install the user units and silently skip the RTX pinning.
for arg in "$@"; do
    case "$arg" in
        --system) system=1 ;;
        -h|--help) echo "usage: $0 [--system]"; exit 0 ;;
        *) echo "unknown argument: $arg" >&2; usage ;;
    esac
done
here="$(cd "$(dirname "$0")" && pwd)"
router_dir="$(cd "$here/.." && pwd)"
ollama="$(command -v ollama || true)"
[ -n "$ollama" ] || { echo "ollama is not on PATH; install it first" >&2; exit 1; }
units="$HOME/.config/systemd/user"
# providers.d is where GPU nodes' registrations go; the router ignores a missing directory.
mkdir -p "$units" "$HOME/.config/x3-router/providers.d" "$HOME/.local/share/x3-router"
env_file="$HOME/.config/x3-router/env"
if [ ! -f "$env_file" ]; then
    printf '# DEEPSEEK_API_KEY=\n# X3_ROUTER_TOKEN=\n# X3_ROUTER_HOST=127.0.0.1\n' > "$env_file"
fi
chmod 600 "$env_file"
# sed replacement text: escape the characters sed treats specially.
escape() { printf '%s' "$1" | sed -e 's/[\\|&]/\\&/g'; }
for unit in ollama-worker-b.service x3-ai-router.service; do
    sed -e "s|@ROUTER_DIR@|$(escape "$router_dir")|g" -e "s|@OLLAMA@|$(escape "$ollama")|g" \
        "$here/$unit" > "$units/$unit"
    chmod 644 "$units/$unit"
done
systemctl --user daemon-reload
systemctl --user enable --now ollama-worker-b.service x3-ai-router.service
loginctl show-user "$USER" -p Linger | grep -q yes || echo "WARN: run 'sudo loginctl enable-linger $USER' so user services start at boot"
if [ "$system" = 1 ]; then
    sudo install -D -m 644 "$here/ollama-rtx.conf" /etc/systemd/system/ollama.service.d/override.conf
    sudo systemctl daemon-reload
    sudo systemctl restart ollama.service
fi
