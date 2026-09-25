#!/usr/bin/env python3
"""Emit the audit completion matrix and the agent work queue from canonical data.

Why this exists: the X3 master completion directive asks for
`docs/audit/X3_FEATURE_COMPLETION_MATRIX.md`, `docs/audit/X3_AGENT_QUEUE.md` and
`audit-artifacts/current/feature-status.json`. Those three files must not become a
fourth, hand-edited opinion about readiness: this repository already has exactly one
canonical source for each layer.

    FEATURE_REGISTRY.toml   coarse product readiness (mode, score, blockers, tests)
    FEATURE_MATRIX.toml     granular engineering readiness (implemented/tested/mainnet)
    + feature-matrix/*.toml fragment files it includes

So this script *derives* the three artifacts from those two files and never invents a
number. A row's state is a pure function of the recorded scores, with the mapping
written down in `STATE_RULES` below so a reader can re-derive every cell by hand.

`--check` fails when the committed artifacts are stale, which is what makes them
evidence instead of a snapshot that drifts.

Usage:
    scripts/x3_audit_matrix.py            # write the artifacts
    scripts/x3_audit_matrix.py --check    # fail if they are stale (CI gate)
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from feature_matrix import composite, load_matrix, load_registry  # noqa: E402

MATRIX_PATH = ROOT / "FEATURE_MATRIX.toml"
REGISTRY_PATH = ROOT / "FEATURE_REGISTRY.toml"
MATRIX_OUT = ROOT / "docs" / "audit" / "X3_FEATURE_COMPLETION_MATRIX.md"
QUEUE_OUT = ROOT / "docs" / "audit" / "X3_AGENT_QUEUE.md"
STATUS_OUT = ROOT / "audit-artifacts" / "current" / "feature-status.json"

# The directive's vocabulary, and the one rule that maps a row onto it. Order matters:
# the first rule that matches wins, so the list reads top to bottom as "best evidence
# first". Every threshold is a documented constant rather than a tuned magic number.
STATE_RULES: tuple[tuple[str, str], ...] = (
    ("BROKEN", "a cited path does not exist on disk, so the row cannot be built from"),
    ("NOT INTEGRATED", "source is not master (open PR or research), so it is not in the tree under test"),
    ("COMPLETE", "implemented >= 80 and tested >= 80 and mainnet_ready >= 80"),
    ("FUNCTIONAL BUT UNHARDENED", "mainnet_ready >= 60"),
    ("PARTIAL", "implemented >= 50"),
    ("STUB", "implemented >= 20"),
    ("NOT STARTED", "implemented < 20"),
)

COMPLETE_FLOOR = 80
UNHARDENED_FLOOR = 60
PARTIAL_FLOOR = 50
STUB_FLOOR = 20

# A blocker line that says it was closed is not open work. The registry keeps closed
# blockers in place on purpose (so the history stays readable and a stray edit does not
# re-measure the score), so the queue has to filter them instead of listing them as
# outstanding.
RESOLVED_BLOCKER = re.compile(r"^\*{0,2}\s*CLOSED\b", re.IGNORECASE)

# Long verbatim blockers are the value of the queue, but a 900-character table cell is
# unreadable. The Markdown cell is clipped and says so; `feature-status.json` carries the
# full text, so nothing is lost.
CELL_CLIP = 240


def _cell(text: str) -> str:
    flat = " ".join(str(text).split()).replace("|", "\\|")
    if len(flat) <= CELL_CLIP:
        return flat
    return flat[:CELL_CLIP].rstrip() + "… (full text in audit-artifacts/current/feature-status.json)"


def _is_open(blocker: str) -> bool:
    return not RESOLVED_BLOCKER.match(str(blocker).strip())


def _triage(score: int) -> str:
    """A triage hint, not the directive's P0-P4 judgement.

    Turning "is this launch-blocking?" into a numeric rule would be a guess, so this
    column says what it actually is: how low the feature's own recorded readiness is.
    The directive's P0-P4 call stays a human decision recorded in FEATURE_MATRIX.toml.
    """
    if score < 40:
        return "below 40pct - highest-triage candidate"
    if score < UNHARDENED_FLOOR:
        return "below 60pct - triage candidate"
    return "at or above 60pct"


def _digest(path: Path) -> str:
    """sha256 of a canonical source file, so freshness is about content, not checkout.

    The artifacts deliberately do NOT record the commit they were generated at: a
    commit-stamped artifact goes stale on every commit, which turns a freshness gate into
    a gate people learn to regenerate blindly. What matters is whether the *sources*
    moved, and that is what these digests pin. A commit-stamped copy belongs in the
    release-evidence bundle, which is commit-specific by design.
    """
    try:
        return hashlib.sha256(path.read_bytes()).hexdigest()
    except OSError:
        return "unreadable"


def _exists(rel: str) -> bool:
    path = rel.split("#", 1)[0].strip()
    if not path or path.lower().startswith(("http://", "https://", "pr #", "pr:", "note:", "workflow:", "commit:", "claim:", "registry:")):
        return True  # not a path claim; the directive's BROKEN state is about paths
    return (ROOT / path).exists()


def _state(feature: dict[str, Any]) -> tuple[str, str]:
    paths = [p for p in feature.get("paths", []) if p]
    if paths and not all(_exists(p) for p in paths):
        return "BROKEN", STATE_RULES[0][1]
    if str(feature.get("source")) != "master" or feature.get("claim_risk"):
        return "NOT INTEGRATED", STATE_RULES[1][1]
    impl = int(feature["implemented"])
    tested = int(feature["tested"])
    mainnet = int(feature["mainnet_ready"])
    if impl >= COMPLETE_FLOOR and tested >= COMPLETE_FLOOR and mainnet >= COMPLETE_FLOOR:
        return "COMPLETE", STATE_RULES[2][1]
    if mainnet >= UNHARDENED_FLOOR:
        return "FUNCTIONAL BUT UNHARDENED", STATE_RULES[3][1]
    if impl >= PARTIAL_FLOOR:
        return "PARTIAL", STATE_RULES[4][1]
    if impl >= STUB_FLOOR:
        return "STUB", STATE_RULES[5][1]
    return "NOT STARTED", STATE_RULES[6][1]


def _registry_index(registry: dict[str, Any]) -> dict[str, dict[str, Any]]:
    """Map every registry feature onto the matrix rows whose paths it cites.

    The two files key different things (a registry name, a matrix id), so join them on
    the path they share instead of on a name that does not match. A registry feature
    with no matrix row keeps its own entry; that is the coarse layer and it is real.
    """
    index: dict[str, dict[str, Any]] = {}
    for name, body in registry.items():
        if not isinstance(body, dict):
            continue
        paths = {str(body.get("crate_or_service", "")).strip()}
        for key in ("paths", "additional_paths"):
            paths.update(str(p).strip() for p in body.get(key, []) or [])
        key = next((p for p in sorted(paths) if p), "")
        if key:
            index.setdefault(key, body)
            index.setdefault(key.rstrip("/") + "/src/lib.rs", body)
    return index


def _registry_for(feature: dict[str, Any], index: dict[str, dict[str, Any]]) -> dict[str, Any] | None:
    for path in feature.get("paths", []) or []:
        clean = path.split("#", 1)[0].strip().rstrip("/")
        if clean in index:
            return index[clean]
        for suffix in ("/src/lib.rs", "/src/main.rs"):
            if clean + suffix in index:
                return index[clean + suffix]
    return None


def _tests_cell(feature: dict[str, Any], registry_row: dict[str, Any] | None) -> str:
    if registry_row and registry_row.get("required_tests"):
        return f"{len(registry_row['required_tests'])} named tests in FEATURE_REGISTRY"
    return f"tested score {feature['tested']}% (no named test list on this row)"


def build(matrix: dict[str, Any], registry: dict[str, Any]) -> dict[str, Any]:
    index = _registry_index(registry)
    rows: list[dict[str, Any]] = []
    for feature in sorted(matrix["feature"], key=lambda f: (f["subsystem"], f["id"])):
        state, rule = _state(feature)
        registry_row = _registry_for(feature, index)
        rows.append(
            {
                "id": feature["id"],
                "name": feature["name"],
                "subsystem": feature["subsystem"],
                "state": state,
                "state_rule": rule,
                "implemented": feature["implemented"],
                "tested": feature["tested"],
                "mainnet_ready": feature["mainnet_ready"],
                "composite": composite(feature),
                "priority": feature["priority"],
                "confidence": feature["confidence"],
                "launch_scope": feature["launch_scope"],
                "source": feature["source"],
                "claim_risk": bool(feature.get("claim_risk")),
                "paths": feature.get("paths", []),
                "missing_work": feature.get("blockers", []),
                "evidence": feature.get("evidence", []),
                "tests": _tests_cell(feature, registry_row),
                "live_tested": (registry_row or {}).get("mode", "not a registered launch feature"),
                "benchmark": feature.get("benchmark", "—"),
            }
        )

    registry_rows = []
    for name in sorted(registry):
        body = registry[name]
        if not isinstance(body, dict) or "readiness_score" not in body:
            continue
        registry_rows.append(
            {
                "feature": name,
                "mode": body.get("mode", "unknown"),
                "readiness_score": body["readiness_score"],
                "crate_or_service": body.get("crate_or_service", ""),
                "blockers": body.get("blockers", []),
                "required_tests": body.get("required_tests", []),
                "proof_report": body.get("proof_report", ""),
            }
        )

    heads = [r["readiness_score"] for r in registry_rows]
    fragments = [MATRIX_PATH.parent / str(rel) for rel in matrix.get("includes", [])]
    sources = {
        str(p.relative_to(ROOT)): _digest(p)
        for p in [MATRIX_PATH, REGISTRY_PATH, *sorted(fragments)]
    }
    return {
        "generated_from": {
            "sources": sources,
            "source_digest": hashlib.sha256(
                "".join(f"{k}:{v}" for k, v in sorted(sources.items())).encode()
            ).hexdigest(),
            "matrix": str(MATRIX_PATH.relative_to(ROOT)),
            "registry": str(REGISTRY_PATH.relative_to(ROOT)),
            "matrix_audit_date": matrix.get("meta", {}).get("audit_date", "unknown"),
            "scoring": matrix.get("meta", {}).get("scoring", "unknown"),
        },
        "state_rules": [{"state": s, "rule": r} for s, r in STATE_RULES],
        "summary": {
            "matrix_features": len(rows),
            "registry_features": len(registry_rows),
            "by_state": {s: sum(1 for r in rows if r["state"] == s) for s, _ in STATE_RULES},
            "registry_mean_readiness": round(sum(heads) / len(heads), 2) if heads else None,
            "registry_min_readiness": min(heads) if heads else None,
            "registry_max_readiness": max(heads) if heads else None,
        },
        "features": rows,
        "registry": registry_rows,
    }


def render_matrix(status: dict[str, Any]) -> str:
    s = status["summary"]
    out = [
        "# X3 Feature Completion Matrix",
        "",
        "Generated by `scripts/x3_audit_matrix.py` from `FEATURE_REGISTRY.toml` (coarse product",
        "readiness) and `FEATURE_MATRIX.toml` + `feature-matrix/*.toml` (granular engineering",
        "readiness). **Do not edit this file by hand** — `scripts/x3_audit_matrix.py --check`",
        "fails when it is stale, so a hand edit is a gate failure, not a claim.",
        "",
        f"- Source digest: `{status['generated_from']['source_digest'][:16]}…` "
        f"({len(status['generated_from']['sources'])} files: `FEATURE_REGISTRY.toml`, "
        f"`FEATURE_MATRIX.toml` + fragments)",
        f"- Matrix audit date: `{status['generated_from']['matrix_audit_date']}`",
        f"- Scores: {s['matrix_features']} granular rows, {s['registry_features']} registry features",
        f"- Registry readiness: mean {s['registry_mean_readiness']}% "
        f"(min {s['registry_min_readiness']}%, max {s['registry_max_readiness']}%)",
        "",
        "## How the state column is derived",
        "",
        "The state is a pure function of the row's own recorded scores. The first rule that",
        "matches wins, so a reader can re-derive every cell from the JSON artifact.",
        "",
        "| State | Rule |",
        "|---|---|",
    ]
    out += [f"| {r['state']} | {r['rule']} |" for r in status["state_rules"]]
    out += [
        "",
        "A state is **not** a launch approval. `COMPLETE` means the three recorded scores are",
        "at or above 80; whether those scores themselves are honest is what the registry's",
        "`required_tests`, `proof_report` and blocker text are for.",
        "",
        "## Matrix",
        "",
        "| Subsystem | Feature | State | Impl | Tested | Mainnet | Composite | Priority | Tests | Live tested | Missing work |",
        "|---|---|---|---:|---:|---:|---:|---|---|---|---|",
    ]
    for f in status["features"]:
        missing = _cell("; ".join(f["missing_work"])) if f["missing_work"] else "—"
        out.append(
            f"| {f['subsystem']} | {f['id']} {f['name']} | {f['state']} | {f['implemented']} | "
            f"{f['tested']} | {f['mainnet_ready']} | {f['composite']} | {f['priority']} | "
            f"{f['tests']} | {f['live_tested']} | {missing} |"
        )
    out += ["", "## Registry features (coarse layer)", "", "| Feature | Mode | Readiness | Path |", "|---|---|---:|---|"]
    for r in status["registry"]:
        out.append(f"| {r['feature']} | {r['mode']} | {r['readiness_score']}% | `{r['crate_or_service']}` |")
    out += [
        "",
        "## Blocking work recorded on registry rows",
        "",
        "Each entry below is a blocker already recorded against that feature, in the words the",
        "registry uses. These are the acceptance criteria the queue in `X3_AGENT_QUEUE.md` is",
        "built from; nothing here is inferred.",
        "",
    ]
    for r in status["registry"]:
        out.append(f"### {r['feature']} — {r['readiness_score']}% ({r['mode']})")
        open_blockers = [b for b in r["blockers"] if _is_open(b)]
        closed = len(r["blockers"]) - len(open_blockers)
        if open_blockers:
            out += [f"- {b}" for b in open_blockers]
        else:
            out.append("- (no open blocker recorded on this row)")
        if closed:
            out.append(f"- {closed} further blocker line(s) on this row are marked CLOSED.")
        out.append("")
    return "\n".join(out).rstrip() + "\n"


def render_queue(status: dict[str, Any]) -> str:
    """The agent work queue: one row per unresolved item that has real evidence behind it.

    Priority is taken from what the repository already says, not re-invented here:
    registry rows sit at P0 when their readiness is below 60 (core launch surface) and P1
    otherwise; matrix rows keep their own recorded priority and are included when their
    state is not COMPLETE. An item with no dependency edge recorded stays `—` rather
    than being given an invented one.
    """
    out = [
        "# X3 Agent Work Queue",
        "",
        "Generated by `scripts/x3_audit_matrix.py`. Each row is an **unresolved** item with real",
        "evidence behind it: either a blocker recorded against a registry feature, or a granular",
        "matrix row that has not reached `COMPLETE`. Regenerate after closing work; the queue is",
        "not a plan, it is the list of things the repository itself says are not done.",
        "",
        f"Source digest: `{status['generated_from']['source_digest'][:16]}…` — the artifacts move when",
        "`FEATURE_REGISTRY.toml` or the matrix fragments move, and `scripts/x3_audit_matrix.py --check`",
        "fails when they do not match.",
        "",
        "| ID | Subsystem | Description (verbatim from the record) | Owner | Dependency | Branch | Triage | Test command | Completion evidence | Merge status |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    rows = 0
    resolved = 0
    for r in sorted(status["registry"], key=lambda x: x["readiness_score"]):
        for i, blocker in enumerate(r["blockers"], start=1):
            if not _is_open(blocker):
                resolved += 1
                continue
            evidence = (
                f"`{r['proof_report']}`" if r["proof_report"]
                else ("required_tests resolved by check-readiness-consistency.sh"
                      if r["required_tests"] else "none recorded - closing this needs a proof artifact")
            )
            out.append(
                f"| REG-{r['feature']}-{i} | {r['feature']} ({r['mode']}) | {_cell(blocker)} | "
                f"unassigned | — | — | {_triage(r['readiness_score'])} | "
                f"`bash scripts/local-ci.sh` | {evidence} | open |"
            )
            rows += 1
    for f in status["features"]:
        if f["state"] == "COMPLETE":
            continue
        missing = _cell("; ".join(f["missing_work"])) if f["missing_work"] else "no blocker recorded; row is below COMPLETE on its scores"
        out.append(
            f"| MTX-{f['id']} | {f['subsystem']} | {f['name']}: {missing} | "
            f"unassigned | — | — | {f['priority']}/{f['confidence']} | "
            f"`bash scripts/local-ci.sh` | {f['state']} · {f['tests']} | open |"
        )
        rows += 1
    out += [
        "",
        f"**{rows} open rows** ({resolved} blocker lines on registry rows are marked CLOSED and are not",
        "listed). A row disappears only when the underlying record changes - a fixed blocker removed",
        "from `FEATURE_REGISTRY.toml`, or a matrix row that reached `COMPLETE`.",
        "",
    ]
    return "\n".join(out)


def write_all(status: dict[str, Any]) -> list[Path]:
    written = []
    for path, payload in (
        (MATRIX_OUT, render_matrix(status)),
        (QUEUE_OUT, render_queue(status)),
        (STATUS_OUT, json.dumps(status, indent=2, sort_keys=True) + "\n"),
    ):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(payload, encoding="utf-8")
        written.append(path)
    return written


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the artifacts are stale")
    args = parser.parse_args(argv)

    status = build(load_matrix(MATRIX_PATH), load_registry(REGISTRY_PATH))
    expected = {
        MATRIX_OUT: render_matrix(status),
        QUEUE_OUT: render_queue(status),
        STATUS_OUT: json.dumps(status, indent=2, sort_keys=True) + "\n",
    }
    if args.check:
        stale = [
            str(p.relative_to(ROOT))
            for p, text in expected.items()
            if not p.exists() or p.read_text(encoding="utf-8") != text
        ]
        if stale:
            for name in stale:
                print(f"ERROR: audit artifact is missing/stale: {name}", file=sys.stderr)
            print("run: scripts/x3_audit_matrix.py", file=sys.stderr)
            return 1
        print("x3-audit-matrix check PASS: artifacts match their sources")
        return 0

    for path in write_all(status):
        print(f"wrote {path.relative_to(ROOT)}")
    s = status["summary"]
    print(
        f"{s['matrix_features']} matrix rows "
        f"({', '.join(f'{k}={v}' for k, v in s['by_state'].items())}), "
        f"{s['registry_features']} registry features"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
