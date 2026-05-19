#!/usr/bin/env bash
# One-command helper to run the X3 full-stack execution prompt.
# Usage:
#   scripts/run-full-stack-build-prompt.sh
#   scripts/run-full-stack-build-prompt.sh --full
#   scripts/run-full-stack-build-prompt.sh --print
#   scripts/run-full-stack-build-prompt.sh --save
#   scripts/run-full-stack-build-prompt.sh --snapshot
#   scripts/run-full-stack-build-prompt.sh --snapshot --latest
#
# Optional execution mode with an external runner:
#   RUNNER_CMD='my-agent-cli run --stdin' scripts/run-full-stack-build-prompt.sh --run

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FULL_PROMPT="${REPO_ROOT}/FULL_STACK_BUILD_EXECUTION_PROMPT.md"
OP_PROMPT="${REPO_ROOT}/FULL_STACK_BUILD_EXECUTION_PROMPT_OPERATOR.md"
OUT_DIR="${REPO_ROOT}/reports/prompt-runs"
MODE="operator"
ACTION="print"
UPDATE_LATEST=0

git_branch() {
  git -C "$REPO_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown"
}

git_sha() {
  git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "unknown"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --full)
      MODE="full"
      shift
      ;;
    --operator)
      MODE="operator"
      shift
      ;;
    --print)
      ACTION="print"
      shift
      ;;
    --save)
      ACTION="save"
      shift
      ;;
    --snapshot)
      ACTION="snapshot"
      shift
      ;;
    --run)
      ACTION="run"
      shift
      ;;
    --latest)
      UPDATE_LATEST=1
      shift
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

PROMPT_PATH="$OP_PROMPT"
if [[ "$MODE" == "full" ]]; then
  PROMPT_PATH="$FULL_PROMPT"
fi

if [[ ! -f "$PROMPT_PATH" ]]; then
  echo "Prompt file not found: $PROMPT_PATH"
  exit 1
fi

case "$ACTION" in
  print)
    echo "Using prompt: $PROMPT_PATH"
    echo ""
    cat "$PROMPT_PATH"
    ;;
  save)
    mkdir -p "$OUT_DIR"
    TS="$(date -u +%Y%m%dT%H%M%SZ)"
    DEST="${OUT_DIR}/prompt-${MODE}-${TS}.md"
    cp "$PROMPT_PATH" "$DEST"
    if [[ "$UPDATE_LATEST" -eq 1 ]]; then
      ln -sfn "$(basename "$DEST")" "${OUT_DIR}/latest-${MODE}.md"
      echo "Updated latest symlink: ${OUT_DIR}/latest-${MODE}.md"
    fi
    echo "Saved prompt copy: $DEST"
    ;;
  snapshot)
    mkdir -p "$OUT_DIR"
    TS="$(date -u +%Y%m%dT%H%M%SZ)"
    DEST="${OUT_DIR}/prompt-${MODE}-${TS}.md"
    {
      echo "# Prompt Snapshot Metadata"
      echo "- generated_at_utc: $TS"
      echo "- mode: $MODE"
      echo "- source_prompt: $PROMPT_PATH"
      echo "- git_branch: $(git_branch)"
      echo "- git_sha: $(git_sha)"
      echo ""
      cat "$PROMPT_PATH"
    } > "$DEST"
    if [[ "$UPDATE_LATEST" -eq 1 ]]; then
      ln -sfn "$(basename "$DEST")" "${OUT_DIR}/latest-${MODE}.md"
      echo "Updated latest symlink: ${OUT_DIR}/latest-${MODE}.md"
    fi
    echo "Saved metadata snapshot: $DEST"
    ;;
  run)
    if [[ -z "${RUNNER_CMD:-}" ]]; then
      echo "RUNNER_CMD is not set. Example:"
      echo "  RUNNER_CMD='my-agent-cli run --stdin' scripts/run-full-stack-build-prompt.sh --run"
      exit 1
    fi
    echo "Running prompt via RUNNER_CMD using: $PROMPT_PATH"
    # shellcheck disable=SC2086
    eval "$RUNNER_CMD" < "$PROMPT_PATH"
    ;;
esac
