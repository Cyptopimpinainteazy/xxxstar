#!/usr/bin/env python3
"""Generate docs/REPO_MAP.md: a map of this repository built from code, not docs.

Sources, all read from the working tree:
  * `cargo metadata --no-deps` for workspace crates, their targets and internal deps
  * `git ls-files` for what is tracked (so build output and node_modules never appear)
  * the `construct_runtime!` blocks in runtime/src/lib.rs for which pallets ship
  * tracked package.json / pyproject.toml files for the JS and Python projects

Usage:
  scripts/gen-repo-map.py           # rewrite docs/REPO_MAP.md
  scripts/gen-repo-map.py --check   # exit 1 if docs/REPO_MAP.md is out of date
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "REPO_MAP.md"
RUNTIME_LIB = ROOT / "runtime" / "src" / "lib.rs"

# Vendored or tool-owned trees: counted in the directory table, not mapped in detail.
VENDORED_PREFIXES = ("patches/", ".toolchain/", "forge-std/")

VENDORED_SEGMENT = re.compile(r"(^|/)[\w.-]*vendor/")

TEST_ATTR = re.compile(r"#\[(?:tokio::)?test\b")
CALL_INDEX = re.compile(r"#\[pallet::call_index\(")
RUNTIME_ENTRY = re.compile(r"^\s*([A-Z]\w*)\s*:\s*([a-z_][\w:]*?)(?:::<[^>]*>)?\s*,", re.M)


def run(*cmd: str) -> str:
    return subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True).stdout


def tracked_files() -> list[str]:
    return [f for f in run("git", "ls-files", "-z").split("\0") if f]


def first_doc_line(path: Path) -> str:
    """First line of a crate's `//!` doc, without heading marks."""
    try:
        for line in path.read_text(errors="replace").splitlines()[:80]:
            s = line.strip()
            if s.startswith("//!"):
                text = s[3:].strip().lstrip("#").strip()
                if text:
                    return text
    except OSError:
        pass
    return ""


def cell(text: str, limit: int = 110) -> str:
    text = " ".join(text.split()).replace("|", "\\|")
    return text if len(text) <= limit else text[: limit - 1].rstrip() + "…"


def owner_of(path: str, dirs: list[str]) -> str | None:
    """The deepest package directory containing `path` (dirs sorted longest first)."""
    for d in dirs:
        if path.startswith(d + "/"):
            return d
    return None


# ── Runtime composition ──────────────────────────────────────────────────────


def runtime_variants() -> list[tuple[str, dict[str, str]]]:
    """Each construct_runtime! block as (cfg condition, {PalletName: crate path})."""
    if not RUNTIME_LIB.exists():
        return []
    src = RUNTIME_LIB.read_text()
    variants = []
    for m in re.finditer(r"^construct_runtime!\s*\(", src, re.M):
        head = src[: m.start()]
        cfg_start = head.rfind("#[cfg(")
        cfg = re.sub(r"\s+", " ", head[cfg_start:].strip()) if cfg_start != -1 else "(always)"
        cfg = cfg.removeprefix("#[cfg(").removesuffix(")]").strip()
        body_end = src.find("\n);", m.end())
        body = src[m.end() : body_end]
        entries = dict(RUNTIME_ENTRY.findall(body))
        variants.append((cfg, entries))
    return variants


def short_cfg(cfg: str) -> str:
    feats = re.findall(r'(not\()?feature = "([\w-]+)"', cfg)
    return " ".join(("!" if neg else "") + f for neg, f in feats) or cfg


# ── Map ──────────────────────────────────────────────────────────────────────


def build() -> str:
    files = tracked_files()
    meta = json.loads(run("cargo", "metadata", "--no-deps", "--format-version", "1"))
    packages = sorted(meta["packages"], key=lambda p: p["manifest_path"])

    pkg_dir = {p["name"]: os.path.relpath(Path(p["manifest_path"]).parent, ROOT) for p in packages}
    dirs = sorted(pkg_dir.values(), key=len, reverse=True)
    dir_to_pkg = {d: n for n, d in pkg_dir.items()}

    rs_lines: Counter[str] = Counter()
    tests: Counter[str] = Counter()
    calls: Counter[str] = Counter()
    for f in files:
        if not f.endswith(".rs"):
            continue
        d = owner_of(f, dirs)
        if d is None:
            continue
        try:
            text = (ROOT / f).read_text(errors="replace")
        except OSError:
            continue
        rs_lines[d] += text.count("\n")
        tests[d] += len(TEST_ATTR.findall(text))
        calls[d] += len(CALL_INDEX.findall(text))

    used_by: Counter[str] = Counter()
    for p in packages:
        for dep in p["dependencies"]:
            if dep.get("path") and dep["name"] in pkg_dir and dep["kind"] in (None, "build"):
                used_by[dep["name"]] += 1

    variants = runtime_variants()
    # construct_runtime! names a dependency by the runtime's own alias for it, which
    # `package = "..."` renames in runtime/Cargo.toml can make differ from the package.
    alias_to_pkg: dict[str, str] = {}
    for p in packages:
        if pkg_dir[p["name"]] == "runtime":
            for dep in p["dependencies"]:
                if dep.get("rename"):
                    alias_to_pkg[dep["rename"].replace("-", "_")] = dep["name"]
    pkg_variants: dict[str, list[int]] = defaultdict(list)
    for i, (_, entries) in enumerate(variants):
        for crate in {path.split("::")[0] for path in entries.values()}:
            pkg = alias_to_pkg.get(crate, crate)
            pkg_variants[pkg.replace("-", "_")].append(i)

    out: list[str] = []
    w = out.append
    w("# Repository map")
    w("")
    w("<!-- Generated by scripts/gen-repo-map.py. Do not edit by hand; rerun the script. -->")
    w("")
    w("Built from code: `cargo metadata`, `git ls-files`, the `construct_runtime!` blocks in")
    w("`runtime/src/lib.rs`, and tracked `package.json` / `pyproject.toml` files. Descriptions")
    w("come from each crate's `Cargo.toml` `description` or the first `//!` line of its entry")
    w("file, so they are only as accurate as those. Treat other docs in this repo as")
    w("possibly stale; this file is regenerated with `python3 scripts/gen-repo-map.py`.")
    w("")
    w("Columns: **lines** = tracked `.rs` lines in the crate, **tests** = `#[test]` functions,")
    w("**calls** = pallet extrinsics (`#[pallet::call_index]`), **used by** = workspace crates")
    w("depending on it, **runtime** = how many `construct_runtime!` variants include it.")
    w("")

    # Top-level directories
    top = Counter(f.split("/")[0] if "/" in f else "(root files)" for f in files)
    members_per_top = Counter(d.split("/")[0] for d in pkg_dir.values())
    js_files = [f for f in files if f.endswith("package.json") and "node_modules/" not in f]
    js_per_top = Counter(f.split("/")[0] for f in js_files if "/" in f)
    w("## Top-level directories")
    w("")
    w(f"{len(files)} tracked files. Directories with fewer than 20 files are summed in the last row.")
    w("")
    w("| directory | files | Rust crates | JS packages |")
    w("|---|---:|---:|---:|")
    small = 0
    for name, n in top.most_common():
        if n < 20:
            small += n
            continue
        w(f"| `{name}` | {n} | {members_per_top.get(name, '')} | {js_per_top.get(name, '')} |")
    w(f"| (others) | {small} | | |")
    w("")

    # Runtime
    if variants:
        w("## Runtime composition")
        w("")
        w("`runtime/src/lib.rs` defines one `construct_runtime!` per feature combination:")
        w("")
        for i, (cfg, entries) in enumerate(variants):
            w(f"- **V{i + 1}** `{short_cfg(cfg)}`: {len(entries)} pallets")
        w("")
        all_names: dict[str, str] = {}
        for _, entries in variants:
            all_names.update(entries)
        w("| pallet | crate | variants |")
        w("|---|---|---|")
        def present(n: str) -> int:
            return sum(n in e for _, e in variants)

        for name in sorted(all_names, key=lambda n: (-present(n), n)):
            crate = all_names[name]
            vs = [f"V{i + 1}" for i, (_, e) in enumerate(variants) if name in e]
            label = "all" if len(vs) == len(variants) else " ".join(vs)
            w(f"| {name} | `{crate}` | {label} |")
        w("")

    # Workspace crates, grouped by directory
    groups: dict[str, list[dict]] = defaultdict(list)
    for p in packages:
        groups[pkg_dir[p["name"]].split("/")[0]].append(p)
    w("## Rust workspace crates")
    w("")
    w(f"{len(packages)} members of the root `Cargo.toml` workspace.")
    for group in sorted(groups, key=lambda g: (-len(groups[g]), g)):
        w("")
        w(f"### `{group}/` ({len(groups[group])})")
        w("")
        is_pallets = group == "pallets"
        if is_pallets:
            w("| crate | path | lines | tests | calls | runtime | description |")
            w("|---|---|---:|---:|---:|---|---|")
        else:
            w("| crate | path | kind | lines | tests | used by | description |")
            w("|---|---|---|---:|---:|---:|---|")
        for p in sorted(groups[group], key=lambda p: p["name"]):
            d = pkg_dir[p["name"]]
            entry = next((t for t in p["targets"] if set(t["kind"]) & {"lib", "proc-macro"}), None)
            entry = entry or next((t for t in p["targets"] if "bin" in t["kind"]), None)
            desc = p.get("description") or (first_doc_line(Path(entry["src_path"])) if entry else "")
            kinds = sorted({k for t in p["targets"] for k in t["kind"]} & {"lib", "bin", "proc-macro"})
            if is_pallets:
                lib_target = next((t for t in p["targets"] if "lib" in t["kind"]), None)
                lib = (lib_target["name"] if lib_target else p["name"]).replace("-", "_")
                nv = len(pkg_variants.get(p["name"].replace("-", "_")) or pkg_variants.get(lib, []))
                rt = f"{nv}/{len(variants)}" if nv else "—"
                w(
                    f"| `{p['name']}` | `{d}` | {rs_lines[d]} | {tests[d]} | {calls[d]} | {rt} "
                    f"| {cell(desc)} |"
                )
            else:
                w(
                    f"| `{p['name']}` | `{d}` | {'+'.join(kinds)} | {rs_lines[d]} | {tests[d]} "
                    f"| {used_by[p['name']]} | {cell(desc)} |"
                )
    w("")

    # Cargo manifests outside the workspace
    manifests = [f for f in files if f.endswith("Cargo.toml") and not f.startswith(VENDORED_PREFIXES)]
    vendored = [f for f in manifests if VENDORED_SEGMENT.search(f)]
    manifests = [f for f in manifests if not VENDORED_SEGMENT.search(f)]
    outside = []
    for f in manifests:
        d = os.path.dirname(f) or "."
        if d in dir_to_pkg or d == ".":
            continue
        try:
            data = tomllib.loads((ROOT / f).read_text())
        except (OSError, tomllib.TOMLDecodeError):
            data = {}
        name = data.get("package", {}).get("name", "")
        is_ws = "workspace" in data and "members" in data.get("workspace", {})
        outside.append((d, name, is_ws))
    w("## Rust crates outside the root workspace")
    w("")
    w("These build on their own (or not at all); `cargo test --workspace` at the root does")
    w(f"not cover them. {len(outside)} manifests, excluding vendored `patches/`, `.toolchain/`,")
    w(f"`forge-std/` and {len(vendored)} third-party crates under `*vendor/` directories.")
    w("**workspace** marks a nested workspace root.")
    w("")
    w("| path | package | workspace |")
    w("|---|---|---|")
    for d, name, is_ws in sorted(outside):
        w(f"| `{d}` | {name or '—'} | {'yes' if is_ws else ''} |")
    w("")

    # JS / TS packages
    w("## JavaScript / TypeScript packages")
    w("")
    w("| path | name | scripts | description |")
    w("|---|---|---|---|")
    for f in sorted(js_files):
        try:
            pj = json.loads((ROOT / f).read_text())
        except (OSError, json.JSONDecodeError):
            continue
        scripts = sorted(set(pj.get("scripts", {})) & {"build", "dev", "start", "test", "lint", "typecheck"})
        d = os.path.dirname(f) or "."
        w(f"| `{d}` | {pj.get('name', '—')} | {' '.join(scripts)} | {cell(pj.get('description', ''))} |")
    w("")

    # Python projects
    py = [f for f in files if f.endswith("pyproject.toml") and not f.startswith(VENDORED_PREFIXES)]
    if py:
        w("## Python projects")
        w("")
        w("| path | name | description |")
        w("|---|---|---|")
        for f in sorted(py):
            try:
                proj = tomllib.loads((ROOT / f).read_text()).get("project", {})
            except (OSError, tomllib.TOMLDecodeError):
                proj = {}
            d = os.path.dirname(f) or "."
            w(f"| `{d}` | {proj.get('name', '—')} | {cell(proj.get('description', ''))} |")
        w("")

    return "\n".join(out)


def main() -> int:
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} is out of date; run python3 scripts/gen-repo-map.py", file=sys.stderr)
            return 1
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"wrote {OUT.relative_to(ROOT)} ({text.count(chr(10)) + 1} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
