#!/usr/bin/env python3
"""Export X3's historical Markdown funding research as provenance-labelled JSONL.

No network access, eligibility checks, submission, or financial actions occur.
Records here are research leads, never currently open funding opportunities.
"""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import re

ID_PATTERN = re.compile(r"^C\d+(?:\.[A-Za-z0-9]+)+$")
URL_PATTERN = re.compile(r"https?://[^\s<>|)]+")
VERIFIED_PATTERN = re.compile(r"(?:verified|checked)\s+(20\d\d-\d\d-\d\d)", re.I)
STATUS_VALUES = {
    "RESEARCH", "QUALIFIED", "APPLIED", "AWARDED", "REJECTED",
    "NOT-A-GRANT", "DEFUNCT", "CLOSED", "INACTIVE", "UNKNOWN",
}


def cells(line: str) -> list[str]:
    stripped = line.strip()
    if not (stripped.startswith("|") and stripped.endswith("|")):
        return []
    return [part.strip() for part in stripped.strip("|").split("|")]


def normalize_status(parts: list[str]) -> str:
    # Search named fields, not all prose; some rows mention another program's status.
    for value in reversed(parts):
        candidate = re.sub(r"[*\x60_]+", "", value).strip().upper()
        if candidate in STATUS_VALUES:
            return candidate
    return "UNKNOWN"


def parse_file(path: Path, root: Path) -> list[dict]:
    relative_path = path.relative_to(root).as_posix()
    if path.name in {"INDEX.md"}:
        return []
    output = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        parts = cells(line)
        if len(parts) < 6 or not ID_PATTERN.fullmatch(parts[0]):
            continue
        identifier, name = parts[0], parts[1]
        if not name or name.startswith("---"):
            continue
        # URLs are historical research claims, NOT evidence that programs are open.
        urls = [m.group(0).rstrip(".,;]") for field in parts[2:] for m in URL_PATTERN.finditer(field)]
        status = normalize_status(parts)
        found = VERIFIED_PATTERN.search(line)
        output.append({
            "legacy_id": identifier,
            "name": name,
            "category": path.stem,
            "raw_fields": parts[2:],
            "candidate_urls": list(dict.fromkeys(urls)),
            "historical_status": status,
            "historical_verification_date": found.group(1) if found else None,
            "source_file": relative_path,
            "source_line": line_number,
            "origin": "legacy_markdown_research",
            "current_availability": "UNKNOWN",
            "current_eligibility": "UNKNOWN",
            "application_authorized": False,
        })
    return output


def export(root: Path) -> list[dict]:
    if not root.is_dir():
        raise ValueError(f"Catalog directory does not exist: {root}")
    records = []
    seen = set()
    for path in sorted(root.glob("*.md")):
        for record in parse_file(path, root.parent):
            key = (record["source_file"], record["legacy_id"])
            if key in seen:
                raise ValueError(f"Duplicate legacy ID in file: {key}")
            seen.add(key)
            records.append(record)
    return records


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog-dir", type=Path, default=Path("grants"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--summary", type=Path, required=True)
    args = parser.parse_args()
    records = export(args.catalog_dir)
    if not records:
        raise SystemExit("No catalog records found; refusing to write empty export")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.summary.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("".join(json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in records), encoding="utf-8")
    status_counts = Counter(r["historical_status"] for r in records)
    category_counts = Counter(r["category"] for r in records)
    summary = {
        "total_historical_leads": len(records),
        "historical_status_counts": dict(sorted(status_counts.items())),
        "category_counts": dict(sorted(category_counts.items())),
        "current_open_opportunities_confirmed": 0,
        "note": "No live re-verification performed; zero means not checked, not none available.",
    }
    args.summary.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Exported {len(records)} historical leads (no live funding verification)")


if __name__ == "__main__":
    main()
