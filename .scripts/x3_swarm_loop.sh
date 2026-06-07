#!/usr/bin/env bash
set -euo pipefail

interval="${X3_SWARM_INTERVAL_SECONDS:-60}"

while true; do
  .scripts/x3_level10_swarm.sh
  echo "Sleeping ${interval} seconds..."
  sleep "${interval}"
done
