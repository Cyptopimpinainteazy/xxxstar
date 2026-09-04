#!/usr/bin/env bash
set -euo pipefail

next_sprint() {
  for f in .autoclaw/orchestrator/sprints/sprint-*.yaml; do
    status=$(yq e '.status' "$f" 2>/dev/null || echo "")
    if [[ "$status" == "pending" || "$status" == "assigned" ]]; then
      echo "${f##*/sprint-}"
      return
    fi
  done
  echo "none"
}

while true; do
  sprint=$(next_sprint)
  if [[ "$sprint" == "none" ]]; then
    echo "All sprints completed."
    break
  fi
  echo "Running sprint $sprint..."
  code --command "orchestrate.run $sprint"
  echo "Sprint $sprint finished."
  sleep 1
done