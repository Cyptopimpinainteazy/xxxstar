#!/usr/bin/env python3
"""Read-only X3 CI Autopilot diagnostics. No SSH, sudo, or runner mutations."""
import argparse
import datetime as dt
import json
import os
from pathlib import Path
import shutil
import socket
import sys
import urllib.error
import urllib.request

REPO = "Cyptopimpinainteazy/xxxstar"
DEFAULT_INVENTORY = Path(__file__).resolve().parents[1] / "x3-cluster" / "inventory.json"


def local_capacity():
    total, used, free = shutil.disk_usage(Path.home())
    try:
        load = os.getloadavg()[0]
    except (OSError, AttributeError):
        load = None
    return {
        "hostname": socket.gethostname(),
        "cpu_threads": os.cpu_count(),
        "load_1m": load,
        "disk_free_gb": round(free / 1_000_000_000, 2),
        "disk_total_gb": round(total / 1_000_000_000, 2),
    }


def runner_state(runner, stale_names=()):
    if runner.get("name") in stale_names:
        return "STALE"
    if runner.get("status") != "online":
        return "OFFLINE"
    return "BUSY" if runner.get("busy") else "IDLE"


def evaluate(inventory, runners, capacity, threshold_gb=30):
    """Reconcile declared hardware with observed GitHub runner registrations.

    Offline runners are not proof that the underlying host is powered off.
    Missing registrations are UNKNOWN, never implicitly healthy.
    """
    names = {r["name"]: r for r in runners if isinstance(r, dict) and r.get("name")}
    nodes = []
    for name, spec in inventory.get("nodes", {}).items():
        r = names.get(name)
        state = runner_state(r) if r else "UNREGISTERED"
        nodes.append({"name": name, "role": spec.get("role"), "ci": state,
                      "required": bool(spec.get("required", False)),
                      "labels": sorted(label.get("name", "") for label in r.get("labels", [])) if r else []})
    # Registrations not present in the older cluster inventory must still appear.
    for name, r in names.items():
        if name not in inventory.get("nodes", {}):
            nodes.append({"name": name, "role": "unassigned", "ci": runner_state(r),
                          "required": False,
                          "labels": sorted(label.get("name", "") for label in r.get("labels", []))})
    notes = []
    if capacity["disk_free_gb"] < threshold_gb:
        notes.append("local disk below threshold; block heavy build scheduling")
    if not runners:
        notes.append("no runner registrations returned; GitHub data may be unavailable")
    if any(n["required"] and n["ci"] in ("OFFLINE", "UNREGISTERED") for n in nodes):
        notes.append("at least one required inventory node lacks an online CI runner; verify whether it is intended for CI")
    return {"generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "local": capacity, "nodes": sorted(nodes, key=lambda x: x["name"]),
            "notes": notes, "mode": "read-only"}


def fetch_runners(repo, token):
    """List runner registrations; pagination is bounded and errors are explicit."""
    if not token:
        raise RuntimeError("GH_TOKEN not set; runner states cannot be verified")
    results = []
    for page in range(1, 21):
        url = f"https://api.github.com/repos/{repo}/actions/runners?per_page=100&page={page}"
        request = urllib.request.Request(url, headers={
            "Authorization": f"Bearer {token}", "Accept": "application/vnd.github+json",
            "User-Agent": "x3-ci-autopilot/0.1"})
        try:
            with urllib.request.urlopen(request, timeout=12) as response:
                payload = json.load(response)
        except (urllib.error.URLError, ValueError) as exc:
            raise RuntimeError("GitHub runner query failed (check token permission and connectivity)") from exc
        batch = payload.get("runners")
        if not isinstance(batch, list):
            raise RuntimeError("GitHub runner API response missing runners list")
        results.extend(batch)
        if len(batch) < 100:
            return results
    raise RuntimeError("GitHub runner list exceeded 2000 entries; refusing partial inventory")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, default=DEFAULT_INVENTORY)
    parser.add_argument("--repo", default=REPO)
    parser.add_argument("--runners-json", type=Path, help="offline runner API fixture; no token needed")
    parser.add_argument("--local-only", action="store_true", help="no GitHub calls; runner status is unknown")
    parser.add_argument("--min-free-gb", type=float, default=30)
    parser.add_argument("--output", type=Path, help="optional local JSON evidence path")
    args = parser.parse_args(argv)
    if args.local_only and args.runners_json:
        parser.error("--local-only and --runners-json are mutually exclusive")
    try:
        inventory = json.loads(args.inventory.read_text())
        if not isinstance(inventory.get("nodes"), dict):
            raise ValueError("inventory missing nodes map")
        if args.local_only:
            runners = []
        elif args.runners_json:
            source = json.loads(args.runners_json.read_text())
            runners = source["runners"] if isinstance(source, dict) else source
            if not isinstance(runners, list):
                raise ValueError("runner fixture must contain a list")
        else:
            runners = fetch_runners(args.repo, os.getenv("GH_TOKEN", ""))
        result = evaluate(inventory, runners, local_capacity(), args.min_free_gb)
        if args.local_only:
            for node in result["nodes"]:
                node["ci"] = "UNKNOWN"
            result["notes"] = [n for n in result["notes"] if "registrations" not in n and "required inventory" not in n]
            result["notes"].append("local-only mode: runner registration not checked")
        rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(rendered)
        sys.stdout.write(rendered)
        return 0
    except (OSError, ValueError, RuntimeError, KeyError) as exc:
        print(f"x3-ci-autopilot: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
