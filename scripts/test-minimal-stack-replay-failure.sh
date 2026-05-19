#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== Minimal Stack Replay or Failure Gate ==="
echo "[1/3] Router replay protection"
cargo test -p pallet-x3-cross-vm-router --lib test_duplicate_message_replay_rejected

echo "[2/3] Router timeout failure path"
cargo test -p pallet-x3-cross-vm-router --lib completion_rejected_after_packet_timeout

echo "[3/3] Bridge replay protection"
cargo test -p x3-bridge --lib test_bridge_replay_protection

echo "=== Replay or Failure Gate PASSED ==="