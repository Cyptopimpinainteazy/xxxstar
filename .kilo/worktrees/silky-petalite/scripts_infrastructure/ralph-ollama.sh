#!/bin/bash
# ralph-ollama.sh - Run Ralph with Ollama backend
# Usage: ./scripts/ralph-ollama.sh [project-name] [options]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RALPH_OLLAMA_DIR="/home/lojak/ralph-ollama"

# Default project
PROJECT_NAME="${1:-x3-chain}"

# Check Ollama is running
if ! curl -s http://localhost:11434/api/tags >/dev/null 2>&1; then
    echo "❌ Ollama is not running. Start with: ollama serve"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════"
echo "  Ralph + Ollama Integration"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Show available models
echo "📦 Available Ollama models:"
ollama list | head -10
echo ""

# Check which model is currently loaded
LOADED_MODEL=$(curl -s http://localhost:11434/api/ps | jq -r '.models[0].name // "none"')
if [ "$LOADED_MODEL" != "none" ]; then
    echo "✅ Currently loaded: $LOADED_MODEL"
else
    echo "⚠️  No model currently loaded. Will load on first request."
fi
echo ""

# Set environment for local mode
export RALPH_MODE=local
export OLLAMA_SCHED_SPREAD=1

echo "🚀 Starting Ralph with Ollama backend..."
echo "   Project: $PROJECT_NAME"
echo "   Mode: local (Ollama)"
echo ""

# Run ralph-ollama
if [ -d "$RALPH_OLLAMA_DIR" ]; then
    cd "$RALPH_OLLAMA_DIR"
    RALPH_MODE=local ./start.sh "$PROJECT_NAME" --model local
else
    echo "❌ ralph-ollama not found at $RALPH_OLLAMA_DIR"
    echo ""
    echo "Alternative: Run ralph.py directly"
    echo "  cd $PROJECT_ROOT && python3 ralph.py"
fi