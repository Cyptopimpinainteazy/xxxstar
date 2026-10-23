#!/usr/bin/env python3
"""Index the universal economic request without inventing readiness evidence.

FEATURE_MATRIX.toml and FEATURE_REGISTRY.toml remain readiness authorities.
This catalog records coverage and pending audits, not measured completion.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

from feature_matrix import load_matrix

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "docs/universal-economic/MASTER_REQUEST.md"
OUTPUT = ROOT / "docs/universal-economic/requirements.json"


def requirements(source: str) -> list[dict]:
    rows = []
    headings: list[tuple[int, str]] = []
    fenced = False
    for line_number, raw in enumerate(source.splitlines(), 1):
        text = raw.strip()
        if text.startswith("```"):
            fenced = not fenced
            continue
        if fenced or not text or text == "---":
            continue
        heading = re.match(r"^(#{1,6})\s+(.+)$", text)
        if heading:
            level, text = len(heading[1]), heading[2]
            while headings and headings[-1][0] >= level:
                headings.pop()
            headings.append((level, text))
        else:
            text = re.sub(r"^(?:- |\d+\. )", "", text)
        rows.append({
            "id": f"X3-UNIVERSAL-L{line_number:04d}",
            "name": text,
            "category": " / ".join(name for _, name in headings),
            "source_line": line_number,
            "kind": "section" if heading else "requirement",
            "status": "BLOCKED",
            "audit_state": "NOT RUN",
            "canonical_feature_ids": [],
            "blockers": ["Per-requirement source, wiring, and execution audit not yet recorded."],
            "next_action": "Map to existing canonical feature IDs, inspect implementation, and record scoped execution evidence.",
        })
    return rows


def render() -> str:
    source = SOURCE.read_text()
    matrix = load_matrix(ROOT / "FEATURE_MATRIX.toml")
    data = {
        "schema_version": 1,
        "source": str(SOURCE.relative_to(ROOT)),
        "source_sha256": hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
        "readiness_authorities": ["FEATURE_REGISTRY.toml", "FEATURE_MATRIX.toml"],
        "scope": "Complete source coverage index; pending audit does not mean absent implementation.",
        "canonical_features_available": [feature["id"] for feature in matrix["feature"]],
        "requirements": requirements(source),
    }
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    expected = render()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_text() != expected:
            print("Universal catalog is stale; run python3 scripts/x3_universal_catalog.py")
            return 1
    else:
        OUTPUT.write_text(expected)
    print(f"Universal requirement coverage: {len(requirements(SOURCE.read_text()))} rows; no readiness inferred")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
