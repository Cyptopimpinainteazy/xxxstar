#!/usr/bin/env python3
"""Validate and generate X3's granular feature-readiness matrix.

Mandatory CI is offline and Python-3.11-stdlib-only. FEATURE_MATRIX.toml is the
canonical manifest; it may include reviewable subsystem TOML fragments.
"""
from __future__ import annotations

import argparse
import csv
import io
import json
from collections import defaultdict
from pathlib import Path
import re
import sys
import tempfile
from typing import Any

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # Python 3.10 and older
    # The gate is run by whatever python3 the box has; on 3.10 that is the
    # `tomli` backport. Without one of the two there is no TOML parser and the
    # check cannot mean anything, so say that rather than failing on an import.
    try:
        import tomli as tomllib
    except ModuleNotFoundError as exc:  # pragma: no cover - environment guard
        raise SystemExit(
            "feature_matrix needs a TOML parser: Python 3.11+ (tomllib) or the `tomli` backport"
        ) from exc

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX = ROOT / "FEATURE_MATRIX.toml"
DEFAULT_REGISTRY = ROOT / "FEATURE_REGISTRY.toml"
DEFAULT_OUTPUT = ROOT / "reports" / "feature-matrix"
VALID_PRIORITIES = {"P0", "P1", "P2", "P3"}
VALID_CONFIDENCE = {"high", "medium", "low"}
VALID_SCOPE = {"core", "guarded", "experimental", "dev_tooling", "research"}
VALID_SOURCE = {"master", "open_pr", "research", "claim_risk"}
GENERATED = ("X3_FEATURE_MATRIX.json", "X3_FEATURE_MATRIX.csv", "X3_FEATURE_MATRIX.md", "X3_FEATURE_SUMMARY.md")


def _read_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as fh:
        return tomllib.load(fh)


def load_matrix(path: Path) -> dict[str, Any]:
    """Load root manifest plus deterministic subsystem fragments."""
    data = _read_toml(path)
    features = list(data.get("feature", []))
    for rel in data.get("includes", []):
        fragment = path.parent / str(rel)
        fragment_data = _read_toml(fragment)
        features.extend(fragment_data.get("feature", []))
    data["feature"] = features
    return data


def load_registry(path: Path) -> dict[str, Any]:
    return _read_toml(path)


def composite(feature: dict[str, Any]) -> int:
    return round(feature["implemented"] * .35 + feature["tested"] * .25 + feature["mainnet_ready"] * .40)


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


def _external_evidence(value: str) -> bool:
    return value.lower().strip().startswith(("http://", "https://", "pr #", "pr:", "registry:", "note:", "workflow:", "commit:", "claim:"))


def _evidence_path(value: str) -> str | None:
    return None if _external_evidence(value) else value.split("#", 1)[0].strip()


def _test_exists(root: Path, paths: list[str], test_paths: list[str], name: str) -> bool:
    pattern = re.compile(rf"\bfn\s+{re.escape(name)}\s*\(")
    search_roots: set[Path] = set()
    for raw in paths + test_paths:
        p = root / raw
        search_roots.add(p if p.is_dir() else p.parent)
    for base in search_roots:
        if not base.exists():
            continue
        for rs in base.rglob("*.rs"):
            try:
                if pattern.search(rs.read_text(encoding="utf-8", errors="ignore")):
                    return True
            except OSError:
                pass
    return False


def validate_feature(feature: dict[str, Any], root: Path, registry: dict[str, Any]) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    label = str(feature.get("id") or feature.get("name") or "<unknown>")
    required = ("id", "name", "subsystem", "source", "implemented", "tested", "mainnet_ready", "priority", "confidence", "launch_scope")
    for key in required:
        if key not in feature:
            errors.append(f"{label}: missing required field '{key}'")
    for key in ("implemented", "tested", "mainnet_ready"):
        v = feature.get(key)
        if not isinstance(v, int) or isinstance(v, bool) or not 0 <= v <= 100:
            errors.append(f"{label}: {key} must be between 0 and 100")
    if feature.get("priority") not in VALID_PRIORITIES:
        errors.append(f"{label}: invalid priority")
    if feature.get("confidence") not in VALID_CONFIDENCE:
        errors.append(f"{label}: invalid confidence")
    if feature.get("launch_scope") not in VALID_SCOPE:
        errors.append(f"{label}: invalid launch_scope")
    if feature.get("source") not in VALID_SOURCE:
        errors.append(f"{label}: invalid source")

    blockers = feature.get("blockers", [])
    evidence = feature.get("evidence", [])
    paths = feature.get("paths", [])
    test_paths = feature.get("test_paths", [])
    tests = feature.get("required_tests", [])
    test_evidence = feature.get("test_evidence", [])
    claim_risk = bool(feature.get("claim_risk", False))
    mainnet = feature.get("mainnet_ready")

    if not isinstance(blockers, list):
        errors.append(f"{label}: blockers must be an array"); blockers = []
    if not isinstance(evidence, list) or not evidence:
        errors.append(f"{label}: at least one evidence entry is required"); evidence = []
    if not isinstance(paths, list):
        errors.append(f"{label}: paths must be an array"); paths = []
    if not isinstance(test_paths, list):
        errors.append(f"{label}: test_paths must be an array"); test_paths = []
    if not isinstance(tests, list):
        errors.append(f"{label}: required_tests must be an array"); tests = []
    if not isinstance(test_evidence, list):
        errors.append(f"{label}: test_evidence must be an array"); test_evidence = []

    if isinstance(mainnet, int) and mainnet < 80 and not blockers:
        errors.append(f"{label}: mainnet_ready < 80 requires at least one blocker")
    exempt = feature.get("launch_scope") == "research" or claim_risk
    if not paths and not exempt:
        errors.append(f"{label}: at least one path is required unless research or claim-risk")
    for raw in paths + test_paths:
        if not isinstance(raw, str) or not raw.strip() or not (root / raw).exists():
            errors.append(f"{label}: path does not exist: {raw}")
    for item in evidence + test_evidence:
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{label}: evidence entries must be non-empty strings")
            continue
        local = _evidence_path(item)
        if local and not (root / local).exists():
            errors.append(f"{label}: evidence path does not exist: {local}")
    for test in tests:
        if not isinstance(test, str) or not _test_exists(root, paths, test_paths, test):
            errors.append(f"{label}: required test not found: {test}")
    if isinstance(feature.get("tested"), int) and feature["tested"] >= 80 and not tests and not test_evidence:
        errors.append(f"{label}: tested >= 80 requires required_tests or test_evidence")
    reg = feature.get("registry_feature")
    if reg and reg not in registry:
        errors.append(f"{label}: registry feature '{reg}' does not exist")
    if feature.get("priority") == "P0" and isinstance(mainnet, int) and mainnet >= 80:
        errors.append(f"{label}: P0 feature cannot claim mainnet_ready >= 80")
    if (feature.get("source") == "open_pr" or feature.get("open_prs")) and isinstance(mainnet, int) and mainnet >= 80:
        errors.append(f"{label}: open PR feature cannot claim mainnet_ready >= 80")
    if claim_risk and isinstance(mainnet, int) and mainnet >= 40:
        errors.append(f"{label}: claim-risk feature must remain below 40 mainnet readiness")
    if feature.get("launch_scope") == "research" and isinstance(mainnet, int) and mainnet >= 40:
        errors.append(f"{label}: research feature must remain below 40 mainnet readiness")
    if isinstance(feature.get("implemented"), int) and isinstance(feature.get("tested"), int) and feature["implemented"] - feature["tested"] > 35:
        warnings.append(f"{label}: implementation exceeds testing by more than 35 points")
    if isinstance(feature.get("tested"), int) and isinstance(mainnet, int) and feature["tested"] - mainnet > 30:
        warnings.append(f"{label}: testing exceeds mainnet readiness by more than 30 points")
    if feature.get("source") == "open_pr" or feature.get("open_prs"):
        warnings.append(f"{label}: feature includes open PR work and is not master capability")
    return errors, warnings


def validate_matrix(matrix: dict[str, Any], root: Path, registry: dict[str, Any]) -> tuple[list[str], list[str]]:
    features = matrix.get("feature", [])
    if not isinstance(features, list) or not features:
        return ["matrix must contain at least one feature"], []
    errors: list[str] = []
    warnings: list[str] = []
    expected = matrix.get("meta", {}).get("feature_count_expected")
    if expected is not None and expected != len(features):
        errors.append(f"feature count mismatch: expected {expected}, loaded {len(features)}")
    ids: set[str] = set(); names: set[str] = set()
    for feature in features:
        if not isinstance(feature, dict):
            errors.append("feature entry must be a table"); continue
        fid = feature.get("id"); name = feature.get("name")
        if fid in ids: errors.append(f"duplicate feature id: {fid}")
        elif fid: ids.add(fid)
        if name in names: errors.append(f"duplicate feature name: {name}")
        elif name: names.add(name)
        e, w = validate_feature(feature, root, registry); errors += e; warnings += w
    return errors, warnings


def _normalized(feature: dict[str, Any]) -> dict[str, Any]:
    row = dict(feature); row["composite"] = composite(feature); row["readiness_class"] = readiness_class(feature["mainnet_ready"]); return row


def _summary(features: list[dict[str, Any]]) -> list[dict[str, Any]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for f in features: groups[f["subsystem"]].append(f)
    out = []
    for subsystem in sorted(groups):
        items = groups[subsystem]; n = len(items)
        out.append({"subsystem": subsystem, "count": n, "implemented": round(sum(x["implemented"] for x in items)/n,1), "tested": round(sum(x["tested"] for x in items)/n,1), "mainnet_ready": round(sum(x["mainnet_ready"] for x in items)/n,1), "composite": round(sum(composite(x) for x in items)/n,1), "p0_count": sum(x["priority"]=="P0" for x in items), "under_40_mainnet": sum(x["mainnet_ready"]<40 for x in items)})
    return out


def generate_outputs(matrix: dict[str, Any], root: Path, outdir: Path, registry: dict[str, Any]) -> None:
    outdir.mkdir(parents=True, exist_ok=True)
    features = sorted(matrix["feature"], key=lambda f: (f["subsystem"], f["id"]))
    rows = [_normalized(f) for f in features]; summary = _summary(features)
    payload = {"meta": matrix.get("meta", {}), "feature_count": len(rows), "features": rows, "subsystems": summary}
    (outdir/"X3_FEATURE_MATRIX.json").write_text(json.dumps(payload, indent=2, sort_keys=True)+"\n", encoding="utf-8")
    buf = io.StringIO(newline=""); writer = csv.writer(buf, lineterminator="\n")
    writer.writerow(["ID","Feature","Subsystem","Paths","Source / Open PR","Implemented %","Tested %","Mainnet-Ready %","Composite %","Readiness Class","Blockers","Evidence","Confidence","Priority"])
    for f in rows:
        source = f["source"] + ((" / " + ",".join(f"PR #{n}" for n in f.get("open_prs",[]))) if f.get("open_prs") else "")
        writer.writerow([f["id"],f["name"],f["subsystem"],"; ".join(f.get("paths",[])),source,f["implemented"],f["tested"],f["mainnet_ready"],f["composite"],f["readiness_class"],"; ".join(f.get("blockers",[])),"; ".join(f.get("evidence",[])),f["confidence"],f["priority"]])
    (outdir/"X3_FEATURE_MATRIX.csv").write_text(buf.getvalue(), encoding="utf-8")
    md=["# X3 Feature Matrix","","Generated from `FEATURE_MATRIX.toml` + subsystem fragments. This is evidence inventory, not launch approval.","",f"Feature count: **{len(rows)}**","","| ID | Feature | Subsystem | Impl | Tested | Mainnet | Composite | Class | Priority | Source |","|---|---|---|---:|---:|---:|---:|---|---|---|"]
    for f in rows:
        source=f["source"]+(" "+" ".join(f"PR#{n}" for n in f.get("open_prs",[])) if f.get("open_prs") else "")
        md.append(f"| {f['id']} | {f['name']} | {f['subsystem']} | {f['implemented']} | {f['tested']} | {f['mainnet_ready']} | {f['composite']} | {f['readiness_class']} | {f['priority']} | {source} |")
    md += ["","## Blockers",""]
    for f in rows:
        if f.get("blockers"):
            md.append(f"### {f['id']} — {f['name']}"); md += [f"- {b}" for b in f["blockers"]]; md.append("")
    (outdir/"X3_FEATURE_MATRIX.md").write_text("\n".join(md).rstrip()+"\n", encoding="utf-8")
    sm=["# X3 Feature Summary","","| Subsystem | Count | Implemented | Tested | Mainnet Ready | Composite | P0 | Mainnet <40 |","|---|---:|---:|---:|---:|---:|---:|---:|"]
    for x in summary: sm.append(f"| {x['subsystem']} | {x['count']} | {x['implemented']} | {x['tested']} | {x['mainnet_ready']} | {x['composite']} | {x['p0_count']} | {x['under_40_mainnet']} |")
    p0=[f for f in rows if f["priority"]=="P0"]; sm += ["",f"## P0 features ({len(p0)})",""]+[f"- `{f['id']}` {f['name']} — mainnet {f['mainnet_ready']}%" for f in p0]
    (outdir/"X3_FEATURE_SUMMARY.md").write_text("\n".join(sm).rstrip()+"\n", encoding="utf-8")


def _load(matrix: Path, registry: Path, root: Path):
    m=load_matrix(matrix); r=load_registry(registry); e,w=validate_matrix(m,root,r); return m,r,e,w


def _print(e: list[str], w: list[str]) -> None:
    for x in w: print(f"WARNING: {x}")
    for x in e: print(f"ERROR: {x}", file=sys.stderr)


def cmd_check(args: argparse.Namespace) -> int:
    m,_,e,w=_load(args.matrix,args.registry,args.repo_root); _print(e,w)
    if e: print(f"feature-matrix check FAILED: {len(e)} error(s)",file=sys.stderr); return 1
    print(f"feature-matrix check PASS: {len(m['feature'])} features, {len(w)} warning(s)"); return 0


def cmd_generate(args: argparse.Namespace) -> int:
    m,r,e,w=_load(args.matrix,args.registry,args.repo_root); _print(e,w)
    if e: return 1
    if args.check_clean:
        with tempfile.TemporaryDirectory(prefix="x3-feature-matrix-") as td:
            temp=Path(td); generate_outputs(m,args.repo_root,temp,r)
            stale=[name for name in GENERATED if not (args.output/name).exists() or (args.output/name).read_bytes()!=(temp/name).read_bytes()]
        if stale:
            for name in stale: print(f"ERROR: generated output is missing/stale: {name}",file=sys.stderr)
            return 1
        print("feature-matrix generated outputs are clean"); return 0
    generate_outputs(m,args.repo_root,args.output,r); print(f"generated reports under {args.output}"); return 0


def cmd_summary(args: argparse.Namespace) -> int:
    m,_,e,w=_load(args.matrix,args.registry,args.repo_root); _print(e,w)
    if e: return 1
    print(f"features: {len(m['feature'])}")
    for x in _summary(m["feature"]): print(f"{x['subsystem']}: count={x['count']} implemented={x['implemented']} tested={x['tested']} mainnet={x['mainnet_ready']} p0={x['p0_count']}")
    return 0


def parser() -> argparse.ArgumentParser:
    p=argparse.ArgumentParser(); p.add_argument("--matrix",type=Path,default=DEFAULT_MATRIX); p.add_argument("--registry",type=Path,default=DEFAULT_REGISTRY); p.add_argument("--repo-root",type=Path,default=ROOT)
    sub=p.add_subparsers(dest="command",required=True)
    for name in ("validate","check"):
        q=sub.add_parser(name); q.set_defaults(func=cmd_check)
    g=sub.add_parser("generate"); g.add_argument("--output",type=Path,default=DEFAULT_OUTPUT); g.add_argument("--check-clean",action="store_true"); g.set_defaults(func=cmd_generate)
    s=sub.add_parser("summary"); s.set_defaults(func=cmd_summary); return p


def main(argv: list[str] | None=None) -> int:
    args=parser().parse_args(argv); return int(args.func(args))

if __name__ == "__main__": raise SystemExit(main())
