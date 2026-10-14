#!/usr/bin/env python3
"""X3 Forge context compiler.

The index answers "where is X". This answers the question an agent actually has
before it starts: *what do I need to read, and what am I not reading?* It
selects the smallest set of files that covers the task, attaches the provenance
that makes the selection checkable, and states explicitly what it left out and
why — a context package that silently omits the relevant file is worse than one
that admits the truncation.

    python3 tools/x3-forge/context.py "where is settlement uniqueness enforced?"
    python3 tools/x3-forge/context.py --budget 4000 --max-files 6 "guardian certification"
    python3 tools/x3-forge/context.py --json "atomic swap refund"

Reads `.x3-forge/index.json`; run `index.py build` first.
"""
import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_INDEX = ".x3-forge/index.json"
# Sized for this repository's real modules, not a toy. The default used to be
# 12000 tokens (~9k words), which is smaller than a single first-party file: the
# model router is ~28k tokens. A query about such a module then returned a batch
# of weakly-matching small files and marked the module itself "budget" — the
# compiler could not answer the question it exists to answer. 60000 holds the
# largest current module together with its tests (router ~28k + its tests ~25k),
# and callers who want a tighter package pass --budget explicitly.
DEFAULT_BUDGET = 60000
DEFAULT_MAX_FILES = 12
# Deliberately crude: 4 bytes per token. The number exists to bound the package,
# not to be an accurate tokenizer, and it is labelled as an estimate.
BYTES_PER_TOKEN = 4
# Vendored third-party trees are indexed (an auditor may need them) but they are
# not this project's source. Without this penalty a query for an ordinary word
# like "provider" ranks a vendored crate above the file that implements it.
VENDOR_MARKERS = ("tauri-vendor", "vendor/", "third_party", "thirdparty",
                  "node_modules/", "patches/")
# Generated files are full of plausible-looking symbols that mean nothing for
# context. They stay indexed but rank after hand-written source.
GENERATED_MARKERS = ("weights.rs", "/generated/", ".gen.", "_pb2.py", "/migrations/")

# Words that carry no selection signal. Kept deliberately short: over-filtering
# silently drops terms the caller meant.
STOPWORDS = {
    "a", "an", "the", "is", "are", "where", "what", "how", "why", "of", "in",
    "to", "for", "and", "or", "on", "at", "by", "with", "do", "does", "did",
    "it", "this", "that", "be", "been", "was", "were", "we", "i", "you",
    "enforced", "enforce", "work", "works", "code", "file", "files",
}


def terms_of(query):
    """Query words worth matching on, in first-seen order."""
    words = re.findall(r"[A-Za-z_][A-Za-z0-9_]{1,}", query)
    out = []
    for word in words:
        lowered = word.lower()
        if lowered in STOPWORDS or lowered in out:
            continue
        out.append(lowered)
    return out

_SUFFIXES = ("ations", "ation", "ications", "ication", "ings", "ing", "ers", "er",
             "ors", "or", "ions", "ion", "ies", "es", "ed", "s")


def words_of(name):
    """Split an identifier into matching units: snake_case and camelCase."""
    out = []
    for chunk in re.split(r"_+", name):
        if not chunk:
            continue
        for part in re.findall(r"[A-Z]+(?![a-z])|[A-Z]?[a-z0-9]+", chunk) or [chunk]:
            lowered = part.lower()
            if lowered and lowered not in out:
                out.append(lowered)
    return out or [name.lower()]


def common_prefix(a, b):
    n = 0
    while n < min(len(a), len(b)) and a[n] == b[n]:
        n += 1
    return n


def name_matches(term, name):
    """Score how well a query term matches an identifier, or 0 for no match.

    Word-level rather than substring: `certification` has to reach
    `certify_application` and `revocation` has to reach `revoke_application`, or
    a query about certification cannot find the certification code. The shared
    prefix rule (>=5 strong, >=4 weak) is what bridges those forms; it is a
    heuristic and is documented as one.
    """
    if term == name:
        return 10
    best = 0
    for word in words_of(name):
        if word == term:
            best = max(best, 9)
            continue
        # A prefix is evidence only when the shared run is long enough to mean
        # something. Requiring a minimum on *both* tokens is what stops a
        # single-letter generic parameter (`ParallelProposerFactory<A, B, C, PR>`)
        # from making a file "cover" `critical`, `budget` and `provider` at once —
        # which is how an unrelated crate outranked the module that implements the
        # behaviour for "critical provider budget fallback".
        if len(term) >= 5 and min(len(term), len(word)) >= 5 \
                and (word.startswith(term) or term.startswith(word)):
            best = max(best, 6)
            continue
        # A 4-character prefix is not evidence on a 500k-symbol index: it made
        # `patches/rustix` outrank the settlement engine. It only counts when
        # both words are long enough for four characters to mean something.
        shared = common_prefix(term, word)
        if shared >= 6:
            best = max(best, 5)
        elif shared == 5:
            best = max(best, 4)
        elif shared == 4 and len(term) >= 6 and len(word) >= 6:
            best = max(best, 2)
    return best


def score_file(rel, entry, terms):
    """Rank a file against the query terms. Exact symbol names dominate, path
    matches are worth less, and comments never count."""
    strong = 0
    weak = 0
    covered = set()
    symbols = []
    tests = []
    todos = []
    for item in entry["items"]:
        if item["kind"] == "todo":
            if any(term in item["name"].lower() for term in terms):
                todos.append(item)
            continue
        name = item["name"].lower()
        if item.get("test"):
            tests.append(item)
        matched = False
        for term in terms:
            weight = name_matches(term, name)
            if weight:
                # A 4-character prefix is a hint, not evidence. Let hints nudge a
                # file but never let a file win on hints alone, which is how
                # unrelated vendored code used to outrank the real module.
                covered.add(term)
                if weight >= 3:
                    strong += weight
                else:
                    weak += weight
                matched = True
        if matched:
            symbols.append({"kind": item["kind"], "name": item["name"], "line": item["line"]})
    # A string literal is a mention, not a declaration. `"budget_fallback"` names a
    # config key a file implements, so a file that contains it is relevant — but a
    # mention must never carry the weight of a declared symbol, or a file full of
    # unrelated string literals wins the query. Measured before this split:
    # `services/x3-ai-router/router.py` fell out of the top 6 (dropped by the file
    # budget) for "critical provider budget fallback", beaten by two crates that
    # matched the words only inside their string literals. Coverage and the strong
    # bucket count declarations; a mention only nudges.
    mentions = 0
    for item in entry.get("strings", []):
        name = item["name"].lower()
        if any(name_matches(term, name) for term in terms):
            mentions += 1
            symbols.append({"kind": "string", "name": item["name"], "line": item["line"]})
    # Coverage first. A file that answers three of the query's words is more
    # useful than one that mentions a single word a hundred times, and on a
    # 527k-symbol index raw accumulation just ranks the biggest file first.
    score = 50 * len(covered) + min(strong, 45) + min(weak, 5) + min(mentions, 2)
    lowered_path = rel.lower()
    for term in terms:
        if term in lowered_path:
            score += 3
    # A file that already contains tests for a matched symbol is more useful than
    # one that does not: the tests are the specification of the behaviour.
    if symbols and tests:
        score += 2
    return score, symbols, tests, todos


def build_package(index, query, budget_tokens=DEFAULT_BUDGET, max_files=DEFAULT_MAX_FILES):
    terms = terms_of(query)
    candidates = []
    for rel, entry in index["files"].items():
        score, symbols, tests, todos = score_file(rel, entry, terms)
        if score > 0:
            vendored = any(marker in rel for marker in VENDOR_MARKERS)
            generated = any(marker in rel for marker in GENERATED_MARKERS)
            candidates.append({"path": rel, "score": score, "entry": entry,
                               "vendored": vendored, "generated": generated,
                               "symbols": symbols, "tests": tests, "todos": todos})
    # First-party first, then score, then path. The path tiebreak makes the
    # result order-stable regardless of dict iteration order.
    candidates.sort(key=lambda c: (c["vendored"], c["generated"], -c["score"], c["path"]))

    included = []
    excluded = []
    used_tokens = 0
    for rank, candidate in enumerate(candidates):
        cost = max(1, candidate["entry"]["bytes"] // BYTES_PER_TOKEN)
        if len(included) >= max_files:
            excluded.append({"path": candidate["path"], "score": candidate["score"], "reason": "max_files"})
            continue
        if used_tokens + cost > budget_tokens:
            excluded.append({"path": candidate["path"], "score": candidate["score"], "reason": "budget"})
            continue
        used_tokens += cost
        included.append({
            "path": candidate["path"],
            "score": candidate["score"],
            "sha256": candidate["entry"]["sha256"],
            "bytes": candidate["entry"]["bytes"],
            "estimated_tokens": cost,
            "crate": candidate["entry"].get("crate"),
            "lang": candidate["entry"]["lang"],
            "vendored": candidate["vendored"],
            "generated": candidate["generated"],
            "symbols": sorted(candidate["symbols"], key=lambda s: (s["line"], s["name"]))[:40],
            "tests": [{"name": t["name"], "line": t["line"]} for t in sorted(candidate["tests"], key=lambda t: t["line"])][:40],
            "todos": [{"line": t["line"], "text": t["name"]} for t in sorted(candidate["todos"], key=lambda t: t["line"])][:20],
        })

    manifests = set()
    for item in included:
        if item["crate"]:
            for manifest in index.get("manifests", []):
                if item["path"].startswith(str(Path(manifest).parent) + "/") or str(Path(manifest).parent) == ".":
                    manifests.add(manifest)

    return {
        "query": query,
        "terms": terms,
        "index_commit": index.get("commit"),
        "index_version": index.get("index_version"),
        "included": included,
        "excluded": excluded,
        "manifests": sorted(manifests),
        "files_matched": len(candidates),
        "files_included": len(included),
        "files_excluded": len(excluded),
        "estimated_tokens": used_tokens,
        "budget_tokens": budget_tokens,
        "max_files": max_files,
        "truncated": bool(excluded),
    }


def render(package):
    lines = [
        f"query: {package['query']}",
        f"terms: {' '.join(package['terms']) or '(none)'}",
        f"index: commit {str(package['index_commit'])[:12]}  matched {package['files_matched']} files",
        f"package: {package['files_included']} files, ~{package['estimated_tokens']} tokens "
        f"of {package['budget_tokens']} budget, {package['files_excluded']} excluded",
    ]
    if not package["included"]:
        lines.append("nothing matched. An empty package is the honest answer; widen the query.")
        return "\n".join(lines)
    for item in package["included"]:
        lines.append("")
        lines.append(f"{item['path']}  (score {item['score']}, ~{item['estimated_tokens']} tokens, {item['lang']})")
        lines.append(f"    sha256 {item['sha256'][:16]}  crate {item['crate'] or '-'}")
        for symbol in item["symbols"][:8]:
            lines.append(f"    {symbol['line']:>6}  {symbol['kind']} {symbol['name']}")
        if len(item["symbols"]) > 8:
            lines.append(f"    ... {len(item['symbols']) - 8} more symbols")
        if item["tests"]:
            lines.append(f"    tests: {', '.join(t['name'] for t in item['tests'][:5])}")
    if package["excluded"]:
        lines.append("")
        lines.append("excluded (so you know what was left out):")
        for item in package["excluded"][:10]:
            lines.append(f"    {item['reason']:<10} {item['path']}")
        if len(package["excluded"]) > 10:
            lines.append(f"    ... {len(package['excluded']) - 10} more")
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="X3 Forge context compiler")
    parser.add_argument("query")
    parser.add_argument("--index", default=DEFAULT_INDEX)
    parser.add_argument("--budget", type=int, default=DEFAULT_BUDGET)
    parser.add_argument("--max-files", type=int, default=DEFAULT_MAX_FILES)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    path = Path(args.index)
    if not path.exists():
        sys.exit(f"no index at {path}; run: python3 tools/x3-forge/index.py build")
    index = json.loads(path.read_text(encoding="utf-8"))
    package = build_package(index, args.query, args.budget, args.max_files)
    print(json.dumps(package, indent=1) if args.json else render(package))
    return 0


if __name__ == "__main__":
    sys.exit(main())
