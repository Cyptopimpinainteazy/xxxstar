#!/usr/bin/env python3
"""Convert captured PR-Agent dry-run text to conservative, advisory evidence.

This is a loss-aware adapter, NOT a structured review parser. Only obvious
path:line references become unverified candidates. A missing candidate never
means the review was complete or clean. Raw model output is not stored.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

SHA = re.compile(r"^[a-f0-9]{40}$")
LOC = re.compile(r"(?<![\w/.])([A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+\.[A-Za-z0-9]+):(\d{1,7})(?!\d)")


def bridge(pr_number, head_sha, artifact, exit_code):
    if isinstance(pr_number, bool) or not isinstance(pr_number, int) or pr_number < 1:
        raise ValueError("invalid PR number")
    if not isinstance(head_sha, str) or not SHA.fullmatch(head_sha):
        raise ValueError("invalid PR commit SHA")
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        raise ValueError("exit code must be integer")
    if not isinstance(artifact, str):
        raise ValueError("artifact must be text")
    found = []
    unique = set()
    for path, number in LOC.findall(artifact):
        line = int(number)
        if not line or ".." in Path(path).parts or (path, line) in unique:
            continue
        unique.add((path, line))
        found.append({"head_sha": head_sha, "path": path, "line": line,
                      "severity": "medium", "verification": "UNVERIFIED"})
    ok = (exit_code == 0 and bool(artifact.strip()) and
          "(the tool ran; nothing to publish)" not in artifact)
    return {
        "source": "PR_AGENT", "pr_number": pr_number, "head_sha": head_sha,
        "review_completed": bool(ok), "review_coverage_complete": False,
        "findings": found if ok else [],
        "artifact_sha256": hashlib.sha256(artifact.encode("utf-8")).hexdigest(),
        "candidate_extraction": "HEURISTIC_INCOMPLETE",
        "release_authorized": False,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pr", type=int, required=True)
    p.add_argument("--head-sha", required=True)
    p.add_argument("--artifact", type=Path, required=True)
    p.add_argument("--exit-code", type=int, required=True)
    p.add_argument("--output", type=Path)
    a = p.parse_args(argv)
    try:
        report = bridge(a.pr, a.head_sha, a.artifact.read_text(), a.exit_code)
        rendered = json.dumps(report, sort_keys=True, indent=2) + "\n"
        if a.output:
            a.output.parent.mkdir(parents=True, exist_ok=True)
            a.output.write_text(rendered)
        print(rendered, end="")
        return 0 if report["review_completed"] else 1
    except (OSError, ValueError) as exc:
        p.exit(2, f"PR-Agent evidence invalid: {type(exc).__name__}\n")


if __name__ == "__main__":
    raise SystemExit(main())
