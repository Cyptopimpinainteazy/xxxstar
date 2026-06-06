#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
  echo "Usage: compare_artifacts.sh <run1_dir> <run2_dir> <run3_dir>"
  exit 2
fi

run1=$1
run2=$2
run3=$3

log() { printf "[compare] %s\n" "$1"; }

# Normalize and compute sha256 for canonical outputs
hash_of() {
  local f=$1
  if [ -d "$f" ]; then
    # create a stable tar stream sorted by filename to ensure deterministic hash
    tar --sort=name -cf - -C "$f" . | sha256sum | awk '{print $1}'
  elif [ -f "$f" ]; then
    sha256sum "$f" | awk '{print $1}'
  else
    echo ""
  fi
}

# Compare important artifacts: logs, state-root.json, rpc-responses.json
declare -A hashes
for r in "$run1" "$run2" "$run3"; do
  hashes["$r,logs"]=$(hash_of "$r/logs")
  hashes["$r,state"]=$(hash_of "$r/state-root.json")
  hashes["$r,rpc"]=$(hash_of "$r/rpc-responses.json")
done

# Compare pairwise
if [ "${hashes[$run1,logs]}" = "${hashes[$run2,logs]}" ] && [ "${hashes[$run1,logs]}" = "${hashes[$run3,logs]}" ] && \
   [ "${hashes[$run1,state]}" = "${hashes[$run2,state]}" ] && [ "${hashes[$run1,state]}" = "${hashes[$run3,state]}" ] && \
   [ "${hashes[$run1,rpc]}" = "${hashes[$run2,rpc]}" ] && [ "${hashes[$run1,rpc]}" = "${hashes[$run3,rpc]}" ]; then
  log "Artifacts identical across runs"
  exit 0
else
  log "Mismatch detected — writing diff report to artifacts/compare_report.json"
  mkdir -p artifacts/compare_report
  jq -n --arg r1 "$run1" --arg r2 "$run2" --arg r3 "$run3" \
     --arg r1l "${hashes[$run1,logs]}" --arg r1s "${hashes[$run1,state]}" --arg r1r "${hashes[$run1,rpc]}" \
     --arg r2l "${hashes[$run2,logs]}" --arg r2s "${hashes[$run2,state]}" --arg r2r "${hashes[$run2,rpc]}" \
     --arg r3l "${hashes[$run3,logs]}" --arg r3s "${hashes[$run3,state]}" --arg r3r "${hashes[$run3,rpc]}" \
    '{runs: [$r1,$r2,$r3], hashes: {run1: {logs: $r1l, state: $r1s, rpc: $r1r}, run2: {logs: $r2l, state: $r2s, rpc: $r2r}, run3: {logs: $r3l, state: $r3s, rpc: $r3r}}}' \
    > artifacts/compare_report/report.json || true
  # Save raw artifacts for triage
  cp -r "$run1" artifacts/compare_report/run-1 || true
  cp -r "$run2" artifacts/compare_report/run-2 || true
  cp -r "$run3" artifacts/compare_report/run-3 || true
  exit 1
fi
