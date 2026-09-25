#!/usr/bin/env bash
# Fail when the deployable chain specs gain a problem, or lose one the baseline records.
#
# `check_deployable_bootnodes.sh` answers a question nothing else asks: can a stranger join the
# network these specs describe? The answer for the specs `Dockerfile.validator` and
# `k8s/02-configmaps.yaml` ship was no, four times over — a Live spec with an empty `bootNodes`,
# two bootNodes on `127.0.0.1`, and one Live spec with an empty `genesis.raw.top`, so a node
# started from it has nothing to sync to. Fixing them needs addresses and a genesis that only a
# real ceremony can produce, so this is a ratchet rather than a verdict: it fails when a problem
# that is not in `security/deployable-bootnode-baseline.txt` appears, and when a listed problem is
# gone (so the file cannot describe a tree that no longer exists).
#
# Usage: bash scripts/ci/check_deployable_bootnodes_ratchet.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASELINE="$ROOT/security/deployable-bootnode-baseline.txt"
CHECKER="$ROOT/scripts/ci/check_deployable_bootnodes.sh"

[ -f "$BASELINE" ] || { echo "check-deployable-bootnodes: missing $BASELINE" >&2; exit 2; }
[ -f "$CHECKER" ] || { echo "check-deployable-bootnodes: missing $CHECKER" >&2; exit 2; }

CURRENT="$(bash "$CHECKER" 2>&1 | grep '^FAIL' | sort)"
RECORDED="$(grep '^FAIL' "$BASELINE" | sort)"

NEW="$(comm -23 <(printf '%s\n' "$CURRENT") <(printf '%s\n' "$RECORDED"))"
FIXED="$(comm -13 <(printf '%s\n' "$CURRENT") <(printf '%s\n' "$RECORDED"))"

FAILURES=0
if [ -n "$NEW" ]; then
  printf 'check-deployable-bootnodes: FAIL: problem(s) not on the baseline:\n%s\n' "$NEW" >&2
  FAILURES=1
fi
if [ -n "$FIXED" ]; then
  printf 'check-deployable-bootnodes: FAIL: baseline entries that are gone; remove them from %s:\n%s\n' \
    "security/deployable-bootnode-baseline.txt" "$FIXED" >&2
  FAILURES=1
fi
[ "$FAILURES" -eq 0 ] || exit 1

COUNT="$(grep -c '^FAIL' "$BASELINE")"
echo "check-deployable-bootnodes: OK - no new problems; $COUNT known, all on the shrinking list"
