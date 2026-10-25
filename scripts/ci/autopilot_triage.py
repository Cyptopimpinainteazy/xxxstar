#!/usr/bin/env python3
"""Advisory, read-only GitHub Actions failure classification; not a release gate."""
import argparse
import hashlib
import json
from pathlib import Path
import re

PATTERNS = (
    ("CONFIGURATION", (r"permission denied \(publickey\)", r"authentication failed", r"bad credentials", r"resource not accessible by integration", r"no such host")),
    ("INFRASTRUCTURE", (r"lost communication with the server", r"runner connection lost", r"the runner has received a shutdown signal", r"no space left on device", r"could not resolve host")),
    ("TEST_FAILURE", (r"test result: failed", r"assertion.*failed", r"failures:", r"\bpanicked at\b", r"error: test failed")),
    ("BUILD_FAILURE", (r"error\[e[0-9]{4}\]", r"compilation failed", r"could not compile")),
    ("EVIDENCE_STORAGE", (r"artifact storage quota has been hit", r"unable to upload any new artifacts")),
)


def classify(job, log=""):
    """Keep non-successful conclusions blocked and never declare retry safe."""
    conclusion = job.get("conclusion")
    status = job.get("status")
    if status != "completed":
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
        "id": job.get("id"),
        "status": status,
        "conclusion": conclusion,
        "classification": classification,
        "safe_to_ignore": False,
        "log_sha256": hashlib.sha256(log.encode("utf-8")).hexdigest() if log else None,
        "evidence_present": bool(log),
    }


def report(head_sha, jobs, logs=None):
    """Create an exact-head advisory evidence packet; never authorize release."""
    logs = logs or {}
    items = [classify(job, logs.get(str(job.get("id")), "")) for job in jobs]
    return {
        "head_sha": head_sha,
        "overall": "PASS_OBSERVED" if items and all(item["classification"] == "PASSED" for item in items) else "BLOCKED",
        "production_ready": False,
        "note": "Advisory only: required checks, branch rules and human approval are independent.",
        "jobs": items,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jobs-json", required=True, type=Path)
    parser.add_argument("--logs-json", type=Path, help="Optional mapping of job IDs to log text; never printed")
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
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
