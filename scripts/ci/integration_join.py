#!/usr/bin/env python3
"""Join exact-head GitHub Actions metadata and PR-Agent review evidence.

Advisory consistency check only. No API writes, review generation, job execution,
required-check discovery, production approval, or secret-bearing log output.
"""
import argparse
import json
from pathlib import Path
import re

from integration_e2e import assess

SHA = re.compile(r"^[0-9a-f]{40}$")


def join(github, review):
    problems = []
    if not isinstance(github, dict) or github.get("source") != "GITHUB_ACTIONS_METADATA":
        problems.append("untrusted GitHub evidence source")
        github = {}
    pr = github.get("pr")
    pr = pr if isinstance(pr, dict) else {}
    head = pr.get("head_sha")
    if not isinstance(head, str) or not SHA.fullmatch(head):
        problems.append("invalid PR head identity")
    if not isinstance(review, dict) or review.get("source") != "PR_AGENT":
        problems.append("review source missing or untrusted")
        review = {}
    if review.get("head_sha") != head:
        problems.append("review belongs to another head")
    if review.get("review_completed") is not True:
        problems.append("review execution not verified")
    if review.get("review_coverage_complete") is not True:
        problems.append("review coverage not established")
    if review.get("pr_number") != pr.get("number"):
        problems.append("review belongs to another PR")
    findings = review.get("findings")
    if not isinstance(findings, list):
        problems.append("review findings missing")
        findings = []
    checks = github.get("checks")
    if not isinstance(checks, list):
        problems.append("GitHub checks missing")
        checks = []
    if any(not isinstance(c, dict) or c.get("evidence_state") != "OBSERVED" or
           not isinstance(c.get("workflow_run_id"), int) or
           not isinstance(c.get("run_attempt"), int) or
           c["run_attempt"] < 1 for c in checks):
        problems.append("job metadata incomplete")
    safe_checks = [c for c in checks if isinstance(c, dict)]
    result = assess({"pr": {"head_sha": head}, "checks": safe_checks, "findings": findings})
    problems.extend(result["reasons"])
    return {
        "schema_version": 1, "head_sha": head,
        "integration": "PASS" if not problems else "FAIL",
        "issues": sorted(set(problems)),
        "checks_observed": len(checks),
        "findings_observed": len(findings),
        "required_checks_verified": False,
        "review_correctness_verified": False,
        "live_integration_verified": False,
        "production_ready": False,
        "release_authorized": False,
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--github-evidence", required=True, type=Path)
    p.add_argument("--review-evidence", required=True, type=Path)
    p.add_argument("--output", type=Path)
    args = p.parse_args(argv)
    try:
        result = join(json.loads(args.github_evidence.read_text()),
                      json.loads(args.review_evidence.read_text()))
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered)
        print(rendered, end="")
        return 0 if result["integration"] == "PASS" else 1
    except (OSError, ValueError, TypeError) as exc:
        p.exit(2, f"invalid evidence input: {type(exc).__name__}\n")


if __name__ == "__main__":
    raise SystemExit(main())
