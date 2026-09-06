#!/usr/bin/env bash
# Gate 19: hard-invariant gate for CycloneDX SBOMs (node + runtime).
#
# Pins for each SBOM:
#   - bomFormat, specVersion (exact)
#   - component_count (exact)
#   - global components fingerprint (sha256 of sorted purls)
#   - per-component {name, version, purl} fingerprint keyed by purl
#
# Any new component, dropped component, version bump, or source change
# (purl vcs_url or branch) fails the gate. A dependency flipping from one
# fork to another will flip its purl and therefore its fingerprint.
#
# UPDATE_BASELINE=1 regenerates the baseline, preserving accepted_reason
# per purl when the component fingerprint is unchanged.
#
# Usage: verify_sbom_baseline.sh <sbom.cdx.json> <baseline.json>

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "::error::usage: $0 <sbom.cdx.json> <baseline.json>" >&2
  exit 2
fi

SBOM="$1"
BASELINE="$2"

if [[ ! -f "$SBOM" ]]; then
  echo "::error::SBOM file not found: $SBOM" >&2
  exit 1
fi

UPDATE="${UPDATE_BASELINE:-0}"

python3 - "$SBOM" "$BASELINE" "$UPDATE" <<'PY'
import hashlib
import json
import os
import sys
from pathlib import Path

sbom_path, baseline_path, update = sys.argv[1], sys.argv[2], sys.argv[3] == "1"

sbom = json.loads(Path(sbom_path).read_text())

bom_format = sbom.get("bomFormat")
spec_version = sbom.get("specVersion")
components = sbom.get("components") or []

if bom_format != "CycloneDX":
    print(f"::error::{sbom_path}: bomFormat != 'CycloneDX' (got {bom_format!r})", file=sys.stderr)
    sys.exit(1)
if not isinstance(components, list) or not components:
    print(f"::error::{sbom_path}: components[] missing or empty", file=sys.stderr)
    sys.exit(1)


def fp(obj) -> str:
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode()).hexdigest()[:16]


def component_key(c: dict) -> str:
    # purl is the canonical identity in CycloneDX. If missing, fall back to
    # bom-ref, then name@version. In a healthy SBOM every component has purl.
    return c.get("purl") or c.get("bom-ref") or f'{c.get("name")}@{c.get("version")}'


def component_pins(c: dict) -> dict:
    return {
        "name": c.get("name"),
        "version": c.get("version"),
        "purl": c.get("purl"),
    }


# Fail-fast: duplicate purls in current SBOM would make the map ambiguous.
keys = [component_key(c) for c in components]
dupes = sorted({k for k in keys if keys.count(k) > 1})
if dupes:
    print(f"::error::{sbom_path}: duplicate component keys in SBOM: {dupes[:5]}", file=sys.stderr)
    sys.exit(1)

current_components = {component_key(c): component_pins(c) for c in components}
current_global_fp = hashlib.sha256(
    "\n".join(sorted(current_components.keys())).encode()
).hexdigest()[:16]

basename = Path(sbom_path).name

if update:
    prior = {}
    if Path(baseline_path).exists():
        try:
            prior = json.loads(Path(baseline_path).read_text()).get("components", {})
        except Exception:
            prior = {}
    out_components = {}
    for k, pins in sorted(current_components.items()):
        out_components[k] = {
            "fingerprint": fp(pins),
            "name": pins["name"],
            "version": pins["version"],
            "accepted_reason": (
                prior.get(k, {}).get("accepted_reason")
                if prior.get(k, {}).get("fingerprint") == fp(pins)
                else None
            ),
        }
    out = {
        "_comment": (
            "Baseline for CycloneDX SBOM. Pins bomFormat, specVersion, "
            "component_count, global purl fingerprint, and per-component "
            "{name, version, purl}. Regenerate with UPDATE_BASELINE=1 and fill "
            "accepted_reason per component that changed."
        ),
        "bomFormat": bom_format,
        "specVersion": spec_version,
        "component_count": len(current_components),
        "global_fingerprint": current_global_fp,
        "components": out_components,
    }
    Path(baseline_path).write_text(json.dumps(out, indent=2, sort_keys=True) + "\n")
    print(
        f"baseline rewritten for {basename}: components={len(current_components)}, "
        f"global_fp={current_global_fp}"
    )
    sys.exit(0)

if not Path(baseline_path).exists():
    print(f"::error::baseline not found: {baseline_path}. Seed with UPDATE_BASELINE=1.", file=sys.stderr)
    sys.exit(1)

baseline = json.loads(Path(baseline_path).read_text())
errors = []

if baseline.get("bomFormat") != bom_format:
    errors.append(
        f"bomFormat drifted: baseline={baseline.get('bomFormat')!r} current={bom_format!r}"
    )
if baseline.get("specVersion") != spec_version:
    errors.append(
        f"specVersion drifted: baseline={baseline.get('specVersion')!r} current={spec_version!r}"
    )
if baseline.get("component_count") != len(current_components):
    errors.append(
        f"component_count drifted: baseline={baseline.get('component_count')} "
        f"current={len(current_components)}"
    )
if baseline.get("global_fingerprint") != current_global_fp:
    errors.append(
        f"global_fingerprint drifted: baseline={baseline.get('global_fingerprint')} "
        f"current={current_global_fp}"
    )

baseline_components = baseline.get("components", {})
current_keys = set(current_components.keys())
baseline_keys = set(baseline_components.keys())

added = sorted(current_keys - baseline_keys)
removed = sorted(baseline_keys - current_keys)

# Cap noise but always fail on any drift.
for k in added[:10]:
    pins = current_components[k]
    errors.append(
        f"new component not in baseline: {pins['name']}@{pins['version']} (fp={fp(pins)})"
    )
if len(added) > 10:
    errors.append(f"... and {len(added) - 10} more new components")

for k in removed[:10]:
    b = baseline_components[k]
    errors.append(
        f"component disappeared: {b.get('name')}@{b.get('version')} (fp={b.get('fingerprint')})"
    )
if len(removed) > 10:
    errors.append(f"... and {len(removed) - 10} more removed components")

for k in sorted(current_keys & baseline_keys):
    cur_fp = fp(current_components[k])
    base_fp = baseline_components[k].get("fingerprint")
    if cur_fp != base_fp:
        pins = current_components[k]
        errors.append(
            f"component drifted: {pins['name']}@{pins['version']} "
            f"(baseline fp={base_fp} current fp={cur_fp})"
        )

if errors:
    for e in errors[:50]:
        print(f"::error::{e}", file=sys.stderr)
    if len(errors) > 50:
        print(f"::error::... and {len(errors) - 50} more drifts", file=sys.stderr)
    print(
        "hint: review dependency changes. If intended, regenerate baseline with "
        "UPDATE_BASELINE=1 and fill accepted_reason per changed component.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"{basename} matches baseline (bomFormat={bom_format}, specVersion={spec_version}, "
    f"{len(current_components)} components, global_fp={current_global_fp})"
)
PY
