#!/usr/bin/env bash
# Hard-invariant gate for Substrate raw chain spec files.
#
# A chain spec's genesis state IS the consensus contract. A silent byte
# change in :code (runtime WASM) or any storage key at genesis = a forked
# chain. This gate pins:
#
#   1. Top-level scalars: name, id, chainType, protocolId
#   2. properties (exact)
#   3. bootNodes count + fingerprint (sorted)
#   4. codeSubstitutes (keys sorted + per-key fingerprint)
#   5. :code runtime WASM: sha256 of the raw value + byte length
#   6. Every other genesis.raw.top key: fingerprint of the value
#   7. genesis.raw.childrenDefault: fingerprint + key count
#
# Usage: verify_chain_spec_baseline.sh <spec.json> <baseline.json>
# UPDATE_BASELINE=1 regenerates baseline, preserving accepted_reason per key.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "::error::usage: $0 <spec.json> <baseline.json>"
  exit 1
fi

CURRENT="$1"
BASELINE="$2"

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

# Well-known Substrate storage keys worth identifying in error messages.
CODE_KEY = "0x3a636f6465"  # :code (runtime WASM)


def fp(obj):
    return hashlib.sha256(
        json.dumps(obj, sort_keys=True).encode()
    ).hexdigest()[:16]


def sha256_hex(s):
    return hashlib.sha256(s.encode()).hexdigest()


with open(current_path) as f:
    doc = json.load(f)

errors = []

for k in ("name", "id", "chainType", "protocolId", "genesis"):
    if k not in doc:
        errors.append(f"top-level key '{k}' missing")

if errors:
    for e in errors:
        print(f"::error::{e}")
    sys.exit(1)

genesis = doc["genesis"]
if "raw" not in genesis or "top" not in genesis["raw"]:
    print("::error::genesis.raw.top missing (not a raw chain spec?)")
    sys.exit(1)

top = genesis["raw"]["top"]
children_default = genesis["raw"].get("childrenDefault", {}) or {}

if CODE_KEY not in top:
    print(f"::error::genesis.raw.top missing :code key ({CODE_KEY})")
    sys.exit(1)

code_value = top[CODE_KEY]
if not isinstance(code_value, str) or not code_value.startswith("0x"):
    print("::error:::code value is not a 0x-prefixed hex string")
    sys.exit(1)

# WASM invariants
wasm_hex = code_value[2:]
wasm_len_bytes = len(wasm_hex) // 2
wasm_sha256 = sha256_hex(code_value)

# Storage keys (excluding :code which is pinned separately)
storage_shape = {}
for k, v in top.items():
    if k == CODE_KEY:
        continue
    # Value fingerprint: hash of the raw value (hex string)
    storage_shape[k] = {
        "value_fingerprint": fp(v),
        "value_length": len(v) if isinstance(v, str) else None,
    }

# childrenDefault fingerprint
children_shape = {
    "key_count": len(children_default),
    "fingerprint": fp(children_default),
}

# bootNodes
boot_nodes = doc.get("bootNodes", []) or []
boot_shape = {
    "count": len(boot_nodes),
    "fingerprint": fp(sorted(boot_nodes)),
}

# codeSubstitutes
code_subs = doc.get("codeSubstitutes", {}) or {}
code_subs_shape = {
    "keys_sorted": sorted(code_subs.keys()),
    "fingerprint": fp(code_subs),
}

# Top-level scalars
top_level = {
    "name": doc["name"],
    "id": doc["id"],
    "chainType": doc["chainType"],
    "protocolId": doc["protocolId"],
    "properties": doc.get("properties"),
}

# :code shape
code_shape = {
    "sha256": wasm_sha256,
    "length_bytes": wasm_len_bytes,
}

current = {
    "top_level": top_level,
    "code": code_shape,
    "bootNodes": boot_shape,
    "codeSubstitutes": code_subs_shape,
    "childrenDefault": children_shape,
    "storage_keys": storage_shape,
}

update = os.environ.get("UPDATE_BASELINE") == "1"

if update:
    prior_storage = {}
    if os.path.exists(baseline_path):
        try:
            prior = json.load(open(baseline_path))
            prior_storage = prior.get("storage_keys", {}) or {}
        except Exception:
            prior_storage = {}

    out_storage = {}
    for k, shape in storage_shape.items():
        entry = dict(shape)
        reason = ""
        prev = prior_storage.get(k, {})
        if (
            isinstance(prev, dict)
            and prev.get("value_fingerprint") == shape["value_fingerprint"]
        ):
            reason = prev.get("accepted_reason", "")
        entry["accepted_reason"] = reason
        out_storage[k] = entry

    payload = {
        "_comment": (
            f"Governance baseline for {os.path.basename(current_path)}. "
            "Pins chain spec top-level scalars, bootNodes, codeSubstitutes, "
            "the runtime WASM (:code sha256 + byte length), every genesis "
            "storage key value fingerprint, and childrenDefault. "
            "A single byte change in WASM or any pinned storage value = "
            "forked chain. Regenerate with UPDATE_BASELINE=1 and fill "
            "accepted_reason for each changed storage key."
        ),
        "top_level": top_level,
        "code": code_shape,
        "bootNodes": boot_shape,
        "codeSubstitutes": code_subs_shape,
        "childrenDefault": children_shape,
        "storage_keys": out_storage,
    }
    with open(baseline_path, "w") as f:
        json.dump(payload, f, indent=2, sort_keys=False)
        f.write("\n")
    print(
        f"baseline rewritten for {os.path.basename(current_path)}: "
        f"storage_keys={len(out_storage)}, wasm_bytes={wasm_len_bytes}, "
        f"wasm_sha256={wasm_sha256[:16]}..."
    )
    sys.exit(0)

baseline = json.load(open(baseline_path))

# top_level (exact match)
for k, cv in top_level.items():
    bv = baseline.get("top_level", {}).get(k)
    if bv != cv:
        errors.append(f"top_level.{k} drifted: baseline={bv!r} current={cv!r}")

# :code (exact match)
for k, cv in code_shape.items():
    bv = baseline.get("code", {}).get(k)
    if bv != cv:
        errors.append(f"code.{k} drifted: baseline={bv!r} current={cv!r}")

# bootNodes
for k, cv in boot_shape.items():
    bv = baseline.get("bootNodes", {}).get(k)
    if bv != cv:
        errors.append(f"bootNodes.{k} drifted: baseline={bv!r} current={cv!r}")

# codeSubstitutes
for k, cv in code_subs_shape.items():
    bv = baseline.get("codeSubstitutes", {}).get(k)
    if bv != cv:
        errors.append(f"codeSubstitutes.{k} drifted: baseline={bv!r} current={cv!r}")

# childrenDefault
for k, cv in children_shape.items():
    bv = baseline.get("childrenDefault", {}).get(k)
    if bv != cv:
        errors.append(f"childrenDefault.{k} drifted: baseline={bv!r} current={cv!r}")

# storage keys (per-key)
base_storage = baseline.get("storage_keys", {}) or {}
all_keys = sorted(set(base_storage) | set(storage_shape))
for k in all_keys:
    bv = base_storage.get(k)
    cv = storage_shape.get(k)
    if bv is None and cv is not None:
        errors.append(
            f"new genesis storage key not in baseline: {k} "
            f"(len={cv.get('value_length')}, fp={cv['value_fingerprint']})"
        )
    elif cv is None and bv is not None:
        errors.append(
            f"genesis storage key disappeared: {k} "
            f"(baseline fp={bv.get('value_fingerprint')})"
        )
    else:
        if bv.get("value_fingerprint") != cv["value_fingerprint"]:
            errors.append(
                f"genesis storage key {k} value drifted: "
                f"baseline fp={bv.get('value_fingerprint')} "
                f"current fp={cv['value_fingerprint']} "
                f"(current len={cv.get('value_length')})"
            )

if errors:
    for e in errors:
        print(f"::error::{e}")
    print(
        "hint: review chain spec changes. If intended, regenerate baseline "
        "with UPDATE_BASELINE=1 and fill accepted_reason per changed key."
    )
    sys.exit(1)

print(
    f"{os.path.basename(current_path)} matches baseline "
    f"(name={top_level['name']!r}, id={top_level['id']!r}, "
    f"{len(storage_shape)} storage keys + :code pinned, "
    f"wasm={wasm_len_bytes} bytes sha256={wasm_sha256[:16]}...)"
)
PY
