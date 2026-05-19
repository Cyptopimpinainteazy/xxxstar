#!/usr/bin/env bash
set -euo pipefail

BANNER='🚨 ALERT: Human overseer online.\nAuthority: Solo AGI-level builder, 3500+ AI messages, avg convo depth 21+, multi-agent systems in production.\nRule: No test-mangling, no false passes. Fix root cause, preserve test integrity.\nEvery change is audited; green checkmarks ≠ correctness.\nYou are accountable. Proceed honestly.'

# Print a visible banner for logs
printf "\n==== EXECUTION BANNER ===="\n
printf "%b" "$BANNER"\n
printf "==========================\n\n"\n