#!/usr/bin/env python3
"""X3 CI Autopilot resource placement planner. Pure decisions; never launches jobs."""
import argparse
import json
from pathlib import Path

# Conservative defaults; intended to preserve resources for each host's main role.
DEFAULT_POLICY = {
    "build": {"min_free_disk_gb": 30, "max_load_per_thread": 0.8, "reserve_threads": 2},
    "test": {"min_free_disk_gb": 20, "max_load_per_thread": 0.8, "reserve_threads": 2},
    "simulation": {"min_free_disk_gb": 20, "max_load_per_thread": 0.65, "reserve_threads": 4},
    "fuzz": {"min_free_disk_gb": 30, "max_load_per_thread": 0.65, "reserve_threads": 4},
    "inference": {"min_free_disk_gb": 8, "max_load_per_thread": 0.6, "reserve_threads": 4},
}
ROLE_ELIGIBILITY = {
    "build": {"build", "control"},
    "test": {"build", "control"},
    "simulation": {"sim"},
    "fuzz": {"sim"},
    "inference": {"gpu"},
}


def assess_worker(worker, job_class, policy=None):
    """Fail closed when observations are missing or the worker is unhealthy."""
    rules = (policy or DEFAULT_POLICY).get(job_class)
    if rules is None or job_class not in ROLE_ELIGIBILITY:
        return False, ["unsupported job class"]
    reasons = []
    if worker.get("role") not in ROLE_ELIGIBILITY[job_class]:
        reasons.append("role not eligible")
    if worker.get("ci") != "IDLE":
        reasons.append("runner not confirmed idle")
    # Unknown and stale observations must never authorize real workload placement.
    if worker.get("fresh") is not True:
        reasons.append("resource observations are not confirmed fresh")
    if worker.get("primary_workload_active") is not False:
        reasons.append("primary workload availability not verified")
    for key in ("cpu_threads", "load_1m", "disk_free_gb"):
        v = worker.get(key)
        if isinstance(v, bool) or not isinstance(v, (int, float)) or v < 0:
            reasons.append(f"missing or invalid {key}")
    if reasons:
        return False, reasons
    threads = worker["cpu_threads"]
    if threads <= rules["reserve_threads"]:
        reasons.append("insufficient threads after reserve")
    if worker["disk_free_gb"] < rules["min_free_disk_gb"]:
        reasons.append("disk below class minimum")
    if worker["load_1m"] / max(1, threads) > rules["max_load_per_thread"]:
        reasons.append("load above class maximum")
    return not reasons, reasons


def plan(workers, job_class, policy=None):
    if job_class not in ROLE_ELIGIBILITY:
        return {"job_class": job_class, "decision": "REJECT", "reason": "unsupported job class", "workers": []}
    checked = []
    eligible = []
    for worker in workers:
        ok, reasons = assess_worker(worker, job_class, policy)
        checked.append({"name": worker.get("name", "unknown"), "eligible": ok, "reasons": reasons})
        if ok:
            available = worker["cpu_threads"] - (policy or DEFAULT_POLICY)[job_class]["reserve_threads"]
            score = (worker["load_1m"] / available, -worker["disk_free_gb"], worker["name"])
            eligible.append((score, worker["name"]))
    eligible.sort()
    return {"job_class": job_class, "decision": "RECOMMEND" if eligible else "DEFER",
            "recommended_worker": eligible[0][1] if eligible else None,
            "workers": checked, "execution_authorized": False}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workers-json", required=True, type=Path,
                        help="trusted, current resource snapshot (no network queries)")
    parser.add_argument("--job-class", required=True, choices=sorted(ROLE_ELIGIBILITY))
    args = parser.parse_args(argv)
    try:
        source = json.loads(args.workers_json.read_text())
        workers = source["workers"] if isinstance(source, dict) else source
        if not isinstance(workers, list):
            raise ValueError("workers must be an array")
        print(json.dumps(plan(workers, args.job_class), indent=2))
    except (OSError, ValueError, KeyError, TypeError) as exc:
        parser.exit(2, f"cannot evaluate capacity: {exc}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
