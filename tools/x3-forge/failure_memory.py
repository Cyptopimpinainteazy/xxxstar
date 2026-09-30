#!/usr/bin/env python3
"""X3 Forge memory: failures, successes and dead ends (§17-19).

The spec asks for three memories that share one shape:

* a **failure** so the same problem is not rediscovered (§17),
* a **success** so a working strategy is reused rather than re-derived (§18),
* a **dead end** so a known-broken approach is not attempted again (§19).

They are one append-only JSONL log with a `kind` field, because the fields the
spec lists are the same fields, and three stores would drift.

Append-only on purpose. A memory that rewrites itself cannot be audited, and
this one is evidence: "we already tried this and it failed" is only useful if
the record of it cannot quietly change. Repeat sightings of the same
fingerprint are appended as new entries; search aggregates them.

    python3 tools/x3-forge/failure_memory.py add --kind failure \
        --component crates/cross-vm-coordinator \
        --error "abort after complete refunded both legs" \
        --root-cause "abort() bypassed the transition table" \
        --fix "route abort through the same table" \
        --regression crates/x3-sim/tests/refund_after_claim.rs
    python3 tools/x3-forge/failure_memory.py find "refund after claim"
    python3 tools/x3-forge/failure_memory.py stats

Deliberate limits, so nobody mistakes this for more than it is:
  * Retrieval is lexical (token overlap), not semantic. A match on words is a
    hint to read an entry, not a claim that it is the same bug.
  * Nothing here decides that two failures are the same cause. It proposes a
    fingerprint by normalising the error text; a human or a later harness
    confirms it.
"""
import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

DEFAULT_STORE = ".x3-forge/memory.jsonl"
KINDS = ("failure", "success", "dead_end")

# Fields the spec names, kept as one list so `add`, search and the router
# integration cannot disagree about what an entry is.
TEXT_FIELDS = ("error", "component", "trigger", "root_cause", "fix", "regression",
               "invariant", "historical_analogue", "model", "context_package",
               "approach", "conditions", "commit")

STOPWORDS = {
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "of", "in", "to",
    "for", "and", "or", "on", "at", "by", "with", "it", "this", "that", "we",
    "i", "you", "did", "do", "does", "not", "no", "than", "then", "when", "how",
    "what", "why", "where", "from", "as", "so", "if", "but", "into", "after",
}


def tokens(text):
    """Lowercase word tokens, stopwords removed.

    Over-filtering silently drops terms the caller meant, so the stopword list
    stays short and matching falls back to prefixes rather than requiring exact
    words.
    """
    words = re.findall(r"[a-z0-9_./-]+", str(text).lower())
    return [word for word in words if word and word not in STOPWORDS]


def fingerprint(component, error, kind="failure"):
    """A stable id for "this failure", so repeats can be counted.

    It normalises whitespace, hex addresses and reported line/column positions,
    which are the parts of an error message that change between runs of the
    same bug. It deliberately does **not** normalise every integer: an earlier
    version did, and then twenty distinct failures whose only difference was a
    count collapsed into one row.

    `kind` is part of the id because §17-19 are three memories. The same error
    text recorded as a dead end and as a failure are different knowledge, and
    collapsing them loses one of them.

    It is a proposal either way: two genuinely different problems can still
    collide, so search returns the entries rather than trusting the id alone.
    """
    text = str(error).lower()
    text = re.sub(r"0x[0-9a-f]+", "0xaddr", text)
    text = re.sub(r"\b(line|col|column|offset)\s*[:=]?\s*\d+", r"\1 n", text)
    text = re.sub(r"\s+", " ", text).strip()
    material = str(kind).strip().lower() + "\n" + str(component).strip().lower() + "\n" + text
    return hashlib.sha256(material.encode("utf-8")).hexdigest()[:16]


def now_iso():
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat()


def load(store):
    """Every entry, in the order it was written.

    A malformed line is reported to stderr and skipped rather than aborting the
    read: a truncated append must not make the whole memory unreadable.
    """
    path = Path(store)
    if not path.exists():
        return []
    entries = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            entry = json.loads(line)
        except ValueError:
            print(f"failure_memory: skipping unreadable line {number}", file=sys.stderr)
            continue
        if isinstance(entry, dict):
            entries.append(entry)
    return entries


def record(store, entry, now=None):
    """Append one entry.

    The fingerprint is derived, not supplied, so two callers cannot disagree
    about what "the same failure" means and quietly split one memory in two.
    """
    kind = entry.get("kind", "failure")
    if kind not in KINDS:
        raise ValueError("kind must be one of " + ", ".join(KINDS))
    clean = {"kind": kind, "recorded_at": now or now_iso()}
    for field in TEXT_FIELDS:
        value = entry.get(field)
        if value not in (None, ""):
            clean[field] = value if isinstance(value, str) else str(value)
    if "fingerprint" in entry and entry["fingerprint"]:
        clean["fingerprint"] = str(entry["fingerprint"])
    else:
        clean["fingerprint"] = fingerprint(clean.get("component", ""), clean.get("error", ""), kind)
    if entry.get("occurrences"):
        clean["occurrences"] = int(entry["occurrences"])
    path = Path(store)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(clean, sort_keys=True) + "\n")
    return clean


def aggregate(entries):
    """Collapse repeats: one row per fingerprint, newest text, count of sightings."""
    collapsed = {}
    for entry in entries:
        key = entry.get("fingerprint") or fingerprint(
            entry.get("component", ""), entry.get("error", ""), entry.get("kind", "failure"))
        if key not in collapsed:
            collapsed[key] = dict(entry, fingerprint=key, occurrences=0)
        collapsed[key]["occurrences"] += 1
        # Later entries win the descriptive fields: a later record is usually
        # the better-understood one, because the first is written mid-incident.
        for field in TEXT_FIELDS:
            if entry.get(field):
                collapsed[key][field] = entry[field]
        collapsed[key]["last_seen"] = entry.get("recorded_at")
        collapsed[key].setdefault("first_seen", entry.get("recorded_at"))
    return list(collapsed.values())


def score(entry, query_tokens):
    """How well an entry answers a query. Lexical, and labelled as such."""
    if not query_tokens:
        return 0
    haystack = " ".join(str(entry.get(field, "")) for field in TEXT_FIELDS).lower()
    score = 0
    for token in query_tokens:
        if token in haystack:
            score += 3 if len(token) > 4 else 1
    # A component named in the query is a strong signal: it is the difference
    # between "something like this happened" and "this happened here".
    component = str(entry.get("component", "")).lower()
    if component and any(token in component for token in query_tokens):
        score += 4
    if entry.get("occurrences", 1) > 1:
        score += min(entry["occurrences"], 5)
    return score


def search(entries, query, kind=None, component=None, limit=10):
    """Ranked entries for a query, with the reason each one matched."""
    query_tokens = tokens(query)
    rows = aggregate(entries)
    if kind:
        rows = [row for row in rows if row.get("kind") == kind]
    if component:
        wanted = component.lower()
        rows = [row for row in rows if wanted in str(row.get("component", "")).lower()]
    scored = [(score(row, query_tokens), row) for row in rows]
    scored = [pair for pair in scored if pair[0] > 0]
    scored.sort(key=lambda pair: (-pair[0], str(pair[1].get("fingerprint"))))
    matches = []
    for value, row in scored[:limit]:
        matches.append(dict(row, match_score=value))
    return matches


def summarise(entries):
    rows = aggregate(entries)
    counts = {kind: 0 for kind in KINDS}
    for row in rows:
        counts[row.get("kind", "failure")] = counts.get(row.get("kind", "failure"), 0) + 1
    components = {}
    for row in rows:
        name = row.get("component") or "(unspecified)"
        components[name] = components.get(name, 0) + 1
    return {
        "entries": len(entries),
        "distinct": len(rows),
        "repeats": len(entries) - len(rows),
        "by_kind": counts,
        "by_component": dict(sorted(components.items(), key=lambda item: -item[1])),
        "components": len(components),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description="X3 Forge failure, success and dead-end memory.")
    parser.add_argument("--store", default=DEFAULT_STORE)
    sub = parser.add_subparsers(dest="command", required=True)

    add = sub.add_parser("add", help="record an entry")
    add.add_argument("--kind", default="failure", choices=KINDS)
    for field in TEXT_FIELDS:
        add.add_argument("--" + field.replace("_", "-"), dest=field, default=None)
    add.add_argument("--json", action="store_true")

    find = sub.add_parser("find", help="search the memory")
    find.add_argument("query")
    find.add_argument("--kind", default=None, choices=KINDS)
    find.add_argument("--component", default=None)
    find.add_argument("--limit", type=int, default=10)
    find.add_argument("--json", action="store_true")

    stats = sub.add_parser("stats", help="counts by kind and component")
    stats.add_argument("--json", action="store_true")

    args = parser.parse_args(argv)

    if args.command == "add":
        entry = {field: getattr(args, field) for field in TEXT_FIELDS}
        entry["kind"] = args.kind
        saved = record(args.store, entry)
        print(json.dumps(saved, indent=2, sort_keys=True) if args.json
              else f"recorded {saved['kind']} {saved['fingerprint']}")
        return 0

    if args.command == "find":
        matches = search(load(args.store), args.query, args.kind, args.component, args.limit)
        if args.json:
            print(json.dumps(matches, indent=2, sort_keys=True))
        else:
            if not matches:
                print("no memory matches this query")
            for row in matches:
                print(f"{row['match_score']:>3}  {row.get('kind','failure'):9} "
                      f"{row.get('fingerprint','')}  x{row.get('occurrences',1)}  "
                      f"{row.get('component','')}")
                if row.get("error"):
                    print(f"     {row['error'][:120]}")
                if row.get("fix"):
                    print(f"     fix: {row['fix'][:120]}")
        return 0

    summary = summarise(load(args.store))
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(f"entries   {summary['entries']}")
        print(f"distinct  {summary['distinct']}   repeats {summary['repeats']}")
        for kind, count in sorted(summary["by_kind"].items()):
            print(f"  {kind:9} {count}")
        for name, count in list(summary["by_component"].items())[:10]:
            print(f"  {name}: {count}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
