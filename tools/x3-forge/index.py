#!/usr/bin/env python3
"""X3 Forge: a persistent, provenance-carrying repository index.

The point of this file is that an agent should be able to ask "where is X" and
get a handful of exact locations instead of reading the tree. Everything it
reports carries provenance (commit, file hash), so a stale answer is detectable
rather than silently trusted.

Standard library only, so it runs anywhere the repo does without a build step.

    python3 tools/x3-forge/index.py build
    python3 tools/x3-forge/index.py find CertificationTier
    python3 tools/x3-forge/index.py tests-for pallets/x3-app-registry
    python3 tools/x3-forge/index.py todos
    python3 tools/x3-forge/index.py untested-crates
    python3 tools/x3-forge/index.py stale

Deliberate limits, so nobody mistakes this for a compiler:
  * Rust is parsed by a line-oriented scanner, not a real parser. It finds items
    by shape; it does not resolve types, macros, or cross-crate calls.
  * `callers` is not implemented. Approximating it would be worse than saying so.
"""
import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

INDEX_VERSION = 1
# Bump whenever the parser changes what it extracts, or the shape of what it
# stores. Without this the cache is keyed only on file content, so improving the
# parser leaves every unchanged file holding results the old parser produced —
# silently reverting the improvement. Version 3 split string-literal mentions out
# of `items` into `strings` (see below), which changes the stored schema.
# Version 4 parses multi-line attributes, so `#[ignore = "..."]` tests keep
# their `test`/`ignored` markers instead of disappearing from the index.
# Version 5 stops indexing the frozen extract trees (see FROZEN_PREFIXES),
# which changes the file set.
PARSER_VERSION = 5
DEFAULT_INDEX = ".x3-forge/index.json"

# Directories that are never source of truth for an engineering index.
SKIP_DIRS = {
    ".git", "target", "node_modules", ".wt-", "dist", "build", ".venv",
    "__pycache__", ".x3-forge", "out", "cache", "coverage",
    ".kilo", ".worktrees", "vendor",
}
SKIP_SUFFIX = (".lock", ".min.js", ".map")

# Frozen extract trees. `launch-gates/prepare-phase3-sources.sh` copies pallet
# sources and test files into per-audit packs, and those copies are committed,
# but they are point-in-time extracts of files that are indexed where they
# really live. Indexing them again duplicates every declaration and lets an old
# copy speak for a tree that has since changed: pack-05's router extract kept
# 26 `#[ignore]` markers after the live suite dropped them, and the completion
# engine reported a running required test as skipped. Evidence, not source of
# truth.
FROZEN_PREFIXES = ("launch-gates/sources/",)

RUST_ITEM = re.compile(
    r"^\s*(?P<vis>pub(?:\([^)]*\))?\s+)?"
    r"(?:async\s+|unsafe\s+|const\s+|extern\s+\"[^\"]*\"\s+)*"
    r"(?P<kind>fn|struct|enum|trait|union|type|mod|const|static)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
)
RUST_IMPL = re.compile(r"^\s*impl(?:<[^>]*>)?\s+(?P<name>[A-Za-z_][A-Za-z0-9_:<>, ]*?)\s*(?:where|\{|$)")
RUST_ATTR = re.compile(r"^\s*#\[\s*(?P<attr>[A-Za-z_:]+)")
PY_DEF = re.compile(r"^\s*(?:async\s+)?(?P<kind>def|class)\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)")
TODO = re.compile(r"\b(TODO|FIXME|XXX|HACK|unimplemented!|todo!)\b")
# Identifier-shaped string literals. Config keys such as "budget_fallback" are
# referenced as strings, never as declarations, so without this a query for the
# behaviour cannot find the file that implements it. They are collected
# separately: a mention is evidence, but it is not a declaration. `find` and the
# exact-symbol lookups search declarations only, so a name that appears only as a
# string (including the sentinel a test asserts is absent) must not make them
# report a hit; the context compiler reads `strings` explicitly and weights them
# as hints. Version 2 put these in `items`, and the two leaked into each other.
STRING_LITERAL = re.compile(r"[\"']([a-z][a-z0-9_]{4,})[\"']")
MAX_STRINGS_PER_FILE = 200


def _skip(path: Path, root: Path) -> bool:
    rel = path.relative_to(root)
    if rel.as_posix().startswith(FROZEN_PREFIXES):
        return True
    if any(part in SKIP_DIRS or part.startswith(".wt-") for part in rel.parts):
        return True
    return path.name.endswith(SKIP_SUFFIX)


def file_hash(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def commit(root: Path) -> str:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True, timeout=20
        )
        return out.stdout.strip() if out.returncode == 0 else "UNKNOWN"
    except (OSError, subprocess.SubprocessError):
        return "UNKNOWN"


def crate_for(path: Path, root: Path, crate_dirs):
    """Nearest ancestor directory that declares a Cargo.toml."""
    current = path.parent
    while True:
        if str(current) in crate_dirs:
            return crate_dirs[str(current)]
        if current == root or current.parent == current:
            return None
        current = current.parent


def load_crates(root: Path):
    """Map absolute directory -> crate name, and collect the workspace members."""
    manifests = []
    for path in iter_files(root):
        if path.name == "Cargo.toml":
            manifests.append(path)
    crates = {}
    for manifest in manifests:
        try:
            text = manifest.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        match = re.search(r'^\s*name\s*=\s*"([^"]+)"', text, re.M)
        if match:
            crates[str(manifest.parent)] = match.group(1)
    return crates, [str(m.relative_to(root)) for m in manifests]


def attribute_is_complete(text: str) -> bool:
    """True when an attribute's closing `]` is present outside a string.

    `#[ignore = "..."]` reasons are commonly spread over several lines with
    backslash continuations and may contain a `]`; only a bracket that is not
    inside the quoted reason ends the attribute.
    """
    in_string = False
    escaped = False
    for char in text:
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char == "]":
            return True
    return False


def parse_rust(text: str):
    """Line-oriented extraction. It reports shape, not semantics."""
    items = []
    pending_attrs = []
    in_block_comment = False
    in_attribute = False
    attribute_text = ""
    strings = 0
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        # Track /* */ so a commented-out fn is not indexed as a real one.
        if in_block_comment:
            if "*/" in line:
                in_block_comment = False
            continue
        if line.startswith("/*"):
            if "*/" not in line:
                in_block_comment = True
            continue
        if line.startswith("//"):
            # A commented symbol is exactly the false positive an index must not
            # report; only TODO markers are still worth surfacing.
            if TODO.search(line) and ("TODO" in line or "FIXME" in line or "XXX" in line or "HACK" in line):
                items.append({"kind": "todo", "name": line[:160], "line": number})
            continue

        # An attribute belongs to the item on a *later* line. Falling through to
        # the `pending_attrs = []` reset at the bottom of the loop is what made
        # every `#[test]` disappear, so attribute lines end the iteration here.
        # A multi-line attribute (`#[ignore = "..."]` with continuations) is
        # buffered until its closing bracket, or the `fn` after it would lose
        # its `test`/`ignore` attributes and the test would vanish from the
        # index — a false "required test missing" finding in completion.
        if in_attribute:
            attribute_text += "\n" + raw
            if attribute_is_complete(attribute_text):
                in_attribute = False
                attr = RUST_ATTR.match(attribute_text)
                if attr:
                    name = attr.group("attr")
                    pending_attrs.append(name)
                    if name in ("test", "tokio::test"):
                        pending_attrs.append("test")
            continue
        if line.startswith("#["):
            if not attribute_is_complete(raw):
                in_attribute = True
                attribute_text = raw
                continue
            attr = RUST_ATTR.match(raw)
            if attr:
                name = attr.group("attr")
                pending_attrs.append(name)
                if name in ("test", "tokio::test"):
                    pending_attrs.append("test")
            continue

        todo = TODO.search(raw)
        if todo and not line.startswith("//"):
            items.append({"kind": "todo", "name": line[:160], "line": number})

        for literal in STRING_LITERAL.findall(raw):
            if strings >= MAX_STRINGS_PER_FILE:
                break
            strings += 1
            items.append({"kind": "string", "name": literal, "line": number})

        item = RUST_ITEM.match(raw)
        if item:
            kind = item.group("kind")
            entry = {
                "kind": kind,
                "name": item.group("name"),
                "line": number,
                "visibility": "public" if item.group("vis") else "private",
                "attrs": sorted(set(pending_attrs)),
            }
            if "test" in pending_attrs or "pallet::call" in pending_attrs:
                entry["test"] = "test" in pending_attrs
            if "ignore" in pending_attrs:
                entry["ignored"] = True
            items.append(entry)
            pending_attrs = []
            continue

        impl = RUST_IMPL.match(raw)
        if impl:
            items.append({"kind": "impl", "name": impl.group("name").strip(), "line": number, "attrs": sorted(set(pending_attrs))})
            pending_attrs = []
            continue
        pending_attrs = []
    return items


def parse_python(text: str):
    items = []
    strings = 0
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if line.startswith("#"):
            continue
        for literal in STRING_LITERAL.findall(raw):
            if strings >= MAX_STRINGS_PER_FILE:
                break
            strings += 1
            items.append({"kind": "string", "name": literal, "line": number})
        match = PY_DEF.match(raw)
        if match:
            items.append({"kind": match.group("kind"), "name": match.group("name"), "line": number})
        if TODO.search(raw) and not line.startswith("#"):
            items.append({"kind": "todo", "name": line[:160], "line": number})
    return items


def lang_of(path: Path):
    return {
        ".rs": "rust",
        ".py": "python",
        ".toml": "toml",
        ".json": "json",
        ".md": "markdown",
        ".sh": "shell",
        ".yml": "yaml",
        ".yaml": "yaml",
        ".ts": "typescript",
        ".tsx": "typescript",
        ".js": "javascript",
        ".sol": "solidity",
    }.get(path.suffix)


def iter_files(root: Path):
    """Yield candidate files, pruning skipped directories instead of walking them."""
    for current, dirs, names in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS and not d.startswith(".wt-"))
        for name in sorted(names):
            yield Path(current) / name


def build(root: Path, previous=None):
    """Build or refresh the index. Unchanged files are reused, not reparsed."""
    crates, manifests = load_crates(root)
    previous = previous or {}
    # A cache written by a different parser cannot be trusted, however unchanged
    # the files are.
    previous_files = previous.get("files", {}) if previous.get("parser_version") == PARSER_VERSION else {}
    files = {}
    reused = 0
    parsed = 0
    for path in iter_files(root):
        if not path.is_file() or _skip(path, root):
            continue
        language = lang_of(path)
        if language is None:
            continue
        rel = str(path.relative_to(root))
        try:
            digest = file_hash(path)
            size = path.stat().st_size
        except OSError:
            continue
        cached = previous_files.get(rel)
        if cached and cached.get("sha256") == digest:
            files[rel] = cached
            reused += 1
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        if language == "rust":
            items = parse_rust(text)
        elif language == "python":
            items = parse_python(text)
        else:
            items = [
                {"kind": "todo", "name": line.strip()[:160], "line": number}
                for number, line in enumerate(text.splitlines(), start=1)
                if TODO.search(line) and not line.strip().startswith(("#", "//"))
            ]
        # Declarations and mentions are distinct kinds of evidence, and different
        # consumers want different ones: `find` and completion intelligence want
        # declarations, while the context compiler may use a mention as a hint.
        # Keeping both in `items` let a string literal answer a symbol query.
        declarations = [i for i in items if i["kind"] != "string"]
        strings = [i for i in items if i["kind"] == "string"]
        files[rel] = {
            "sha256": digest,
            "bytes": size,
            "lang": language,
            "crate": crate_for(path, root, crates),
            "items": declarations,
            "strings": strings,
        }
        parsed += 1
    return {
        "index_version": INDEX_VERSION,
        "parser_version": PARSER_VERSION,
        "root": str(root),
        "commit": commit(root),
        "crates": len(manifests),
        "manifests": manifests,
        "files": files,
        "stats": {
            "files_indexed": len(files),
            "parsed": parsed,
            "reused": reused,
            "items": sum(len(f["items"]) for f in files.values()),
            "strings": sum(len(f.get("strings", [])) for f in files.values()),
            "todos": sum(1 for f in files.values() for i in f["items"] if i["kind"] == "todo"),
            "tests": sum(1 for f in files.values() for i in f["items"] if i.get("test")),
        },
    }


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def cmd_build(args):
    root = Path(args.root).resolve()
    out = root / args.out
    previous = load(out) if out.exists() else None
    index = build(root, previous)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(index, indent=1) + "\n", encoding="utf-8")
    stats = index["stats"]
    print(f"indexed {stats['files_indexed']} files ({stats['parsed']} parsed, {stats['reused']} reused)")
    print(f"items {stats['items']}  strings {stats['strings']}  todos {stats['todos']}  tests {stats['tests']}  crates {index['crates']}")
    print(f"commit {index['commit']}  -> {out}")
    return 0


def cmd_find(args):
    index = load(Path(args.index))
    needle = args.name
    hits = []
    for rel, entry in index["files"].items():
        for item in entry["items"]:
            if item["kind"] == "todo":
                continue
            if item["name"] == needle or (args.substring and needle.lower() in item["name"].lower()):
                hits.append((rel, item))
    if not hits:
        print(f"no matches for {needle!r}")
        return 1
    for rel, item in hits[: args.limit]:
        print(f"{rel}:{item['line']}  {item['kind']} {item['name']}")
    if len(hits) > args.limit:
        print(f"... {len(hits) - args.limit} more")
    return 0


def cmd_tests_for(args):
    index = load(Path(args.index))
    prefix = args.path.rstrip("/")
    found = 0
    for rel, entry in sorted(index["files"].items()):
        if not rel.startswith(prefix):
            continue
        tests = [i for i in entry["items"] if i.get("test")]
        if tests:
            print(f"{rel}: {len(tests)} test(s)")
            for item in tests:
                print(f"    line {item['line']}  {item['name']}")
            found += len(tests)
    if not found:
        print(f"no tests indexed under {prefix}")
        return 1
    print(f"total {found}")
    return 0


def cmd_todos(args):
    index = load(Path(args.index))
    count = 0
    for rel, entry in sorted(index["files"].items()):
        for item in entry["items"]:
            if item["kind"] == "todo":
                print(f"{rel}:{item['line']}  {item['name']}")
                count += 1
    print(f"total {count}")
    return 0


def cmd_untested_crates(args):
    """Crates with no indexed test at all. This is a discovery surface, not a
    judgement: a crate can be covered by another crate's tests."""
    index = load(Path(args.index))
    tested = set()
    all_crates = set()
    for entry in index["files"].values():
        crate = entry.get("crate")
        if not crate:
            continue
        all_crates.add(crate)
        if any(i.get("test") for i in entry["items"]):
            tested.add(crate)
    untested = sorted(all_crates - tested)
    for crate in untested:
        print(crate)
    print(f"{len(untested)} of {len(all_crates)} crates have no indexed test")
    return 0


def cmd_stale(args):
    """Files that changed since the index was built. This is what makes a cached
    answer safe to trust or not."""
    root = Path(args.root).resolve()
    index = load(Path(args.index))
    stale = []
    for rel, entry in index["files"].items():
        path = root / rel
        if not path.exists():
            stale.append((rel, "deleted"))
            continue
        if file_hash(path) != entry["sha256"]:
            stale.append((rel, "changed"))
    if index["commit"] != commit(root):
        print(f"HEAD moved: index {index['commit'][:12]} -> {commit(root)[:12]}")
    for rel, why in stale:
        print(f"{why}: {rel}")
    print(f"{len(stale)} stale of {len(index['files'])} indexed")
    return 0


def main():
    parser = argparse.ArgumentParser(description="X3 Forge repository index")
    parser.add_argument("--root", default=".")
    parser.add_argument("--index", default=DEFAULT_INDEX)
    sub = parser.add_subparsers(dest="command", required=True)

    build_parser = sub.add_parser("build")
    build_parser.add_argument("--out", default=DEFAULT_INDEX)
    build_parser.set_defaults(func=cmd_build)

    find_parser = sub.add_parser("find")
    find_parser.add_argument("name")
    find_parser.add_argument("--substring", action="store_true")
    find_parser.add_argument("--limit", type=int, default=20)
    find_parser.set_defaults(func=cmd_find)

    tests_parser = sub.add_parser("tests-for")
    tests_parser.add_argument("path")
    tests_parser.set_defaults(func=cmd_tests_for)

    sub.add_parser("todos").set_defaults(func=cmd_todos)
    sub.add_parser("untested-crates").set_defaults(func=cmd_untested_crates)
    sub.add_parser("stale").set_defaults(func=cmd_stale)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
