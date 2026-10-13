#!/usr/bin/env python3
"""Read-only inventory of local git repositories and worktrees (Resurrection mission, Phase A).

Writes reports/recovery/local-repository-inventory.{json,md}. Nothing in the
inventoried repositories is modified: every command is a read (`git status`
uses --no-optional-locks so it does not even refresh the index).

    python3 scripts/recovery/local_repo_inventory.py [--roots DIR ...] [--max-depth N]
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import os
import subprocess
from pathlib import Path

HOME = Path.home()
DEFAULT_ROOTS = [HOME, HOME / "Desktop", HOME / "Documents", HOME / ".openclaw" / "workspace",
                 HOME / "x3-infrastructure"]
# Never descend into these: caches, toolchains, build output, other people's data.
SKIP = {"node_modules", "target", ".cargo", ".rustup", ".cache", ".npm", ".nvm", ".local",
        ".mozilla", ".config", "snap", ".git", "go", ".venv", "venv", "__pycache__", "x3-recovery"}
BUILD_FILES = {"Cargo.toml": "cargo", "package.json": "npm", "pyproject.toml": "python",
               "setup.py": "python", "go.mod": "go", "Makefile": "make", "foundry.toml": "foundry",
               "hardhat.config.ts": "hardhat", "Anchor.toml": "anchor", "Dockerfile": "docker"}
LANG = {".rs": "Rust", ".ts": "TypeScript", ".tsx": "TypeScript", ".js": "JavaScript",
        ".jsx": "JavaScript", ".py": "Python", ".sol": "Solidity", ".go": "Go", ".x3": "X3",
        ".sh": "Shell", ".md": "Markdown", ".toml": "TOML", ".yml": "YAML", ".yaml": "YAML"}


def git(repo: Path, *args: str, timeout: int = 60) -> str:
    try:
        out = subprocess.run(["git", "--no-optional-locks", "-C", str(repo), *args],
                             capture_output=True, text=True, timeout=timeout)
        return out.stdout.strip() if out.returncode == 0 else ""
    except subprocess.TimeoutExpired:
        return ""


def find_repos(roots: list[Path], max_depth: int) -> list[Path]:
    found: set[Path] = set()
    for root in roots:
        if not root.is_dir():
            continue
        base = len(root.parts)
        for dirpath, dirnames, filenames in os.walk(root):
            here = Path(dirpath)
            if ".git" in dirnames or ".git" in filenames:
                found.add(here.resolve())
            if len(here.parts) - base >= max_depth:
                dirnames[:] = []
                continue
            dirnames[:] = [d for d in dirnames if d not in SKIP and not d.startswith(".wt-")
                           and not (d.startswith(".") and d not in (".openclaw",))]
    return sorted(found)


def describe(repo: Path) -> dict:
    is_linked = (repo / ".git").is_file()
    remotes = {}
    for line in git(repo, "remote", "-v").splitlines():
        name, url, *_ = line.split()
        remotes[name] = url
    upstream = git(repo, "rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}")
    ahead = behind = None
    if upstream:
        counts = git(repo, "rev-list", "--left-right", "--count", f"HEAD...{upstream}").split()
        if len(counts) == 2:
            ahead, behind = int(counts[0]), int(counts[1])
    porcelain = git(repo, "status", "--porcelain", "--untracked-files=normal", timeout=120).splitlines()
    files = git(repo, "ls-files", timeout=120).splitlines()
    langs = collections.Counter(LANG[Path(f).suffix] for f in files if Path(f).suffix in LANG)
    top = set(git(repo, "ls-files", "--", ":(glob)*").splitlines())
    build = sorted({v for k, v in BUILD_FILES.items() if k in top})
    default = git(repo, "symbolic-ref", "--short", "refs/remotes/origin/HEAD").removeprefix("origin/")
    return {
        "path": str(repo),
        "name": repo.name,
        "kind": "linked-worktree" if is_linked else "repository",
        "common_dir": git(repo, "rev-parse", "--path-format=absolute", "--git-common-dir"),
        "remotes": remotes,
        "default_branch": default or None,
        "current_branch": git(repo, "branch", "--show-current") or "(detached)",
        "head": git(repo, "rev-parse", "HEAD") or None,
        "last_commit": git(repo, "log", "-1", "--format=%cI %s")[:160] or None,
        "status": {"modified": sum(1 for l in porcelain if not l.startswith("??")),
                   "untracked": sum(1 for l in porcelain if l.startswith("??"))},
        "upstream": upstream or None, "ahead": ahead, "behind": behind,
        "local_branches": len(git(repo, "for-each-ref", "--format=x", "refs/heads").splitlines()),
        "remote_branches": len(git(repo, "for-each-ref", "--format=x", "refs/remotes").splitlines()),
        "tags": len(git(repo, "tag").splitlines()),
        "stashes": len(git(repo, "stash", "list").splitlines()) if not is_linked else None,
        "worktrees": len([l for l in git(repo, "worktree", "list", "--porcelain").splitlines()
                          if l.startswith("worktree ")]) if not is_linked else None,
        "tracked_files": len(files),
        "languages": dict(langs.most_common(6)),
        "build_systems": build,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--roots", nargs="*", type=Path, default=DEFAULT_ROOTS)
    ap.add_argument("--max-depth", type=int, default=4)
    ap.add_argument("--out", type=Path, default=Path("reports/recovery"))
    args = ap.parse_args()
    repos = find_repos(args.roots, args.max_depth)
    rows = [describe(r) for r in repos]
    args.out.mkdir(parents=True, exist_ok=True)
    stamp = dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")
    payload = {"collected": stamp, "host": os.uname().nodename, "roots": [str(r) for r in args.roots],
               "max_depth": args.max_depth, "repositories": rows}
    (args.out / "local-repository-inventory.json").write_text(json.dumps(payload, indent=2) + "\n")
    lines = [f"# Local repository inventory ({os.uname().nodename})", "",
             f"Collected {stamp}; roots {', '.join(str(r) for r in args.roots)}, depth {args.max_depth}.",
             "Read-only scan: nothing in these repositories was modified.", "",
             "| Path | Kind | Branch | HEAD | Modified/untracked | Ahead/behind | Remotes | Build | Languages |",
             "|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        rem = ", ".join(sorted({u.rsplit('/', 1)[-1].removesuffix('.git') for u in r["remotes"].values()})) or "-"
        ab = "-" if r["ahead"] is None else f"{r['ahead']}/{r['behind']}"
        lang = ", ".join(r["languages"]) or "-"
        lines.append(f"| `{r['path']}` | {r['kind']} | {r['current_branch']} | {(r['head'] or '-')[:9]} | "
                     f"{r['status']['modified']}/{r['status']['untracked']} | {ab} | {rem} | "
                     f"{', '.join(r['build_systems']) or '-'} | {lang} |")
    (args.out / "local-repository-inventory.md").write_text("\n".join(lines) + "\n")
    print(f"{len(rows)} repositories/worktrees -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
