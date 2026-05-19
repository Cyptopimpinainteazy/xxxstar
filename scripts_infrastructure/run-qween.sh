#!/bin/bash
# run-qween.sh - Run Ollama models with stdin input
# Usage: echo "prompt" | ./run-qween.sh -m model_name

set -e

# Default model
MODEL="qwen2.5-coder:7b"

# Parse arguments
while getopts "m:" opt; do
    case $opt in
        m) MODEL="$OPTARG" ;;
        *) echo "Usage: $0 [-m model_name]" >&2; exit 1 ;;
    esac
done

# Read from stdin
PROMPT=$(cat)

if [ -z "$PROMPT" ]; then
    echo "Error: No prompt provided via stdin" >&2
    exit 1
fi

# Check if Ollama is running
if ! curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    echo "Error: Ollama is not running. Start with: ollama serve" >&2
    exit 1
fi

# Run the model
curl -s http://localhost:11434/api/generate \
    -H "Content-Type: application/json" \
    -d "{\"model\": \"${MODEL}\", \"prompt\": \"${PROMPT//\"/\\\"}\", \"stream\": false}" | \
    jq -r '.response // .error // empty'