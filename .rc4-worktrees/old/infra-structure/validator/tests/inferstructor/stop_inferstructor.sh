#!/usr/bin/env bash
# Stop all Inferstructor services

set -euo pipefail

echo "🛑 Stopping Inferstructor services..."

if [ -f .inferstructor.pids ]; then
    while read -r pid; do
        if ps -p "$pid" > /dev/null 2>&1; then
            echo "   Killing PID $pid..."
            kill "$pid" 2>/dev/null || true
        fi
    done < .inferstructor.pids
    
    rm .inferstructor.pids
else
    echo "   No PID file found, trying to kill by name..."
    pkill -f "validator_registry.py" 2>/dev/null || true
    pkill -f "tps_bridge.py" 2>/dev/null || true
    pkill -f "metrics_dashboard.py" 2>/dev/null || true
    pkill -f "lane_orchestrator.py" 2>/dev/null || true
fi

sleep 1

echo "✅ All services stopped"
