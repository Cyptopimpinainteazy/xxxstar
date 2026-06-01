#!/usr/bin/env bash
# x3_router.sh — Start/stop the X3 Router service
# Usage:
#   ./x3_router.sh start          # Start the router in background
#   ./x3_router.sh stop           # Stop the router
#   ./x3_router.sh status         # Check if running
#   ./x3_router.sh test           # Run classifier tests
#   ./x3_router.sh foreground     # Run in foreground (for debugging)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROUTER_PID="/tmp/x3-router.pid"
ROUTER_LOG="/tmp/x3-router.log"
ROUTER_PORT="${X3_ROUTER_PORT:-11435}"
OLLAMA_HOST="${OLLAMA_HOST:-http://localhost:11434}"

start_router() {
    if [ -f "$ROUTER_PID" ] && kill -0 "$(cat "$ROUTER_PID")" 2>/dev/null; then
        echo "X3 Router is already running (PID $(cat "$ROUTER_PID"))"
        return 0
    fi

    echo "Starting X3 Router on port $ROUTER_PORT..."
    nohup python3 "$SCRIPT_DIR/x3_router.py" \
        --port "$ROUTER_PORT" \
        --ollama-host "$OLLAMA_HOST" \
        > "$ROUTER_LOG" 2>&1 &
    echo $! > "$ROUTER_PID"
    sleep 2

    if kill -0 "$(cat "$ROUTER_PID")" 2>/dev/null; then
        echo "X3 Router started (PID $(cat "$ROUTER_PID"))"
        echo "  Listening on: http://localhost:$ROUTER_PORT"
        echo "  Ollama host:   $OLLAMA_HOST"
        echo "  Log file:      $ROUTER_LOG"
        echo ""
        echo "Configure Cline with:"
        echo "  Provider:      Ollama"
        echo "  Base URL:      http://localhost:$ROUTER_PORT"
        echo "  Model:         lojak/cryptomaster"
        echo "  Context:       32768"
    else
        echo "ERROR: X3 Router failed to start. Check $ROUTER_LOG"
        rm -f "$ROUTER_PID"
        return 1
    fi
}

stop_router() {
    if [ -f "$ROUTER_PID" ]; then
        PID="$(cat "$ROUTER_PID")"
        if kill -0 "$PID" 2>/dev/null; then
            echo "Stopping X3 Router (PID $PID)..."
            kill "$PID"
            sleep 1
            if kill -0 "$PID" 2>/dev/null; then
                echo "Force killing..."
                kill -9 "$PID"
            fi
            rm -f "$ROUTER_PID"
            echo "X3 Router stopped"
        else
            echo "X3 Router is not running (stale PID file)"
            rm -f "$ROUTER_PID"
        fi
    else
        echo "X3 Router is not running"
    fi
}

status_router() {
    if [ -f "$ROUTER_PID" ] && kill -0 "$(cat "$ROUTER_PID")" 2>/dev/null; then
        echo "X3 Router is running (PID $(cat "$ROUTER_PID"))"
        echo "  Port: $ROUTER_PORT"
        echo "  Ollama: $OLLAMA_HOST"
        # Quick health check
        if curl -s "http://localhost:$ROUTER_PORT/api/tags" > /dev/null 2>&1; then
            echo "  Health: OK (connected to Ollama)"
        else
            echo "  Health: WARNING (cannot reach Ollama through router)"
        fi
    else
        echo "X3 Router is not running"
        rm -f "$ROUTER_PID" 2>/dev/null
    fi
}

case "${1:-status}" in
    start)
        start_router
        ;;
    stop)
        stop_router
        ;;
    restart)
        stop_router
        sleep 1
        start_router
        ;;
    status)
        status_router
        ;;
    test)
        python3 "$SCRIPT_DIR/x3_router_test.py"
        ;;
    foreground)
        python3 "$SCRIPT_DIR/x3_router.py" --port "$ROUTER_PORT" --ollama-host "$OLLAMA_HOST"
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|test|foreground}"
        echo ""
        echo "Environment variables:"
        echo "  X3_ROUTER_PORT  Router port (default: 11435)"
        echo "  OLLAMA_HOST     Ollama host (default: http://localhost:11434)"
        exit 1
        ;;
esac