#!/usr/bin/env python3
"""Advisory exact-identity check for X3 CI evidence; never skips gates.

JSON fixtures are not trusted attestations. An exact match is only a lead for an
independent verifier that can authenticate run origin and original logs.
"""
import argparse
import json
import re
import sys
from pathlib import Path

# Only deterministic, non-release gate families can produce reuse candidates.
ADVISORY_GATES = frozenset({
    "format-check", "agent-guards", "workflow-schema", "script-syntax",
    "test-integrity-diff", "readiness-consistency",
})
IDENTITY_DIGESTS = (
    "gate_spec_sha256", "toolchain_sha256", "dependencies_sha256",
    "environment_sha256",
)
IDENTITY = ("head_sha", "base_sha", "gate_id", "profile") + IDENTITY_DIGESTS
HEX40 = re.compile(r"[0-9a-f]{40}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")


def _valid_target(t):
    return (
        isinstance(t, dict)
        and isinstance(t.get("head_sha"), str) and HEX40.fullmatch(t["head_sha"]) is not None
        and isinstance(t.get("base_sha"), str) and HEX40.fullmatch(t["base_sha"]) is not None
        and t.get("profile") == "fast"
        and isinstance(t.get("gate_id"), str) and t["gate_id"] in ADVISORY_GATES
        and all(isinstance(t.get(k), str) and HEX64.fullmatch(t[k]) for k in IDENTITY_DIGESTS)
    )


def _valid_success(r):
    return (
        r.get("status") == "completed"
        and r.get("conclusion") == "success"
        and type(r.get("exit_code")) is int and r["exit_code"] == 0
        and type(r.get("run_id")) is int and r["run_id"] > 0
        and type(r.get("run_attempt")) is int and r["run_attempt"] > 0
        and isinstance(r.get("log_sha256"), str)
        and HEX64.fullmatch(r["log_sha256"]) is not None
    )


def _result(decision, reason, ids=()):
    return {
        "decision": decision,
        "reason": reason,
        "candidate_run_ids": sorted(set(ids)),
        "skip_authorized": False,
        "trusted_provenance_verified": False,
        "production_ready": False,
    }


def decide(target, receipts):
    """Require an independent run unless a non-authoritative exact match exists.

    An untrusted receipt cannot authorize reuse, independent of its contents.
    Even matching results never become a required-branch-protection verdict.
    """
    if not _valid_target(target):
        return _result("RUN_REQUIRED", "incomplete identity or gate not reusable")
    if not isinstance(receipts, list) or len(receipts) > 256 or any(not isinstance(r, dict) for r in receipts):
        return _result("RUN_REQUIRED", "invalid or unbounded receipt collection")
    matched = [r for r in receipts if all(r.get(key) == target[key] for key in IDENTITY)]
    if not matched:
        return _result("RUN_REQUIRED", "no exact-identity evidence")
    if not all(_valid_success(r) for r in matched):
        return _result("RUN_REQUIRED", "incomplete or conflicting exact-identity evidence")
    return _result("EVIDENCE_CANDIDATE", "matching untrusted receipts; independently verify original run and logs", [r["run_id"] for r in matched])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", type=Path, required=True)
    parser.add_argument("--receipts", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        target = json.loads(args.target.read_text())
        receipts = json.loads(args.receipts.read_text())
    except (OSError, ValueError) as exc:
        print(f"reuse inspection unavailable: {type(exc).__name__}", file=sys.stderr)
        return 2
    print(json.dumps(decide(target, receipts), sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
