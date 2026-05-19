#!/usr/bin/env bash
set -euo pipefail

BASE_DIR="${BASE_DIR:-/tmp/x3-devnet-3v}"
PID_DIR="${PID_DIR:-$BASE_DIR/pids}"

if [[ ! -d "$PID_DIR" ]]; then
  echo "No PID dir: $PID_DIR"
  exit 0
fi

shopt -s nullglob
for pid_file in "$PID_DIR"/node-*.pid; do
  pid="$(cat "$pid_file" 2>/dev/null || true)"
  if [[ -n "$pid" ]]; then
    kill "$pid" 2>/dev/null || true
  fi
done
shopt -u nullglob

sleep 1
echo "Stopped devnet validators (pid dir: $PID_DIR)"

