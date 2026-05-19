#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "Usage: $0 <role> '<prompt>'"
  echo "Roles: planner|code|testbuilder|auditor|breaker|marketing|grant|generic"
  exit 1
fi

ROLE="$1"
PROMPT="$2"

PREFIX=""
case "$ROLE" in
  planner) PREFIX="[PlannerAgent GPU0]" ;;
  code) PREFIX="[CodeAgent GPU0]" ;;
  testbuilder) PREFIX="[TestBuilderAgent GPU1]" ;;
  auditor) PREFIX="[AuditorAgent GPU2]" ;;
  breaker) PREFIX="[BreakerAgent GPU2]" ;;
  marketing) PREFIX="[MarketingAgent GPU3]" ;;
  grant) PREFIX="[GrantAgent GPU3]" ;;
  generic) PREFIX="[GenericAgent]" ;;
  *)
    echo "Unknown role: $ROLE"
    exit 1
    ;;
esac

echo "Asking GPU worker for role $ROLE."
if command -v ollama >/dev/null 2>&1; then
  ollama run qwen3:8b "$PREFIX $PROMPT"
else
  echo "ERROR: ollama not installed or not available in PATH."
  exit 1
fi
