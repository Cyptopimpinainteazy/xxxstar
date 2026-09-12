#!/usr/bin/env python3
"""Repository-native X3 granular feature matrix validator and report generator.

The mandatory gate is intentionally offline and Python-stdlib-only. It validates
FEATURE_MATRIX.toml against the repository filesystem and FEATURE_REGISTRY.toml,
then generates deterministic JSON/CSV/Markdown planning reports.
"""
from __future__ import annotations

import argparse
import csv
import io
import json
from pathlib import Path
import re
import sys
import tempfile
import tomllib
from collections import defaultdict
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX = ROOT / "FEATURE_MATRIX.toml"
DEFAULT_REGISTRY = ROOT / "FEATURE_REGISTRY.toml"
DEFAULT_OUTPUT = ROOT / "reports" / "feature-matrix"

VALID_PRIORITIES = {"P0", "P1", "P2", "P3"}
VALID_CONFIDENCE = {"high", "medium", "low"}
VALID_LAUNCH_SCOPE = {"core", "guarded", "experimental", "dev_tooling", "research"}
VALID_SOURCE = {"master", "open_pr", "research", "claim_risk"}
SCORE_FIELDS = ("implemented", "tested", "mainnet_ready")
GENERATED_FILES = (
    "X3_FEATURE_MATRIX.json",
    "X3_FEATURE_MATRIX.csv",
    "X3_FEATURE_MATRIX.md",
    "X3_FEATURE_SUMMARY.md",
)


def load_matrix(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_registry(path: Path) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def composite(feature: dict[str, Any]) -> int:
    return round(
        int(feature["implemented"]) * 0.35
        + int(feature["tested"]) * 0.25
        + int(feature["mainnet_ready"]) * 0.40
    )


def readiness_class(score: int) -> str:
    if score >= 80:
        return "mainnet_candidate"
    if score >= 60:
        return "guarded_near_ready"
    if score >= 40:
        return "partial"
    if score >= 20:
        return "experimental"
    return "claim_or_research"


def _feature_label(feature: dict[str, Any]) -> str:
    return str(feature.get("id") or feature.get("name") or "<unknown feature>")


def _is_external_evidence(value: str) -> bool:
    lower = value.lower().strip()
    return lower.startswith(
        (
            "http://",
            "https://",
            "pr #",
            "pr:",
            "registry:",
            "note:",
            "workflow:",
            "commit:",
            "claim:",
        )
    )


def _local_evidence_path(value: str) -> str | None:
    if _is_external_evidence(value):
        return None
    # Allow FEATURE_REGISTRY.toml#atomic_router style anchors.
    return value.split("#", 1)[0].strip()


def _search_test_name(repo_root: Path, paths: list[str], test_paths: list[str], test_name: str) -> bool:
    roots: list[Path] = []
    for raw in paths + test_paths:
        p = repo_root / raw
        roots.append(p if p.is_dir() else p.parent)
    pattern = re.compile(rf"\bfn\s+{re.escape(test_name)}\s*\(")
    visited: set[Path] = set()
    for root in roots:
        if root in visited or not root.exists():
            continue
        visited.add(root)
        for candidate in root.rglob("*.rs"):
            try:
                if pattern.search(candidate.read_text(encoding="utf-8", errors="ignore")):
                    return True
            except OSError:
                continue
    return False


def validate_feature(
    feature: dict[str, Any],
    repo_root: Path,
    registry: dict[str, Any],
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    label = _feature_label(feature)

    required_scalar = (
        "id",
        "name",
        "subsystem",
        "source",
        "implemented",
        "tested",
        "mainnet_ready",
        "priority",
        "confidence",
        "launch_scope",
    )
    for key in required_scalar:
        if key not in feature:
            errors.append(f"{label}: missing required field '{key}'")

    for score in SCORE_FIELDS:
        value = feature.get(score)
        if not isinstance(value, int) or isinstance(value, bool) or not 0 <= value <= 100:
            errors.append(f"{label}: {score} must be between 0 and 100")

    priority = feature.get("priority")
    if priority not in VALID_PRIORITIES:
        errors.append(f"{label}: priority must be one of {sorted(VALID_PRIORITIES)}")
    confidence = feature.get("confidence")
    if confidence not in VALID_CONFIDENCE:
        errors.append(f"{label}: confidence must be one of {sorted(VALID_CONFIDENCE)}")
    launch_scope = feature.get("launch_scope")
    if launch_scope not in VALID_LAUNCH_SCOPE:
        errors.append(f"{label}: launch_scope must be one of {sorted(VALID_LAUNCH_SCOPE)}")
    source = feature.get("source")
    if source not in VALID_SOURCE:
        errors.append(f"{label}: source must be one of {sorted(VALID_SOURCE)}")

    blockers = feature.get("blockers", [])
    evidence = feature.get("evidence", [])
    paths = feature.get("paths", [])
    test_paths = feature.get("test_paths", [])
    required_tests = feature.get("required_tests", [])
    test_evidence = feature.get("test_evidence", [])
    claim_risk = bool(feature.get("claim_risk", False))

    if not isinstance(blockers, list):
        errors.append(f"{label}: blockers must be an array")
        blockers = []
    if not isinstance(evidence, list) or not evidence:
        errors.append(f"{label}: at least one evidence entry is required")
        evidence = []
    if not isinstance(paths, list):
        errors.append(f"{label}: paths must be an array")
        paths = []
    if not isinstance(test_paths, list):
        errors.append(f"{label}: test_paths must be an array")
        test_paths = []
    if not isinstance(required_tests, list):
        errors.append(f"{label}: required_tests must be an array")
        required_tests = []
    if not isinstance(test_evidence, list):
        errors.append(f"{label}: test_evidence must be an array")
        test_evidence = []

    mainnet = feature.get("mainnet_ready")
    tested = feature.get("tested")
    implemented = feature.get("implemented")

    if isinstance(mainnet, int) and mainnet < 80 and not blockers:
        errors.append(f"{label}: mainnet_ready < 80 requires at least one blocker")

    path_exempt = launch_scope == "research" or claim_risk
    if not paths and not path_exempt:
        errors.append(f"{label}: at least one path is required unless research or claim-risk")
    for raw in paths + test_paths:
        if not isinstance(raw, str) or not raw.strip():
            errors.append(f"{label}: invalid empty path")
            continue
        if not (repo_root / raw).exists():
            errors.append(f"{label}: path does not exist: {raw}")

    for item in evidence + test_evidence:
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{label}: evidence entries must be non-empty strings")
            continue
        local = _local_evidence_path(item)
        if local and not (repo_root / local).exists():
            errors.append(f"{label}: evidence path does not exist: {local}")

    for test_name in required_tests:
        if not isinstance(test_name, str) or not test_name.strip():
            errors.append(f"{label}: invalid required test name")
            continue
        if not _search_test_name(repo_root, paths, test_paths, test_name):
            errors.append(f"{label}: required test not found: {test_name}")

    if isinstance(tested, int) and tested >= 80 and not required_tests and not test_evidence:
        errors.append(f"{label}: tested >= 80 requires required_tests or test_evidence")

    registry_feature = feature.get("registry_feature")
    if registry_feature and registry_feature not in registry:
        errors.append(f"{label}: registry feature '{registry_feature}' does not exist")

    if priority == "P0" and isinstance(mainnet, int) and mainnet >= 80:
        errors.append(f"{label}: P0 feature cannot claim mainnet_ready >= 80")
    if (source == "open_pr" or feature.get("open_prs")) and isinstance(mainnet, int) and mainnet >= 80:
        errors.append(f"{label}: open PR feature cannot claim mainnet_ready >= 80")
    if claim_risk and isinstance(mainnet, int) and mainnet >= 40:
        errors.append(f"{label}: claim-risk feature must remain below 40 mainnet readiness")
    if launch_scope == "research" and isinstance(mainnet, int) and mainnet >= 40:
        errors.append(f"{label}: research feature must remain below 40 mainnet readiness")

    if isinstance(implemented, int) and isinstance(tested, int) and implemented - tested > 35:
        warnings.append(f"{label}: implementation exceeds testing by more than 35 points")
    if isinstance(tested, int) and isinstance(mainnet, int) and tested - mainnet > 30:
        warnings.append(f"{label}: testing exceeds mainnet readiness by more than 30 points")
    if source == "open_pr" or feature.get("open_prs"):
        warnings.append(f"{label}: feature includes open PR work and is not master capability")

    return errors, warnings


def validate_matrix(
    matrix: dict[str, Any],
    repo_root: Path,
    registry: dict[str, Any],
) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    features = matrix.get("feature", [])
    if not isinstance(features, list) or not features:
        return ["matrix must contain at least one [[feature]] entry"], []

    ids: set[str] = set()
    names: set[str] = set()
    for feature in features:
        if not isinstance(feature, dict):
            errors.append("feature entry must be a table")
            continue
        feature_id = feature.get("id")
        name = feature.get("name")
        if feature_id in ids:
            errors.append(f"duplicate feature id: {feature_id}")
        elif feature_id:
            ids.add(feature_id)
        if name in names:
            errors.append(f"duplicate feature name: {name}")
        elif name:
            names.add(name)
        feature_errors, feature_warnings = validate_feature(feature, repo_root, registry)
        errors.extend(feature_errors)
        warnings.extend(feature_warnings)

    return errors, warnings


def _normalized_feature(feature: dict[str, Any]) -> dict[str, Any]:
    row = dict(feature)
    row["composite"] = composite(feature)
    row["readiness_class"] = readiness_class(int(feature["mainnet_ready"]))
    return row


def _subsystem_summary(features: list[dict[str, Any]], registry: dict[str, Any]) -> list[dict[str, Any]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for feature in features:
        grouped[str(feature["subsystem"])].append(feature)
    result: list[dict[str, Any]] = []
    for subsystem in sorted(grouped):
        items = grouped[subsystem]
        count = len(items)
        result.append(
            {
                "subsystem": subsystem,
                "count": count,
                "implemented": round(sum(int(x["implemented"]) for x in items) / count, 1),
                "tested": round(sum(int(x["tested"]) for x in items) / count, 1),
                "mainnet_ready": round(sum(int(x["mainnet_ready"]) for x in items) / count, 1),
                "composite": round(sum(composite(x) for x in items) / count, 1),
                "p0_count": sum(1 for x in items if x.get("priority") == "P0"),
                "under_40_mainnet": sum(1 for x in items if int(x["mainnet_ready"]) < 40),
            }
        )
    return result


def generate_outputs(
    matrix: dict[str, Any],
    repo_root: Path,
    output_dir: Path,
    registry: dict[str, Any],
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    features = sorted(matrix.get("feature", []), key=lambda f: (str(f.get("subsystem")), str(f.get("id"))))
    normalized = [_normalized_feature(feature) for feature in features]
    summary = _subsystem_summary(features, registry)

    json_payload = {
        "meta": matrix.get("meta", {}),
        "feature_count": len(normalized),
        "features": normalized,
        "subsystems": summary,
    }
    (output_dir / "X3_FEATURE_MATRIX.json").write_text(
        json.dumps(json_payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    csv_buffer = io.StringIO(newline="")
    writer = csv.writer(csv_buffer, lineterminator="\n")
    writer.writerow(
        [
            "ID", "Feature", "Subsystem", "Paths", "Source / Open PR", "Implemented %",
            "Tested %", "Mainnet-Ready %", "Composite %", "Readiness Class", "Blockers",
            "Evidence", "Confidence", "Priority",
        ]
    )
    for feature in normalized:
        source = feature.get("source", "")
        if feature.get("open_prs"):
            source += " / " + ",".join(f"PR #{n}" for n in feature["open_prs"])
        writer.writerow(
            [
                feature["id"], feature["name"], feature["subsystem"], "; ".join(feature.get("paths", [])),
                source, feature["implemented"], feature["tested"], feature["mainnet_ready"],
                feature["composite"], feature["readiness_class"], "; ".join(feature.get("blockers", [])),
                "; ".join(feature.get("evidence", [])), feature["confidence"], feature["priority"],
            ]
        )
    (output_dir / "X3_FEATURE_MATRIX.csv").write_text(csv_buffer.getvalue(), encoding="utf-8")

    lines = [
        "# X3 Feature Matrix",
        "",
        "Generated from `FEATURE_MATRIX.toml`. This report is evidence inventory, not launch approval.",
        "",
        f"Feature count: **{len(normalized)}**",
        "",
        "| ID | Feature | Subsystem | Impl | Tested | Mainnet | Composite | Class | Priority | Source |",
        "|---|---|---|---:|---:|---:|---:|---|---|---|",
    ]
    for feature in normalized:
        source = feature.get("source", "")
        if feature.get("open_prs"):
            source += " " + " ".join(f"PR#{n}" for n in feature["open_prs"])
        lines.append(
            f"| {feature['id']} | {feature['name']} | {feature['subsystem']} | {feature['implemented']} | "
            f"{feature['tested']} | {feature['mainnet_ready']} | {feature['composite']} | "
            f"{feature['readiness_class']} | {feature['priority']} | {source} |"
        )
    lines.extend(["", "## Blockers", ""])
    for feature in normalized:
        if feature.get("blockers"):
            lines.append(f"### {feature['id']} — {feature['name']}")
            for blocker in feature["blockers"]:
                lines.append(f"- {blocker}")
            lines.append("")
    (output_dir / "X3_FEATURE_MATRIX.md").write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")

    summary_lines = [
        "# X3 Feature Summary",
        "",
        "| Subsystem | Count | Implemented | Tested | Mainnet Ready | Composite | P0 | Mainnet <40 |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for item in summary:
        summary_lines.append(
            f"| {item['subsystem']} | {item['count']} | {item['implemented']} | {item['tested']} | "
            f"{item['mainnet_ready']} | {item['composite']} | {item['p0_count']} | {item['under_40_mainnet']} |"
        )
    p0 = [f for f in normalized if f.get("priority") == "P0"]
    summary_lines.extend(["", f"## P0 features ({len(p0)})", ""])
    for feature in p0:
        summary_lines.append(f"- `{feature['id']}` {feature['name']} — mainnet {feature['mainnet_ready']}%")
    (output_dir / "X3_FEATURE_SUMMARY.md").write_text("\n".join(summary_lines).rstrip() + "\n", encoding="utf-8")


def _compare_generated(expected_dir: Path, actual_dir: Path) -> list[str]:
    mismatches: list[str] = []
    for name in GENERATED_FILES:
        expected = expected_dir / name
        actual = actual_dir / name
        if not expected.exists():
            mismatches.append(f"tracked generated output missing: {expected}")
        elif expected.read_bytes() != actual.read_bytes():
            mismatches.append(f"generated output is stale: {expected}")
    return mismatches


def _load_and_validate(matrix_path: Path, registry_path: Path, repo_root: Path):
    matrix = load_matrix(matrix_path)
    registry = load_registry(registry_path)
    errors, warnings = validate_matrix(matrix, repo_root, registry)
    return matrix, registry, errors, warnings


def _print_findings(errors: list[str], warnings: list[str]) -> None:
    for warning in warnings:
        print(f"WARNING: {warning}")
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)


def cmd_validate(args: argparse.Namespace) -> int:
    _, _, errors, warnings = _load_and_validate(args.matrix, args.registry, args.repo_root)
    _print_findings(errors, warnings)
    if errors:
        print(f"feature-matrix validation FAILED: {len(errors)} error(s)", file=sys.stderr)
        return 1
    print(f"feature-matrix validation PASS ({len(warnings)} warning(s))")
    return 0


def cmd_check(args: argparse.Namespace) -> int:
    matrix, registry, errors, warnings = _load_and_validate(args.matrix, args.registry, args.repo_root)
    _print_findings(errors, warnings)
    if errors:
        print(f"feature-matrix check FAILED: {len(errors)} error(s)", file=sys.stderr)
        return 1
    print(f"feature-matrix check PASS: {len(matrix.get('feature', []))} features, {len(warnings)} warning(s)")
    return 0


def cmd_generate(args: argparse.Namespace) -> int:
    matrix, registry, errors, warnings = _load_and_validate(args.matrix, args.registry, args.repo_root)
    _print_findings(errors, warnings)
    if errors:
        return 1
    if args.check_clean:
        with tempfile.TemporaryDirectory(prefix="x3-feature-matrix-") as tmp:
            temp_dir = Path(tmp)
            generate_outputs(matrix, args.repo_root, temp_dir, registry)
            mismatches = _compare_generated(args.output, temp_dir)
        if mismatches:
            for mismatch in mismatches:
                print(f"ERROR: {mismatch}", file=sys.stderr)
            return 1
        print("feature-matrix generated outputs are clean")
        return 0
    generate_outputs(matrix, args.repo_root, args.output, registry)
    print(f"generated feature-matrix reports under {args.output}")
    return 0


def cmd_summary(args: argparse.Namespace) -> int:
    matrix, registry, errors, warnings = _load_and_validate(args.matrix, args.registry, args.repo_root)
    _print_findings(errors, warnings)
    if errors:
        return 1
    features = matrix.get("feature", [])
    print(f"features: {len(features)}")
    for item in _subsystem_summary(features, registry):
        print(
            f"{item['subsystem']}: count={item['count']} implemented={item['implemented']} "
            f"tested={item['tested']} mainnet={item['mainnet_ready']} p0={item['p0_count']}"
        )
    return 0


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    p.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    p.add_argument("--repo-root", type=Path, default=ROOT)
    sub = p.add_subparsers(dest="command", required=True)
    sub.add_parser("validate").set_defaults(func=cmd_validate)
    sub.add_parser("check").set_defaults(func=cmd_check)
    gen = sub.add_parser("generate")
    gen.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    gen.add_argument("--check-clean", action="store_true")
    gen.set_defaults(func=cmd_generate)
    sub.add_parser("summary").set_defaults(func=cmd_summary)
    return p


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
