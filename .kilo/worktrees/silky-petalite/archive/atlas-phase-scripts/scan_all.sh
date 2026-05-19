#!/usr/bin/env bash
# automation script for scanning the repo
set -e
cd "$(dirname "$0")"

# ensure node deps exist (needed for openzeppelin)
npm install --legacy-peer-deps

# regenerate remappings
cat <<'EOF' > remappings.txt
@openzeppelin/contracts-upgradeable/=contracts/@openzeppelin/contracts-upgradeable/
@openzeppelin/contracts/=contracts/@openzeppelin/contracts/
forge-std/=contracts/forge-std/src/
EOF

export SLITHER_REMAPPINGS="$(cat remappings.txt | paste -sd, -)"

# run slither on each solidity file (excluding tests to reduce noise)
find contracts -type f -name '*.sol' ! -path '*/test/*' -print0 | 
  while IFS= read -r -d '' file; do
    echo "=== SLITHER $file ==="
    slither "$file" --exclude-dependencies || true
  done

# semgrep scan
semgrep --config=auto --lang solidity contracts/ || true

# run forge tests in any subproject containing a test directory
find contracts -maxdepth 3 -type d -name test -print0 | 
  while IFS= read -r -d '' dir; do
    echo "=== FORGE TEST in $dir ==="
    (cd "$dir/.." && forge install --no-commit && forge test -vvv) || true
  done

echo "scan complete"