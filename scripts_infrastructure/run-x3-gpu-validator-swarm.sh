#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/logs/x3-gpu-validator-swarm"
PID_DIR="$ROOT_DIR/.pids"
ORCH_PID_FILE="$PID_DIR/x3-swarm-orchestrator.pid"
VAL_PID_FILE="$PID_DIR/x3-validator.pid"
TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/x3-target-swarm}"

mkdir -p "$LOG_DIR" "$PID_DIR"

start_orchestrator() {
  if [[ -f "$ORCH_PID_FILE" ]] && kill -0 "$(cat "$ORCH_PID_FILE")" 2>/dev/null; then
    echo "orchestrator already running (pid $(cat "$ORCH_PID_FILE"))"
    return
  fi

  cd "$ROOT_DIR"
  nohup env CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$TARGET_DIR" \
    cargo run -p x3-gpu-validator-swarm --bin x3-swarm-orchestrator -- run \
    >"$LOG_DIR/orchestrator.log" 2>&1 &
  echo $! > "$ORCH_PID_FILE"
  echo "orchestrator started (pid $!, log $LOG_DIR/orchestrator.log)"
}

start_validator() {
  if [[ -f "$VAL_PID_FILE" ]] && kill -0 "$(cat "$VAL_PID_FILE")" 2>/dev/null; then
    echo "validator already running (pid $(cat "$VAL_PID_FILE"))"
    return
  fi

  cd "$ROOT_DIR"
  nohup env CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$TARGET_DIR" \
    cargo run -p x3-gpu-validator-swarm --bin x3-validator -- run \
    >"$LOG_DIR/validator.log" 2>&1 &
  echo $! > "$VAL_PID_FILE"
  echo "validator started (pid $!, log $LOG_DIR/validator.log)"
}

stop_service() {
  local name="$1"
  local pid_file="$2"
  if [[ ! -f "$pid_file" ]]; then
    echo "$name not running"
    return
  fi

  local pid
  pid="$(cat "$pid_file")"
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" || true
    sleep 1
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" || true
    fi
    echo "$name stopped"
  else
    echo "$name process not found, cleaning stale pid"
  fi
  rm -f "$pid_file"
}

status() {
  if [[ -f "$ORCH_PID_FILE" ]] && kill -0 "$(cat "$ORCH_PID_FILE")" 2>/dev/null; then
    echo "orchestrator: running (pid $(cat "$ORCH_PID_FILE"))"
  else
    echo "orchestrator: stopped"
  fi

  if [[ -f "$VAL_PID_FILE" ]] && kill -0 "$(cat "$VAL_PID_FILE")" 2>/dev/null; then
    echo "validator: running (pid $(cat "$VAL_PID_FILE"))"
  else
    echo "validator: stopped"
  fi

  echo "logs: $LOG_DIR"
}

usage() {
  cat <<USAGE
Usage: $(basename "$0") <command>

Commands:
  start-orchestrator  Start orchestrator only
  start-validator     Start validator only
  start-both          Start orchestrator and validator
  stop-orchestrator   Stop orchestrator
  stop-validator      Stop validator
  stop-all            Stop both
  status              Show process status
  logs                Tail both logs
USAGE
}

case "${1:-}" in
  start-orchestrator) start_orchestrator ;;
  start-validator) start_validator ;;
  start-both) start_orchestrator; start_validator ;;
  stop-orchestrator) stop_service "orchestrator" "$ORCH_PID_FILE" ;;
  stop-validator) stop_service "validator" "$VAL_PID_FILE" ;;
  stop-all)
    stop_service "orchestrator" "$ORCH_PID_FILE"
    stop_service "validator" "$VAL_PID_FILE"
    ;;
  status) status ;;
  logs)
    touch "$LOG_DIR/orchestrator.log" "$LOG_DIR/validator.log"
    tail -f "$LOG_DIR/orchestrator.log" "$LOG_DIR/validator.log"
    ;;
  *)
    usage
    exit 1
    ;;
esac
