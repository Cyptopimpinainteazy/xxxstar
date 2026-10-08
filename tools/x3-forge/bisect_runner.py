#!/usr/bin/env python3
"""Run `git bisect run` for a failing reproducer and report the first bad commit.

When a gate or a simulator seed goes red on HEAD but was green on some known
commit, the expensive part is not the fix — it is finding *which commit* moved
the behaviour. `git bisect run` automates the search, but only when it is given
a reproducer whose exit status is honest, and only when the surrounding rules
are right: a dirty worktree must be refused (bisect checks out other commits,
and a stray `git checkout` would clobber it), a `good` revision that is not an
ancestor of `bad` must be refused (the search would be meaningless), and a
reproducer that does not actually distinguish the two ends must be refused
*before* the search starts, not after twenty rebuilds.

This tool is that wrapper. It refuses the three cases above, verifies the
reproducer at both ends, runs the search, resets the worktree, and emits a
packet-shaped JSON (plus a human summary) with the first bad commit, its
changed files, and a replay command — the same shape the X3 Forge
failure-packets use, so a regression bisect can be handed to the same
root-cause flow as a simulator failure.

Usage:
    python3 tools/x3-forge/bisect_runner.py \
        --good <known-good-rev> --bad HEAD \
        [--repo PATH] [--timeout SECS] [--out DIR] [--json] \
        -- <reproducer argv...>

    # or seed the search from a failure packet the rest of the forge already
    # produces (x3-sim's `x3-failure-packet-v2`, the gate wrapper's
    # `x3-gate-failure-packet-v1`):
    python3 tools/x3-forge/bisect_runner.py \
        --good <known-good-rev> --packet failure-packets/<id>.json

The reproducer is executed with `argv` directly — never through a shell — and
must exit 0 on `good` and non-zero on `bad`. Exit code 125 is passed through to
git as "skip this commit" (its documented meaning), and any run that exceeds
--timeout aborts the search rather than reporting a fabricated verdict.

A packet's `replay_command` is a shell string, so it runs through `bash -lc`.
Gate packets prefix it with `cd <original-checkout> &&`; that prefix is stripped
(and recorded as `packet_cd_stripped`), because every probe must run in the
checkout the search is moving, not in the checkout that recorded the failure.

Exit codes: 0 first bad commit found; 1 nothing usable (refused or search
aborted); 2 usage/IO error.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import time

TIMEOUT_MARKER_ENV = "X3_BISECT_TIMEOUT_MARKER"

PACKET_SCHEMAS = ("x3-failure-packet-v2", "x3-gate-failure-packet-v1")

# `shlex.join` quotes the directory, so accept bare, single- and double-quoted
# forms; only a single leading `cd … &&` is stripped.
_CD_PREFIX = re.compile(r"""^cd\s+(?:'[^']*'|"[^"]*"|[^\s&]+)\s+&&\s+""")


def reproducer_from_packet(data: dict) -> tuple[str, str, bool]:
    """Return (shell command, failing commit, cd prefix stripped)."""
    schema = str(data.get("schema", ""))
    if schema not in PACKET_SCHEMAS:
        fail(
            f"packet schema {schema!r} is not one this tool understands "
            f"(expected one of {', '.join(PACKET_SCHEMAS)})"
        )
    command = ""
    if schema == "x3-failure-packet-v2":
        # Prefer the verified minimal reproducer: it is the smallest run that
        # still violates the invariant, and the generator only attaches it
        # after re-running it.
        minimized = data.get("minimized") or {}
        command = str(minimized.get("replay_command") or data.get("replay_command") or "")
    else:
        command = str(data.get("replay_command") or "")
    command = command.strip()
    if not command:
        fail("packet has no replay_command; there is nothing to bisect with")
    stripped = _CD_PREFIX.sub("", command, count=1)
    return stripped, str(data.get("commit") or "").strip(), stripped != command


def git(repo: pathlib.Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=check,
    )


def rev_parse(repo: pathlib.Path, rev: str) -> str:
    proc = git(repo, "rev-parse", "--verify", f"{rev}^{{commit}}", check=False)
    if proc.returncode != 0:
        fail(f"{rev!r} does not resolve to a commit in {repo}")
    return proc.stdout.strip()


def fail(message: str) -> None:
    print(f"bisect_runner: REFUSED: {message}", file=sys.stderr)
    raise SystemExit(1)


def ensure_clean(repo: pathlib.Path) -> None:
    # Modified *tracked* files are refused: `git checkout` carries them into
    # every step of the search, so the reproducer would run against a tree that
    # matches no commit, and the verdict would describe a state nobody can
    # reproduce. Untracked files (build output, runlogs) are tolerated: the
    # checkouts do not need to overwrite them, and refusing on them would make
    # the tool unusable on any real worktree.
    dirty = [
        line
        for line in git(repo, "status", "--porcelain").stdout.splitlines()
        if line.strip() and not line.startswith("??")
    ]
    if dirty:
        fail(
            "the worktree has uncommitted changes to tracked files; bisect "
            "checks out other commits and would run the reproducer against a "
            "tree that matches no commit. Commit or stash first. "
            "git status --porcelain says:\n" + "\n".join(dirty)
        )


def ensure_no_bisect_in_progress(repo: pathlib.Path) -> None:
    head_name = git(repo, "rev-parse", "--git-path", "BISECT_START", check=False).stdout.strip()
    if head_name and pathlib.Path(head_name if pathlib.Path(head_name).is_absolute()
                                 else repo / head_name).exists():
        fail("a git bisect is already in progress in this repository; run "
             "`git bisect reset` first")


def ensure_ancestor(repo: pathlib.Path, good: str, bad: str) -> None:
    proc = subprocess.run(
        ["git", "-C", str(repo), "merge-base", "--is-ancestor", good, bad],
        capture_output=True,
    )
    if proc.returncode != 0:
        fail(f"--good {good[:12]} is not an ancestor of --bad {bad[:12]}; "
             "bisect needs a linear range")


def run_repro(
    repo: pathlib.Path, argv: list[str], timeout: float
) -> tuple[int | None, str, float]:
    """Return (exit status or None on timeout, output tail, seconds)."""
    started = time.monotonic()
    try:
        proc = subprocess.run(
            argv,
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as expired:
        elapsed = time.monotonic() - started
        out = (expired.stdout or "") + (expired.stderr or "")
        return None, out[-2000:], elapsed
    except OSError as error:
        fail(f"could not execute the reproducer {argv[0]!r}: {error}")
    elapsed = time.monotonic() - started
    return proc.returncode, (proc.stdout + proc.stderr)[-2000:], elapsed


def verify_endpoints(
    repo: pathlib.Path, good: str, bad: str, argv: list[str], timeout: float
) -> None:
    original = git(repo, "rev-parse", "HEAD").stdout.strip()
    original_ref = git(repo, "symbolic-ref", "--quiet", "HEAD", check=False).stdout.strip()
    try:
        for rev, want_zero, label in ((good, True, "good"), (bad, False, "bad")):
            git(repo, "checkout", "--quiet", rev)
            status, out, _ = run_repro(repo, argv, timeout)
            if want_zero and status != 0:
                fail(
                    f"the reproducer does not pass on --{label} {rev[:12]} "
                    f"(exit {status!r}); a bisect from a broken known-good end "
                    f"would blame the wrong commit. Output tail:\n{out}"
                )
            if not want_zero and status == 0:
                fail(
                    f"the reproducer passes on --{label} {rev[:12]} "
                    "(exit 0); there is no regression to bisect"
                )
    finally:
        # `git checkout refs/heads/x` detaches HEAD at x's commit; checkout the
        # short branch name so the worktree is left on the branch it started on
        # and `git bisect reset` can return to it too.
        target = original_ref.removeprefix("refs/heads/") if original_ref else original
        git(repo, "checkout", "--quiet", target)


def bisect(
    repo: pathlib.Path, good: str, bad: str, argv: list[str], timeout: float
) -> dict:
    git(repo, "bisect", "start")
    # A timed-out probe must abort the search, but the exit code that means
    # "abort" to `git bisect run` is version-dependent (git 2.34 treats 127 as
    # "bad", later versions treat it as "abort"). So the timeout is recorded
    # out-of-band in a marker file and the caller discards whatever verdict the
    # search reached once a probe has timed out.
    handle, marker_name = tempfile.mkstemp(prefix="x3-bisect-timeout-", suffix=".marker")
    os.close(handle)
    marker = pathlib.Path(marker_name)
    marker.unlink()
    env = dict(os.environ, **{TIMEOUT_MARKER_ENV: str(marker)})
    try:
        git(repo, "bisect", "bad", bad)
        git(repo, "bisect", "good", good)
        # `git bisect run` treats 125 as "skip", 127 as "abort", 0 as good and
        # any other status as bad. The timeout wrapper below turns a hung run
        # into 127 (abort) rather than letting the search block forever.
        wrapper = [
            sys.executable,
            str(pathlib.Path(__file__).resolve()),
            "--internal-timeout-run",
            str(timeout),
            *argv,
        ]
        log_before = git(repo, "bisect", "log").stdout
        search = subprocess.run(
            ["git", "-C", str(repo), "bisect", "run", *wrapper],
            capture_output=True,
            text=True,
            env=env,
        )
        log_after = git(repo, "bisect", "log").stdout
        first_bad = git(repo, "rev-parse", "refs/bisect/bad", check=False).stdout.strip()
        timed_out_at = marker.read_text().strip() if marker.exists() else ""
        steps = [
            line for line in log_after.splitlines() if line.startswith("# ")
        ]
        return {
            "search_exit": search.returncode,
            "search_output": (search.stdout + search.stderr)[-4000:],
            "first_bad": first_bad,
            "timed_out_at": timed_out_at,
            "steps": len(steps),
            "log_before_lines": len(log_before.splitlines()),
        }
    finally:
        git(repo, "bisect", "reset", check=False)
        marker.unlink(missing_ok=True)


def commit_info(repo: pathlib.Path, sha: str, name_only: bool = False) -> str:
    if name_only:
        return git(repo, "show", "--name-status", "--format=", sha).stdout.strip()
    return git(repo, "show", "--no-patch", "--format=%H%n%an <%ae>%n%ad%n%s", sha).stdout.strip()


def build_packet(repo: pathlib.Path, good: str, bad: str, argv: list[str], result: dict) -> dict:
    timed_out = bool(result.get("timed_out_at"))
    # A search that probed a hung commit produced a verdict from a tree that
    # never answered; drop it rather than blame a commit nobody can reproduce.
    first_bad = result["first_bad"] if not timed_out else ""
    info = commit_info(repo, first_bad).splitlines() if first_bad else []
    changed = commit_info(repo, first_bad, name_only=True) if first_bad else ""
    parent = git(repo, "rev-parse", f"{first_bad}^", check=False).stdout.strip() if first_bad else ""
    return {
        "schema": "x3-bisect-packet-v1",
        "kind": "regression-bisect",
        "repro_command": " ".join(argv),
        "good_requested": good,
        "bad_requested": bad,
        "last_good_commit": parent,
        "first_bad_commit": first_bad,
        "first_bad_subject": info[3] if len(info) > 3 else "",
        "first_bad_author": info[1] if len(info) > 1 else "",
        "first_bad_date": info[2] if len(info) > 2 else "",
        "changed_files": [line for line in changed.splitlines() if line.strip()],
        "search_steps": result["steps"],
        "search_exit": result["search_exit"],
        "timed_out_at": result.get("timed_out_at", ""),
        "verified": bool(first_bad) and result["search_exit"] == 0 and not timed_out,
        "replay_command": f"git checkout {first_bad} && {' '.join(argv)}",
        "suggested_next": (
            "inspect changed_files, add the reproducer as a regression test, "
            "then fix; re-run the bisect after the fix to confirm the range "
            "collapses"
        ),
    }


def render_markdown(packet: dict) -> str:
    lines = [
        "# X3 regression bisect",
        "",
        f"- reproducer: `{packet['repro_command']}`",
        f"- range: `{packet['good_requested'][:12]}` (good) .. `{packet['bad_requested'][:12]}` (bad)",
        f"- steps: {packet['search_steps']}",
        "",
    ]
    if packet["first_bad_commit"]:
        lines += [
            f"## First bad commit: `{packet['first_bad_commit'][:12]}`",
            "",
            f"    {packet['first_bad_subject']}",
            f"    {packet['first_bad_author']}  {packet['first_bad_date']}",
            "",
            "### Changed files",
            "",
        ]
        lines += [f"- `{entry}`" for entry in packet["changed_files"]] or ["- (none recorded)"]
        lines += [
            "",
            "### Replay",
            "",
            f"    {packet['replay_command']}",
            "",
        ]
    else:
        lines += ["No first-bad commit was produced; the search aborted.", ""]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="git bisect run wrapper that emits an X3 failure packet",
        usage="bisect_runner.py --good REV [--bad REV | --packet FILE] [--repo PATH] "
              "[--timeout SECS] [--out DIR] [--json] -- REPRODUCER [ARGS...]",
    )
    parser.add_argument("--good", required=False, help="known-good revision")
    parser.add_argument("--bad", required=False, default=None,
                        help="known-bad revision (default: the packet's commit, else HEAD)")
    parser.add_argument("--packet",
                        help="failure packet (x3-failure-packet-v2 or x3-gate-failure-packet-v1) "
                             "whose replay command and commit seed the search")
    parser.add_argument("--repo", default=".", help="repository to search (default: cwd)")
    parser.add_argument("--timeout", type=float, default=1800.0,
                        help="seconds allowed per reproducer run (default 1800)")
    parser.add_argument("--out", help="directory for bisect-<id>.json and .md")
    parser.add_argument("--json", action="store_true", help="print the packet to stdout")
    parser.add_argument("--internal-timeout-run", type=float, default=None,
                        help=argparse.SUPPRESS)
    parser.add_argument("repro", nargs=argparse.REMAINDER,
                        help="reproducer argv after --")
    args = parser.parse_args()

    if args.internal_timeout_run is not None:
        # Internal helper mode invoked by `git bisect run`: execute the real
        # reproducer with a wall-clock bound and map a timeout to 127 (abort).
        argv = args.repro[1:] if args.repro and args.repro[0] == "--" else args.repro
        status, _, _ = run_repro(pathlib.Path.cwd(), argv, args.internal_timeout_run)
        if status is None:
            marker = os.environ.get(TIMEOUT_MARKER_ENV)
            if marker:
                head = git(pathlib.Path.cwd(), "rev-parse", "HEAD", check=False).stdout.strip()
                pathlib.Path(marker).write_text(f"{head}\n")
            return 127
        return status

    if not args.good:
        parser.error("--good is required")
    argv = args.repro[1:] if args.repro and args.repro[0] == "--" else args.repro
    source_packet = None
    packet_commit = ""
    if args.packet:
        if argv:
            parser.error("pass either --packet or an explicit reproducer after `--`, not both")
        packet_path = pathlib.Path(args.packet)
        try:
            data = json.loads(packet_path.read_text())
        except OSError as error:
            print(f"bisect_runner: cannot read packet {packet_path}: {error}", file=sys.stderr)
            return 2
        except json.JSONDecodeError as error:
            fail(f"packet {packet_path} is not valid JSON: {error}")
        command, packet_commit, cd_stripped = reproducer_from_packet(data)
        argv = ["bash", "-lc", command]
        source_packet = {
            "path": str(packet_path),
            "schema": str(data.get("schema", "")),
            "failure_id": str(data.get("failure_id", "")),
            "cd_prefix_stripped": cd_stripped,
        }
    if not argv:
        parser.error("no reproducer given; pass it after `--` or seed one with --packet")

    repo = pathlib.Path(args.repo).resolve()
    if not git(repo, "rev-parse", "--git-dir", check=False).stdout.strip():
        print(f"bisect_runner: {repo} is not a git repository", file=sys.stderr)
        return 2

    ensure_clean(repo)
    ensure_no_bisect_in_progress(repo)
    good = rev_parse(repo, args.good)
    bad = rev_parse(repo, args.bad or packet_commit or "HEAD")
    if good == bad:
        fail("--good and --bad resolve to the same commit")
    ensure_ancestor(repo, good, bad)
    verify_endpoints(repo, good, bad, argv, args.timeout)

    result = bisect(repo, good, bad, argv, args.timeout)
    packet = build_packet(repo, good, bad, argv, result)
    if source_packet:
        packet["source_packet"] = source_packet

    if args.out:
        out_dir = pathlib.Path(args.out)
        out_dir.mkdir(parents=True, exist_ok=True)
        slug = (packet["first_bad_commit"] or "unknown")[:12]
        (out_dir / f"bisect-{slug}.json").write_text(json.dumps(packet, indent=2) + "\n")
        (out_dir / f"bisect-{slug}.md").write_text(render_markdown(packet))
    if args.json:
        print(json.dumps(packet, indent=2))
    else:
        sys.stdout.write(render_markdown(packet))

    if not packet["verified"]:
        if result.get("timed_out_at"):
            print("bisect_runner: the reproducer did not finish (timed out) at "
                  f"{result['timed_out_at'][:12]}; the search verdict is "
                  "discarded rather than guessed. Raise --timeout or make the "
                  "reproducer terminate.", file=sys.stderr)
            return 1
        print("bisect_runner: search did not finish with a first-bad commit "
              f"(exit {result['search_exit']}); output tail:\n{result['search_output']}",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
