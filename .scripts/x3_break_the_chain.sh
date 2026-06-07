#!/usr/bin/env bash
set -euo pipefail

mkdir -p .x3/reports

report=".x3/reports/BREAK_THE_CHAIN_RESULTS.md"

cat > "${report}" <<'EOF'
# X3 Break-The-Chain Results

These attacks must be represented by tests before mainnet.

## Required Attack Tests

- [ ] Asset Kernel Supply Drift
- [ ] Cross-VM Partial Commit
- [ ] Bridge Replay
- [ ] Expired Bridge Message
- [ ] Wrong Chain ID / Domain
- [ ] DEX Reserve Drift
- [ ] Liquidity Lock Bypass
- [ ] Launchpad Anti-Rug Bypass
- [ ] Runtime Panic Path
- [ ] Genesis/Mainnet Config Footgun

## Current Automated Signal Scan

EOF

rg -n \
  -g '!launch-gates/packs/**' \
  -g '!launch-gates/reports/**' \
  -g '!**/*.cdx.json' \
  -g '!**/out/**' \
  -g '!**/target/**' \
  -g '!**/node_modules/**' \
  "replay|nonce|rollback|canonical_supply|expiry|deadline|chain_id|domain|liquidity.?lock|anti.?rug|panic" \
  runtime crates pallets contracts X3-contracts tests launch-gates \
  >> "${report}" || true

cat "${report}"
