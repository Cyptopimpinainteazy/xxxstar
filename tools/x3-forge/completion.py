#!/usr/bin/env python3
"""X3 Forge completion intelligence: what is actually unfinished (§1-22).

Answers one question:

> Exactly what code is unfinished, partially implemented, disconnected,
> untested, unreachable, placeholder, or preventing a feature from being
> production-complete?

Two existing sources are cross-referenced rather than replaced:

* `FEATURE_MATRIX.toml` (plus `feature-matrix/*.toml`) is the canonical
  feature list: 153 rows with declared paths, `implemented`/`tested`/
  `mainnet_ready` percentages, blockers, evidence and required tests. Its
  scoring formula is the project's, and this file uses it unchanged.
* `.x3-forge/index.json` is the repository index: every symbol, test and TODO
  marker, with provenance.

The matrix is a set of *claims*; the index is *evidence*. The useful output is
therefore not a second opinion on the percentages — it is the list of rows
where the claim and the evidence disagree, with the exact file and symbol to
look at.

    python3 tools/x3-forge/completion.py scan
    python3 tools/x3-forge/completion.py feature X3-XVM-001
    python3 tools/x3-forge/completion.py subsystem cross_vm_atomic
    python3 tools/x3-forge/completion.py blockers
    python3 tools/x3-forge/completion.py next

Deliberate limits:
  * Only the states the index can support are derived. `ADVERSARIALLY_TESTED`,
    `VERIFIED` and `RELEASE_GATED` are not machine-determinable from a symbol
    index, and are reported as such rather than guessed (§22).
  * "Reachable" here means a symbol is named somewhere else in the index. That
    is a reference check, not a call graph; the index documents that it does
    not resolve calls.
"""
import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_INDEX = ROOT / ".x3-forge" / "index.json"
DEFAULT_MATRIX = ROOT / "FEATURE_MATRIX.toml"

# §22. Ordered weakest to strongest; a feature is only as complete as the
# strongest state the evidence supports, and the states this tool cannot
# determine are named rather than skipped.
STATES = ("MISSING", "IMPLEMENTED", "WIRED", "TESTED",
          "ADVERSARIALLY_TESTED", "VERIFIED", "RELEASE_GATED")
NOT_DETERMINABLE = ("ADVERSARIALLY_TESTED", "VERIFIED", "RELEASE_GATED")

SEVERITY_WEIGHT = {"critical": 8, "high": 4, "medium": 2, "low": 1}

# Languages whose files carry test symbols. A gate definition (YAML), a
# manifest (TOML) or a document (Markdown) can be a declared path without being
# somewhere a test could live, so the "claims tested but has no test symbol"
# rule must not fire on them.
CODE_LANGS = {"rust", "python", "typescript", "javascript", "tsx", "jsx", "go", "c", "cpp"}


def _load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def feature_matrix_module():
    """The project's own matrix loader and scoring, not a second copy."""
    return _load_module("x3_feature_matrix", ROOT / "scripts" / "feature_matrix.py")


def load_index(path):
    with Path(path).open(encoding="utf-8") as handle:
        return json.load(handle)


def test_symbol_names(index):
    """Every function the index marked as a test, by name."""
    names = set()
    for entry in index["files"].values():
        for item in entry.get("items", []):
            if item.get("test") and item.get("name"):
                names.add(item["name"])
    return names


def test_occurrences(index):
    """Test name -> every indexed declaration, with its ignore marker.

    A name is not a declaration: the same test function can be declared in
    several files, and the marker belongs to the declaration. Keeping only the
    last file seen (a dict overwrite) let a stale copy of a suite speak for the
    live tree -- `launch-gates/sources/pack-05-test-gap` carried ignore markers
    on tests the live router suite had un-ignored, and a running required test
    was reported as skipped.
    """
    occurrences = {}
    for rel, entry in index["files"].items():
        for item in entry.get("items", []):
            if item.get("test") and item.get("name"):
                occurrences.setdefault(item["name"], []).append(
                    {"file": rel, "ignored": bool(item.get("ignored"))})
    return occurrences


def gated_ignored_targets(root=ROOT):
    """Test targets some gate runs with `--ignored`.

    An ignored test is not a passing test unless something actually runs it.
    A gate line that pairs `--ignored` with a `--test <target>` executes that
    target's ignored tests; its file stem is what the gate names.
    """
    targets = set()
    candidates = [root / "scripts" / "local-ci.sh", root / "Makefile"]
    workflows = root / ".github" / "workflows"
    if workflows.is_dir():
        candidates += sorted(workflows.glob("*.yml"))
        candidates += sorted(workflows.glob("*.yaml"))
    target = re.compile(r"--test\s+([A-Za-z0-9_\-]+)")
    for candidate in candidates:
        if not candidate.is_file():
            continue
        text = candidate.read_text(encoding="utf-8", errors="replace")
        for line in text.splitlines():
            if "--ignored" in line:
                targets.update(target.findall(line))
    return targets


def crate_dependents(index, root=ROOT):
    """Crate name -> how many manifests name it as a dependency.

    A crate nothing else depends on is either a binary, a test fixture, or a
    component that was never wired in. The index does not carry dependency
    edges, so this reads the manifests it already lists.

    Counting only `path = "..."` entries was wrong and produced false "never
    wired" claims on 47 rows: most X3 crates are pulled in as
    `pallet-x3-cross-vm-router = { workspace = true }`, which has no path to
    follow. This counts any manifest that names a known crate as a dependency
    key, which covers path, version and workspace forms alike.

    It is a reference count, not a resolved dependency graph.
    """
    manifests = {}
    texts = {}
    for manifest in index.get("manifests", []):
        path = root / manifest
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        texts[manifest] = text
        match = re.search(r'(?m)^\s*name\s*=\s*"([A-Za-z0-9_-]+)"', text)
        if match:
            manifests[match.group(1)] = manifest

    dependents = {name: 0 for name in manifests}
    known = set(manifests)
    for manifest, text in texts.items():
        for key in set(re.findall(r'(?m)^\s*([A-Za-z0-9_-]+)\s*=', text)):
            if key in known and manifests[key] != manifest:
                dependents[key] += 1
    return dependents


def binary_crates(index, root=ROOT):
    """Crates that produce a binary rather than a library.

    A binary has no dependents by definition, so "no manifest depends on it"
    says nothing about whether it is wired in. Treating it as evidence produced
    false "never wired" findings on `node` and `crates/x3-gateway`, both of
    which ship a `src/main.rs`.
    """
    names = set()
    for manifest in index.get("manifests", []):
        path = root / manifest
        try:
            text = path.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        match = re.search(r'(?m)^\s*name\s*=\s*"([A-Za-z0-9_-]+)"', text)
        if not match:
            continue
        has_bin = "[[bin]]" in text or "src/main.rs" in text
        if not has_bin:
            # `index["files"]` is keyed by repository-relative path, so the
            # lookup has to use the manifest's relative path too. Building it
            # from the absolute path silently never matched.
            sibling = str(Path(manifest).parent / "src" / "main.rs")
            has_bin = sibling in index["files"]
        if has_bin:
            names.add(match.group(1))
    return names


def items_for(index, paths):
    """Index items belonging to any of `paths` (a file, or a directory prefix)."""
    found = []
    for path in paths:
        clean = str(path).split("#", 1)[0].strip()
        if not clean:
            continue
        for rel, entry in index["files"].items():
            if rel == clean or rel.startswith(clean.rstrip("/") + "/"):
                found.append((rel, entry))
    return found


def evidence_scope(index, paths):
    """Where to look for tests and markers for a declared path.

    A feature row almost always names one file (`pallets/x/src/lib.rs`), while
    that pallet's tests live in sibling files (`tests.rs`, `src/tests_*.rs`).
    Looking only at the named file reported "claims tested=86 but no test
    symbol" for the cross-VM router, which has one of the largest suites in the
    tree — a false positive that would have sent an agent chasing a
    non-problem. The project's own matrix loader searches the parent directory
    for exactly this reason; this mirrors it.
    """
    scope = []
    for raw in paths:
        clean = str(raw).split("#", 1)[0].strip()
        if not clean:
            continue
        if clean in index["files"]:
            scope.append(str(Path(clean).parent))
        else:
            scope.append(clean)
    return sorted(set(scope))


def marker_gaps(index, paths, limit=5):
    """§2. Markers are hints, never proof — so they are reported, not scored.

    Only source files are scanned. A marker inside a workflow or a manifest is
    usually not an unfinished-code marker: the benchmark-regression workflow
    contains `grep -r "TODO"` and `echo "TODO markers: $TODO_COUNT"`, which are
    a CI step counting markers, and reporting those as incomplete work for the
    feature that declares the workflow sent the reader somewhere with nothing
    wrong. Pointing at the wrong file is worse than a slightly smaller scan.
    """
    gaps = []
    for rel, entry in items_for(index, paths):
        if entry.get("lang") not in CODE_LANGS:
            continue
        for item in entry.get("items", []):
            if item.get("kind") == "todo":
                gaps.append({"kind": "marker", "severity": "medium", "file": rel,
                             "line": item.get("line"), "symbol": item.get("name", "")[:120],
                             "detail": "incomplete-code marker in a path this feature claims"})
                if len(gaps) >= limit:
                    return gaps
    return gaps


def is_external(value):
    return str(value).lower().strip().startswith(
        ("http://", "https://", "pr #", "pr:", "registry:", "note:", "workflow:",
         "commit:", "claim:"))


def analyze_feature(feature, index, tests, dependents, binaries, matrix, root=ROOT,
                    occurrences=None, gated=None):
    """One feature row, cross-referenced against the index."""
    occurrences = occurrences or {}
    gated = gated or set()
    paths = [str(p) for p in (feature.get("paths") or [])]
    test_paths = [str(p) for p in (feature.get("test_paths") or [])]
    declared = paths + test_paths
    present, missing = [], []
    for raw in declared:
        clean = raw.split("#", 1)[0].strip()
        if not clean:
            continue
        exists = clean in index["files"] or any(
            rel.startswith(clean.rstrip("/") + "/") for rel in index["files"])
        (present if exists else missing).append(clean)

    items = items_for(index, evidence_scope(index, present))
    test_items = sum(1 for _, entry in items
                     for item in entry.get("items", []) if item.get("test"))
    markers = sum(1 for _, entry in items
                  for item in entry.get("items", []) if item.get("kind") == "todo")

    required = [str(name) for name in (feature.get("required_tests") or [])]
    required_missing = [name for name in required if name not in tests]
    # A required test can exist and still prove nothing if every indexed
    # declaration of its name carries the ignore attribute and no gate runs any declaring
    # target with `--ignored`. One un-ignored declaration, or one gate that
    # executes an ignored one, means the name is not evidence of a skipped
    # test. The gate corpus above is how that is distinguished from a test a
    # dedicated gate really executes.
    required_ignored_ungated = []
    for name in required:
        declarations = occurrences.get(name)
        if not declarations:
            continue
        if any(not d["ignored"] for d in declarations):
            continue
        if any(Path(d["file"]).stem in gated for d in declarations):
            continue
        required_ignored_ungated.append(name)

    crate = None
    for _, entry in items:
        if entry.get("crate"):
            crate = entry["crate"]
            break
    # Wiring is tri-state on purpose. `dependents` only knows crates that have
    # a manifest in the index, so a crate that is absent from it is *unknown*,
    # not unwired. Collapsing those reported "nothing depends on this" for
    # every crate whose manifest the index had not read.
    if not crate or crate not in dependents:
        wired = None
    else:
        wired = dependents[crate] > 0

    gaps = []
    # "The index cannot see it" and "the claim is wrong" are different findings
    # and must not share a severity. The index deliberately covers source file
    # types; a feature whose declared path is a `.tla` directory, a `Makefile`
    # or an `.html` page exists and is simply outside what the index reads.
    absent = [path for path in missing if not (root / path).exists()]
    unindexed = [path for path in missing if (root / path).exists()]
    if absent:
        gaps.append({"kind": "missing_path", "severity": "critical", "file": absent[0],
                     "symbol": "", "line": None,
                     "detail": f"{len(absent)} declared path(s) do not exist on disk"})
    if unindexed:
        gaps.append({"kind": "unindexed_path", "severity": "low", "file": unindexed[0],
                     "symbol": "", "line": None,
                     "detail": f"{len(unindexed)} declared path(s) exist but are outside the index; "
                               "their evidence cannot be checked here"})
    if not present:
        gaps.append({"kind": "no_declared_path", "severity": "high", "file": "",
                     "symbol": "", "line": None,
                     "detail": "the feature row points at no file the index has seen"})
    if required_missing:
        gaps.append({"kind": "unsupported_test_claim", "severity": "high", "file": present[0] if present else "",
                     "symbol": required_missing[0], "line": None,
                     "detail": f"{len(required_missing)} of {len(required)} required tests are not in the index"})
    if required_ignored_ungated:
        name = required_ignored_ungated[0]
        gaps.append({"kind": "ignored_required_test", "severity": "medium",
                     "file": occurrences[name][0]["file"], "symbol": name, "line": None,
                     "detail": "required test is #[ignore]d and no gate runs its target "
                               "with --ignored; a skipped test is not a passing test"})
    code_paths = [rel for rel, entry in items if entry.get("lang") in CODE_LANGS]
    if int(feature.get("tested") or 0) >= 50 and test_items == 0 and code_paths:
        # Only where the declared home is library source. A row whose only path
        # is a gate script (`scripts/mainnet_release_gate.py`) keeps its tests
        # elsewhere, so the absence of a test symbol there says nothing.
        library_home = any(rel.endswith(".rs") and "/src/" in rel for rel in code_paths)
        if library_home:
            gaps.append({"kind": "tested_without_tests", "severity": "high",
                         "file": present[0] if present else "", "symbol": "", "line": None,
                         "detail": f"claims tested={feature.get('tested')} but no test symbol is indexed in its paths"})
    if (int(feature.get("mainnet_ready") or 0) >= 60 and present and wired is False and crate
            and crate not in binaries):
        gaps.append({"kind": "unwired_crate", "severity": "medium",
                     "file": present[0], "symbol": crate, "line": None,
                     "detail": f"claims mainnet_ready={feature.get('mainnet_ready')} but no manifest depends on {crate}"})
    gaps.extend(marker_gaps(index, present))

    # §22: only the states the evidence can support.
    if not present:
        state = "MISSING"
    elif wired is False:
        state = "IMPLEMENTED"
    elif test_items == 0:
        state = "WIRED"
    else:
        state = "TESTED"

    return {
        "id": feature.get("id"),
        "name": feature.get("name"),
        "subsystem": feature.get("subsystem"),
        "priority": feature.get("priority"),
        "launch_scope": feature.get("launch_scope"),
        "confidence": feature.get("confidence"),
        "score": matrix.composite(feature),
        "readiness_class": matrix.readiness_class(matrix.composite(feature)),
        "state": state,
        "states_not_determinable": list(NOT_DETERMINABLE),
        "claim": {"implemented": feature.get("implemented"),
                  "tested": feature.get("tested"),
                  "mainnet_ready": feature.get("mainnet_ready")},
        "wired": wired,
        "crate": crate,
        "evidence": {
            "paths_present": len(present),
            "paths_missing": len(missing),
            "test_items": test_items,
            "markers": markers,
            "required_tests": len(required),
            "required_tests_missing": len(required_missing),
            "required_tests_ignored_ungated": len(required_ignored_ungated),
        },
        "gaps": gaps,
        "declared_blockers": [b for b in (feature.get("blockers") or []) if not is_external(b)][:3],
    }


def analyze(index, matrix, features, root=ROOT):
    tests = test_symbol_names(index)
    dependents = crate_dependents(index, root)
    binaries = binary_crates(index, root)
    occurrences = test_occurrences(index)
    gated = gated_ignored_targets(root)
    return [analyze_feature(feature, index, tests, dependents, binaries, matrix, root,
                            occurrences=occurrences, gated=gated)
            for feature in features]


def severity_of(analysis):
    if not analysis["gaps"]:
        return "none"
    return max((gap["severity"] for gap in analysis["gaps"]),
               key=lambda name: SEVERITY_WEIGHT.get(name, 0))


def rank_key(analysis):
    """§20's ranking, with every factor nameable.

    security severity x blocked features x architectural importance x launch
    dependency / estimated cost. Cost is not modelled — there is no data for it
    — so it is 1.0 for everything and the score says so rather than pretending.
    """
    severity = SEVERITY_WEIGHT.get(severity_of(analysis), 0)
    blocked = max(1, len(analysis["gaps"]))
    importance = {"P0": 3.0, "P1": 2.0, "P2": 1.2, "P3": 0.6}.get(
        str(analysis.get("priority")), 1.0)
    launch = {"core": 2.0, "guarded": 1.5, "experimental": 1.0,
              "dev_tooling": 0.6, "research": 0.5}.get(str(analysis.get("launch_scope")), 1.0)
    cost = 1.0
    score = severity * blocked * importance * launch / cost
    return score, {
        "severity": severity_of(analysis),
        "severity_weight": severity,
        "gaps": blocked,
        "priority_weight": importance,
        "launch_weight": launch,
        "cost_estimate": cost,
        "note": "cost is not modelled; every row is cost 1.0",
    }


def root_blockers(analyses, limit=10):
    """§12. One missing file or symbol usually explains many rows."""
    clusters = {}
    for analysis in analyses:
        for gap in analysis["gaps"]:
            target = gap.get("file") or gap.get("symbol") or gap["kind"]
            entry = clusters.setdefault(target, {
                "target": target, "kind": gap["kind"], "severity": gap["severity"],
                "features": [], "gaps": 0, "detail": gap["detail"]})
            entry["gaps"] += 1
            entry["features"].append(analysis["id"])
            if SEVERITY_WEIGHT.get(gap["severity"], 0) > SEVERITY_WEIGHT.get(entry["severity"], 0):
                entry["severity"] = gap["severity"]
    ranked = sorted(clusters.values(),
                    key=lambda row: (-len(set(row["features"])),
                                     -SEVERITY_WEIGHT.get(row["severity"], 0), row["target"]))
    for row in ranked:
        row["features"] = sorted(set(row["features"]))
        row["blocks"] = len(row["features"])
    return ranked[:limit]


def scan_summary(analyses):
    counts = {severity: 0 for severity in SEVERITY_WEIGHT}
    for analysis in analyses:
        severity = severity_of(analysis)
        if severity != "none":
            counts[severity] += 1
    return {
        "features": len(analyses),
        "unfinished": {k: v for k, v in counts.items() if v},
        "clean": sum(1 for analysis in analyses if severity_of(analysis) == "none"),
        "by_state": {state: sum(1 for analysis in analyses if analysis["state"] == state)
                     for state in STATES},
        "states_not_determinable": list(NOT_DETERMINABLE),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description="X3 Forge completion intelligence.")
    parser.add_argument("--index", default=str(DEFAULT_INDEX))
    parser.add_argument("--matrix", default=str(DEFAULT_MATRIX))
    sub = parser.add_subparsers(dest="command", required=True)

    for name in ("scan", "blockers", "next"):
        p = sub.add_parser(name)
        p.add_argument("--limit", type=int, default=10)
        p.add_argument("--json", action="store_true")
    feature = sub.add_parser("feature")
    feature.add_argument("id")
    feature.add_argument("--json", action="store_true")
    subsystem = sub.add_parser("subsystem")
    subsystem.add_argument("name")
    subsystem.add_argument("--json", action="store_true")

    args = parser.parse_args(argv)

    matrix = feature_matrix_module()
    data = matrix.load_matrix(Path(args.matrix))
    index = load_index(args.index)
    analyses = analyze(index, matrix, data.get("feature", []))

    if args.command == "scan":
        summary = scan_summary(analyses)
        blockers = root_blockers(analyses, args.limit)
        if args.json:
            print(json.dumps({"summary": summary, "root_blockers": blockers},
                             indent=2, sort_keys=True))
            return 0
        print("X3 COMPLETION ANALYSIS")
        print(f"features                 {summary['features']}")
        for severity in ("critical", "high", "medium", "low"):
            if severity in summary["unfinished"]:
                print(f"{severity + ' unfinished':24} {summary['unfinished'][severity]}")
        print(f"no gap found             {summary['clean']}")
        print("states                   " + ", ".join(
            f"{k}={v}" for k, v in summary["by_state"].items() if v))
        print("not machine-determinable " + ", ".join(NOT_DETERMINABLE))
        if blockers:
            top = blockers[0]
            print(f"\nhighest-impact target: {top['target']}")
            print(f"  blocks {top['blocks']} feature(s) over {top['gaps']} gap(s): "
                  + ", ".join(top["features"][:6]))
            print(f"  {top['detail']}")
        return 0

    if args.command == "blockers":
        blockers = root_blockers(analyses, args.limit)
        if args.json:
            print(json.dumps(blockers, indent=2, sort_keys=True))
            return 0
        for row in blockers:
            print(f"{row['severity']:9} blocks {row['blocks']:3}  {row['target']}")
            print(f"          {row['detail']}")
            print(f"          features: {', '.join(row['features'][:8])}")
        return 0

    if args.command == "next":
        ranked = []
        for analysis in analyses:
            score, why = rank_key(analysis)
            if score > 0:
                ranked.append((score, analysis, why))
        ranked.sort(key=lambda row: (-row[0], str(row[1]["id"])))
        if args.json:
            print(json.dumps([{"id": a["id"], "name": a["name"], "score": round(s, 2),
                               "why": why, "score_factors": why, "gaps": a["gaps"][:3]}
                              for s, a, why in ranked[:args.limit]], indent=2, sort_keys=True))
            return 0
        for score, analysis, why in ranked[:args.limit]:
            print(f"{score:8.1f}  {analysis['id']:14} {analysis['name']}")
            print(f"          {analysis['subsystem']}  priority={analysis['priority']} "
                  f"scope={analysis['launch_scope']} state={analysis['state']}")
            print(f"          why: severity={why['severity']} x gaps={why['gaps']} "
                  f"x priority={why['priority_weight']} x launch={why['launch_weight']}")
            for gap in analysis["gaps"][:2]:
                where = gap.get("file") or gap.get("symbol") or ""
                print(f"          target: {where}"
                      + (f"::{gap['symbol']}" if gap.get("symbol") and gap.get("file") else ""))
                print(f"                  {gap['detail']}")
        return 0

    if args.command in ("feature", "subsystem"):
        if args.command == "feature":
            selected = [a for a in analyses if a["id"] == args.id or a["name"] == args.id]
        else:
            selected = [a for a in analyses if a["subsystem"] == args.name]
        if not selected:
            print(f"no feature matched {args.id!r}", file=sys.stderr)
            return 1
        if args.json:
            print(json.dumps(selected, indent=2, sort_keys=True))
            return 0
        for analysis in selected:
            print(f"{analysis['id']}  {analysis['name']}")
            print(f"  subsystem {analysis['subsystem']}   priority {analysis['priority']}"
                  f"   scope {analysis['launch_scope']}")
            print(f"  claim   implemented={analysis['claim']['implemented']}"
                  f" tested={analysis['claim']['tested']}"
                  f" mainnet_ready={analysis['claim']['mainnet_ready']}")
            print(f"  score   {analysis['score']} ({analysis['readiness_class']})")
            print(f"  state   {analysis['state']}   "
                  f"wired={analysis['wired']} crate={analysis['crate']}")
            print(f"  cannot be determined here: {', '.join(analysis['states_not_determinable'])}")
            print(f"  evidence {analysis['evidence']}")
            if not analysis["gaps"]:
                print("  gaps    none found in the index")
            for gap in analysis["gaps"]:
                print(f"  GAP {gap['severity']:8} {gap['kind']:22} "
                      f"{gap.get('file') or ''}"
                      + (f"::{gap['symbol']}" if gap.get("symbol") else ""))
                print(f"      {gap['detail']}")
        return 0

    parser.error("unknown command")
    return 2


if __name__ == "__main__":
    sys.exit(main())
