#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

base_sha=${BASE_SHA:-}
head_sha=${HEAD_SHA:-$(git rev-parse HEAD)}
review_note_file=${REVIEW_NOTE_FILE:-X3_LAUNCHOPS_CONTRACT_REVIEW_NOTE.md}
launchops_bin=${LAUNCHOPS_BIN:-$repo_root/target/debug/launchops}

if [[ -z "$base_sha" || "$base_sha" =~ ^0+$ ]]; then
  echo "::notice::Skipping LaunchOps contract review gate because no usable BASE_SHA was provided."
  exit 0
fi

if [[ ! -x "$launchops_bin" ]]; then
  echo "::error::LaunchOps binary not found at $launchops_bin. Run cargo before this gate."
  exit 1
fi

artifacts=(
  frontend_route_allowlist.json
  sidecar_adapter_backlog.json
)

for artifact in "${artifacts[@]}"; do
  if [[ ! -f ".launchops/$artifact" ]]; then
    echo "::error::Missing generated artifact .launchops/$artifact before review gate."
    exit 1
  fi
done

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

git archive "$base_sha" | tar -x -C "$tmpdir"

(
  cd "$tmpdir"
  "$launchops_bin" inventory-contracts >/dev/null
)

changed_artifacts=()
for artifact in "${artifacts[@]}"; do
  if ! cmp -s ".launchops/$artifact" "$tmpdir/.launchops/$artifact"; then
    changed_artifacts+=("$artifact")
  fi
done

if [[ ${#changed_artifacts[@]} -eq 0 ]]; then
  echo "LaunchOps contract review gate: no allowlist or backlog drift against $base_sha."
  exit 0
fi

if git diff --name-only "$base_sha" "$head_sha" -- "$review_note_file" | grep -q .; then
  echo "LaunchOps contract review gate: drift detected in ${changed_artifacts[*]}, but $review_note_file was updated."
  exit 0
fi

echo "::error::LaunchOps contract drift detected in ${changed_artifacts[*]} without an updated $review_note_file review note."
echo "Update $review_note_file with the intentional contract change, why it is safe, and what consumers must do next."
exit 1