#!/usr/bin/env bash
# LaunchOps artifact schema-pin gate.
#
# The LaunchOps scan writes two meta-artifacts that describe every
# other artifact it produces:
#   * .launchops/artifact_manifest.json  -- list of files + kinds +
#     schema_version references
#   * .launchops/artifact_schemas.json   -- per-kind field catalog
#     (name/type/required)
#
# Every other LaunchOps gate in this repo (review-note, no-blockers,
# deep_red_flags, drift_flags, gates, readiness, test_results, rpc
# contract matrix) relies on these shapes being stable. If the
# upstream scanner silently renames a field, adds a new artifact,
# drops an artifact, or bumps a schema_version, the downstream gates
# can start reading the wrong keys and either pass-by-accident or
# fail-on-shape rather than on content.
#
# This gate pins:
#   1. top-level schema_version of each meta-artifact,
#   2. the full set of (file, kind) pairs and their per-artifact
#      schema_version / schema_ref in artifact_manifest.json, and
#   3. the set of fields (sorted name + type_name + required) per
#      kind in artifact_schemas.json.
#
# Mismatches fail the gate. To accept a real schema change run with
# UPDATE_BASELINE=1 and commit the regenerated baseline alongside the
# PR that updates the consuming gates.
set -euo pipefail

manifest=${MANIFEST_FILE:-.launchops/artifact_manifest.json}
schemas=${SCHEMAS_FILE:-.launchops/artifact_schemas.json}
baseline=${BASELINE_FILE:-.launchops/artifact_schemas.baseline.json}
update_baseline=${UPDATE_BASELINE:-0}

for f in "$manifest" "$schemas"; do
  if [[ ! -f "$f" ]]; then
    echo "::error::missing $f; run 'cargo run -p launchops -- scan' first"
    exit 1
  fi
done

if [[ ! -f "$baseline" ]]; then
  echo "::error::missing $baseline; seed it with UPDATE_BASELINE=1 $0"
  exit 1
fi

python3 - "$manifest" "$schemas" "$baseline" "$update_baseline" <<'PY'
import hashlib
import json
import sys

manifest_path, schemas_path, baseline_path, update_flag = sys.argv[1:5]

with open(manifest_path, encoding="utf-8") as handle:
    manifest = json.load(handle)
with open(schemas_path, encoding="utf-8") as handle:
    schemas = json.load(handle)
with open(baseline_path, encoding="utf-8") as handle:
    baseline = json.load(handle)


def manifest_fingerprint():
    rows = []
    for entry in manifest.get("artifacts", []):
        rows.append({
            "file": entry.get("file"),
            "kind": entry.get("kind"),
            "format": entry.get("format"),
            "schema_version": entry.get("schema_version"),
            "schema_ref": entry.get("schema_ref"),
        })
    rows.sort(key=lambda row: (row["file"] or "", row["kind"] or ""))
    return rows


def schemas_fingerprint():
    rows = []
    for entry in schemas.get("artifacts", []):
        fields = []
        for field in entry.get("fields", []) or []:
            fields.append({
                "name": field.get("name"),
                "type_name": field.get("type_name"),
                "required": bool(field.get("required", False)),
            })
        fields.sort(key=lambda f: f["name"] or "")
        shape = entry.get("shape")
        blob = json.dumps(
            {"shape": shape, "fields": fields}, sort_keys=True
        ).encode("utf-8")
        rows.append({
            "file": entry.get("file"),
            "kind": entry.get("kind"),
            "format": entry.get("format"),
            "shape": shape,
            "fields_digest": hashlib.sha256(blob).hexdigest()[:16],
            "field_names": [f["name"] for f in fields],
        })
    rows.sort(key=lambda row: (row["file"] or "", row["kind"] or ""))
    return rows


current = {
    "manifest_schema_version": manifest.get("schema_version"),
    "schemas_schema_version": schemas.get("schema_version"),
    "manifest_artifacts": manifest_fingerprint(),
    "schemas_artifacts": schemas_fingerprint(),
}

if update_flag == "1":
    baseline["manifest_schema_version"] = current["manifest_schema_version"]
    baseline["schemas_schema_version"] = current["schemas_schema_version"]
    baseline["manifest_artifacts"] = current["manifest_artifacts"]
    baseline["schemas_artifacts"] = current["schemas_artifacts"]
    baseline.setdefault(
        "accepted_reason",
        "TODO: document why this schema pin is accepted at baseline update time",
    )
    with open(baseline_path, "w", encoding="utf-8") as handle:
        json.dump(baseline, handle, indent=2)
        handle.write("\n")
    print(
        "LaunchOps artifact-schema baseline rewritten: manifest="
        f"{len(current['manifest_artifacts'])} artifacts, schemas="
        f"{len(current['schemas_artifacts'])} artifacts"
    )
    sys.exit(0)


failed = False

for key, label in [
    ("manifest_schema_version", "artifact_manifest.json schema_version"),
    ("schemas_schema_version", "artifact_schemas.json schema_version"),
]:
    expected = baseline.get(key)
    actual = current[key]
    if expected != actual:
        failed = True
        print(
            f"::error::{label} changed: expected {expected!r}, got "
            f"{actual!r}; refresh with UPDATE_BASELINE=1 after updating "
            "downstream LaunchOps gates"
        )


def diff_rows(expected_rows, actual_rows, label, key_fields):
    failures = 0
    exp_by_key = {tuple(r[k] for k in key_fields): r for r in expected_rows}
    act_by_key = {tuple(r[k] for k in key_fields): r for r in actual_rows}

    new_keys = sorted(set(act_by_key) - set(exp_by_key))
    removed_keys = sorted(set(exp_by_key) - set(act_by_key))

    if new_keys:
        failures += len(new_keys)
        print(
            f"::error::{len(new_keys)} new {label} entry/entries not in "
            "baseline; refresh with UPDATE_BASELINE=1"
        )
        for key in new_keys:
            print(f"  + {dict(zip(key_fields, key))}")

    if removed_keys:
        failures += len(removed_keys)
        print(
            f"::error::{len(removed_keys)} {label} baseline entry/entries no "
            "longer present in scan; refresh with UPDATE_BASELINE=1"
        )
        for key in removed_keys:
            print(f"  - {dict(zip(key_fields, key))}")

    for key in sorted(set(exp_by_key) & set(act_by_key)):
        exp = exp_by_key[key]
        act = act_by_key[key]
        if exp != act:
            failures += 1
            print(
                f"::error::{label} entry drifted from baseline: "
                f"{dict(zip(key_fields, key))}"
            )
            for field in sorted(set(exp) | set(act)):
                if exp.get(field) != act.get(field):
                    print(
                        f"    {field}: expected {exp.get(field)!r} "
                        f"got {act.get(field)!r}"
                    )
    return failures


failures = 0
failures += diff_rows(
    baseline.get("manifest_artifacts", []),
    current["manifest_artifacts"],
    "artifact_manifest",
    ("file", "kind"),
)
failures += diff_rows(
    baseline.get("schemas_artifacts", []),
    current["schemas_artifacts"],
    "artifact_schemas",
    ("file", "kind"),
)

if failed or failures:
    sys.exit(1)

print(
    "LaunchOps artifact-schema baseline: manifest="
    f"{len(current['manifest_artifacts'])} artifacts, schemas="
    f"{len(current['schemas_artifacts'])} artifacts match baseline"
)
PY
