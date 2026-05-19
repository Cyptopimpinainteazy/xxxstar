#!/bin/bash
# Ralph Wiggum - Long-running AI agent loop
# Usage: ./ralph.sh [--tool amp|claude|ollama|openrouter] [max_iterations]
#
# Supported tools:
#   amp        - Amp CLI (paid credits required for non‑interactive use)
#   claude     - Claude Code CLI
#   ollama     - Local Ollama models (use MODEL variable to pick one)
#   openrouter - OpenRouter cloud service (requires OPENROUTER_API_KEY)

set -e

# Parse arguments
TOOL="amp"  # Default to amp for backwards compatibility
MAX_ITERATIONS=10
STRICT_MODE=false

# Determine script directory early (used for default paths and config loading)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PROMPT_FILE="$SCRIPT_DIR/prompt.md"
if [[ ! -f "$PROMPT_FILE" && -f "$SCRIPT_DIR/../../ralph/prompt.md" ]]; then
  PROMPT_FILE="$SCRIPT_DIR/../../ralph/prompt.md"
fi

CLAUDE_FILE="$SCRIPT_DIR/CLAUDE.md"
if [[ ! -f "$CLAUDE_FILE" && -f "$SCRIPT_DIR/../../ralph/CLAUDE.md" ]]; then
  CLAUDE_FILE="$SCRIPT_DIR/../../ralph/CLAUDE.md"
fi

# Load optional configuration (allows overriding defaults without editing this script)
# create a file named ralph.conf in the same directory with key=value pairs,
# e.g.:
#   TOOL=claude
#   MAX_ITERATIONS=20
#   PRD_FILE=/path/to/your/prd.json
#   PROGRESS_FILE=/path/to/your/progress.txt
CONFIG_FILE="$SCRIPT_DIR/ralph.conf"
if [[ -f "$CONFIG_FILE" ]]; then
  # shellcheck source=/dev/null
  source "$CONFIG_FILE"
fi

while [[ $# -gt 0 ]]; do
  case $1 in
    --tool)
      TOOL="$2"
      shift 2
      ;;
    --tool=*)
      TOOL="${1#*=}"
      shift
      ;;
    --strict)
      STRICT_MODE=true
      shift
      ;;
    *)
      # Assume it's max_iterations if it's a number
      if [[ "$1" =~ ^[0-9]+$ ]]; then
        MAX_ITERATIONS="$1"
      fi
      shift
      ;;
  esac
done

# Validate tool choice
if [[ "$TOOL" != "amp" && "$TOOL" != "claude" && "$TOOL" != "ollama" && "$TOOL" != "openrouter" ]]; then
  echo "Error: Invalid tool '$TOOL'. Must be one of amp, claude, ollama, or openrouter."
  exit 1
fi

PRD_FILE="$SCRIPT_DIR/prd.json"
PROGRESS_FILE="$SCRIPT_DIR/progress.txt"
ARCHIVE_DIR="$SCRIPT_DIR/archive"
LAST_BRANCH_FILE="$SCRIPT_DIR/.last-branch"

if [[ "$TOOL" == "amp" || "$TOOL" == "ollama" ]]; then
  if [[ ! -f "$PROMPT_FILE" ]]; then
    echo "Error: prompt file not found at $PROMPT_FILE" >&2
    exit 1
  fi
fi

if [[ "$TOOL" == "claude" ]]; then
  if [[ ! -f "$CLAUDE_FILE" ]]; then
    echo "Error: CLAUDE file not found at $CLAUDE_FILE" >&2
    exit 1
  fi
fi

# Archive previous run if branch changed
if [ -f "$PRD_FILE" ] && [ -f "$LAST_BRANCH_FILE" ]; then
  CURRENT_BRANCH=$(jq -r '.branchName // empty' "$PRD_FILE" 2>/dev/null || echo "")
  LAST_BRANCH=$(cat "$LAST_BRANCH_FILE" 2>/dev/null || echo "")
  
  if [ -n "$CURRENT_BRANCH" ] && [ -n "$LAST_BRANCH" ] && [ "$CURRENT_BRANCH" != "$LAST_BRANCH" ]; then
    # Archive the previous run
    DATE=$(date +%Y-%m-%d)
    # Strip "ralph/" prefix from branch name for folder
    FOLDER_NAME=$(echo "$LAST_BRANCH" | sed 's|^ralph/||')
    ARCHIVE_FOLDER="$ARCHIVE_DIR/$DATE-$FOLDER_NAME"
    
    echo "Archiving previous run: $LAST_BRANCH"
    mkdir -p "$ARCHIVE_FOLDER"
    [ -f "$PRD_FILE" ] && cp "$PRD_FILE" "$ARCHIVE_FOLDER/"
    [ -f "$PROGRESS_FILE" ] && cp "$PROGRESS_FILE" "$ARCHIVE_FOLDER/"
    echo "   Archived to: $ARCHIVE_FOLDER"
    
    # Reset progress file for new run
    echo "# Ralph Progress Log" > "$PROGRESS_FILE"
    echo "Started: $(date)" >> "$PROGRESS_FILE"
    echo "---" >> "$PROGRESS_FILE"
  fi
fi

# Track current branch
if [ -f "$PRD_FILE" ]; then
  CURRENT_BRANCH=$(jq -r '.branchName // empty' "$PRD_FILE" 2>/dev/null || echo "")
  if [ -n "$CURRENT_BRANCH" ]; then
    echo "$CURRENT_BRANCH" > "$LAST_BRANCH_FILE"
  fi
fi

# Initialize progress file if it doesn't exist
if [ ! -f "$PROGRESS_FILE" ]; then
  echo "# Ralph Progress Log" > "$PROGRESS_FILE"
  echo "Started: $(date)" >> "$PROGRESS_FILE"
  echo "---" >> "$PROGRESS_FILE"
fi

echo "Starting Ralph - Tool: $TOOL - Max iterations: $MAX_ITERATIONS - Strict: $STRICT_MODE"

# Export STRICT_MODE for prompt substitution
export STRICT_MODE

for i in $(seq 1 $MAX_ITERATIONS); do
  echo ""
  echo "==============================================================="
  echo "  Ralph Iteration $i of $MAX_ITERATIONS ($TOOL)"
  echo "==============================================================="

  # Run the selected tool with the ralph prompt
  if [[ "$TOOL" == "amp" ]]; then
    OUTPUT=$(cat "$PROMPT_FILE" | amp --dangerously-allow-all 2>&1 | tee /dev/stderr) || true
  elif [[ "$TOOL" == "claude" ]]; then
    OUTPUT=$(claude --dangerously-skip-permissions --print < "$CLAUDE_FILE" 2>&1 | tee /dev/stderr) || true
  elif [[ "$TOOL" == "ollama" ]]; then
    if [[ -z "$MODEL" ]]; then
      if ollama list | awk '{print $1}' | grep -qx 'qwen2.5-coder:0.5b'; then
        MODEL=qwen2.5-coder:0.5b
      elif ollama list | awk '{print $1}' | grep -qx 'qwen2.5-coder:1.5b-base'; then
        MODEL=qwen2.5-coder:1.5b-base
      elif ollama list | awk '{print $1}' | grep -qx 'qwen3:8b'; then
        MODEL=qwen3:8b
      else
        MODEL=codellama:latest
      fi
    fi
    echo "Using Ollama model: $MODEL"
    PROMPT_CONTENT=$(<"$PROMPT_FILE")
    OUTPUT=$(ollama run "$MODEL" "$PROMPT_CONTENT" --nowordwrap --think low --keepalive 30m 2>&1 | tee /dev/stderr) || true
    if echo "$OUTPUT" | grep -q "does not support thinking"; then
      echo "Using Ollama model without thinking mode (not supported by this model)"
      OUTPUT=$(ollama run "$MODEL" "$PROMPT_CONTENT" --nowordwrap --keepalive 30m 2>&1 | tee /dev/stderr) || true
    fi
  elif [[ "$TOOL" == "openrouter" ]]; then
    if [[ -z "$OPENROUTER_API_KEY" ]]; then
      echo "Error: OPENROUTER_API_KEY not set for openrouter tool" >&2
      exit 1
    fi
    MODEL=${OPENROUTER_MODEL:-gpt-4o-mini}
    PROMPT_JSON=$(jq -Rs '.' < "$SCRIPT_DIR/prompt.md")
    OUTPUT=$(curl -s -X POST https://api.openrouter.ai/v1/chat/completions \
          -H "Authorization: Bearer $OPENROUTER_API_KEY" \
          -H "Content-Type: application/json" \
          -d "{\"model\":\"$MODEL\",\"input\":$PROMPT_JSON}" \
          2>&1 | tee /dev/stderr) || true
  fi
  
  # Check for completion signal
  if echo "$OUTPUT" | grep -q "<promise>COMPLETE</promise>"; then
    echo ""
    echo "Ralph completed all tasks!"
    echo "Completed at iteration $i of $MAX_ITERATIONS"
    exit 0
  fi
  
  echo "Iteration $i complete. Continuing..."
  sleep 2
done

echo ""
echo "Ralph reached max iterations ($MAX_ITERATIONS) without completing all tasks."
echo "Check $PROGRESS_FILE for status."
exit 1
