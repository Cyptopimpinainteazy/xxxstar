#!/usr/bin/env python3
"""Aggregate completion across every tracked checklist.

Each subsystem keeps its own checklist under docs/<name>/checklist.json, derived
from its own specification section by section, so the denominator cannot drift
to whatever happens to be finished. This prints the running total:

    python3 scripts/x3_status.py                # totals + per-project
    python3 scripts/x3_status.py --remaining    # everything still open
    python3 scripts/x3_status.py --done         # finished items with evidence
    python3 scripts/x3_status.py --mark ID DONE --evidence "..."
"""
import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
ORDER = ["DONE", "DOING", "TODO", "BLOCKED"]


def checklists():
    found = sorted(DOCS.glob("*/checklist.json"))
    if not found:
        sys.exit("no checklists found under docs/*/checklist.json")
    return found


def bar(done, total, width=28):
    filled = 0 if not total else round(width * done / total)
    return "[" + "#" * filled + "." * (width - filled) + "]"


def find_item(path, item_id):
    data = json.loads(path.read_text(encoding="utf-8"))
    for item in data["items"]:
        if item["id"] == item_id:
            return path, data, item
    return None, None, None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--remaining", action="store_true")
    parser.add_argument("--done", action="store_true")
    parser.add_argument("--by-section", action="store_true")
    parser.add_argument("--mark", nargs=2, metavar=("ID", "STATUS"))
    parser.add_argument("--evidence", default=None)
    args = parser.parse_args()

    paths = checklists()

    if args.mark:
        item_id, status = args.mark
        status = status.upper()
        if status not in ORDER:
            sys.exit(f"status must be one of {ORDER}")
        for path in paths:
            found_path, data, item = find_item(path, item_id)
            if item is None:
                continue
            item["status"] = status
            if args.evidence is not None:
                item["evidence"] = args.evidence
            found_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
            print(f"{item_id} -> {status} ({found_path.relative_to(ROOT)})")
            break
        else:
            sys.exit(f"no item with id {item_id} in any checklist")

    total_all = done_all = 0
    rows = []
    for path in paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        items = data["items"]
        counts = {s: sum(1 for i in items if i["status"] == s) for s in ORDER}
        total_all += len(items)
        done_all += counts["DONE"]
        rows.append((data.get("project", path.parent.name), path, items, counts))

    print(f"X3 TOTAL: {done_all}/{total_all} complete  {bar(done_all, total_all)}  "
          f"{100 * done_all // max(total_all, 1)}%   REMAINING {total_all - done_all}")
    print()
    for name, path, items, counts in rows:
        print(f"  {name:<12} {counts['DONE']:>3}/{len(items):<3} {bar(counts['DONE'], len(items), 16)}"
              f"  doing {counts['DOING']:>2}  todo {counts['TODO']:>3}  blocked {counts['BLOCKED']}")
        if args.by_section:
            sections = {}
            for item in items:
                key = item["sec"].split("/")[0]
                bucket = sections.setdefault(key, [0, 0])
                bucket[1] += 1
                if item["status"] == "DONE":
                    bucket[0] += 1
            for key in sorted(sections, key=lambda k: (len(k), k)):
                d, t = sections[key]
                print(f"      {'§' + key:<8} {d:>3}/{t:<3} {bar(d, t, 12)}")

    if args.done:
        print("\nfinished:")
        for name, path, items, _ in rows:
            for item in items:
                if item["status"] == "DONE":
                    print(f"  [{item['id']}] §{item['sec']} {item['item']}")
                    if item.get("evidence"):
                        print(f"        evidence: {item['evidence']}")

    if args.remaining:
        print("\nstill open:")
        for name, path, items, _ in rows:
            for item in items:
                if item["status"] != "DONE":
                    flag = " (partial)" if item.get("partial") else ""
                    print(f"  [{item['status']:<7}] [{item['id']}] §{item['sec']} {item['item']}{flag}")


if __name__ == "__main__":
    main()
