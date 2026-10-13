#!/usr/bin/env python3
"""Read-only inventory of the X3-related GitHub repositories (Resurrection mission, Phase A).

Uses the authenticated `gh` CLI for repository/PR metadata and the bare mirrors under
~/x3-recovery/mirrors (made with `git clone --mirror`) for branch ancestry, so nothing
in any working checkout is touched. Writes, under reports/recovery/:

  remote-repository-inventory.json   every repository, branch and PR record
  branch-inventory.md                per repository: branches not merged into the default branch
  pull-request-inventory.md          open PRs, and closed-unmerged PRs (possible abandoned work)
"""
from __future__ import annotations

import datetime as dt
import json
import re
import subprocess
from pathlib import Path

OWNER = "Cyptopimpinainteazy"
MIRRORS = Path.home() / "x3-recovery" / "mirrors"
CANONICAL = Path.home() / "Desktop" / "xxxstar-main"  # holds origin/* for xxxstar
PATTERN = re.compile(r"x3|atlas|star|swarm|atomic|gpu|agent|reaper|court|universe|explorer|wallet|router|compiler", re.I)
OUT = Path("reports/recovery")


def gh(*args: str) -> str:
    return subprocess.run(["gh", *args], capture_output=True, text=True, check=True, timeout=600).stdout


def git(repo: Path, *args: str) -> str:
    out = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, timeout=600)
    return out.stdout.strip() if out.returncode == 0 else ""


def branch_rows(repo: Path, prefix: str, default: str) -> list[dict]:
    base = f"{prefix}{default}"
    rows = []
    fmt = "%(refname:short)\t%(objectname:short)\t%(committerdate:short)\t%(subject)"
    for line in git(repo, "for-each-ref", f"--format={fmt}", f"refs/{'remotes/origin' if prefix else 'heads'}").splitlines():
        name, sha, date, subject = (line.split("\t") + ["", "", ""])[:4]
        short = name.removeprefix("origin/")
        if short in ("HEAD", default):
            continue
        ahead = git(repo, "rev-list", "--count", f"{base}..{name}") or "0"
        rows.append({"branch": short, "sha": sha, "date": date, "subject": subject[:120],
                     "ahead_of_default": int(ahead)})
    return rows


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    repos = json.loads(gh("repo", "list", OWNER, "--limit", "300", "--json",
                          "name,isPrivate,isFork,isArchived,pushedAt,diskUsage,primaryLanguage,defaultBranchRef,description"))
    x3 = [r for r in repos if PATTERN.search(r["name"]) or PATTERN.search(r.get("description") or "")]
    records = []
    for r in sorted(x3, key=lambda r: r["pushedAt"], reverse=True):
        name = r["name"]
        default = (r.get("defaultBranchRef") or {}).get("name")
        mirror = MIRRORS / f"{name}.git"
        if name == "xxxstar":
            src, prefix = CANONICAL, "origin/"
        elif mirror.is_dir():
            src, prefix = mirror, ""
        else:
            src, prefix = None, ""
        branches = branch_rows(src, prefix, default) if (src and default) else []
        tags = git(src, "tag").splitlines() if src else []
        prs = []
        if not r["isArchived"]:
            try:
                prs = json.loads(gh("pr", "list", "-R", f"{OWNER}/{name}", "--state", "all", "--limit", "1000",
                                    "--json", "number,title,state,headRefName,createdAt,mergedAt,closedAt,isDraft"))
            except subprocess.CalledProcessError:
                prs = []
        records.append({
            "name": name, "private": r["isPrivate"], "fork": r["isFork"], "archived": r["isArchived"],
            "pushed": r["pushedAt"], "disk_kb": r["diskUsage"],
            "language": (r.get("primaryLanguage") or {}).get("name"), "default_branch": default,
            "description": r.get("description"), "mirrored": src is not None,
            "branches": branches, "tags": tags, "pull_requests": prs,
        })
    stamp = dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")
    (OUT / "remote-repository-inventory.json").write_text(
        json.dumps({"collected": stamp, "owner": OWNER, "repositories": records}, indent=2) + "\n")

    b = [f"# Branch inventory ({stamp})", "",
         "Branches that carry commits their repository's default branch does not have.",
         "`ahead` counts commits, not content: a branch whose work landed by squash still shows",
         "commits ahead (see the 2026-09-21 reconciliation for the content-level method).", ""]
    for rec in records:
        live = [x for x in rec["branches"] if x["ahead_of_default"] > 0]
        b.append(f"## {rec['name']} — default `{rec['default_branch']}`, {len(rec['branches'])} other branches, "
                 f"{len(live)} with commits ahead, {len(rec['tags'])} tags")
        if not rec["mirrored"]:
            b.append("\n(not mirrored locally; branch ancestry not computed)\n")
            continue
        if live:
            b += ["", "| Branch | Ahead | Last commit | Subject |", "|---|---|---|---|"]
            for x in sorted(live, key=lambda x: x["date"], reverse=True)[:60]:
                b.append(f"| `{x['branch']}` | {x['ahead_of_default']} | {x['date']} | {x['subject'].replace('|', '/')} |")
            if len(live) > 60:
                b.append(f"| … {len(live) - 60} more in the JSON | | | |")
        b.append("")
    (OUT / "branch-inventory.md").write_text("\n".join(b) + "\n")

    p = [f"# Pull request inventory ({stamp})", ""]
    for rec in records:
        prs = rec["pull_requests"]
        if not prs:
            continue
        open_ = [x for x in prs if x["state"] == "OPEN"]
        abandoned = [x for x in prs if x["state"] == "CLOSED" and not x["mergedAt"]]
        merged = [x for x in prs if x["mergedAt"]]
        p.append(f"## {rec['name']}: {len(open_)} open, {len(merged)} merged, {len(abandoned)} closed without merge")
        for title, rows in (("Open", open_), ("Closed without merge (possible abandoned work)", abandoned)):
            if rows:
                p += ["", f"### {title}", "", "| # | Branch | Created | Title |", "|---|---|---|---|"]
                for x in sorted(rows, key=lambda x: x["number"], reverse=True)[:80]:
                    p.append(f"| {x['number']} | `{x['headRefName']}` | {x['createdAt'][:10]} | {x['title'].replace('|', '/')[:100]} |")
        p.append("")
    (OUT / "pull-request-inventory.md").write_text("\n".join(p) + "\n")
    print(f"{len(records)} X3-related repositories -> {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
