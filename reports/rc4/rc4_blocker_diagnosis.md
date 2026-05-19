# RC4 Blocker Diagnosis

## Blocker Category
Script/report mismatch: shell exit code nonzero despite PASS verdict in report.

## Exact Failing Command
scripts/mainnet/rc4_runtime_upgrade_rehearsal.sh

## Exact Error Output
reports/rc4/run_exit.txt: `20260514T184625Z rc=1`

## Exact Missing File/Function/Feature
- No explicit blocker diagnosis file was written after the last run.
- Script may not exit 0 on PASS, causing automation to treat RC4 as blocked.

## Exact Fix Plan
- Patch scripts/mainnet/rc4_runtime_upgrade_rehearsal.sh to:
  - Exit 0 only if all checks pass and report verdict is PASS.
  - Write rc4_blocker_diagnosis.md if any check fails or exit code is nonzero.
- Re-run the script and confirm consistent PASS/FAIL reporting.

## Files to Patch
- scripts/mainnet/rc4_runtime_upgrade_rehearsal.sh
- (optional) reporting logic in related scripts if reused elsewhere.
