#!/usr/bin/env python3
"""Static, fail-closed integration audit for X3 PR-Agent and CI Autopilot.

Checks repository wiring only. Passing does NOT prove live GitHub, model,
runner or release behavior. No SSH, network access, secrets or state changes.
"""
import argparse
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CONTRACTS = {
    "manual-review": (".github/workflows/pr-agent.yml", (
        "workflow_dispatch:", "services/pr-agent/review.sh",
        "services/pr-agent/selftest.sh", "dry_run:",
        "permissions:", "pull-requests: write")),
    "automatic-review": (".github/workflows/pr-agent-auto.yml", (
        "pull_request_target:", "issue_comment:", "pull_request.head.repo.full_name == github.repository",
        "github.event.repository.default_branch", "services/pr-agent/review.sh",
        "services/pr-agent/on-comment.sh", "pull-requests: write")),
    "review-wrapper": ("services/pr-agent/review.sh", (
        "GITHUB__USER_TOKEN", "--dry-run", "CONFIG__FALLBACK_MODELS",
        "--extra_config_url=")),
    "review-rules": (".pr_agent.toml", (
        "atomic", "rollback", "f64", "WeightInfo", "require_security_review = true")),
    "review-selftest": ("services/pr-agent/selftest.sh", (
        "expect_dies", "expect_argv", "on-comment.sh")),
    "autopilot-workflow": (".github/workflows/x3-ci-autopilot.yml", (
        "autopilot-tests:", "test_autopilot*.py", "github.event.pull_request.head.repo.full_name == github.repository")),
    "autopilot-triage": ("scripts/ci/autopilot_triage.py", (
        "NOT_FINISHED", "production_ready", "log_sha256")),
    "autopilot-placement": ("scripts/ci/autopilot_capacity.py", (
        "execution_authorized", "primary_workload_active", "fresh")),
    "local-ci": (".github/workflows/local-ci.yml", (
        "scripts/local-ci.sh", "pull_request:", "self-hosted")),
}


def audit(root):
    checks = []
    for name, (rel, required) in CONTRACTS.items():
        path = root / rel
        if not path.is_file():
            checks.append({"contract": name, "status": "FAIL", "reason": "file missing", "path": rel})
            continue
        data = path.read_text(encoding="utf-8")
        missing = [marker for marker in required if marker not in data]
        checks.append({"contract": name, "status": "PASS" if not missing else "FAIL",
                       "path": rel, "reason": "required wiring marker absent" if missing else "markers present",
                       "missing_markers": missing})
    return {"schema_version": 1, "audit_type": "STATIC_WIRING_ONLY",
            "status": "PASS" if all(x["status"] == "PASS" for x in checks) else "FAIL",
            "live_integration_verified": False, "release_authorized": False,
            "checks": checks}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        result = audit(args.root.resolve())
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        print(rendered, end="")
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        return 0 if result["status"] == "PASS" else 1
    except (OSError, UnicodeError) as exc:
        print(f"integration audit error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
