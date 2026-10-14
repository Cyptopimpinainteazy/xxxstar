#!/usr/bin/env python3
"""Report X3 Guardian completion as done / total, with what is left.

`docs/guardian/checklist.json` is the denominator. It was derived from the
Guardian specification section by section, so the count cannot drift to whatever
happens to be finished. Run this after every completed item:

    python3 scripts/guardian_status.py            # summary + remaining list
    python3 scripts/guardian_status.py --by-section
    python3 scripts/guardian_status.py --done     # what is finished, with evidence
    python3 scripts/guardian_status.py --mark ID DONE --evidence "command + result"
"""
import argparse
import json
import os
import sys
from pathlib import Path

# Overridable so tests (and other checkouts) can point at a scratch copy without
# ever writing to the tracked tracker.
CHECKLIST = Path(
    os.environ.get(
        "X3_GUARDIAN_CHECKLIST",
        Path(__file__).resolve().parent.parent / "docs" / "guardian" / "checklist.json",
    )
)
ORDER = ["DONE", "DOING", "TODO", "BLOCKED"]


def load():
    data = json.loads(CHECKLIST.read_text(encoding="utf-8"))
    return data, data["items"]


def save(data):
    CHECKLIST.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def bar(done, total, width=28):
    filled = 0 if not total else round(width * done / total)
    return "[" + "#" * filled + "." * (width - filled) + "]"


def apply_mark(items, item_id, status, evidence):
    """Apply a status change in place and return the new status.

    Raising `SystemExit` on any refusal keeps the CLI honest: a tracker whose
    whole purpose is "the count cannot drift" must not accept `DONE` with no
    evidence, because that is exactly how a count drifts upward without work.
    """
    status = status.upper()
    if status not in ORDER:
        sys.exit(f"status must be one of {ORDER}")
    match = [i for i in items if i["id"] == item_id]
    if not match:
        sys.exit(f"no item with id {item_id}")
    prior = match[0].get("evidence", "")
    new_evidence = evidence if evidence is not None else prior
    if status == "DONE" and not new_evidence.strip():
        sys.exit(
            f"refusing to mark {item_id} DONE with no evidence; pass "
            f"--evidence \"<command and its result>\" or fix the item first"
        )
    match[0]["status"] = status
    if evidence is not None:
        match[0]["evidence"] = evidence
    return status


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--by-section", action="store_true", help="break the counts down by spec section")
    parser.add_argument("--done", action="store_true", help="list finished items with their evidence")
    parser.add_argument("--remaining", action="store_true", help="list everything still open")
    parser.add_argument("--mark", nargs=2, metavar=("ID", "STATUS"), help="set an item's status")
    parser.add_argument("--evidence", default=None, help="evidence string for --mark")
    args = parser.parse_args()

    data, items = load()

    if args.mark:
        item_id, status = args.mark
        status = apply_mark(items, item_id, status, args.evidence)
        save(data)
        print(f"{item_id} -> {status}")

    counts = {s: sum(1 for i in items if i["status"] == s) for s in ORDER}
    total = len(items)
    done = counts["DONE"]
    remaining = total - done

    print(f"X3 Guardian: {done}/{total} complete  {bar(done, total)}  {100 * done // max(total, 1)}%")
    print(f"  done {counts['DONE']}   doing {counts['DOING']}   todo {counts['TODO']}   blocked {counts['BLOCKED']}")
    print(f"  REMAINING: {remaining}")

    if args.by_section:
        print("\nby section:")
        sections = {}
        for item in items:
            key = item["sec"].split("/")[0]
            bucket = sections.setdefault(key, [0, 0])
            bucket[1] += 1
            if item["status"] == "DONE":
                bucket[0] += 1
        for key in sorted(sections, key=lambda k: (len(k), k)):
            d, t = sections[key]
            print(f"  {'§' + key:<7} {d:>3}/{t:<3} {bar(d, t, 16)}")

    if args.done:
        print("\nfinished:")
        for item in items:
            if item["status"] == "DONE":
                print(f"  [{item['id']}] §{item['sec']} {item['item']}")
                if item["evidence"]:
                    print(f"        evidence: {item['evidence']}")

    if args.remaining:
        print("\nstill open:")
        for item in items:
            if item["status"] != "DONE":
                print(f"  [{item['status']:<7}] [{item['id']}] §{item['sec']} {item['item']}")


if __name__ == "__main__":
    main()
