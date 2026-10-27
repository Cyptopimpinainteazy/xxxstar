#!/usr/bin/env python3
"""Advisory, read-only GitHub Actions failure classification; not a release gate."""
import argparse
import hashlib
import json
from pathlib import Path
import re

PATTERNS = (
    ("CONFIGURATION", (r"permission denied \\(publickey\\)", r"authentication failed", r"bad credentials", r"resource not accessible by integration", r"no such host")),
    ("INFRASTRUCTURE", (r"lost communication with the server", r"runner connection lost", r"the runner has received a shutdown signal", r"no space left on device", r"could not resolve host")),
    ("PYTHON_DEPENDENCY", (r"no module named", r"pyyaml is required", r"pytest: command not found", r"module\\s+notfounderror")),
    ("RUST_FORMAT", (r"diff in .*\\.rs:", r"rustfmt failed", r"cargo fmt.*--check.*failed")),
    ("LOCKFILE_DRIFT", (r"lock file.*needs to be updated but --locked", r"lockfile.*needs to be updated")),
    ("ARTIFACT_STORAGE", (r"artifact storage quota", r"storage quota has been hit", r"artifact quota")),
    ("TEST_FAILURE", (r"test result: failed", r"assertion.*failed", r"failures:", r"\\bpanicked at\\b", r"error: test failed")),
    ("BUILD_FAILURE", (r"error\\[e[0-9]{4}\\]", r"compilation failed", r"could not compile")),
)

# Recommendations are diagnostic commands/instructions only; none change a
# branch, install packages, re-run a CI job or approve a release.
REPRODUCERS = {
    "RUST_FORMAT": "cargo fmt --all -- --check",
    "LOCKFILE_DRIFT": "cargo check --locked --workspace",
    "PYTHON_DEPENDENCY": "python3 -m pytest --version",
    "TEST_FAILURE": "run the exact failing test command from the trusted job",
    "BUILD_FAILURE": "run the exact failing cargo check command from the trusted job",
    "ARTIFACT_STORAGE": "inspect Actions artifact quota and retention without deleting artifacts",
    "INFRASTRUCTURE": "inspect runner health and disk space; do not auto-retry",
    "CONFIGURATION": "verify runner configuration and permissions without exposing credentials",
}



def classify(job, log=""):
    """Keep non-successful conclusions blocked and never declare retry safe."""
    conclusion = job.get("conclusion")
    status = job.get("status")
    evidence_state = job.get("evidence_state", "OBSERVED")
    if evidence_state != "OBSERVED":
        classification = "MISSING_EVIDENCE"
    elif status != "completed":
        classification = "NOT_FINISHED"
    elif conclusion == "success":
        classification = "PASSED"
    elif conclusion in ("skipped", "cancelled", "neutral"):
        classification = "NON_PASSING"
    elif conclusion != "failure":
        classification = "UNKNOWN"
    else:
        classification = "UNKNOWN"
        for label, patterns in PATTERNS:
            if any(re.search(pattern, log, re.IGNORECASE) for pattern in patterns):
                classification = label
                break
    return {
        "name": str(job.get("name") or "unnamed"),
        "id": job.get("id", job.get("job_id")),
        "status": status,
        "conclusion": conclusion,
        "classification": classification,
        "safe_to_ignore": False,
        "suggested_diagnostic": REPRODUCERS.get(classification),
        "log_sha256": hashlib.sha256(log.encode("utf-8")).hexdigest() if log else None,
        "evidence_present": bool(log),
    }


def report(head_sha, jobs, logs=None):
    """Create an exact-head advisory evidence packet; never authorize release."""
    logs = logs or {}
    items = [classify(job, logs.get(str(job.get("id", job.get("job_id"))), "")) for job in jobs]
    return {
        "head_sha": head_sha,
        "overall": "PASS_OBSERVED" if items and all(item["classification"] == "PASSED" for item in items) else "BLOCKED",
        "production_ready": False,
        "note": "Advisory only: required checks, branch rules and human approval are independent.",
        "jobs": items,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--jobs-json", type=Path)
    source.add_argument("--github-evidence", type=Path,
                        help="Exact-head metadata packet from github_evidence.py")
    parser.add_argument("--expected-head", help="Reject stale or changed PR head")
    parser.add_argument("--logs-json", type=Path, help="Optional mapping of job IDs to log text; never printed")
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.github_evidence:
            payload = json.loads(args.github_evidence.read_text())
            if not isinstance(payload, dict) or payload.get("source") != "GITHUB_ACTIONS_METADATA":
                raise ValueError("untrusted GitHub evidence source")
            pr = payload.get("pr")
            if not isinstance(pr, dict) or pr.get("head_sha") != args.head_sha:
                raise ValueError("GitHub evidence is from another commit")
            if args.expected_head is not None and args.expected_head != args.head_sha:
                raise ValueError("expected PR head does not match")
            jobs = payload.get("checks")
            if not isinstance(jobs, list) or any(
                not isinstance(job, dict) or job.get("head_sha") != args.head_sha
                for job in jobs
            ):
                raise ValueError("GitHub checks malformed or wrong head")
        else:
            payload = json.loads(args.jobs_json.read_text())
            jobs = payload["jobs"] if isinstance(payload, dict) else payload
            if not isinstance(jobs, list):
                raise ValueError("jobs must be a list")
        logs = json.loads(args.logs_json.read_text()) if args.logs_json else {}
        if not isinstance(logs, dict) or any(not isinstance(k, str) or not isinstance(v, str) for k, v in logs.items()):
            raise ValueError("logs must map job ID strings to log strings")
        result = report(args.head_sha, jobs, logs)
        data = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(data)
        print(data, end="")
        return 0
    except (OSError, ValueError, KeyError, TypeError) as exc:
        parser.exit(2, f"triage input error: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
