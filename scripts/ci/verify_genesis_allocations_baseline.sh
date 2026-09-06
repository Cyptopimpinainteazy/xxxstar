#!/usr/bin/env bash
# Hard-invariant gate for deployment/genesis/x3-testnet-allocations.json
#
# This file is the source of truth for X3 token supply and genesis
# allocation. A silent change here = broken tokenomics at launch.
#
# Pins:
#   1. token.asset_id, symbol, decimals, total_supply, total_supply_base
#      (exact string match — decimal shift = catastrophic)
#   2. Treasury: multisig_threshold, sorted signer set hash, address,
#      allocation.amount_base
#   3. Per-bucket fingerprint (bucket -> {address, amount_base})
#   4. Bonus pool breakdown (key -> percentage string)
#
# Also asserts the accounting identity:
#   treasury.allocation.amount_base
#     + sum(allocations[].amount_base)
#     == token.total_supply_base
# This catches any partial edit that forgets to rebalance.
#
# UPDATE_BASELINE=1 regenerates, preserving accepted_reason per bucket.
set -euo pipefail

CURRENT="deployment/genesis/x3-testnet-allocations.json"
BASELINE="deployment/genesis/x3-testnet-allocations.baseline.json"

if [[ ! -f "$CURRENT" ]]; then
  echo "::error::$CURRENT missing"
  exit 1
fi

if [[ "${UPDATE_BASELINE:-0}" != "1" && ! -f "$BASELINE" ]]; then
  echo "::error::$BASELINE missing (run with UPDATE_BASELINE=1 to seed)"
  exit 1
fi

python3 - "$CURRENT" "$BASELINE" <<'PY'
import json, hashlib, os, sys

current_path, baseline_path = sys.argv[1], sys.argv[2]


def fp(obj):
    return hashlib.sha256(
        json.dumps(obj, sort_keys=True).encode()
    ).hexdigest()[:16]


with open(current_path) as f:
    doc = json.load(f)

errors = []

# required top-level keys
for k in ("token", "treasury", "allocations", "bonus_pool_breakdown"):
    if k not in doc:
        errors.append(f"top-level key '{k}' missing")
if errors:
    for e in errors:
        print(f"::error::{e}")
    sys.exit(1)

token = doc["token"]
treasury = doc["treasury"]
allocations = doc["allocations"]
bonus = doc["bonus_pool_breakdown"]

for k in ("asset_id", "symbol", "decimals", "total_supply", "total_supply_base"):
    if k not in token:
        errors.append(f"token.{k} missing")
for k in ("multisig_threshold", "multisig_signers", "address", "allocation"):
    if k not in treasury:
        errors.append(f"treasury.{k} missing")
if not isinstance(allocations, list) or not allocations:
    errors.append("allocations must be a non-empty list")

# accounting identity (only enforce if we have the pieces)
if not errors:
    try:
        total = int(token["total_supply_base"])
        t_amt = int(treasury["allocation"]["amount_base"])
        a_sum = sum(int(a["amount_base"]) for a in allocations)
        if t_amt + a_sum != total:
            errors.append(
                f"accounting identity broken: treasury({t_amt}) + allocations({a_sum}) "
                f"= {t_amt + a_sum} != total_supply_base({total})"
            )
    except (KeyError, ValueError, TypeError) as exc:
        errors.append(f"accounting identity check failed to parse: {exc}")

# bucket uniqueness
buckets = [a.get("bucket") for a in allocations]
if len(buckets) != len(set(buckets)):
    from collections import Counter
    dupes = [b for b, c in Counter(buckets).items() if c > 1]
    errors.append(f"duplicate allocation bucket(s): {sorted(dupes)}")

if errors:
    for e in errors:
        print(f"::error::{e}")
    sys.exit(1)

# build normalized current shape
treasury_shape = {
    "multisig_threshold": treasury["multisig_threshold"],
    "address": treasury["address"],
    "allocation_amount_base": str(treasury["allocation"]["amount_base"]),
    "signer_set_fingerprint": fp(sorted(treasury["multisig_signers"])),
    "signer_count": len(treasury["multisig_signers"]),
}

token_shape = {
    "asset_id": token["asset_id"],
    "symbol": token["symbol"],
    "decimals": token["decimals"],
    "total_supply": str(token["total_supply"]),
    "total_supply_base": str(token["total_supply_base"]),
}

bucket_shape = {
    a["bucket"]: {
        "address": a["address"],
        "amount_base": str(a["amount_base"]),
        "fingerprint": fp(
            {
                "bucket": a["bucket"],
                "address": a["address"],
                "amount_base": str(a["amount_base"]),
            }
        ),
    }
    for a in allocations
}

bonus_shape = {str(k): str(v) for k, v in bonus.items()}

current = {
    "token": token_shape,
    "treasury": treasury_shape,
    "buckets": bucket_shape,
    "bonus_pool_breakdown": bonus_shape,
}

update = os.environ.get("UPDATE_BASELINE") == "1"

if update:
    prior_buckets = {}
    if os.path.exists(baseline_path):
        try:
            prior = json.load(open(baseline_path))
            prior_buckets = prior.get("buckets", {}) or {}
        except Exception:
            prior_buckets = {}

    out_buckets = {}
    for name, shape in bucket_shape.items():
        entry = dict(shape)
        reason = ""
        prev = prior_buckets.get(name, {})
        if (
            isinstance(prev, dict)
            and prev.get("fingerprint") == shape["fingerprint"]
        ):
            reason = prev.get("accepted_reason", "")
        entry["accepted_reason"] = reason
        out_buckets[name] = entry

    payload = {
        "_comment": (
            "Governance baseline for deployment/genesis/x3-testnet-allocations.json. "
            "Pins token supply, treasury multisig signer set (by sorted-hash), "
            "per-bucket allocation fingerprints, and bonus pool breakdown. "
            "The gate also asserts the accounting identity: "
            "treasury + sum(allocations) == total_supply_base. "
            "Regenerate with UPDATE_BASELINE=1 and fill accepted_reason per "
            "changed bucket."
        ),
        "token": token_shape,
        "treasury": treasury_shape,
        "bonus_pool_breakdown": bonus_shape,
        "buckets": out_buckets,
    }
    with open(baseline_path, "w") as f:
        json.dump(payload, f, indent=2, sort_keys=False)
        f.write("\n")
    print(
        f"baseline rewritten: total_supply_base={token_shape['total_supply_base']}, "
        f"buckets={len(out_buckets)}, signers={treasury_shape['signer_count']}"
    )
    sys.exit(0)

baseline = json.load(open(baseline_path))

# token exact
for k, cv in token_shape.items():
    bv = baseline.get("token", {}).get(k)
    if bv != cv:
        errors.append(f"token.{k} drifted: baseline={bv!r} current={cv!r}")

# treasury exact (including signer set fingerprint)
for k, cv in treasury_shape.items():
    bv = baseline.get("treasury", {}).get(k)
    if bv != cv:
        errors.append(f"treasury.{k} drifted: baseline={bv!r} current={cv!r}")

# bonus breakdown exact
base_bonus = baseline.get("bonus_pool_breakdown", {}) or {}
all_bonus_keys = sorted(set(base_bonus) | set(bonus_shape))
for k in all_bonus_keys:
    bv = base_bonus.get(k)
    cv = bonus_shape.get(k)
    if bv is None and cv is not None:
        errors.append(f"bonus_pool_breakdown.{k} new key (value={cv!r})")
    elif cv is None and bv is not None:
        errors.append(f"bonus_pool_breakdown.{k} disappeared (baseline={bv!r})")
    elif bv != cv:
        errors.append(f"bonus_pool_breakdown.{k} drifted: baseline={bv!r} current={cv!r}")

# per-bucket
base_buckets = baseline.get("buckets", {}) or {}
all_bucket_names = sorted(set(base_buckets) | set(bucket_shape))
for name in all_bucket_names:
    bv = base_buckets.get(name)
    cv = bucket_shape.get(name)
    if bv is None and cv is not None:
        errors.append(
            f"new bucket '{name}' not in baseline "
            f"(address={cv['address']}, amount_base={cv['amount_base']})"
        )
    elif cv is None and bv is not None:
        errors.append(
            f"bucket '{name}' disappeared "
            f"(baseline amount_base={bv.get('amount_base')!r})"
        )
    else:
        base_fp = bv.get("fingerprint")
        if base_fp != cv["fingerprint"]:
            errors.append(
                f"bucket '{name}' drifted: "
                f"baseline fp={base_fp} current fp={cv['fingerprint']} "
                f"(current address={cv['address']}, amount_base={cv['amount_base']})"
            )

if errors:
    for e in errors:
        print(f"::error::{e}")
    print(
        "hint: review the genesis allocation changes, then regenerate with "
        "UPDATE_BASELINE=1 and fill in accepted_reason for each changed bucket."
    )
    sys.exit(1)

print(
    f"x3-testnet-allocations.json matches baseline "
    f"(total_supply_base={token_shape['total_supply_base']}, "
    f"{len(bucket_shape)} buckets pinned, accounting identity holds)"
)
PY
