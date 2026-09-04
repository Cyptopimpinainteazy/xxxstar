#!/usr/bin/env bash
set -euo pipefail

echo "[fresh-machine-check] checking toolchain presence"
command -v python >/dev/null
command -v cargo >/dev/null || true
command -v node >/dev/null || true

echo "[fresh-machine-check] running baseline guards"
python scripts/agent_guard.py
python scripts/test_cheat_guard.py

echo "[fresh-machine-check] ok"
