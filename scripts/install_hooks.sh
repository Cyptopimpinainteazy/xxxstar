#!/usr/bin/env bash
set -euo pipefail

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push .githooks/commit-msg
chmod +x scripts/*.py scripts/*.sh || true
echo "[install_hooks] core.hooksPath set to .githooks"
