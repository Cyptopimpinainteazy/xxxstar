#!/usr/bin/env bash
set -euo pipefail

mkdir -p .reports

OLD_PROJECT_ROOT="${OLD_PROJECT_ROOT:-../old-x3-project}"
roots=(.)

if [ -d "$OLD_PROJECT_ROOT" ]; then
  roots+=("$OLD_PROJECT_ROOT")
fi

rg --hidden -n \
  -g '!node_modules/**' \
  -g '!target/**' \
  -g '!target_strict/**' \
  -g '!.git/**' \
  -g '!dist/**' \
  -g '!build/**' \
  -g '!coverage/**' \
  -g '!.reports/x3_smells.txt' \
  -g '!.reports/smells.txt' \
  -g '!.cache/**' \
  -g '!CODE_COVERAGE_TRACKER.md' \
  "TODO|FIXME|stub|mock|placeholder|unwrap\(|expect\(|panic!|unimplemented!|todo!|unsafe|hardcoded|localhost|1704067200|H256::from_low_u64_be|canonical_supply|nonce|replay|rollback|expiry|deadline|bridge|genesis|chain_spec" \
  "${roots[@]}" \
  | sed -E 's/((SECRET|TOKEN|KEY|PASSWORD|PASS|PRIVATE|SEED|MNEMONIC|API_KEY)[A-Z0-9_]*[[:space:]]*[:=][[:space:]]*)[^[:space:]]+/\1[REDACTED_SECRET]/Ig' \
  > .reports/x3_smells.txt || true

cp .reports/x3_smells.txt .reports/smells.txt

cat .reports/x3_smells.txt
