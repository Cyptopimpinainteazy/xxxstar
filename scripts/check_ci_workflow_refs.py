#!/usr/bin/env python3
"""Local CI gate - prove the workflow wiring in this repo is real.

GitHub-hosted Actions cannot run for this account (step-less runs, 2-4s), so a
workflow that invokes a script that does not exist, or that is wired to a branch
this repo does not have, stays invisible until a self-hosted run reaches it -
which may be never. That is the same class of break that let the #208
CARGO_TARGET_DIR regression through: the change moved where an artefact lands
and the workflow/script pair stopped matching.

Two verdicts, and they are deliberately different:

  HARD FAIL - a workflow invokes a script, in-repo action or make target that is
              not in the working tree. The step cannot work for anybody.
  REPORT    - a workflow can never fire (its branch filter does not include the
              default branch) or is hosted-only (cannot execute on this
              account). Printed, recorded in --json, not a hard failure: hosted
              routing is a separate decision, and the local CI is what covers it.

What counts as an invocation (and nothing else - file *arguments* such as
`--baseline reports/x.json` or `cp foo.js public/` are ignored on purpose):
  bash|sh|dash|source|python|node <flags> <path>
  ./path  at command position
  make <target> / $(MAKE) <target>
  uses: ./local/action   (step and job level)

Usage:
  scripts/check_ci_workflow_refs.py            # check, non-zero on missing refs
  scripts/check_ci_workflow_refs.py --parity   # + runner/trigger coverage report
  scripts/check_ci_workflow_refs.py --json     # machine-readable report
  scripts/check_ci_workflow_refs.py --quiet    # only print failures
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
WORKFLOW_DIR = ROOT / ".github" / "workflows"

PATH = r"[\w.@+-]+(?:/[\w.@+-]+)*\.(?:sh|py|js|mjs|cjs)"
# `bash -euo pipefail scripts/x.sh`, `python3 -q tools/y.py`, ...
INVOKE_RE = re.compile(
    rf"\b(?:bash|sh|dash|source|python3?|node|deno|pwsh)"
    rf"\s+(?:-[A-Za-z0-9-]+\s+)*({PATH})"
)
# `./scripts/x.sh --flag` at command position
EXEC_RE = re.compile(rf"(?:^|&&\s*|\|\|\s*|[;&|(]\s*)(\./{PATH})", re.M)
# make <target> / $(MAKE) <target>
MAKE_RE = re.compile(
    r"(?:\$\(\s*MAKE\s*\)|(?<![\w./-])make)"
    r"\s+(?:-C\s+\S+\s+|-\S+\s+|--\S+\s+)*"
    r"([a-zA-Z0-9_][a-zA-Z0-9_.-]*)"
)
# `cd <dir>` inside the same run block makes following relative paths resolvable.
CD_RE = re.compile(r"\bcd\s+(?:\"([^\"]+)\"|'([^']+)'|([^\s;&|<>]+))")
LOCAL_USES_RE = re.compile(r"^\./")

HOSTED_RUNNER_RE = re.compile(r"^(ubuntu|macos|windows)-[\w.-]+$")
HEREDOC_RE = re.compile(r"<<-?\s*['\"]?(\w+)['\"]?")
# Lines that only *print* a command are not invocations.
NOISE_PREFIXES = ("echo ", "echo\t", "printf ", "#")
# Events that fire regardless of a branch filter.
UNFILTERED_EVENTS = (
    "schedule",
    "workflow_dispatch",
    "workflow_call",
    "release",
    "repository_dispatch",
    "merge_group",
    "workflow_run",
    "issue_comment",
    "deployment",
)
BRANCH_EVENTS = ("push", "pull_request", "pull_request_target")


def workflow_files() -> list[pathlib.Path]:
    return sorted(WORKFLOW_DIR.glob("*.y*ml"))


def load_yaml(path: pathlib.Path):
    try:
        import yaml
    except ModuleNotFoundError:  # pragma: no cover - environment dependent
        print(
            "check_ci_workflow_refs: PyYAML is required (pip install pyyaml)",
            file=sys.stderr,
        )
        raise SystemExit(2)
    try:
        return yaml.safe_load(path.read_text())
    except Exception as exc:  # noqa: BLE001 - report the file, not a traceback
        print(f"check_ci_workflow_refs: cannot parse {path}: {exc}", file=sys.stderr)
        raise SystemExit(2)


def default_branch() -> str:
    """The repo's default branch, as origin reports it (falls back to master)."""
    for args in (
        ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        ["rev-parse", "--abbrev-ref", "origin/HEAD"],
    ):
        try:
            result = subprocess.run(
                ["git", *args], cwd=ROOT, text=True, capture_output=True, check=False
            )
        except OSError:
            break
        if result.returncode == 0:
            name = result.stdout.strip().rsplit("/", 1)[-1]
            if name and name != "HEAD":
                return name
    return "master"


def events_of(doc: dict) -> dict:
    """`on:` is parsed by PyYAML as boolean True - accept both spellings."""
    if not isinstance(doc, dict):
        return {}
    on = doc.get("on", doc.get(True))  # noqa: FBT003 - YAML key
    if isinstance(on, str):
        return {on: None}
    if isinstance(on, list):
        return {entry: None for entry in on if isinstance(entry, str)}
    return on if isinstance(on, dict) else {}


def event_branches(events: dict, event_names: tuple[str, ...]) -> list[str] | None:
    """Branch filter for these events, or None when there is no filter at all."""
    branches: set[str] = set()
    saw_event = False
    for event in event_names:
        config = events.get(event)
        if config is None and event not in events:
            continue
        saw_event = True
        if not isinstance(config, dict):
            continue  # event present with no configuration == no branch filter
        for key in ("branches", "branches-ignore"):
            value = config.get(key)
            if isinstance(value, str):
                branches.add(value)
            elif isinstance(value, list):
                branches.update(entry for entry in value if isinstance(entry, str))
    if not saw_event:
        return []
    if not branches:
        return None
    return sorted(branches)


def strip_dot_slash(value: str) -> str:
    """`./scripts/x.sh` -> `scripts/x.sh`, leaving `.github/...` intact."""
    while value.startswith("./"):
        value = value[2:]
    return value


def makefile_targets() -> set[str]:
    """Every target name declared in the repo's root Makefile.

    Workflows drive the root Makefile; nested Makefiles are not wired to CI, so
    accepting a target that only exists there would hide a real break.
    """
    targets: set[str] = set()
    path = ROOT / "Makefile"
    if not path.exists():
        return targets
    target_re = re.compile(r"^([a-zA-Z0-9_][a-zA-Z0-9_.-]*)\s*:(?!=)")
    for line in path.read_text().splitlines():
        match = target_re.match(line)
        if match:
            targets.add(match.group(1))
    return targets


def resolvable(raw: str, cds: list[str]) -> bool:
    raw = strip_dot_slash(raw.strip().strip("\"'"))
    if not raw or "$" in raw or "{" in raw:
        return True  # computed path - not a static reference we can check
    candidates = [ROOT / raw]
    for directory in cds:
        base = (ROOT / directory).resolve()
        if base != ROOT and ROOT not in base.parents:
            continue  # a cd escaping the repo is not our business
        candidates.append(base / raw)
    return any(candidate.exists() for candidate in candidates)


def working_dirs(node) -> list[str]:
    """`working-directory` values a job/step inherits or sets."""
    if not isinstance(node, dict):
        return []
    found: list[str] = []
    defaults = node.get("defaults")
    if isinstance(defaults, dict):
        run = defaults.get("run")
        if isinstance(run, dict):
            value = run.get("working-directory")
            if isinstance(value, str):
                found.append(value)
    value = node.get("working-directory")
    if isinstance(value, str):
        found.append(value)
    return found


def command_lines(script: str) -> list[str]:
    """Drop comments, echo/printf noise, and heredoc bodies from a run block."""
    lines: list[str] = []
    heredoc: str | None = None
    for line in script.splitlines():
        stripped = line.strip()
        if heredoc is not None:
            if stripped == heredoc:
                heredoc = None
            continue
        if stripped.startswith(NOISE_PREFIXES):
            continue
        match = HEREDOC_RE.search(line)
        if match:
            heredoc = match.group(1)
        lines.append(line)
    return lines


def collect(path: pathlib.Path, known_targets: set[str], branch: str):
    doc = load_yaml(path) or {}
    rel = path.relative_to(ROOT).as_posix()
    events = events_of(doc)
    scripts: list[str] = []
    actions: list[str] = []
    targets: list[str] = []
    missing: list[str] = []
    workflow_dirs = working_dirs(doc)
    hosted_runners: set[str] = set()
    self_hosted = False

    for directory in workflow_dirs:
        if not resolvable(directory, []):
            missing.append(f"{rel}: defaults: working-directory {directory}")

    for job in (doc.get("jobs") or {}).values():
        if not isinstance(job, dict):
            continue
        runners = job.get("runs-on")
        for entry in runners if isinstance(runners, list) else [runners]:
            if not isinstance(entry, str):
                continue
            if "self-hosted" in entry:
                self_hosted = True
            elif HOSTED_RUNNER_RE.match(entry):
                hosted_runners.add(entry)

        job_dirs = workflow_dirs + working_dirs(job)
        for directory in working_dirs(job):
            if not resolvable(directory, workflow_dirs):
                missing.append(f"{rel}: job working-directory {directory}")

        if isinstance(job.get("uses"), str) and LOCAL_USES_RE.match(job["uses"]):
            if resolvable(job["uses"], []):
                actions.append(job["uses"])
            else:
                missing.append(f"{rel}: uses: {job['uses']}")

        for step in job.get("steps") or []:
            if not isinstance(step, dict):
                continue
            if isinstance(step.get("uses"), str) and LOCAL_USES_RE.match(step["uses"]):
                if resolvable(step["uses"], []):
                    actions.append(step["uses"])
                else:
                    missing.append(f"{rel}: uses: {step['uses']}")
            for directory in working_dirs(step):
                if not resolvable(directory, job_dirs):
                    missing.append(f"{rel}: step working-directory {directory}")
            script = step.get("run")
            if not isinstance(script, str):
                continue
            lines = command_lines(script)
            bases = job_dirs + working_dirs(step)
            cds = bases + [
                next(group for group in match.groups() if group)
                for line in lines
                for match in CD_RE.finditer(line)
            ]
            for line in lines:
                for token in INVOKE_RE.findall(line) + EXEC_RE.findall(line):
                    if resolvable(token, cds):
                        scripts.append(token)
                    else:
                        missing.append(f"{rel}: run: {token}")
                for target in MAKE_RE.findall(line):
                    if target in known_targets:
                        targets.append(target)
                    else:
                        missing.append(f"{rel}: make: {target}")

    unfiltered = [event for event in UNFILTERED_EVENTS if event in events]
    unknown = sorted(set(events) - set(BRANCH_EVENTS) - set(UNFILTERED_EVENTS))
    has_push = "push" in events
    has_pr = "pull_request" in events or "pull_request_target" in events
    push_branches = event_branches(events, ("push",))
    pr_branches = event_branches(events, ("pull_request", "pull_request_target"))
    # A workflow can still fire if it has any unfiltered or unrecognised event.
    escapes_filter = bool(unfiltered) or bool(unknown)
    push_reachable = (
        (not has_push)
        or escapes_filter
        or push_branches is None
        or branch in push_branches
    )
    pr_reachable = (
        (not has_pr)
        or escapes_filter
        or pr_branches is None
        or branch in pr_branches
    )
    gaps = []
    if has_push and not push_reachable:
        gaps.append({"event": "push", "branches": push_branches or []})
    if has_pr and not pr_reachable:
        gaps.append({"event": "pull_request", "branches": pr_branches or []})
    unreachable = bool(gaps) and not push_reachable and not pr_reachable
    branches = sorted(set(push_branches or []) | set(pr_branches or []))

    return {
        "workflow": rel,
        "scripts": scripts,
        "actions": actions,
        "make_targets": targets,
        "hosted_runners": sorted(hosted_runners),
        "self_hosted": self_hosted,
        "branches": branches,
        "push_branches": push_branches or [],
        "pull_request_branches": pr_branches or [],
        "unfiltered_events": unfiltered,
        "unreachable_on_default_branch": unreachable,
        "trigger_gaps": gaps,
        "missing": missing,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parity", action="store_true", help="runner/trigger coverage report")
    parser.add_argument("--json", action="store_true", help="emit JSON report")
    parser.add_argument("--quiet", action="store_true", help="only print failures")
    args = parser.parse_args()

    branch = default_branch()
    targets = makefile_targets()
    records = [collect(path, targets, branch) for path in workflow_files()]

    script_refs = [ref for record in records for ref in record["scripts"]]
    action_refs = [ref for record in records for ref in record["actions"]]
    make_refs = [ref for record in records for ref in record["make_targets"]]
    missing = [item for record in records for item in record["missing"]]

    hosted_only = [
        record["workflow"]
        for record in records
        if record["hosted_runners"] and not record["self_hosted"]
    ]
    self_hosted = [record["workflow"] for record in records if record["self_hosted"]]
    unreachable = [
        {"workflow": record["workflow"], "branches": record["branches"]}
        for record in records
        if record["unreachable_on_default_branch"]
    ]
    trigger_gaps = [
        {"workflow": record["workflow"], **gap}
        for record in records
        for gap in record["trigger_gaps"]
    ]
    # A self-hosted workflow that can never fire is our own wiring, so it fails.
    dead_local = [
        item["workflow"] for item in unreachable if item["workflow"] in self_hosted
    ]

    report = {
        "default_branch": branch,
        "workflows": len(records),
        "script_refs": len(script_refs),
        "unique_scripts": sorted(set(script_refs)),
        "action_refs": len(action_refs),
        "make_targets": len(make_refs),
        "unique_make_targets": sorted(set(make_refs)),
        "hosted_only_workflows": hosted_only,
        "self_hosted_workflows": self_hosted,
        "unreachable_workflows": unreachable,
        "trigger_gaps": trigger_gaps,
        "dead_local_workflows": dead_local,
        "missing": missing,
        "ok": not missing and not dead_local,
    }

    if args.json:
        print(json.dumps(report, indent=2))
        return 0 if report["ok"] else 1

    if not args.quiet:
        print(f"CI wiring check (default branch: {branch})")
        print(f"  workflows parsed:      {len(records)}")
        print(
            f"  script refs checked:   {len(script_refs)} "
            f"({len(set(script_refs))} unique)"
        )
        print(f"  action refs checked:   {len(action_refs)}")
        print(
            f"  make targets checked:  {len(make_refs)} ({len(set(make_refs))} unique)"
        )
        print(f"  make targets in tree:  {len(targets)}")
        if args.parity:
            print(f"  self-hosted workflows: {len(self_hosted)}")
            print(f"  hosted-only workflows: {len(hosted_only)} (cannot execute here)")
            for name in hosted_only:
                print(f"    - {name}")
            print(f"  unreachable workflows: {len(unreachable)} (branch filter excludes {branch})")
            for item in unreachable:
                listed = ", ".join(item["branches"])
                print(f"    - {item['workflow']} (branches: {listed})")
            print(f"  trigger gaps:          {len(trigger_gaps)}")
            for item in trigger_gaps:
                listed = ", ".join(item["branches"]) or "none"
                print(f"    - {item['workflow']}: {item['event']} (branches: {listed})")
        print(f"  missing refs:          {len(missing)}")

    if missing or dead_local:
        if missing:
            print("\nMISSING REFERENCES (a workflow step would fail here):")
            for item in missing:
                print(f"  x {item}")
        if dead_local:
            print("\nDEAD SELF-HOSTED WORKFLOWS (wired to a branch this repo lacks):")
            for name in dead_local:
                print(f"  x {name}")
        return 1

    if not args.quiet:
        print("\nOK - every workflow reference resolves")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
