#!/usr/bin/env python3
"""External toolchain inventory and gap analysis (PHASE 0 of the master prompt).

The master prompt's first instruction is "do NOT blindly install software.
First inspect the repository and current environment. Reuse existing working
integrations." Its second is that "an installed executable does NOT count as
implemented":

    installed -> configured -> connected to X3 -> exercised
    -> produces useful results -> evidence captured -> repeatable

This tool answers only the part of that question a filesystem can answer, and
refuses to answer the rest. It reports, per tool, with evidence:

    AVAILABLE     the tool exists somewhere reachable
    INSTALLED     a binary resolves on this machine (path recorded)
    CONFIGURED    a configuration file for it exists in the repo
    WIRED         repository files reference it (file:line recorded)
    GATED         a CI workflow or gate list runs it

and then stops. EXERCISED / EVIDENCED / REPEATABLE are **not** inferred from
file presence: a workflow that mentions `kani` is not proof that kani ran, let
alone that its result was captured. Those are reported as `not_determinable`
with what would be needed to establish them.

    python3 tools/x3-forge/toolchain.py inventory --out reports/toolchain
    python3 tools/x3-forge/toolchain.py show kani
    python3 tools/x3-forge/toolchain.py gaps
    python3 tools/x3-forge/toolchain.py priority

Writes `external-tool-inventory.json` and `external-tool-gap-analysis.md`.

Deliberate limits:
  * The catalogue is transcribed from the master prompt's own lists. Adding a
    tool the prompt names and this catalogue omits is a bug in this file.
  * Evidence is lexical: a mention in a comment counts the same as a real
    invocation. That is why a mention never upgrades a tool past WIRED.
  * Nothing here runs the tools. Installing or invoking them is a separate,
    operator-approved step.
"""
import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# name, aliases for searching, executable to resolve, family, priority group.
# Groups follow the prompt: 1 = before the seven-validator campaign (§77),
# 2 = evaluate next (§78), 3 = reference only (§79).
CATALOG = [
    # ── §1 Rust fuzzing ──────────────────────────────────────────────────
    ("cargo-fuzz", ("cargo-fuzz", "cargo fuzz", "libfuzzer"), "cargo-fuzz", "fuzzing", 1),
    ("afl++", ("afl++", "afl-fuzz", "cargo-afl"), "afl-fuzz", "fuzzing", 2),
    ("honggfuzz", ("honggfuzz", "cargo-hfuzz"), "honggfuzz", "fuzzing", 2),
    ("proptest", ("proptest",), None, "property", 1),
    ("proptest-state-machine", ("proptest-state-machine", "prop_state_machine"), None, "property", 1),
    # ── §2 correctness / concurrency ─────────────────────────────────────
    ("miri", ("miri", "cargo miri"), "cargo-miri", "correctness", 2),
    ("loom", ("loom",), None, "concurrency", 2),
    ("shuttle", ("shuttle",), None, "concurrency", 2),
    # ── §3 formal ────────────────────────────────────────────────────────
    ("kani", ("kani", "cargo kani"), "kani", "formal", 1),
    # ── §4 mutation ──────────────────────────────────────────────────────
    ("cargo-mutants", ("cargo-mutants", "cargo mutants"), "cargo-mutants", "mutation", 1),
    # ── §6-§8 network / runtime testing ──────────────────────────────────
    ("zombienet", ("zombienet",), "zombienet", "multi-validator", 1),
    ("chopsticks", ("chopsticks",), "chopsticks", "runtime-testing", 2),
    ("try-runtime", ("try-runtime", "try_runtime"), "try-runtime", "runtime-testing", 1),
    ("frame-benchmarking", ("frame-benchmarking", "frame_benchmarking", "benchmarking"), None, "runtime-testing", 1),
    ("srtool", ("srtool",), "srtool", "reproducibility", 2),
    ("subwasm", ("subwasm",), "subwasm", "reproducibility", 2),
    # ── §12-§18 EVM ──────────────────────────────────────────────────────
    ("foundry", ("foundry", "forge ", "cast ", "anvil"), "forge", "evm", 1),
    ("anvil", ("anvil",), "anvil", "evm", 1),
    ("slither", ("slither",), "slither", "evm-static", 1),
    ("echidna", ("echidna",), "echidna", "evm-fuzz", 1),
    ("medusa", ("medusa",), "medusa", "evm-fuzz", 2),
    ("halmos", ("halmos",), "halmos", "evm-symbolic", 2),
    ("mythril", ("mythril", "myth "), "myth", "evm-symbolic", 2),
    ("revm", ("revm",), None, "evm-oracle", 1),
    ("reth", ("reth",), None, "evm-reference", 3),
    # ── §21-§24 SVM ──────────────────────────────────────────────────────
    ("litesvm", ("litesvm",), None, "svm", 1),
    ("mollusk", ("mollusk",), None, "svm", 1),
    ("trident", ("trident",), None, "svm-fuzz", 2),
    ("solana-test-validator", ("solana-test-validator", "solana_test_validator"),
     "solana-test-validator", "svm", 1),
    # ── §25-§28 network chaos ────────────────────────────────────────────
    ("toxiproxy", ("toxiproxy",), "toxiproxy", "chaos", 1),
    ("tc/netem", ("netem", "tc qdisc", "tc/netem"), "tc", "chaos", 1),
    ("pumba", ("pumba",), "pumba", "chaos", 2),
    ("chaos-mesh", ("chaos-mesh", "chaos mesh"), None, "chaos", 2),
    # ── §29-§30 load ─────────────────────────────────────────────────────
    ("k6", ("k6",), "k6", "load", 1),
    ("wrk2", ("wrk2", "wrk"), "wrk", "load", 2),
    # ── static analysis / supply chain ───────────────────────────────────
    ("semgrep", ("semgrep",), "semgrep", "static", 1),
    ("codeql", ("codeql",), "codeql", "static", 2),
    ("cargo-audit", ("cargo-audit", "cargo audit"), "cargo-audit", "supply-chain", 1),
    ("cargo-deny", ("cargo-deny", "cargo deny"), "cargo-deny", "supply-chain", 1),
    ("osv-scanner", ("osv-scanner", "osv-scan"), "osv-scanner", "supply-chain", 2),
    ("trivy", ("trivy",), "trivy", "supply-chain", 2),
    ("syft", ("syft",), "syft", "sbom", 2),
    ("grype", ("grype",), "grype", "sbom", 2),
    ("gitleaks", ("gitleaks",), "gitleaks", "secrets", 1),
    ("trufflehog", ("trufflehog",), "trufflehog", "secrets", 2),
    # ── observability ────────────────────────────────────────────────────
    ("prometheus", ("prometheus",), "prometheus", "observability", 1),
    ("grafana", ("grafana",), "grafana-server", "observability", 1),
    ("loki", ("loki",), "loki", "observability", 2),
    ("opentelemetry", ("opentelemetry", "otel"), None, "observability", 2),
    ("tempo", ("tempo",), None, "observability", 2),
    # ── infrastructure ───────────────────────────────────────────────────
    ("docker", ("docker",), "docker", "infrastructure", 2),
    ("podman", ("podman",), "podman", "infrastructure", 2),
    ("kubernetes", ("kubernetes", "kubectl", "helm"), "kubectl", "infrastructure", 3),
    ("ansible", ("ansible",), "ansible", "infrastructure", 1),
    # ── networking / consensus reference ─────────────────────────────────
    ("rust-libp2p", ("libp2p",), None, "networking", 1),
    ("malachite", ("malachite",), None, "consensus-reference", 3),
    ("cometbft", ("cometbft", "tendermint"), None, "consensus-reference", 3),
    ("ethereum-hive", ("hive", "ethereum hive"), None, "conformance-reference", 3),
    ("hermes", ("hermes", "ibc"), None, "cross-chain-reference", 3),
    ("narwhal", ("narwhal",), None, "da-reference", 3),
    # ── storage ──────────────────────────────────────────────────────────
    ("paritydb", ("paritydb", "parity-db"), None, "storage", 3),
    ("rocksdb", ("rocksdb",), None, "storage", 3),
    ("jsonrpsee", ("jsonrpsee",), None, "rpc", 1),
    # ── formal protocol modelling ────────────────────────────────────────
    ("tla+/apalache", ("apalache", "tla+", "tlaplus"), "apalache-mc", "formal", 2),
    # ── failure corpus ───────────────────────────────────────────────────
    ("defihacklabs", ("defihacklabs", "defi-hack-labs"), None, "failure-corpus", 2),
    ("cve-datasets", ("cve", "nvd"), None, "failure-corpus", 2),
]

# Files that would count as a tool being configured.
CONFIG_HINTS = {
    "cargo-deny": ("deny.toml", ".cargo/deny.toml"),
    "cargo-audit": ("audit.toml", ".cargo/audit.toml"),
    "slither": ("slither.config.json", "slither.config.json5"),
    "echidna": ("echidna.yaml", "echidna.yml"),
    "foundry": ("foundry.toml",),
    "semgrep": (".semgrep.yml", ".semgrep.yaml", "semgrep.yml"),
    "gitleaks": (".gitleaks.toml", "gitleaks.toml"),
    "prometheus": ("prometheus.yml", "prometheus.yaml"),
}

# Not every named tool is a binary. `proptest` is a Rust crate you depend on,
# `revm` is a crate used as an oracle, and `malachite` is a project to read.
# Checking all of them with `which` reported every library as not installed.
TOOL_KIND = {
    "proptest": "crate", "proptest-state-machine": "crate", "loom": "crate",
    "shuttle": "crate", "revm": "crate", "reth": "crate", "litesvm": "crate",
    "mollusk": "crate", "trident": "crate", "jsonrpsee": "crate",
    "frame-benchmarking": "crate", "rust-libp2p": "crate", "rocksdb": "crate",
    "paritydb": "crate", "opentelemetry": "crate", "tempo": "crate",
    "malachite": "reference", "cometbft": "reference", "ethereum-hive": "reference",
    "hermes": "reference", "narwhal": "reference", "defihacklabs": "reference",
    "cve-datasets": "reference", "chaos-mesh": "reference",
}

# Where a strong signal may come from. A mention in prose is weaker than a
# dependency line or a workflow step, and the two must not collapse into one
# "WIRED" state: a doc that names `kani` is not kani being wired in.
DEP_FILES = ("Cargo.toml", "Cargo.lock", "package.json", "requirements.txt", "pyproject.toml")
STRONG_PREFIXES = (".github/workflows/", "scripts/", "tools/", "Makefile", "justfile")


def evidence_strength(rel):
    if rel in DEP_FILES:
        return "dependency"
    if rel.startswith(".github/workflows/"):
        return "workflow"
    if rel.startswith(("scripts/", "tools/")) or rel in ("Makefile", "justfile"):
        return "script"
    if rel.endswith((".rs", ".py", ".ts", ".js")):
        return "source"
    if rel.endswith((".dockerfile", ".sh")) or "Dockerfile" in rel or "compose" in rel:
        return "infrastructure"
    return "prose"

# Paths never worth searching for tool evidence.
#
# The `.wt-*` entries matter more than they look: this repo keeps several full
# worktree copies (`.wt-agent` alone is 3.1GB, `.wt-matrix` 1.2GB). Walking
# them made a scan that should take seconds run for many minutes and count the
# same evidence several times over. Anything hidden is skipped for the same
# reason, with the two directories that hold real evidence kept.
SKIP_DIRS = {".git", "target", "node_modules", ".next", "dist", "__pycache__",
             "tauri-vendor", ".venv", "vendor", "reports"}
KEEP_HIDDEN = {".github", ".cargo"}

# This file and its test name every tool in the catalogue. Left in the corpus
# they are "evidence" that every tool is wired and gated — the inventory
# measured itself, reporting all 66 tools as GATED and nothing else. A file
# that lists a tool is not a file that uses it.
SELF_FILES = {"tools/x3-forge/toolchain.py", "tools/x3-forge/test_toolchain.py"}


def _skip(part):
    if part in SKIP_DIRS:
        return True
    if part.startswith(".") and part not in KEEP_HIDDEN:
        return True
    return False
SEARCH_SUFFIXES = {".toml", ".yml", ".yaml", ".json", ".sh", ".md", ".rs", ".py", ".lock",
                   ".mk", ".cfg", ".txt", ".js", ".ts", ""}


def repo_files():
    """Every searchable file, once."""
    seen = []
    for path in ROOT.rglob("*"):
        if not path.is_file():
            continue
        if any(_skip(part) for part in path.relative_to(ROOT).parts):
            continue
        if path.suffix not in SEARCH_SUFFIXES and path.name not in ("Makefile", "Dockerfile"):
            continue
        seen.append(path)
    return seen


def load_corpus(files):
    """path -> text, read once (the repo is large; this is the only read pass)."""
    corpus = {}
    for path in files:
        try:
            if path.relative_to(ROOT).as_posix() in SELF_FILES:
                continue
            if path.stat().st_size > 2_000_000:
                continue
            corpus[path.relative_to(ROOT).as_posix()] = path.read_text(
                encoding="utf-8", errors="ignore")
        except OSError:
            continue
    return corpus


def mentions(corpus, terms, limit=12):
    """Where a tool is referenced, with the line, so the claim is checkable."""
    hits = []
    lowered = [term.lower() for term in terms if term]
    # Strong files first: a dependency line or a workflow step is the evidence
    # that matters, and finding it early means the report leads with it.
    order = {"dependency": 0, "workflow": 1, "script": 2, "infrastructure": 3,
             "source": 4, "prose": 5}
    for rel, text in sorted(corpus.items(), key=lambda item: order[evidence_strength(item[0])]):
        low = text.lower()
        for term in lowered:
            index = low.find(term)
            if index < 0:
                continue
            line = text.count("\n", 0, index) + 1
            hits.append({"file": rel, "line": line, "term": term,
                         "strength": evidence_strength(rel)})
            break
        if len(hits) >= limit:
            break
    return hits


def classify(name, terms, executable, corpus, config_hints):
    """The states a filesystem can actually establish, and nothing more."""
    kind = TOOL_KIND.get(name, "binary")
    installed_path = shutil.which(executable) if (executable and kind == "binary") else None
    found = mentions(corpus, terms)
    strengths = {hit["strength"] for hit in found}

    if kind == "crate":
        installed = "dependency" in strengths
        installed_path = next((hit["file"] for hit in found
                               if hit["strength"] == "dependency"), None)
    elif kind == "reference":
        installed = None  # nothing to install; it is something to read
    else:
        installed = installed_path is not None

    gated = sorted({hit["file"] for hit in found if hit["strength"] in ("workflow", "script")})
    # A configuration file counts when it is on disk, not when a doc names it.
    configured = [hint for hint in config_hints if (ROOT / hint).exists()]
    fuzz_dir = any(rel.endswith("/fuzz/Cargo.toml") for rel in corpus)

    return {
        "tool": name,
        "kind": kind,
        "available": bool(found) or bool(installed_path),
        "installed": installed,
        "installed_path": installed_path,
        "configured": bool(configured),
        "config_files": sorted(configured),
        "wired": bool(strengths & {"dependency", "workflow", "script", "infrastructure", "source"}),
        "referenced_only": bool(installed is False and strengths == {"prose"}),
        "evidence": found,
        "gated": bool(gated),
        "gate_files": gated,
        "evidence_strengths": sorted(strengths),
        "fuzz_crates_present": fuzz_dir if name.startswith(("cargo-fuzz", "afl", "honggfuzz")) else None,
        # The prompt is explicit that this is the part file presence cannot prove.
        "exercised": "not_determinable",
        "evidenced": "not_determinable",
        "repeatable": "not_determinable",
        "why_not_determinable": (
            "a mention proves wiring, not execution. Establishing these needs a recorded run "
            "bound to a commit with its configuration, result and artifacts."),
    }


def status_of(row):
    """§85's ladder, stopping where the evidence stops."""
    if row["gated"]:
        return "GATED"
    if row["wired"]:
        return "WIRED"
    if row["configured"]:
        return "CONFIGURED"
    if row["installed"]:
        return "INSTALLED_ONLY"
    if row["installed"] is False and row["referenced_only"]:
        return "REFERENCE_ONLY"
    if row["available"]:
        return "AVAILABLE"
    return "MISSING"


def build():
    corpus = load_corpus(repo_files())
    rows = []
    for name, terms, executable, family, group in CATALOG:
        row = classify(name, terms, executable, corpus,
                       CONFIG_HINTS.get(name, ("fuzz/Cargo.toml",) if family == "fuzzing" else ()))
        row.update({"family": family, "priority_group": group, "status": None})
        row["status"] = status_of(row)
        rows.append(row)
    return rows, corpus


def summary_rows(rows):
    order = {"GATED": 0, "WIRED": 1, "CONFIGURED": 2, "INSTALLED_ONLY": 3,
             "REFERENCE_ONLY": 4, "AVAILABLE": 5, "MISSING": 6}
    ranked = sorted(rows, key=lambda row: (order.get(row["status"], 9),
                                           row["priority_group"], row["tool"]))
    return ranked


def build_gap_markdown(rows, corpus):
    counts = {}
    for row in rows:
        counts[row["status"]] = counts.get(row["status"], 0) + 1
    by_group = {}
    for row in rows:
        by_group.setdefault(row["priority_group"], []).append(row)

    lines = [
        "# X3 external toolchain — gap analysis",
        "",
        "Generated by `tools/x3-forge/toolchain.py` (PHASE 0 of the master prompt).",
        "Every row carries the files it was judged from; nothing here is a guess.",
        "",
        "## Status counts",
        "",
    ]
    for status in ("GATED", "WIRED", "CONFIGURED", "INSTALLED_ONLY", "REFERENCE_ONLY",
                   "AVAILABLE", "MISSING"):
        lines.append(f"- {status}: {counts.get(status, 0)}")
    lines += [
        "",
        "## What these states mean, and what they do not",
        "",
        "`GATED` means a CI workflow or the local gate list references the tool. That is",
        "wiring. It is **not** proof the tool ran, and not proof its result was captured.",
        "The prompt's own rule is that an installed executable does not count as",
        "implemented, and this file will not pretend otherwise: EXERCISED, EVIDENCED and",
        "REPEATABLE are reported as `not_determinable` for every row, because",
        "establishing them needs a recorded run bound to a commit.",
        "",
        "## Priority group 1 — required before the seven-validator campaign (§77)",
        "",
        "| Tool | Status | Installed | Gated |",
        "| --- | --- | --- | --- |",
    ]
    for row in sorted(by_group.get(1, []), key=lambda r: r["tool"]):
        lines.append(f"| {row['tool']} | {row['status']} | "
                     f"{'yes' if row['installed'] else 'no'} | {'yes' if row['gated'] else 'no'} |")

    lines += ["", "## Priority group 2 (§78)", "",
              "| Tool | Status | Installed | Gated |", "| --- | --- | --- | --- |"]
    for row in sorted(by_group.get(2, []), key=lambda r: r["tool"]):
        lines.append(f"| {row['tool']} | {row['status']} | "
                     f"{'yes' if row['installed'] else 'no'} | {'yes' if row['gated'] else 'no'} |")

    lines += ["", "## Priority group 3 — reference only (§79)", "",
              "| Tool | Status | Installed |", "| --- | --- | --- |"]
    for row in sorted(by_group.get(3, []), key=lambda r: r["tool"]):
        lines.append(f"| {row['tool']} | {row['status']} | "
                     f"{'yes' if row['installed'] else 'no'} |")

    lines += ["", "## Top gaps by launch value", "",
              "Group 1 tools with the least evidence, worst first. A tool that is not",
              "installed and not gated is the largest gap; a tool that is gated needs only",
              "a recorded run to move up.", "",
              "| Tool | Status | Installed | Wiring evidence |", "| --- | --- | --- | --- |"]
    worst = [row for row in rows if row["priority_group"] == 1 and row["status"] != "GATED"]
    for row in sorted(worst, key=lambda r: (r["installed"], r["tool"]))[:15]:
        where = row["evidence"][0]["file"] if row["evidence"] else "—"
        lines.append(f"| {row['tool']} | {row['status']} | "
                     f"{'yes' if row['installed'] else 'no'} | {where} |")

    lines += ["", "## Not covered by this scan", "",
              "The catalogue is the master prompt's own tool lists. Tools the prompt names",
              "and this file omits are a bug in `toolchain.py`. The scan is lexical: a",
              "mention in a comment counts the same as a real invocation, which is exactly",
              "why a mention never promotes a tool past WIRED.", ""]
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser(description="X3 external toolchain inventory (PHASE 0).")
    sub = parser.add_subparsers(dest="command", required=True)
    inv = sub.add_parser("inventory")
    inv.add_argument("--out", default=None, help="directory for the two artifacts")
    inv.add_argument("--json", action="store_true")
    show = sub.add_parser("show")
    show.add_argument("tool")
    show.add_argument("--json", action="store_true")
    sub.add_parser("gaps").add_argument("--limit", type=int, default=20)
    sub.add_parser("priority").add_argument("--group", type=int, default=1)
    args = parser.parse_args(argv)

    rows, corpus = build()

    if args.command == "inventory":
        payload = {"root": str(ROOT), "tools": rows,
                   "counts": {status: sum(1 for r in rows if r["status"] == status)
                              for status in ("GATED", "WIRED", "CONFIGURED", "INSTALLED_ONLY",
                                             "REFERENCE_ONLY", "AVAILABLE", "MISSING")}}
        if args.out:
            out = Path(args.out)
            out.mkdir(parents=True, exist_ok=True)
            (out / "external-tool-inventory.json").write_text(
                json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
            (out / "external-tool-gap-analysis.md").write_text(
                build_gap_markdown(rows, corpus), encoding="utf-8")
            print(f"wrote {out}/external-tool-inventory.json")
            print(f"wrote {out}/external-tool-gap-analysis.md")
        if args.json or not args.out:
            print(json.dumps(payload["counts"], indent=2, sort_keys=True))
        return 0

    if args.command == "show":
        match = [row for row in rows if row["tool"] == args.tool]
        if not match:
            print(f"no catalogue entry for {args.tool!r}", file=sys.stderr)
            return 1
        row = match[0]
        if args.json:
            print(json.dumps(row, indent=2, sort_keys=True))
            return 0
        print(f"{row['tool']}  [{row['status']}]  family={row['family']} group={row['priority_group']}")
        print(f"  installed  {row['installed']}  {row['installed_path'] or ''}")
        print(f"  configured {row['configured']}  {', '.join(row['config_files'])}")
        print(f"  gated      {row['gated']}  {', '.join(row['gate_files'])}")
        print(f"  exercised  {row['exercised']} — {row['why_not_determinable']}")
        for hit in row["evidence"][:8]:
            print(f"  wiring     {hit['file']}:{hit['line']} ({hit['term']})")
        return 0

    if args.command == "gaps":
        ranked = summary_rows(rows)
        for row in ranked[:args.limit]:
            where = row["evidence"][0]["file"] if row["evidence"] else "—"
            print(f"{row['status']:14} g{row['priority_group']} {row['tool']:26} "
                  f"installed={'yes' if row['installed'] else 'no ':3} {where}")
        return 0

    print(f"priority group {args.group}")
    for row in sorted((r for r in rows if r["priority_group"] == args.group),
                      key=lambda r: r["tool"]):
        print(f"  {row['status']:14} {row['tool']:26} "
              f"installed={'yes' if row['installed'] else 'no'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
