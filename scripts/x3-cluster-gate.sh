#!/usr/bin/env bash
# First cluster gate: reachability, SSH, role identity, clock, disk, GPU workers,
# storage. Writes audit-artifacts/x3-cluster/<commit>/gate-<node>.json; nonzero on failure.
set -euo pipefail
exec python3 "$(dirname "$(readlink -f "$0")")/x3-cluster/x3cluster.py" gate "$@"
