#!/usr/bin/env bash
# Contract-matrix invariants: the RPC contract matrix must never contain
# duplicate registrations (two sites registering the same method, which would
# let one silently override the other) or bucket drift (a method's classifier
# bucket disagreeing with its declared consumer bucket). Both are reported by
# `launchops scan` but were previously never enforced; this script turns them
# into hard CI failures.
set -euo pipefail

matrix=${MATRIX_FILE:-.launchops/rpc_contract_matrix.json}

if [[ ! -f "$matrix" ]]; then
  echo "::error::missing $matrix; run 'cargo run -p launchops -- scan' first"
  exit 1
fi

python3 - "$matrix" <<'PY'
import json
import sys

matrix_path = sys.argv[1]

with open(matrix_path, encoding="utf-8") as handle:
    matrix = json.load(handle)

dup_count = matrix.get("duplicate_registration_count", 0)
drift_count = matrix.get("bucket_drift_count", 0)

failed = False

if dup_count:
    failed = True
    print(
        f"::error::rpc_contract_matrix reports {dup_count} duplicate "
        "registration(s); two or more sites are registering the same RPC "
        "method and one will silently override the other"
    )
    for method in matrix.get("methods", []):
        registrations = method.get("registrations") or []
        if len(registrations) > 1:
            sites = ", ".join(
                f"{reg.get('file', '?')}:{reg.get('line', '?')}"
                for reg in registrations
            )
            print(f"  - {method.get('method', '?')} @ {sites}")

if drift_count:
    failed = True
    print(
        f"::error::rpc_contract_matrix reports {drift_count} bucket drift "
        "flag(s); a method's declared consumer bucket disagrees with its "
        "classifier bucket"
    )
    for method in matrix.get("methods", []):
        if method.get("bucket_drift"):
            print(
                f"  - {method.get('method', '?')} "
                f"declared={method.get('declared_bucket', '?')} "
                f"classified={method.get('classified_bucket', '?')}"
            )

if failed:
    sys.exit(1)

print("LaunchOps rpc_contract_matrix invariants: OK")
PY
