#!/bin/bash
# Health check script for CCGV services

CCGV_PORT="${CCGV_VALIDATOR_PORT:-8000}"
CCGV_HOST="${CCGV_VALIDATOR_HOST:-localhost}"

# Check metrics endpoint
if curl -f -s "http://$CCGV_HOST:$CCGV_PORT/metrics.json" > /dev/null 2>&1; then
    exit 0
else
    exit 1
fi
