import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SUPERVISOR = Path(__file__).parents[1] / "scripts_infrastructure" / "pr_supervisor.py"


def git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


class PrSupervisorTests(unittest.TestCase):
    def test_removed_secret_like_text_does_not_fail_scan(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            git(repo, "init", "-b", "master")
            git(repo, "config", "user.email", "ci@example.invalid")
            git(repo, "config", "user.name", "CI Test")
            tracked = repo / "config.txt"
            tracked.write_text('API_KEY="this-was-an-old-placeholder-value"\n')
            git(repo, "add", "config.txt")
            git(repo, "commit", "-m", "baseline")
            git(repo, "branch", "origin/master")
            tracked.write_text("credential removed\n")
            git(repo, "add", "config.txt")
            git(repo, "commit", "-m", "remove placeholder")

            result = subprocess.run(
                [sys.executable, str(SUPERVISOR), "--base", "origin/master"],
                cwd=repo,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )

            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
