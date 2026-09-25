#!/usr/bin/env python3
"""Fail when a JS project has an npm advisory that is not accounted for.

`npm audit` is the JS half of the dependency gate that `.cargo/audit.toml` and `deny.toml`
are for the Rust half, and no gate ran it. Measured on 2026-09-25 across the 36 projects
with a committed `package-lock.json` plus the one pnpm project: 62 advisories in 8
projects, including a critical (`websocket-driver` in the app-store frontend) and nine
highs (`axios`, `form-data`, `picomatch`, `brace-expansion`, `ip-address`). None of it
was visible to any gate.

Policy, in order of what it protects:

  * `critical` and `high` findings always fail, and can never be added to the baseline --
    the baseline is not a place to park something that is actually dangerous.
  * `moderate` and `low` findings must either be fixed or listed in
    `security/npm-audit-baseline.json` with the project and package. The baseline is a
    ratchet in both directions: a finding that is not listed fails, and a listed finding
    that has gone away also fails, so an entry cannot outlive the exposure it describes.
  * a project that cannot be audited (no lockfile it can resolve, npm error, timeout)
    fails rather than being skipped silently.

Usage:
  scripts/check-npm-audit.py              # check, non-zero on drift
  scripts/check-npm-audit.py --list       # print every finding and exit
  scripts/check-npm-audit.py --update     # rewrite the baseline from today's findings
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASELINE = ROOT / "security" / "npm-audit-baseline.json"
SKIP_DIRS = {"node_modules", "target", ".git"}
PER_PROJECT_TIMEOUT = 300

# Findings at these severities are never acceptable, baseline or not.
FATAL = ("critical", "high")


def projects():
    """Every directory holding a tracked package-lock.json, plus pnpm projects."""
    listing = subprocess.run(
        ["git", "ls-files", "*package-lock.json", "*/pnpm-lock.yaml"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout.split()
    found = {}
    for entry in listing:
        path = pathlib.Path(entry)
        if any(part in SKIP_DIRS for part in path.parts):
            continue
        directory = path.parent if str(path.parent) != "." else pathlib.Path(".")
        if "package-lock.json" in entry:
            found[str(directory)] = "npm"
        else:
            found[str(directory)] = "pnpm"
    return sorted(found.items())


def audit(project, manager):
    if manager == "npm":
        command = ["npm", "audit", "--json"]
    else:
        command = ["corepack", "pnpm", "audit", "--json"]
    try:
        completed = subprocess.run(
            command, cwd=ROOT / project, capture_output=True, text=True,
            timeout=PER_PROJECT_TIMEOUT,
        )
    except subprocess.TimeoutExpired:
        return None, "timed out after %ds" % PER_PROJECT_TIMEOUT
    except OSError as error:
        return None, "could not run %s: %s" % (command[0], error)
    text = completed.stdout.strip()
    if not text:
        tail = (completed.stderr or "").strip().splitlines()[-2:]
        return None, "no output (exit %s): %s" % (completed.returncode, " | ".join(tail))
    try:
        data = json.loads(text)
    except json.JSONDecodeError as error:
        return None, "unreadable JSON (%s)" % error
    findings = {}
    for name, entry in (data.get("vulnerabilities") or {}).items():
        severity = entry.get("severity")
        if severity and severity != "info":
            findings[name] = severity
    return findings, None


def load_baseline():
    if not BASELINE.exists():
        sys.exit("check-npm-audit: missing %s" % BASELINE.relative_to(ROOT))
    return json.loads(BASELINE.read_text())


def main():
    parser = argparse.ArgumentParser(description="Check npm advisories against the baseline.")
    parser.add_argument("--list", action="store_true", help="print findings and exit")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline from today's findings")
    args = parser.parse_args()

    results = {}
    failures = []
    for project, manager in projects():
        findings, error = audit(project, manager)
        if error:
            failures.append("%s: %s" % (project, error))
            continue
        results[project] = findings

    if args.list:
        for project in sorted(results):
            for name, severity in sorted(results[project].items()):
                print("%-46s %-9s %s" % (project, severity, name))
        print("\nprojects audited: %d, findings: %d" % (
            len(results), sum(len(v) for v in results.values())))
        return 0

    if args.update:
        baseline = load_baseline()
        baseline["allowed"] = {
            project: {name: sev for name, sev in sorted(results[project].items())}
            for project in sorted(results) if results[project]
        }
        BASELINE.write_text(json.dumps(baseline, indent=2) + "\n")
        print("check-npm-audit: wrote %s" % BASELINE.relative_to(ROOT))
        return 0

    baseline = load_baseline().get("allowed", {})

    for project in sorted(results):
        current = results[project]
        allowed = baseline.get(project, {})
        for name, severity in sorted(current.items()):
            if severity in FATAL:
                failures.append(
                    "%s: %s is %s; this severity is never allowed, fix it or remove the "
                    "dependency" % (project, name, severity)
                )
                continue
            if name not in allowed:
                failures.append(
                    "%s: %s is %s and is not in the baseline; fix it, or add it to %s "
                    "with a reason" % (project, name, severity, BASELINE.relative_to(ROOT))
                )
        for name in sorted(set(allowed) - set(current)):
            failures.append(
                "%s: the baseline lists %s, but npm no longer reports it; remove the "
                "entry so the baseline keeps describing the current tree" % (project, name)
            )

    for project in sorted(set(baseline) - set(results)):
        failures.append(
            "%s: the baseline has entries for a project that was not audited; either the "
            "project lost its lockfile or it failed to audit" % project
        )

    if failures:
        for failure in failures:
            print("check-npm-audit: FAIL: %s" % failure, file=sys.stderr)
        return 1

    total = sum(len(v) for v in results.values())
    print(
        "check-npm-audit: OK - %d project(s) audited, %d baseline finding(s), "
        "no critical/high" % (len(results), total)
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
