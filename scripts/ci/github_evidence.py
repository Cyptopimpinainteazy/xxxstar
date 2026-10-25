#!/usr/bin/env python3
"""Read-only GitHub Actions evidence collector; no implicit release verdict.

Uses an immutable PR head, bounded GitHub REST pagination, and metadata only.
Does not download logs, publish comments, modify workflows, or identify required
checks. CI statuses are *observed*, not a branch-protection admission decision.
"""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import urllib.request

SHA = re.compile(r"^[a-f0-9]{40}$")
REPO = "Cyptopimpinainteazy/xxxstar"


def get_json(path, token):
    request = urllib.request.Request("https://api.github.com/" + path, headers={
        "Accept": "application/vnd.github+json",
        "Authorization": "Bearer " + token,
        "User-Agent": "x3-ci-evidence/0.1",
        "X-GitHub-Api-Version": "2022-11-28",
    })
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.load(response)


def gather(repo, number, token, fetch=get_json, max_pages=5):
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo):
        raise ValueError("invalid repository")
    if number < 1 or max_pages < 1:
        raise ValueError("invalid PR number or pagination")
    pr = fetch(f"repos/{repo}/pulls/{number}", token)
    sha = (pr.get("head") or {}).get("sha")
    if not isinstance(sha, str) or not SHA.fullmatch(sha):
        raise ValueError("PR head is not a full commit SHA")
    if (pr.get("base") or {}).get("repo", {}).get("full_name") not in (None, repo):
        raise ValueError("PR does not target requested repository")
    runs = []
    for page in range(1, max_pages + 1):
        batch = fetch(f"repos/{repo}/actions/runs?head_sha={sha}&per_page=100&page={page}", token).get("workflow_runs")
        if not isinstance(batch, list):
            raise ValueError("workflow run API shape invalid")
        runs.extend(batch)
        if len(batch) < 100:
            break
    else:
        raise ValueError("run pagination limit reached; refusing partial evidence")
    checks = []
    for run in runs:
        if run.get("head_sha") != sha:
            raise ValueError("stale run returned for PR head")
        if not isinstance(run.get("id"), int) or not isinstance(run.get("run_attempt"), int):
            raise ValueError("run missing identity or attempt")
        jobs = []
        for page in range(1, max_pages + 1):
            payload = fetch(f"repos/{repo}/actions/runs/{run['id']}/jobs?per_page=100&page={page}", token)
            batch = payload.get("jobs")
            if not isinstance(batch, list):
                raise ValueError("job API shape invalid")
            jobs.extend(batch)
            if len(batch) < 100:
                break
        else:
            raise ValueError("job pagination limit reached; refusing partial evidence")
        if not jobs:
            checks.append({"head_sha": sha, "name": run.get("name"), "workflow_run_id": run["id"],
                           "run_attempt": run["run_attempt"], "status": "unknown",
                           "conclusion": None, "evidence_state": "NO_JOBS"})
        for job in jobs:
            checks.append({"head_sha": sha, "name": job.get("name"),
                           "workflow_run_id": run["id"], "run_attempt": run["run_attempt"],
                           "job_id": job.get("id"), "status": job.get("status"),
                           "conclusion": job.get("conclusion"), "evidence_state": "OBSERVED"})
    return {
        "schema_version": 1, "source": "GITHUB_ACTIONS_METADATA",
        "pr": {"number": number, "head_sha": sha},
        "checks": checks, "findings": [],
        "required_checks_verified": False,
        "pr_agent_output_verified": False,
        "live_integration_verified": False,
        "production_ready": False,
        "release_authorized": False,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", default=REPO)
    p.add_argument("--pr", type=int, required=True)
    p.add_argument("--output", type=Path)
    args = p.parse_args(argv)
    token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
    if not token:
        p.exit(2, "GH_TOKEN or GITHUB_TOKEN required for read-only metadata query\n")
    try:
        packet = gather(args.repo, args.pr, token)
        data = json.dumps(packet, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(data)
        print(data, end="")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as exc:
        p.exit(2, f"GitHub evidence unavailable: {type(exc).__name__}\n")


if __name__ == "__main__":
    raise SystemExit(main())
