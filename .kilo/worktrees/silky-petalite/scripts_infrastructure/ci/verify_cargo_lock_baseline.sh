#!/usr/bin/env bash
# Gate 22: hard-invariant gate for Cargo.lock.
#
# Pins:
#   - version field of Cargo.lock (the format version: v3, v4, etc.)
#   - package_count (exact)
#   - global fingerprint (sha256 of sorted package keys)
#   - per-package fingerprint keyed by name@version@source over
#     {name, version, source, checksum}
#
# Any silent dependency bump, new dependency, removed dependency, source
# switch (registry<->git<->path), or checksum change fails the gate.
# Workspace crates (source == null, no checksum) are included too so
# workspace-member additions and removals are caught.
#
# UPDATE_BASELINE=1 regenerates the baseline, preserving accepted_reason
# per package key when the fingerprint is unchanged.
#
# Usage: verify_cargo_lock_baseline.sh <Cargo.lock> <baseline.json>

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "::error::usage: $0 <Cargo.lock> <baseline.json>" >&2
  exit 2
fi

LOCKFILE="$1"
BASELINE="$2"

if [[ ! -f "$LOCKFILE" ]]; then
  echo "::error::Cargo.lock not found: $LOCKFILE" >&2
  exit 1
fi

UPDATE="${UPDATE_BASELINE:-0}"

# Pick a Python >=3.11 interpreter for tomllib. CI runners ship 3.11+; local
# dev boxes may default to 3.10 with 3.11 available alongside.
PYBIN=""
for cand in python3.13 python3.12 python3.11 python3; do
  if command -v "$cand" >/dev/null 2>&1 && \
     "$cand" -c 'import sys; sys.exit(0 if sys.version_info >= (3,11) else 1)' 2>/dev/null; then
    PYBIN="$cand"
    break
  fi
done
if [[ -z "$PYBIN" ]]; then
  echo "::error::no python >=3.11 found on PATH (needed for tomllib)" >&2
  exit 2
fi

"$PYBIN" - "$LOCKFILE" "$BASELINE" "$UPDATE" <<'PY'
import hashlib
import json
import sys
import tomllib
from pathlib import Path

lock_path, baseline_path, update = sys.argv[1], sys.argv[2], sys.argv[3] == "1"

lock = tomllib.loads(Path(lock_path).read_text())

version = lock.get("version")
packages = lock.get("package") or []

if not isinstance(packages, list) or not packages:
    print(f"::error::{lock_path}: [[package]] missing or empty", file=sys.stderr)
    sys.exit(1)


def fp(obj) -> str:
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode()).hexdigest()[:16]


def pkg_key(p: dict) -> str:
    return f'{p.get("name")}@{p.get("version")}@{p.get("source") or "workspace"}'


def pkg_pins(p: dict) -> dict:
    return {
        "name": p.get("name"),
        "version": p.get("version"),
        "source": p.get("source"),
        "checksum": p.get("checksum"),
    }


# Fail-fast: duplicate (name, version, source) tuples make the map ambiguous.
keys = [pkg_key(p) for p in packages]
dupes = sorted({k for k in keys if keys.count(k) > 1})
if dupes:
    print(f"::error::{lock_path}: duplicate package keys: {dupes[:5]}", file=sys.stderr)
    sys.exit(1)

current = {pkg_key(p): pkg_pins(p) for p in packages}
current_global_fp = hashlib.sha256(
    "\n".join(sorted(current.keys())).encode()
).hexdigest()[:16]

basename = Path(lock_path).name

if update:
    prior = {}
    if Path(baseline_path).exists():
        try:
            prior = json.loads(Path(baseline_path).read_text()).get("packages", {})
        except Exception:
            prior = {}
    out_pkgs = {}
    for k, pins in sorted(current.items()):
        out_pkgs[k] = {
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
            "Baseline for Cargo.lock. Pins lockfile version, package_count, "
            "global package-key fingerprint, and per-package "
            "{name, version, source, checksum}. Regenerate with "
            "UPDATE_BASELINE=1 and fill accepted_reason per changed entry."
        ),
        "version": version,
        "package_count": len(current),
        "global_fingerprint": current_global_fp,
        "packages": out_pkgs,
    }
    Path(baseline_path).write_text(json.dumps(out, indent=2, sort_keys=True) + "\n")
    print(
        f"baseline rewritten for {basename}: packages={len(current)}, "
        f"global_fp={current_global_fp}"
    )
    sys.exit(0)

if not Path(baseline_path).exists():
    print(f"::error::baseline not found: {baseline_path}. Seed with UPDATE_BASELINE=1.", file=sys.stderr)
    sys.exit(1)

baseline = json.loads(Path(baseline_path).read_text())
errors = []

if baseline.get("version") != version:
    errors.append(
        f"lockfile version drifted: baseline={baseline.get('version')!r} current={version!r}"
    )
if baseline.get("package_count") != len(current):
    errors.append(
        f"package_count drifted: baseline={baseline.get('package_count')} "
        f"current={len(current)}"
    )
if baseline.get("global_fingerprint") != current_global_fp:
    errors.append(
        f"global_fingerprint drifted: baseline={baseline.get('global_fingerprint')} "
        f"current={current_global_fp}"
    )

baseline_pkgs = baseline.get("packages", {})
current_keys = set(current.keys())
baseline_keys = set(baseline_pkgs.keys())

added = sorted(current_keys - baseline_keys)
removed = sorted(baseline_keys - current_keys)

for k in added[:10]:
    pins = current[k]
    errors.append(
        f"new package not in baseline: {pins['name']}@{pins['version']} "
        f"(source={pins.get('source') or 'workspace'}, fp={fp(pins)})"
    )
if len(added) > 10:
    errors.append(f"... and {len(added) - 10} more new packages")

for k in removed[:10]:
    b = baseline_pkgs[k]
    errors.append(
        f"package disappeared: {b.get('name')}@{b.get('version')} (fp={b.get('fingerprint')})"
    )
if len(removed) > 10:
    errors.append(f"... and {len(removed) - 10} more removed packages")

for k in sorted(current_keys & baseline_keys):
    cur_fp = fp(current[k])
    base_fp = baseline_pkgs[k].get("fingerprint")
    if cur_fp != base_fp:
        pins = current[k]
        errors.append(
            f"package drifted: {pins['name']}@{pins['version']} "
            f"(baseline fp={base_fp} current fp={cur_fp})"
        )

if errors:
    for e in errors[:50]:
        print(f"::error::{e}", file=sys.stderr)
    if len(errors) > 50:
        print(f"::error::... and {len(errors) - 50} more drifts", file=sys.stderr)
    print(
        "hint: if dependency changes are intended, regenerate baseline with "
        "UPDATE_BASELINE=1 and fill accepted_reason per changed package.",
        file=sys.stderr,
    )
    sys.exit(1)

print(
    f"{basename} matches baseline (version={version}, {len(current)} packages, "
    f"global_fp={current_global_fp})"
)
PY
