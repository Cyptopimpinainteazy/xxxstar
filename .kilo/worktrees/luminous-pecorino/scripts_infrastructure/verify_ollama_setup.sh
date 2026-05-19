#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

fail() {
  echo -e "${RED}✗ $1${NC}"
  exit 1
}

pass() {
  echo -e "${GREEN}✓ $1${NC}"
}

info() {
  echo -e "${YELLOW}→ $1${NC}"
}

check_command() {
  local command_name="$1"
  if command -v "$command_name" >/dev/null 2>&1; then
    pass "Command available: $command_name"
  else
    fail "Missing command: $command_name"
  fi
}

check_ollama_daemon() {
  if ollama ps >/dev/null 2>&1; then
    pass "Ollama daemon is running"
  else
    fail "Ollama daemon is not running"
  fi
}

check_models() {
  local model_count
  model_count=$(ollama list 2>/dev/null | tail -n +2 | sed '/^\s*$/d' | wc -l | tr -d ' ')

  if [[ "$model_count" -gt 0 ]]; then
    pass "Local models available: $model_count"
  else
    fail "No local Ollama models found"
  fi
}

check_claude_mcp_ollama() {
  local mcp_output
  mcp_output=$(claude mcp list 2>&1)

  if echo "$mcp_output" | grep -Eq 'ollama: .*Connected|ollama: .*✓ Connected'; then
    pass "Claude MCP server 'ollama' is connected"
  else
    echo "$mcp_output"
    fail "Claude MCP server 'ollama' is not connected"
  fi
}

main() {
  info "Verifying Claude Code + Ollama setup"

  check_command "ollama"
  check_command "claude"

  check_ollama_daemon
  check_models
  check_claude_mcp_ollama

  echo
  pass "All checks passed. Setup is healthy."
}

main
