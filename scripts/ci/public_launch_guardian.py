#!/usr/bin/env python3
"""Fail-closed, read-only public-testnet evidence assessment.

A successful report means "eligible for independent review", NEVER permission to
merge, publish, start public RPC, or deploy. Inputs include potentially forged
operator claims; this tool cannot cryptographically verify those claims.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from pathlib import Path
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

HERE = Path(__file__).resolve().parent
DEFAULT_POLICY = HERE / "public-launch-policy.json"
DEFAULT_REPO = "Cyptopimpinainteazy/xxxstar"
SHA = re.compile(r"^[a-f0-9]{40}$")
HEX32 = re.compile(r"^0x[a-fA-F0-9]{64}$")
DIGEST = re.compile(r"^[a-f0-9]{64}$")
REPO = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")


def require_policy(policy):
    if not isinstance(policy, dict) or policy.get("schema_version") != 1:
        raise ValueError("unknown launch policy schema")
    gates = policy.get("required_workflows")
    if not isinstance(gates, list) or not gates:
        raise ValueError("missing required workflows")
    names, paths = set(), set()
    for item in gates:
        if not isinstance(item, dict):
            raise ValueError("invalid workflow policy entry")
        name, path = item.get("gate"), item.get("path")
        if (not isinstance(name, str) or not name or name in names or
                not isinstance(path, str) or
                not re.fullmatch(r"\.github/workflows/[\w-]+\.ya?ml", path) or path in paths):
            raise ValueError("invalid or duplicate workflow identity")
        minimum = item.get("min_successful_jobs", 1)
        if type(minimum) is not int or minimum < 1:
            raise ValueError("invalid minimum required job count")
        names.add(name)
        paths.add(path)
    for field in ("required_fleet_assertions", "required_operator_assertions"):
        values = policy.get(field)
        if (not isinstance(values, list) or not values or
                not all(isinstance(v, str) and v for v in values) or
                len(values) != len(set(values))):
            raise ValueError("invalid policy assertion names")
    value = policy.get("min_validators")
    if isinstance(value, bool) or not isinstance(value, int) or value < 7:
        raise ValueError("validator admission minimum cannot be below seven")
    return policy


def run_path(run, repo):
    raw = run.get("path")
    if not isinstance(raw, str):
        return None
    source = raw.split("@", 1)[0]
    prefix = repo + "/"
    if source.startswith(prefix):
        source = source[len(prefix):]
    return source if source.startswith(".github/workflows/") else None


def timestamp(raw):
    if not isinstance(raw, str):
        return None
    try:
        result = dt.datetime.fromisoformat(raw.replace("Z", "+00:00"))
    except ValueError:
        return None
    return result.astimezone(dt.timezone.utc) if result.tzinfo else None


def check_workflows(candidate, runs, policy, repo, now, max_age_days=14):
    if not isinstance(runs, list):
        raise ValueError("workflow_runs must be an array")
    outcome, reasons = [], []
    for requirement in policy["required_workflows"]:
        path, name = requirement["path"], requirement["gate"]
        observed = [run for run in runs if isinstance(run, dict) and run_path(run, repo) == path]
        if any(run.get("head_sha") != candidate for run in observed):
            reasons.append(f"{name}: mismatched-commit workflow evidence")
            outcome.append({"gate": name, "status": "BLOCKED", "run_id": None})
            continue
        # Latest run on the exact candidate wins. A previous green run cannot
        # override a later failure, cancellation, skip or in-progress rerun.
        observed.sort(key=lambda run: (
            timestamp(run.get("created_at")) or dt.datetime.min.replace(tzinfo=dt.timezone.utc),
            run.get("id") if type(run.get("id")) is int else -1,
            run.get("run_attempt") if type(run.get("run_attempt")) is int else -1,
        ), reverse=True)
        if not observed:
            reasons.append(f"{name}: required workflow never observed for candidate")
            outcome.append({"gate": name, "status": "MISSING", "run_id": None})
            continue
        run = observed[0]
        created = timestamp(run.get("created_at"))
        valid_identity = (type(run.get("id")) is int and run["id"] > 0 and
                          type(run.get("run_attempt")) is int and run["run_attempt"] > 0)
        if not valid_identity or created is None:
            status = "INVALID"
        elif created > now + dt.timedelta(minutes=10) or now - created > dt.timedelta(days=max_age_days):
            status = "STALE"
        elif run.get("status") == "completed" and run.get("conclusion") == "success":
            jobs = run.get("jobs")
            if not isinstance(jobs, list) or len(jobs) < requirement.get("min_successful_jobs", 1):
                status = "NO_JOBS"
            elif any(
                not isinstance(job, dict) or
                type(job.get("id")) is not int or job["id"] <= 0 or
                not isinstance(job.get("name"), str) or not job["name"] or
                job.get("head_sha") not in (None, candidate) or
                job.get("status") != "completed" or job.get("conclusion") != "success"
                for job in jobs
            ):
                status = "JOBS_NOT_GREEN"
            else:
                status = "PASS"
        else:
            status = "BLOCKED"
        if status != "PASS":
            reasons.append(f"{name}: latest exact-head workflow is {status.lower()}")
        outcome.append({"gate": name, "status": status,
                        "run_id": run.get("id") if valid_identity else None})
    return outcome, reasons


def check_fleet(candidate, evidence, policy):
    if not isinstance(evidence, dict):
        return ["fleet: independent validator evidence missing"]
    problems = []
    if evidence.get("candidate_sha") != candidate:
        problems.append("fleet: candidate digest mismatch")
    if not isinstance(evidence.get("genesis_hash"), str) or not HEX32.fullmatch(evidence["genesis_hash"]):
        problems.append("fleet: canonical genesis hash missing or invalid")
    if not isinstance(evidence.get("evidence_sha256"), str) or not DIGEST.fullmatch(evidence["evidence_sha256"]):
        problems.append("fleet: evidence bundle digest missing")
    nodes = evidence.get("validators")
    if not isinstance(nodes, list) or len(nodes) < policy["min_validators"]:
        problems.append("fleet: fewer than required validator observations")
    else:
        peers, keys, finalities = [], [], []
        for node in nodes:
            if not isinstance(node, dict):
                problems.append("fleet: malformed validator observation")
                continue
            peer, key = node.get("peer_id"), node.get("authority_key")
            h, block_hash = node.get("finalized_number"), node.get("finalized_hash")
            if not isinstance(peer, str) or not peer.strip():
                problems.append("fleet: missing peer identity")
            else:
                peers.append(peer)
            if not isinstance(key, str) or not HEX32.fullmatch(key):
                problems.append("fleet: malformed authority key")
            else:
                keys.append(key.lower())
            if type(h) is not int or h < 1 or not isinstance(block_hash, str) or not HEX32.fullmatch(block_hash):
                problems.append("fleet: missing canonical finalized height/hash")
            else:
                finalities.append((h, block_hash.lower()))
        if len(set(peers)) != len(nodes):
            problems.append("fleet: duplicate or missing peer identities")
        if len(set(keys)) != len(nodes):
            problems.append("fleet: duplicate or missing authority keys")
        if len(finalities) != len(nodes) or len(set(finalities)) != 1:
            problems.append("fleet: divergent or incomplete finalized checkpoints")
    for assertion in policy["required_fleet_assertions"]:
        if evidence.get(assertion) is not True:
            problems.append(f"fleet: {assertion} not evidenced")
    return problems


def check_operator(candidate, evidence, policy):
    if not isinstance(evidence, dict):
        return ["operations: launch controls not independently attested"]
    problems = []
    if evidence.get("candidate_sha") != candidate:
        problems.append("operations: candidate digest mismatch")
    if not isinstance(evidence.get("evidence_sha256"), str) or not DIGEST.fullmatch(evidence["evidence_sha256"]):
        problems.append("operations: evidence bundle digest missing")
    for assertion in policy["required_operator_assertions"]:
        if evidence.get(assertion) is not True:
            problems.append(f"operations: {assertion} not evidenced")
    return problems


def assess(candidate_sha, runs, policy, fleet=None, operations=None,
           repo=DEFAULT_REPO, source="fixture", now=None):
    if not isinstance(candidate_sha, str) or not SHA.fullmatch(candidate_sha):
        raise ValueError("candidate_sha must be a full lowercase Git commit")
    if not isinstance(repo, str) or not REPO.fullmatch(repo):
        raise ValueError("invalid GitHub repository")
    require_policy(policy)
    now = now or dt.datetime.now(dt.timezone.utc)
    if now.tzinfo is None:
        raise ValueError("now must be timezone-aware")
    checks, reasons = check_workflows(candidate_sha, runs, policy, repo, now)
    reasons += check_fleet(candidate_sha, fleet, policy)
    reasons += check_operator(candidate_sha, operations, policy)
    if source != "github_live":
        reasons.append("CI metadata source unverified: local fixtures cannot authorize admission")
    status = "BLOCKED" if reasons else "INDEPENDENT_REVIEW_REQUIRED"
    return {
        "schema_version": 1,
        "candidate_sha": candidate_sha,
        "decision": status,
        "checks": checks,
        "blockers": sorted(set(reasons)),
        "ci_source": source,
        "operator_claims_cryptographically_verified": False,
        "independent_security_review_verified": False,
        "physical_fleet_verified_by_guardian": False,
        "release_authorized": False,
        "disclaimer": "Assessment only. No merge, deployment, or public-access authorization.",
    }


def fetch_github_runs(repo, sha, token, max_pages=10):
    """Read GitHub workflow metadata with bounded pagination; do not fetch logs."""
    if not token:
        raise ValueError("GH_TOKEN/GITHUB_TOKEN required for live CI collection")
    collected = []
    for page in range(1, max_pages + 1):
        query = urllib.parse.urlencode({"head_sha": sha, "per_page": 100, "page": page})
        request = urllib.request.Request(
            f"https://api.github.com/repos/{repo}/actions/runs?{query}",
            headers={"Authorization": "Bearer " + token,
                     "Accept": "application/vnd.github+json",
                     "User-Agent": "x3-public-launch-guardian/1",
                     "X-GitHub-Api-Version": "2022-11-28"},
        )
        with urllib.request.urlopen(request, timeout=20) as response:
            page_data = json.load(response)
        batch = page_data.get("workflow_runs")
        if not isinstance(batch, list):
            raise ValueError("GitHub workflow run API response invalid")
        if any(not isinstance(run, dict) or run.get("head_sha") != sha for run in batch):
            raise ValueError("GitHub returned non-candidate workflow runs")
        collected.extend(batch)
        if len(batch) < 100:
            return collected
    raise ValueError("GitHub run pagination limit reached; no partial admission")


def fetch_required_jobs(repo, runs, policy, sha, token, max_pages=10):
    """Enrich only the latest required run with exact-head jobs.

    The workflow's green result is not sufficient; missing or skipped jobs
    block admission. A partial job page is never accepted as complete.
    """
    for required in policy["required_workflows"]:
        matching = [run for run in runs if run_path(run, repo) == required["path"]
                    and run.get("head_sha") == sha]
        if not matching:
            continue
        matching.sort(key=lambda run: (
            timestamp(run.get("created_at")) or dt.datetime.min.replace(tzinfo=dt.timezone.utc),
            run.get("id") if type(run.get("id")) is int else -1,
            run.get("run_attempt") if type(run.get("run_attempt")) is int else -1,
        ), reverse=True)
        run = matching[0]
        if type(run.get("id")) is not int or run["id"] < 1:
            continue
        jobs = []
        for page in range(1, max_pages + 1):
            query = urllib.parse.urlencode({"per_page": 100, "page": page})
            request = urllib.request.Request(
                f"https://api.github.com/repos/{repo}/actions/runs/{run['id']}/jobs?{query}",
                headers={"Authorization": "Bearer " + token,
                         "Accept": "application/vnd.github+json",
                         "User-Agent": "x3-public-launch-guardian/1",
                         "X-GitHub-Api-Version": "2022-11-28"},
            )
            with urllib.request.urlopen(request, timeout=20) as response:
                payload = json.load(response)
            batch = payload.get("jobs")
            if not isinstance(batch, list):
                raise ValueError("GitHub workflow jobs API response invalid")
            jobs.extend(batch)
            if len(batch) < 100:
                break
        else:
            raise ValueError("GitHub jobs pagination limit reached; no partial admission")
        # A rerun or forced update can race the metadata collection. Require
        # the observed attempt to match the latest run attempt at the end.
        run["jobs"] = jobs
    return runs


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--repo", default=DEFAULT_REPO)
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--runs-json", type=Path, help="Untrusted/offline fixture; never authoritative")
    source.add_argument("--fetch-github", action="store_true", help="Live read-only GitHub Actions API")
    parser.add_argument("--fleet-json", type=Path, help="Operator-provided physical validator observations")
    parser.add_argument("--operations-json", type=Path, help="Operator-provided launch control attestations")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        policy = json.loads(args.policy.read_text(encoding="utf-8"))
        if not SHA.fullmatch(args.candidate_sha) or not REPO.fullmatch(args.repo):
            raise ValueError("invalid candidate SHA or repository")
        if args.fetch_github:
            token = os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
            require_policy(policy)
            runs = fetch_github_runs(args.repo, args.candidate_sha, token)
            runs = fetch_required_jobs(args.repo, runs, policy, args.candidate_sha, token)
            source_label = "github_live"
        else:
            fixture = json.loads(args.runs_json.read_text(encoding="utf-8"))
            runs = fixture["workflow_runs"] if isinstance(fixture, dict) else fixture
            source_label = "fixture"
        fleet = json.loads(args.fleet_json.read_text(encoding="utf-8")) if args.fleet_json else None
        operations = json.loads(args.operations_json.read_text(encoding="utf-8")) if args.operations_json else None
        report = assess(args.candidate_sha, runs, policy, fleet, operations,
                        repo=args.repo, source=source_label)
        rendered = json.dumps(report, sort_keys=True, indent=2) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered, encoding="utf-8")
        sys.stdout.write(rendered)
        return 0 if report["decision"] == "INDEPENDENT_REVIEW_REQUIRED" else 1
    except (OSError, ValueError, TypeError, KeyError, urllib.error.URLError) as exc:
        print(f"x3 launch guardian: cannot assess evidence ({type(exc).__name__})", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
