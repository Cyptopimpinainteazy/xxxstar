#!/usr/bin/env bash
set -euo pipefail

RPC_WS_URL="${1:-ws://localhost:9944}"
DURATION_MIN="${2:-30}"

cat <<EOF
[hub-fee-monitor]
rpc_ws_url=${RPC_WS_URL}
duration_min=${DURATION_MIN}

This helper is a runbook shim for operators.
Use your existing chain event subscriber to filter for HubFeeCollected events
and enforce fee-rate checks at 250 bps.

Suggested checks:
1) Event count >= number of revenue submissions
2) fee_amount == gross_amount * 250 / 10000
3) no missing/null event fields
4) no decoding failures

Note: This script does not directly subscribe via WS to avoid forcing extra runtime deps
in constrained environments. It provides a strict execution checklist for monitoring.
EOF
