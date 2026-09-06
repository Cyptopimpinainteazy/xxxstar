#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

require_grep() {
  local pattern="$1"
  local file="$2"
  local message="$3"

  if ! grep -Eq "$pattern" "$file"; then
    echo "foundry-production-check: FAIL: $message" >&2
    echo "  missing pattern: $pattern" >&2
    echo "  file: $file" >&2
    exit 1
  fi
}

require_absent() {
  local pattern="$1"
  local file="$2"
  local message="$3"

  if grep -Eq "$pattern" "$file"; then
    echo "foundry-production-check: FAIL: $message" >&2
    echo "  forbidden pattern: $pattern" >&2
    echo "  file: $file" >&2
    exit 1
  fi
}

echo "foundry-production-check: verifying canonical fee defaults"
require_grep 'DEFAULT_MIN_PLATFORM_FEE_BPS = 200' \
  'X3-contracts/evm/contracts/foundry/FoundryFeeConfig.sol' \
  'Solidity fee floor must match canonical 2% platform minimum'
require_grep 'DEFAULT_PLATFORM_FEE_BPS: u64 = 200' \
  'crates/x3-foundry-revenue/src/lib.rs' \
  'Rust revenue calculator must default to 2% platform share'
require_grep 'DEFAULT_CREATOR_FEE_BPS: u64 = 9_700' \
  'crates/x3-foundry-revenue/src/lib.rs' \
  'Rust revenue calculator must default to 97% creator share'
require_grep 'DEFAULT_REFERRAL_FEE_BPS: u64 = 50' \
  'crates/x3-foundry-revenue/src/lib.rs' \
  'Rust revenue calculator must default to 0.5% referral share'
require_grep 'DEFAULT_TREASURY_FEE_BPS: u64 = 50' \
  'crates/x3-foundry-revenue/src/lib.rs' \
  'Rust revenue calculator must default to 0.5% treasury share'

echo "foundry-production-check: verifying no silent simulated mainnet deploys"
require_grep 'X3_FOUNDRY_ALLOW_SIMULATED_DEPLOY' \
  'crates/x3-foundry-core/src/deployer.rs' \
  'Production-like simulated deploys must require an explicit drill escape hatch'
require_grep 'Refusing simulated Foundry deployment' \
  'crates/x3-foundry-core/src/deployer.rs' \
  'Production-like simulated deploys must fail closed by default'
require_absent 'No deployment path silently returns simulated success in production mode\. \[ \]' \
  'docs/current/FAILURES_AND_TODOS.md' \
  'Do not mark production deploy safety complete in TODO docs unless verified'

echo "foundry-production-check: PASS"
