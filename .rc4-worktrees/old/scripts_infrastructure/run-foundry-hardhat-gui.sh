#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$REPO_ROOT/tools/foundry-hardhat-gui/server.py" --workspace "$REPO_ROOT" "$@"
