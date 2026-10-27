#!/usr/bin/env python3
"""Read-only X3 CI queue assessment. Neither this code nor its workflow cancels runs.

Pull-request and workflow-run metadata can be incomplete. Classification is a
suggestion for human inspection, not an authorization to cancel or retry work.
"""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

SHA = re.compile(r"^[0-9a-f]{40}$")
REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
STATUSES = {"queued", "in_progress"}


def _head_map(pulls):
    if not isinstance(pulls, list):
        raise ValueError("pulls input must be complete list")
    heads = {}
    for pr in pulls:
        if not isinstance(pr, dict):
            raise ValueError("malformed pull request")
        number, head = pr.get("number"), pr.get("head")
        sha = head.get("sha") if isinstance(head, dict) else None
        if type(number) is not int or number < 1 or number in heads or not isinstance(sha, str) or not SHA.fullmatch(sha):
            raise ValueError("invalid or duplicate open pull request identity")
        heads[number] = sha
    return heads


def _linked_pr_numbers(run):
    links = run.get("pull_requests")
    if not isinstance(links, list):
        return []
    ids = [item.get("number") for item in links if isinstance(item, dict)]
    return ids if len(ids) == len(links) and all(type(n) is int and n > 0 for n in ids) else []


def classify(run, heads, duplicate_ids=()):
    """Uncertain ancestry is UNKNOWN, not an invitation to cancel the run."""
    run_id = run["id"]
    linked = _linked_pr_numbers(run)
    if run["status"] == "in_progress":
        verdict = "KEEP_IN_PROGRESS"
    elif len(linked) != 1 or linked[0] not in heads:
        verdict = "UNKNOWN_IDENTITY"
    elif run["head_sha"] != heads[linked[0]]:
        verdict = "REVIEW_OUTDATED_HEAD"
    elif run_id in duplicate_ids:
        verdict = "REVIEW_DUPLICATE"
    else:
        verdict = "KEEP_CURRENT_HEAD"
    return {
        "id": run_id,
        "status": run["status"],
        "head_sha": run["head_sha"],
        "workflow_path": run["path"],
        "pr_number": linked[0] if len(linked) == 1 else None,
        "classification": verdict,
        "action": "NONE",
        "cancel_authorized": False,
    }


def audit(pulls, runs, source="fixture"):
    heads = _head_map(pulls)
    if not isinstance(runs, list):
        raise ValueError("runs input must be a list")
    seen = set()
    for run in runs:
        if not isinstance(run, dict):
            raise ValueError("invalid workflow run")
        run_id, sha, path, status = (run.get(k) for k in ("id", "head_sha", "path", "status"))
        if (type(run_id) is not int or run_id < 1 or run_id in seen or
                not isinstance(sha, str) or not SHA.fullmatch(sha) or
                not isinstance(path, str) or not path or status not in STATUSES):
            raise ValueError("invalid, duplicate or out-of-scope workflow run")
        seen.add(run_id)
    # A newer queued run for the exact same PR/workflow/head supersedes the
    # older *queued* run for inspection purposes only. No cancellation occurs.
    newest = {}
    duplicates = set()
    for run in sorted(runs, key=lambda value: value["id"], reverse=True):
        linked = _linked_pr_numbers(run)
        if (run["status"] != "queued" or len(linked) != 1 or
                linked[0] not in heads or run["head_sha"] != heads[linked[0]]):
            continue
        identity = (linked[0], run["head_sha"], run["path"])
        if identity in newest:
            duplicates.add(run["id"])
        else:
            newest[identity] = run["id"]
    items = [classify(run, heads, duplicates) for run in sorted(runs, key=lambda value: value["id"], reverse=True)]
    counts = {name: sum(item["classification"] == name for item in items)
              for name in ("KEEP_CURRENT_HEAD", "KEEP_IN_PROGRESS", "REVIEW_OUTDATED_HEAD",
                           "REVIEW_DUPLICATE", "UNKNOWN_IDENTITY")}
    return {
        "schema_version": 1,
        "source": source,
        "scope": "queued_and_in_progress_pull_request_run_metadata_only",
        "open_pr_count": len(heads),
        "pending_run_count": len(items),
        "counts": counts,
        "runs": items,
        "live_registry_verified": False,
        "cancel_authorized": False,
        "retry_authorized": False,
        "release_authorized": False,
        "note": "Read-only advisory: no cancellations, retries, runner restarts, or release decisions.",
    }


def fetch_pages(repo, token, endpoint, field, limit=10):
    """Bounded GitHub GET pagination; partial lists must error, not look empty."""
    result = []
    for page in range(1, limit + 1):
        join = "&" if "?" in endpoint else "?"
        url = f"https://api.github.com/repos/{repo}/{endpoint}{join}per_page=100&page={page}"
        request = urllib.request.Request(url, headers={
            "Authorization": "Bearer " + token,
            "Accept": "application/vnd.github+json",
            "User-Agent": "x3-ci-queue-inspector/1",
            "X-GitHub-Api-Version": "2022-11-28",
        }, method="GET")
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = json.load(response)
        batch = payload.get(field) if isinstance(payload, dict) else payload
        if not isinstance(batch, list):
            raise ValueError("GitHub response lacks expected collection")
        result.extend(batch)
        if len(batch) < 100:
            return result
    raise ValueError("GitHub pagination limit reached; partial queue evidence refused")


def fetch_live(repo, token):
    if not isinstance(repo, str) or not REPO.fullmatch(repo) or not token:
        raise ValueError("valid GITHUB_REPOSITORY and read-only GitHub token required")
    pulls = fetch_pages(repo, token, "pulls?state=open", "pulls")
    # GitHub's PR list endpoint returns a JSON array; a single pagination
    # helper deliberately accepts this shape without assuming the API filtered
    # fork PRs or resolved each run's ancestry.
    runs = []
    for status in ("queued", "in_progress"):
        runs.extend(fetch_pages(repo, token, f"actions/runs?event=pull_request&status={status}", "workflow_runs"))
    return pulls, runs


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--live", action="store_true", help="read-only GitHub GET requests")
    mode.add_argument("--fixture", type=Path, help="offline input with pulls and runs")
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", "Cyptopimpinainteazy/xxxstar"))
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.live:
            pulls, runs = fetch_live(args.repo, os.getenv("GH_TOKEN") or os.getenv("GITHUB_TOKEN"))
        else:
            data = json.loads(args.fixture.read_text(encoding="utf-8"))
            pulls, runs = data["pulls"], data["runs"]
        result = audit(pulls, runs, source="github_get" if args.live else "fixture")
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0
    except (OSError, ValueError, TypeError, KeyError, urllib.error.URLError) as exc:
        print(f"queue inspection incomplete: {type(exc).__name__}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
