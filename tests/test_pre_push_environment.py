"""A push gate must not export the sending repository into test repositories."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PushEnvironmentTests(unittest.TestCase):
    def test_foreign_repository_keeps_its_own_head_and_index(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sending = root / "sending"
            sending.mkdir()
            env = {key: value for key, value in os.environ.items()
                   if not key.startswith("GIT_")}
            def git(*args):
                return subprocess.run(["git", "-C", str(sending),
                    "-c", "core.hooksPath=/dev/null", *args], env=env,
                    check=True, capture_output=True, text=True)
            git("init", "-q")
            (sending / "kept.txt").write_text("preserve this repository\n")
            git("add", "kept.txt")
            git("-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                "commit", "-qm", "initial")
            head = git("rev-parse", "HEAD").stdout.strip()
            scripts = sending / "scripts"
            scripts.mkdir()
            # This is the gate's foreign-repository operation, without a build.
            (scripts / "local-ci.sh").write_text(
                "#!/usr/bin/env bash\nset -euo pipefail\n"
                'git init -q "$FOREIGN_REPO"\n'
                'git -C "$FOREIGN_REPO" -c core.hooksPath=/dev/null '
                '-c user.name=Test -c user.email=test@example.invalid '
                'commit --allow-empty -qm fixture\n')
            result = subprocess.run(["bash", str(ROOT / ".githooks/pre-push")],
                cwd=sending, env={**env, "GIT_DIR": str(sending / ".git"),
                    "GIT_WORK_TREE": str(sending),
                    "GIT_INDEX_FILE": str(sending / ".git/index"),
                    "FOREIGN_REPO": str(root / "foreign")},
                input=f"refs/heads/topic {head} refs/heads/topic {'0' * 40}\n",
                capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(git("rev-parse", "HEAD").stdout.strip(), head)
            self.assertEqual(git("ls-files").stdout.splitlines(), ["kept.txt"])
            foreign = subprocess.run(["git", "-C", str(root / "foreign"),
                "rev-parse", "HEAD"], env=env, capture_output=True, text=True)
            self.assertEqual(foreign.returncode, 0, foreign.stderr)
            self.assertNotEqual(foreign.stdout.strip(), head)


if __name__ == "__main__":
    unittest.main()
