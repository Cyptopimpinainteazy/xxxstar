#!/usr/bin/env python3
"""Offline synthetic PR-Agent/CI evidence contract; not a live integration gate."""
import argparse
import hashlib
import json
from pathlib import Path
import re

SHA = re.compile(r"^[a-f0-9]{40}$")
SEVERITIES = {"critical", "high", "medium", "low"}


def assess(bundle):
    errors = []
    pr = bundle.get("pr", {})
    sha = pr.get("head_sha")
    if not isinstance(sha, str) or not SHA.fullmatch(sha):
        errors.append("invalid PR head SHA")
    checks = bundle.get("checks")
    if not isinstance(checks, list) or not checks:
        errors.append("missing check evidence")
        checks = []
    for check in checks:
        if not isinstance(check, dict) or check.get("head_sha") != sha:
            errors.append("check evidence stale or malformed")
        elif check.get("status") != "completed" or check.get("conclusion") != "success":
            errors.append("required check incomplete or non-passing")
    findings = bundle.get("findings")
    if not isinstance(findings, list):
        errors.append("missing review evidence")
        findings = []
    for f in findings:
        if not isinstance(f, dict):
            errors.append("malformed review finding")
            continue
        if f.get("head_sha") != sha:
            errors.append("finding targets a different commit")
        path, line = f.get("path"), f.get("line")
        if not isinstance(path, str) or not path or path.startswith("/") or ".." in Path(path).parts or not isinstance(line, int) or isinstance(line, bool) or line < 1:
            errors.append("finding lacks valid source location")
        if f.get("severity") not in SEVERITIES:
            errors.append("invalid severity")
        if f.get("verification") not in ("UNVERIFIED", "REPRODUCED"):
            errors.append("invalid verification")
        if f.get("verification") == "REPRODUCED" and not f.get("reproducer"):
            errors.append("reproduced claim lacks evidence")
    return {"integration": "FAIL" if errors else "PASS", "reasons": sorted(set(errors)),
            "head_sha": sha, "review_findings": len(findings), "checks_observed": len(checks),
            "production_ready": False, "release_authorized": False}


def synthetic_suite():
    sha = "a" * 40
    clean = {"pr": {"head_sha": sha}, "checks": [{"head_sha": sha, "status": "completed", "conclusion": "success"}], "findings": []}
    cases = [
        ("clean-evidence", clean, "PASS"),
        ("failed-check", {**clean, "checks": [{"head_sha": sha, "status": "completed", "conclusion": "failure"}]}, "FAIL"),
        ("queued-check", {**clean, "checks": [{"head_sha": sha, "status": "queued", "conclusion": None}]}, "FAIL"),
        ("stale-head", {**clean, "checks": [{"head_sha": "b" * 40, "status": "completed", "conclusion": "success"}]}, "FAIL"),
        ("fabricated-reproduction", {**clean, "findings": [{"head_sha": sha, "path": "src/main.rs", "line": 10, "severity": "high", "verification": "REPRODUCED"}]}, "FAIL"),
        ("missing-checks", {**clean, "checks": []}, "FAIL"),
    ]
    results = []
    for name, payload, expected in cases:
        observed = assess(payload)["integration"]
        results.append({"case": name, "expected": expected, "observed": observed,
                        "passed": observed == expected,
                        "evidence_sha256": hashlib.sha256(json.dumps(payload, sort_keys=True).encode()).hexdigest()})
    return {"suite": "synthetic-integration-v1", "passed": all(r["passed"] for r in results),
            "live_integration_verified": False, "production_ready": False, "cases": results}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--fixture", type=Path)
    p.add_argument("--output", type=Path)
    args = p.parse_args(argv)
    try:
        result = assess(json.loads(args.fixture.read_text())) if args.fixture else synthetic_suite()
        rendered = json.dumps(result, sort_keys=True, indent=2) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered)
        print(rendered, end="")
        return 0 if result.get("passed", result.get("integration") == "PASS") else 1
    except (OSError, ValueError, TypeError, AttributeError) as exc:
        p.exit(2, f"invalid integration evidence: {exc}\n")


if __name__ == "__main__":
    raise SystemExit(main())
