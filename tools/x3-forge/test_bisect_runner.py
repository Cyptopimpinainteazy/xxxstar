#!/usr/bin/env python3
"""Tests for the regression bisect runner.

The properties worth pinning are the refusals. A bisect that runs from a
broken end blames the wrong commit and reads exactly like a correct answer, so
the tests build small synthetic repositories and check that the tool refuses a
dirty tree, a non-ancestor range, a reproducer that passes on the bad end and
one that fails on the good end — and that a real search still finds the first
bad commit, restores the worktree afterwards, and aborts (rather than guesses)
when the reproducer hangs.

Everything runs against throwaway repositories under a temporary directory, so
the suite is deterministic and needs no network.
"""

import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

TOOL = pathlib.Path(__file__).with_name("bisect_runner.py")


def run_git(repo: pathlib.Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def commit_file(repo: pathlib.Path, name: str, content: str, message: str) -> str:
    (repo / name).write_text(content)
    run_git(repo, "add", name)
    run_git(repo, "commit", "-q", "-m", message)
    return run_git(repo, "rev-parse", "HEAD")


# A reproducer that exits 0 while version.txt is not "broken" and 1 once it
# is: the regression is the single commit that writes "broken".
REPRO = [sys.executable, "-c",
         "import sys; sys.exit(0 if open('version.txt').read().strip() != 'broken' else 1)"]


class BisectFixture(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = pathlib.Path(self.tmp.name) / "repo"
        self.repo.mkdir()
        run_git(self.repo, "init", "-q", "-b", "master")
        run_git(self.repo, "config", "user.email", "test@x3.local")
        run_git(self.repo, "config", "user.name", "x3 test")
        self.commits = []
        for index in range(1, 5):
            self.commits.append(
                commit_file(self.repo, "version.txt", f"{index}", f"c{index}")
            )
        self.bad_sha = commit_file(self.repo, "version.txt", "broken", "c5-bad")
        for index in range(6, 9):
            self.commits.append(
                commit_file(self.repo, "changelog.txt", f"attempt {index}", f"c{index}")
            )
        self.head = run_git(self.repo, "rev-parse", "HEAD")

    def tearDown(self):
        self.tmp.cleanup()

    def run_tool(self, *args: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(TOOL), "--repo", str(self.repo), *args],
            capture_output=True,
            text=True,
        )

    # ── the happy path ───────────────────────────────────────────────────

    def test_finds_the_first_bad_commit(self):
        result = self.run_tool(
            "--good", "master~4", "--bad", "HEAD", "--json", "--", *REPRO
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        packet = json.loads(result.stdout)
        self.assertEqual(packet["first_bad_commit"], self.bad_sha)
        self.assertEqual(packet["last_good_commit"], self.commits[3])
        self.assertTrue(packet["verified"])
        self.assertIn("version.txt", " ".join(packet["changed_files"]))
        self.assertIn(self.bad_sha, packet["replay_command"])
        self.assertGreaterEqual(packet["search_steps"], 2)

    def test_worktree_is_restored_and_bisect_state_cleaned(self):
        self.run_tool("--good", "master~4", "--bad", "HEAD", "--", *REPRO)
        self.assertEqual(run_git(self.repo, "rev-parse", "HEAD"), self.head)
        self.assertEqual(run_git(self.repo, "symbolic-ref", "HEAD"), "refs/heads/master")
        self.assertFalse((self.repo / ".git" / "BISECT_START").exists())

    def test_untracked_files_do_not_block_the_search(self):
        (self.repo / "runlog.txt").write_text("untracked build output\n")
        result = self.run_tool(
            "--good", "master~4", "--bad", "HEAD", "--json", "--", *REPRO
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["first_bad_commit"], self.bad_sha)

    # ── refusals ─────────────────────────────────────────────────────────

    def test_refuses_a_dirty_tracked_worktree(self):
        (self.repo / "version.txt").write_text("locally edited")
        result = self.run_tool("--good", "master~4", "--bad", "HEAD", "--", *REPRO)
        self.assertEqual(result.returncode, 1)
        self.assertIn("uncommitted changes", result.stderr)
        self.assertNotIn("BISECT_START", result.stdout)

    def test_refuses_a_good_that_is_not_an_ancestor(self):
        run_git(self.repo, "checkout", "-q", "-b", "side", "master~5")
        side = commit_file(self.repo, "version.txt", "side", "side commit")
        run_git(self.repo, "checkout", "-q", "master")
        result = self.run_tool("--good", side, "--bad", "HEAD", "--", *REPRO)
        self.assertEqual(result.returncode, 1)
        self.assertIn("not an ancestor", result.stderr)

    def test_refuses_when_reproducer_passes_on_the_bad_end(self):
        result = self.run_tool(
            "--good", "master~4", "--bad", "HEAD",
            "--", sys.executable, "-c", "import sys; sys.exit(0)",
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("passes on --bad", result.stderr)

    def test_refuses_when_reproducer_fails_on_the_good_end(self):
        result = self.run_tool(
            "--good", "master~4", "--bad", "HEAD",
            "--", sys.executable, "-c", "import sys; sys.exit(1)",
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("does not pass on --good", result.stderr)

    def test_refuses_when_good_equals_bad(self):
        result = self.run_tool("--good", "HEAD", "--bad", "HEAD", "--", *REPRO)
        self.assertEqual(result.returncode, 1)
        self.assertIn("same commit", result.stderr)

    def test_refuses_when_a_bisect_is_already_in_progress(self):
        run_git(self.repo, "bisect", "start")
        run_git(self.repo, "bisect", "bad", self.head)
        try:
            result = self.run_tool("--good", "master~4", "--bad", "HEAD", "--", *REPRO)
            self.assertEqual(result.returncode, 1)
            self.assertIn("already in progress", result.stderr)
        finally:
            run_git(self.repo, "bisect", "reset")

    # ── hung reproducer ──────────────────────────────────────────────────

    # ── failure-packet input ─────────────────────────────────────────────

    def write_packet(self, packet: dict) -> str:
        path = pathlib.Path(self.tmp.name) / "packet.json"
        path.write_text(json.dumps(packet))
        return str(path)

    def test_a_gate_packet_seeds_the_search(self):
        # The gate wrapper's packets replay with `cd <original-root> && …`;
        # that prefix must be stripped or every probe would run in the
        # original checkout instead of the commit under test.
        packet = self.write_packet({
            "schema": "x3-gate-failure-packet-v1",
            "failure_id": "deadbeefdeadbeef",
            "commit": self.head,
            "replay_command": (
                f"cd '{self.repo}' && {sys.executable} -c "
                "\"import sys; sys.exit(0 if open('version.txt').read().strip() != 'broken' else 1)\""
            ),
        })
        result = self.run_tool("--good", "master~4", "--packet", packet, "--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        out = json.loads(result.stdout)
        self.assertEqual(out["first_bad_commit"], self.bad_sha)
        self.assertEqual(out["bad_requested"], self.head)
        self.assertEqual(out["source_packet"]["failure_id"], "deadbeefdeadbeef")
        self.assertTrue(out["source_packet"]["cd_prefix_stripped"])

    def test_a_sim_packet_prefers_the_minimized_reproducer(self):
        # replay_command passes everywhere; only the minimized form
        # distinguishes the ends. If the tool used the wrong one, the
        # endpoint verification would refuse the search.
        packet = self.write_packet({
            "schema": "x3-failure-packet-v2",
            "commit": self.head,
            "replay_command": f"{sys.executable} -c \"import sys; sys.exit(0)\"",
            "minimized": {"replay_command": (
                f"{sys.executable} -c "
                "\"import sys; sys.exit(0 if open('version.txt').read().strip() != 'broken' else 1)\""
            )},
        })
        result = self.run_tool("--good", "master~4", "--packet", packet, "--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["first_bad_commit"], self.bad_sha)

    def test_packet_without_a_replay_command_is_refused(self):
        packet = self.write_packet({
            "schema": "x3-gate-failure-packet-v1",
            "commit": self.head,
        })
        result = self.run_tool("--good", "master~4", "--packet", packet)
        self.assertEqual(result.returncode, 1)
        self.assertIn("no replay_command", result.stderr)

    def test_unknown_packet_schema_is_refused(self):
        packet = self.write_packet({
            "schema": "x3-failure-packet-v9",
            "commit": self.head,
            "replay_command": "true",
        })
        result = self.run_tool("--good", "master~4", "--packet", packet)
        self.assertEqual(result.returncode, 1)
        self.assertIn("not one this tool understands", result.stderr)

    def test_a_packet_commit_missing_from_the_repo_is_refused(self):
        packet = self.write_packet({
            "schema": "x3-gate-failure-packet-v1",
            "commit": "0" * 40,
            "replay_command": "true",
        })
        result = self.run_tool("--good", "master~4", "--packet", packet)
        self.assertEqual(result.returncode, 1)
        self.assertIn("does not resolve", result.stderr)

    def test_a_hung_reproducer_aborts_instead_of_guessing(self):
        # A dedicated three-commit range so "hang" is the only commit between
        # the ends and bisect is guaranteed to probe it. The reproducer then
        # sleeps past --timeout and the wrapper must map that to 127 (git's
        # "abort"), which makes the tool exit 1 rather than name a first-bad
        # commit.
        run_git(self.repo, "checkout", "-q", "-b", "hang-range")
        first = commit_file(self.repo, "version.txt", "ok", "hang-good")
        hang = commit_file(self.repo, "version.txt", "hang", "hang")
        last = commit_file(self.repo, "version.txt", "broken", "hang-bad")
        start_head = run_git(self.repo, "rev-parse", "HEAD")
        repro = [
            sys.executable, "-c",
            "import sys, time\n"
            "v = open('version.txt').read().strip()\n"
            "time.sleep(5) if v == 'hang' else sys.exit(0 if v != 'broken' else 1)\n",
        ]
        result = self.run_tool(
            "--good", first, "--bad", last, "--timeout", "0.5", "--json", "--", *repro
        )
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("did not finish", result.stderr)
        self.assertEqual(run_git(self.repo, "rev-parse", "HEAD"), start_head)
        self.assertEqual(run_git(self.repo, "symbolic-ref", "HEAD"), "refs/heads/hang-range")
        self.assertNotEqual(run_git(self.repo, "rev-parse", "HEAD"), hang)

    # ── packet rendering ─────────────────────────────────────────────────

    def test_out_directory_receives_json_and_markdown(self):
        out = pathlib.Path(self.tmp.name) / "out"
        result = self.run_tool(
            "--good", "master~4", "--bad", "HEAD", "--out", str(out), "--", *REPRO
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        slug = self.bad_sha[:12]
        packet = json.loads((out / f"bisect-{slug}.json").read_text())
        self.assertEqual(packet["first_bad_commit"], self.bad_sha)
        markdown = (out / f"bisect-{slug}.md").read_text()
        self.assertIn("First bad commit", markdown)
        self.assertIn("version.txt", markdown)


if __name__ == "__main__":
    unittest.main()
